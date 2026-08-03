use crate::brain::Brain;
use crate::protocol::{MutationProposal, Verdict, Evaluation, EvolutionContext, CouncilVote};
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
                council_votes: vec![],  // <-- أضفنا هذا الحقل
                metrics: vec!["realistic_high_confidence".to_string()],
                fitness_delta: delta,
            };
        }
        if realistic_score < 0.2 && matches!(realistic_verdict, Verdict::Reject { .. }) {
            return Evaluation {
                verdict: realistic_verdict,
                score: realistic_score,
                council_votes: vec![],
                metrics: vec!["realistic_low_confidence".to_string()],
                fitness_delta: delta,
            };
        }

        let default_model = self.config.models.get("default").cloned().unwrap_or_else(|| "qwen2.5-coder:7b".into());
        let llm_eval = self.llm_evaluation_with_model(brain, proposal, ctx, phenotype, errors, realistic_score, delta, &default_model);
        let final_score = (realistic_score + llm_eval.score) / 2.0;
        let verdict = llm_eval.verdict;
        let mut metrics = llm_eval.metrics;
        metrics.push(format!("realistic_score: {:.2}", realistic_score));

        Evaluation {
            verdict,
            score: final_score,
            council_votes: llm_eval.council_votes,  // <-- نأخذ الأصوات من التقييم اللغوي
            metrics,
            fitness_delta: delta,
        }
    }

    // ... (باقي التوابع مثل realistic_assessment و llm_evaluation_with_model تظل كما هي، لكن يجب أن تُعيد Evaluation تحتوي على council_votes)
}
