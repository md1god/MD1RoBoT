use crate::brain::{Brain, TaskType, ThoughtRequest};
use crate::protocol::{MutationProposal, AgentRole, EvolutionContext, Hypothesis};
use serde_json;
use uuid::Uuid;

pub struct Crazy;

impl Crazy {
    /// توليد طفرات مع فرضيات، باستخدام نظريات من Theory Bank إن وجدت.
    pub fn propose_mutations(
        brain: &mut Brain,
        ctx: &EvolutionContext,
        max_mutations: usize,
    ) -> Result<Vec<MutationProposal>, String> {
        // 1. صياغة فرضيات مرشحة (Hypotheses) بناءً على السياق والنظريات النشطة
        let hypothesis_requests = self.build_hypothesis_requests(ctx, max_mutations);
        let mut hypotheses = Vec::new();
        for hyp_req in hypothesis_requests {
            let response = brain.think(hyp_req)?;
            // تنظيف الاستجابة: نأخذ أول سطر غير فارغ
            let statement = response.lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("No hypothesis generated")
                .to_string();
            let hyp = Hypothesis {
                id: Uuid::new_v4().to_string(),
                statement,
                context_tags: self.extract_tags(ctx),
                confidence: 0.5,
                generation: ctx.world_state.current_generation,
            };
            hypotheses.push(hyp);
        }

        // 2. بناء قائمة الأفكار الفاشلة سابقاً
        let failed_ids = ctx.recent_experiments.iter()
            .filter(|e| e.verdict == "REJECTED")
            .map(|e| e.experiment_id.clone())
            .collect::<Vec<_>>()
            .join(", ");

        // 3. تحضير نصوص النظريات النشطة لإرشاد Crazy
        let theory_hints = if ctx.active_theories.is_empty() {
            "No prior theories available.".to_string()
        } else {
            ctx.active_theories.iter()
                .map(|t| format!("Theory (conf={:.2}): {}", t.confidence, t.statement))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // 4. توليد الطفرات نفسها، مع إرفاق كل واحدة بفرضيتها
        let mut proposals = Vec::new();
        for hyp in &hypotheses {
            let constraints = vec![
                format!("Generate exactly 1 mutation proposal for this hypothesis: {}", hyp.statement),
                "Output format: JSON object with keys: suggestion (object), hypothesis (string), expected_fitness_gain (number), risk (number).".to_string(),
                format!("Avoid previously failed experiments: {}", failed_ids),
                format!("Relevant theories:\n{}", theory_hints),
                "The suggestion object must contain: id, agent (Crazy), generation, file_path, language, original_snippet, new_snippet, reason, objective, confidence (0.0-1.0), priority (0.0-1.0), risk (0.0-1.0), expected_gain (string).".to_string(),
                "Diversity: Use one of these styles: incremental improvement, structural refactoring, or architectural change.".to_string(),
            ];

            let request = ThoughtRequest {
                task_type: TaskType::GenerateMutation,
                goal: format!("Test hypothesis: {}", hyp.statement),
                context: ctx.clone(),
                constraints,
                agent: AgentRole::Crazy,
                language_hint: None,
            };

            let response = brain.think(request)?;
            // استخراج JSON واحد بدل مصفوفة
            let start = response.find('{').ok_or("No JSON object in response")?;
            let end = response.rfind('}').ok_or("JSON object incomplete")?;
            let json_str = &response[start..=end];
            let mut proposal: MutationProposal = serde_json::from_str(json_str)
                .map_err(|e| format!("Invalid MutationProposal: {e}"))?;
            // نربط الفرضية (نضمن استخدام الفرضية التي أنشأناها)
            proposal.hypothesis = hyp.clone();
            proposals.push(proposal);
        }

        Ok(proposals)
    }

    /// بناء طلبات لتوليد فرضيات متنوعة
    fn build_hypothesis_requests(&self, ctx: &EvolutionContext, count: usize) -> Vec<ThoughtRequest> {
        let base_prompt = format!(
            "You are a creative AI scientist. Based on the current system state (generation {}, health {:.2}), suggest a falsifiable hypothesis for improving the code. A hypothesis is a statement like 'Removing X will improve Y'.",
            ctx.world_state.current_generation, ctx.world_state.health_score
        );
        let mut requests = Vec::new();
        let styles = vec![
            "Focus on performance optimization.",
            "Focus on memory reduction.",
            "Focus on code clarity and maintainability.",
            "Focus on reducing technical debt.",
            "Propose a risky, high-reward hypothesis.",
        ];
        for i in 0..count {
            let style = styles[i % styles.len()];
            let goal = format!("{} {}", base_prompt, style);
            let req = ThoughtRequest {
                task_type: TaskType::ReasonAboutCode, // أقرب نوع للتفكير الحر
                goal,
                context: ctx.clone(),
                constraints: vec!["Reply with only the hypothesis statement, one sentence.".into()],
                agent: AgentRole::Crazy,
                language_hint: None,
            };
            requests.push(req);
        }
        requests
    }

    /// استخراج وسوم سياقية من السياق الحالي لاستخدامها في مطابقة النظريات
    fn extract_tags(&self, ctx: &EvolutionContext) -> Vec<String> {
        let mut tags = Vec::new();
        // نضيف أسماء الملفات التي تم التعديل عليها مؤخراً
        for exp in &ctx.recent_experiments {
            if let Some(ext) = exp.file_path.split('.').last() {
                tags.push(format!("lang:{}", ext));
            }
            tags.push(format!("file:{}", exp.file_path));
        }
        tags.push(format!("health:{}", ctx.world_state.health_score));
        tags.dedup();
        tags
    }
}
