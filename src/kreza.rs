use crate::brain::{Brain, TaskType, ThoughtRequest};
use crate::protocol::{MutationProposal, Verdict, Evaluation, AgentRole, EvolutionContext};
use crate::db::{Phenotype, Fitness};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct KrezaResponse {
    verdict: Option<String>,
    score: Option<f32>,
    metrics: Option<Vec<String>>,
    reason: Option<String>,
}

pub struct Kreza;

impl Kreza {
    pub fn evaluate(
        brain: &mut Brain,
        proposal: &MutationProposal,
        ctx: &EvolutionContext,
        phenotype: Option<&Phenotype>,
        errors: &[String],
    ) -> Evaluation {
        let (realistic_score, realistic_verdict, delta) = Self::realistic_assessment(proposal, ctx, phenotype, errors);

        if realistic_score > 0.8 && matches!(realistic_verdict, Verdict::Approve) {
            return Evaluation {
                verdict: realistic_verdict,
                score: realistic_score,
                metrics: vec!["realistic_high_confidence".to_string()],
                fitness_delta: delta,
            };
        }
        if realistic_score < 0.2 && matches!(realistic_verdict, Verdict::Reject { .. }) {
            return Evaluation {
                verdict: realistic_verdict,
                score: realistic_score,
                metrics: vec!["realistic_low_confidence".to_string()],
                fitness_delta: delta,
            };
        }

        let llm_eval = Self::llm_evaluation(brain, proposal, ctx, phenotype, errors, realistic_score, delta);
        let final_score = (realistic_score + llm_eval.score) / 2.0;
        let verdict = llm_eval.verdict;
        let mut metrics = llm_eval.metrics;
        metrics.push(format!("realistic_score: {:.2}", realistic_score));

        Evaluation {
            verdict,
            score: final_score,
            metrics,
            fitness_delta: delta,
        }
    }

    fn realistic_assessment(
        proposal: &MutationProposal,
        ctx: &EvolutionContext,
        phenotype: Option<&Phenotype>,
        errors: &[String],
    ) -> (f32, Verdict, Option<f64>) {
        let sug = &proposal.suggestion;
        let new_fitness = phenotype.map(|p| {
            Fitness {
                performance: if p.error_rate == 0.0 { 0.9 } else { 0.5 },
                memory: 0.8,
                reliability: if p.error_rate == 0.0 { 1.0 } else { 0.0 },
                maintainability: 0.7,
            }
        });
        let new_overall = new_fitness.as_ref().map(|f| f.overall());
        let current_overall = ctx.current_genome.as_ref().map(|g| g.fitness.overall()).unwrap_or(0.0);
        let delta = new_overall.map(|new| new - current_overall);

        if !errors.is_empty() || phenotype.is_none() {
            return (0.0, Verdict::Reject { reason: format!("فشل التحقق: {:?}", errors) }, None);
        }

        let mut score = 0.5;
        if phenotype.is_some() { score += 0.3; }
        if let Some(d) = delta {
            if d > 0.1 { score += 0.3; }
            else if d > 0.0 { score += 0.15; }
            else if d < -0.1 { score -= 0.2; }
        }
        let risk_penalty = (proposal.risk * 0.3).min(0.3);
        score -= risk_penalty;
        score += (sug.confidence as f64 * 0.1) as f64;
        score = score.clamp(0.0, 1.0);

        let verdict = if score > 0.7 {
            Verdict::Approve
        } else if score > 0.4 {
            Verdict::NeedsExperiment { reason: "تحسن طفيف، يفضل مزيد من القياسات".into() }
        } else {
            Verdict::Reject { reason: "تأثير سلبي أو مخاطرة عالية".into() }
        };

        (score as f32, verdict, delta)
    }

    fn llm_evaluation(
        brain: &mut Brain,
        proposal: &MutationProposal,
        ctx: &EvolutionContext,
        phenotype: Option<&Phenotype>,
        errors: &[String],
        realistic_score: f32,
        delta: Option<f64>,
    ) -> Evaluation {
        let sug = &proposal.suggestion;
        let pheno_str = phenotype.map(|p| format!("بناء:{}ms, خطأ:{}", p.build_time_ms, p.error_rate)).unwrap_or_default();
        let delta_str = delta.map(|d| format!("{:.2}", d)).unwrap_or_else(|| "لا توجد".into());
        let error_str = errors.join(", ");

        let context_hint = format!(
            "الملف: {}, السبب: {}, الهدف: {}, الفرضية: {}, الثقة: {}, المخاطرة: {:.2}, نتيجة المختبر: {}, الفينوتيب: {}, تحسن اللياقة: {}, الأخطاء: {}",
            sug.file_path, sug.reason, sug.objective, proposal.hypothesis,
            sug.confidence, proposal.risk, if phenotype.is_some() { "نجح" } else { "فشل" },
            pheno_str, delta_str, error_str
        );

        let system_context = format!(
            "حالة النظام: الجيل {}، الصحة {:.2}، أضعف نقطة: {}",
            ctx.world_state.current_generation, ctx.world_state.health_score, ctx.self_assessment.weakest_point
        );

        let prompt = format!(
            "{}\n{}\nقم بتقييم الطفرة أعلاه. يجب أن تُرجع JSON بالشكل: {{\"verdict\":\"approve|reject|needs_more_research|needs_experiment|rollback\",\"score\":0.0-1.0,\"metrics\":[\"سبب1\",\"سبب2\"],\"reason\":\"...\"}}. النتيجة الواقعية الأولية كانت {:.2}.",
            system_context, context_hint, realistic_score
        );

        let request = ThoughtRequest {
            task_type: TaskType::EvaluateMutation,
            goal: "تقييم طفرة".into(),
            context: ctx.clone(),
            constraints: vec!["JSON فقط".into()],
            agent: AgentRole::Kreza,
            language_hint: Some(sug.language.clone()),
        };

        let response = match brain.think(request) {
            Ok(r) => r,
            Err(_) => return Evaluation {
                verdict: Verdict::Reject { reason: "فشل النموذج".into() },
                score: 0.0,
                metrics: vec![],
                fitness_delta: None,
            },
        };

        match serde_json::from_str::<KrezaResponse>(&response) {
            Ok(kr) => {
                let verdict = match kr.verdict.as_deref() {
                    Some("approve") => Verdict::Approve,
                    Some("reject") => Verdict::Reject { reason: kr.reason.unwrap_or_default() },
                    Some("needs_more_research") => Verdict::NeedsMoreResearch { reason: kr.reason.unwrap_or_default() },
                    Some("needs_experiment") => Verdict::NeedsExperiment { reason: kr.reason.unwrap_or_default() },
                    Some("rollback") => Verdict::Rollback { reason: kr.reason.unwrap_or_default() },
                    _ => Verdict::Reject { reason: "تنسيق غير معروف".into() },
                };
                let score = kr.score.unwrap_or(0.5);
                let metrics = kr.metrics.unwrap_or_default();
                Evaluation {
                    verdict,
                    score,
                    metrics,
                    fitness_delta: delta,
                }
            },
            Err(e) => Evaluation {
                verdict: Verdict::Reject { reason: format!("JSON غير صالح: {}", e) },
                score: 0.0,
                metrics: vec![],
                fitness_delta: None,
            },
        }
    }
}
