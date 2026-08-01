use crate::brain::{Brain, TaskType, ThoughtRequest};
use crate::protocol::{MutationProposal, AgentRole, EvolutionContext};
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
            format!("Maximum {} proposals", max_mutations),
            "JSON array format: [{suggestion:{...}, hypothesis, expected_fitness_gain, risk}]".to_string(),
            format!("Avoid previously failed ideas: {}", failed),
        ];

        let request = ThoughtRequest {
            task_type: TaskType::GenerateMutation,
            goal: "Generate evolutionary mutations with expected gain".to_string(),
            context: ctx.clone(),
            constraints,
            agent: AgentRole::Crazy,
            language_hint: None,
        };

        let response = brain.think(request)?;
        let start = response.find('[').ok_or("No JSON array found")?;
        let end = response.rfind(']').ok_or("JSON array incomplete")?;
        let json_str = &response[start..=end];
        let proposals: Vec<MutationProposal> = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse MutationProposals: {e}"))?;
        Ok(proposals)
    }
}
