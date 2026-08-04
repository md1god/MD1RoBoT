use serde::{Deserialize, Deserializer, Serialize};
use crate::db::{Fitness, Phenotype};
use crate::genome::GenomeNode;

/// يقبل هذا الحقل رقمًا (1) أو نصًا يمثل رقمًا ("1") من مخرجات النموذج،
/// ويحوّله دائمًا إلى u64. يحمي من أخطاء "invalid type: string, expected u64".
fn lenient_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrStr {
        Num(u64),
        Str(String),
    }
    match NumOrStr::deserialize(deserializer)? {
        NumOrStr::Num(n) => Ok(n),
        NumOrStr::Str(s) => s.trim().parse::<u64>().map_err(serde::de::Error::custom),
    }
}

/// يقبل هذا الحقل نصًا ("1") أو رقمًا (1) من مخرجات النموذج،
/// ويحوّله دائمًا إلى String. يحمي من أخطاء "invalid type: integer, expected a string".
fn lenient_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrNum {
        Str(String),
        Int(i64),
        Float(f64),
    }
    match StrOrNum::deserialize(deserializer)? {
        StrOrNum::Str(s) => Ok(s),
        StrOrNum::Int(n) => Ok(n.to_string()),
        StrOrNum::Float(f) => Ok(f.to_string()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentRole {
    Crazy,
    Kreza,
    Researcher,
    Tester,
    Architect,
    Coder,
    Reviewer,
}

impl AgentRole {
    pub fn agent_role(&self) -> String {
        match self {
            AgentRole::Crazy => "Crazy".to_string(),
            AgentRole::Kreza => "Kreza".to_string(),
            AgentRole::Researcher => "Researcher".to_string(),
            AgentRole::Tester => "Tester".to_string(),
            AgentRole::Architect => "Architect".to_string(),
            AgentRole::Coder => "Coder".to_string(),
            AgentRole::Reviewer => "Reviewer".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    #[serde(deserialize_with = "lenient_string")]
    pub id: String,
    pub agent: AgentRole,
    #[serde(deserialize_with = "lenient_u64")]
    pub generation: u64,
    #[serde(deserialize_with = "lenient_string")]
    pub file_path: String,
    #[serde(deserialize_with = "lenient_string")]
    pub language: String,
    #[serde(deserialize_with = "lenient_string")]
    pub original_snippet: String,
    #[serde(deserialize_with = "lenient_string")]
    pub new_snippet: String,
    #[serde(deserialize_with = "lenient_string")]
    pub reason: String,
    #[serde(deserialize_with = "lenient_string")]
    pub objective: String,
    pub confidence: f32,
    pub priority: f32,
    pub risk: f32,
    #[serde(deserialize_with = "lenient_string")]
    pub expected_gain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    #[serde(deserialize_with = "lenient_string")]
    pub id: String,
    #[serde(deserialize_with = "lenient_string")]
    pub statement: String,
    pub context_tags: Vec<String>,
    pub confidence: f32,
    #[serde(deserialize_with = "lenient_u64")]
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationProposal {
    pub suggestion: Suggestion,
    pub hypothesis: Hypothesis,
    pub expected_fitness_gain: f64,
    pub risk: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Verdict {
    Approve,
    Reject { reason: String },
    Modify { suggestion: String },
    NeedsMoreResearch { reason: String },
    NeedsExperiment { reason: String },
    Rollback { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouncilVote {
    pub agent: AgentRole,
    pub benefit: f64,
    pub novelty: f64,
    pub risk: f64,
    pub cost: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub verdict: Verdict,
    pub score: f32,
    pub council_votes: Vec<CouncilVote>,
    pub metrics: Vec<String>,
    pub fitness_delta: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionGene {
    pub id: String,
    pub description: String,
    pub language: String,
    pub applicability_tags: Vec<String>,
    pub benefits: Vec<String>,
    pub risks: Vec<String>,
    pub evidence_count: u32,
    pub success_rate: f64,
    pub last_used_generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theory {
    pub id: String,
    pub statement: String,
    pub hypotheses: Vec<String>,
    pub confidence: f64,
    pub evidence_experiments: u32,
    pub applicable_languages: Vec<String>,
    pub related_genes: Vec<String>,
    pub created_generation: u64,
    pub last_validated_generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: String,
    pub topic: String,
    pub summary: String,
    pub source_type: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentRecord {
    pub experiment_id: String,
    pub generation: u64,
    pub file_path: String,
    pub verdict: String,
    pub fitness: Option<Fitness>,
    pub phenotype: Option<Phenotype>,
    pub error_hash: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub total_files: usize,
    pub modules: usize,
    pub lines_of_code: usize,
    pub test_coverage: f64,
    pub current_generation: u64,
    pub active_branch: String,
    pub health_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceState {
    pub cpu_usage_percent: f64,
    pub memory_available_mb: u64,
    pub disk_free_gb: u64,
    pub network_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTasks {
    pub searching: bool,
    pub evolving: bool,
    pub testing: bool,
    pub waiting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfAssessment {
    pub weakest_point: String,
    pub improvement_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionContext {
    pub world_state: WorldState,
    pub current_genome: Option<GenomeNode>,
    pub goals: Vec<String>,
    pub recent_experiments: Vec<ExperimentRecord>,
    pub knowledge_base: Vec<KnowledgeItem>,
    pub resource_state: ResourceState,
    pub active_tasks: ActiveTasks,
    pub self_assessment: SelfAssessment,
    pub active_theories: Vec<Theory>,
    pub active_genes: Vec<DecisionGene>,
    pub current_hypothesis: Option<Hypothesis>,
}

impl EvolutionContext {
    pub fn minimal() -> Self {
        EvolutionContext {
            world_state: WorldState {
                total_files: 0,
                modules: 0,
                lines_of_code: 0,
                test_coverage: 0.0,
                current_generation: 0,
                active_branch: "main".into(),
                health_score: 1.0,
            },
            current_genome: None,
            goals: vec![],
            recent_experiments: vec![],
            knowledge_base: vec![],
            resource_state: ResourceState {
                cpu_usage_percent: 0.0,
                memory_available_mb: 0,
                disk_free_gb: 0,
                network_connected: true,
            },
            active_tasks: ActiveTasks {
                searching: false,
                evolving: false,
                testing: false,
                waiting: false,
            },
            self_assessment: SelfAssessment {
                weakest_point: "غير معروف".into(),
                improvement_score: 0.0,
            },
            active_theories: vec![],
            active_genes: vec![],
            current_hypothesis: None,
        }
    }
}

impl Evaluation {
    pub fn compute_weighted_score(votes: &[CouncilVote]) -> f32 {
        if votes.is_empty() {
            return 0.0;
        }
        let n = votes.len() as f64;
        let avg_benefit = votes.iter().map(|v| v.benefit).sum::<f64>() / n;
        let avg_novelty = votes.iter().map(|v| v.novelty).sum::<f64>() / n;
        let avg_risk = votes.iter().map(|v| v.risk).sum::<f64>() / n;
        let avg_cost = votes.iter().map(|v| v.cost).sum::<f64>() / n;
        let avg_confidence = votes.iter().map(|v| v.confidence).sum::<f64>() / n;
        let raw = 0.4 * avg_benefit + 0.2 * avg_novelty - 0.2 * avg_risk - 0.2 * avg_cost;
        (raw * avg_confidence) as f32
    }
}
