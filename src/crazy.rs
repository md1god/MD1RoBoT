use crate::brain::{Brain, TaskType, ThoughtRequest};
use crate::protocol::{MutationProposal, Suggestion, AgentRole, EvolutionContext};
use serde_json;

pub struct Crazy;

impl Crazy {
    pub fn propose_mutations(
        brain: &mut Brain,
        ctx: &EvolutionContext,
        max_mutations: usize,
    ) -> Result<Vec<MutationProposal>, String> {
        let failed = ctx.recent_experiments.iter()
            .filter(|e| e.verdict == "REJECTED")
            .map(|e| e.experiment_id.clone())
            .collect::<Vec<_>>()
            .join(", ");

        let constraints = vec![
            format!("أقصى عدد {} اقتراحات", max_mutations),
            "JSON array بالشكل: [{suggestion:{...}, hypothesis, expected_fitness_gain, risk}]".to_string(),
            format!("تجنب الأفكار السابقة الفاشلة: {}", failed),
        ];

        let request = ThoughtRequest {
            task_type: TaskType::GenerateMutation,
            goal: "توليد طفرات تطورية ذات مكسب متوقع".to_string(),
            context: ctx.clone(),
            constraints,
            agent: AgentRole::Crazy,
            language_hint: None, // سيتم تحديدها من السياق لاحقًا
        };

        let response = brain.think(request)?;
        let start = response.find('[').ok_or("لا يوجد JSON array")?;
        let end = response.rfind(']').ok_or("JSON array غير مكتمل")?;
        let json_str = &response[start..=end];
        let proposals: Vec<MutationProposal> = serde_json::from_str(json_str)
            .map_err(|e| format!("فشل تحليل MutationProposals: {e}"))?;
        Ok(proposals)
    }
}
