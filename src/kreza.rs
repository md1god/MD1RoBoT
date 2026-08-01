use crate::brain::Brain;
use crate::protocol::{MutationProposal, Verdict, Evaluation, EvolutionContext};
use crate::db::{Phenotype, Fitness};
use crate::config_loader::AppConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct KrezaResponse {
    verdict: Option<String>,
    score: Option<f32>,
    metrics: Option<Vec<String>>,
    reason: Option<String>,
}

pub struct Kreza {
    config: AppConfig,
}

impl Kreza {
    pub fn new(config: AppConfig) -> Self {
        Kreza { config }
    }

    pub fn evaluate(
        &self,
        brain: &mut Brain,
        proposal: &MutationProposal,
        ctx: &EvolutionContext,
        phenotype: Option<&Phenotype>,
        errors: &[String],
    ) -> Evaluation {
        let (realistic_score, realistic_verdict, delta) = self.realistic_assessment(proposal, ctx, phenotype, errors);

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

        if self.config.kreza_use_ensemble && !self.config.kreza_ensemble_models.is_empty() {
            let mut total_score = 0.0f32;
            let mut all_verdicts = Vec::new();
            for model in &self.config.kreza_ensemble_models {
                let eval = self.llm_evaluation_with_model(brain, proposal, ctx, phenotype, errors, realistic_score, delta, model);
                total_score += eval.score;
                all_verdicts.push(eval.verdict);
            }
            let avg_score = total_score / self.config.kreza_ensemble_models.len() as f32;
            let verdict = if all_verdicts.iter().any(|v| matches!(v, Verdict::Approve)) {
                Verdict::Approve
            } else {
                all_verdicts.into_iter().next().unwrap_or(Verdict::Reject { reason: "ensemble failed".into() })
            };
            return Evaluation {
                verdict,
                score: avg_score,
                metrics: vec!["ensemble".to_string()],
                fitness_delta: delta,
            };
        } else {
            let default_model = self.config.models.get("default").cloned().unwrap_or_else(|| "qwen2.5-coder:7b".into());
            let llm_eval = self.llm_evaluation_with_model(brain, proposal, ctx, phenotype, errors, realistic_score, delta, &default_model);
            let final_score = (realistic_score + llm_eval.score) / 2.0;
            let verdict = llm_eval.verdict;
            let mut metrics = llm_eval.metrics;
            metrics.push(format!("realistic_score: {:.2}", realistic_score));
            return Evaluation {
                verdict,
                score: final_score,
                metrics,
                fitness_delta: delta,
            };
        }
    }

    fn realistic_assessment(
        &self,
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
            return (0.0, Verdict::Reject { reason: format!("Build/test failure: {:?}", errors) }, None);
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
            Verdict::NeedsExperiment { reason: "Marginal improvement, needs more measurements".into() }
        } else {
            Verdict::Reject { reason: "Negative impact or high risk".into() }
        };

        (score as f32, verdict, delta)
    }

    fn llm_evaluation_with_model(
        &self,
        _brain: &mut Brain,
        proposal: &MutationProposal,
        ctx: &EvolutionContext,
        phenotype: Option<&Phenotype>,
        errors: &[String],
        realistic_score: f32,
        delta: Option<f64>,
        model_name: &str,
    ) -> Evaluation {
        let sug = &proposal.suggestion;
        let pheno_str = phenotype.map(|p| format!("build:{}ms, error_rate:{}", p.build_time_ms, p.error_rate)).unwrap_or_default();
        let delta_str = delta.map(|d| format!("{:.2}", d)).unwrap_or_else(|| "N/A".into());
        let error_str = errors.join(", ");

        let context_hint = format!(
            "File: {}, Reason: {}, Objective: {}, Hypothesis: {}, Confidence: {}, Risk: {:.2}, Lab result: {}, Phenotype: {}, Fitness delta: {}, Errors: {}",
            sug.file_path, sug.reason, sug.objective, proposal.hypothesis,
            sug.confidence, proposal.risk, if phenotype.is_some() { "passed" } else { "failed" },
            pheno_str, delta_str, error_str
        );

        let system_context = format!(
            "System state: generation {}, health {:.2}, weakest point: {}",
            ctx.world_state.current_generation, ctx.world_state.health_score, ctx.self_assessment.weakest_point
        );

        let prompt = format!(
            "{}\n{}\nEvaluate the above mutation. Return JSON with keys: verdict (approve/reject/needs_more_research/needs_experiment/rollback), score (0.0-1.0), metrics (list of strings), reason (string). Realistic score was {:.2}.",
            system_context, context_hint, realistic_score
        );

        use crate::ollama_client::OllamaClient;
        let client = OllamaClient::new(Some(model_name));
        let response = match client.generate(&prompt) {
            Ok(r) => r,
            Err(_) => return Evaluation {
                verdict: Verdict::Reject { reason: "Model evaluation failed".into() },
                score: 0.0,
                metrics: vec![],
                fitness_delta: delta,
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
                    _ => Verdict::Reject { reason: "Unknown verdict".into() },
                };
                let score = kr.score.unwrap_or(0.5);
                let metrics = kr.metrics.unwrap_or_default();
                Evaluation { verdict, score, metrics, fitness_delta: delta }
            },
            Err(e) => Evaluation {
                verdict: Verdict::Reject { reason: format!("Invalid JSON: {}", e) },
                score: 0.0,
                metrics: vec![],
                fitness_delta: delta,
            },
        }
    }
}
