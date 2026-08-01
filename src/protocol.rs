use serde::{Deserialize, Serialize};
use crate::db::{Fitness, Phenotype};
use crate::genome::{GenomeNode, GenomeStatus, KnowledgeReference};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    Crazy,
    Kreza,
    Researcher,
    Tester,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub agent: AgentRole,
    pub generation: u64,
    pub file_path: String,
    pub language: String,          // مثل "rust", "python", "javascript"
    pub original_snippet: String,
    pub new_snippet: String,
    pub reason: String,
    pub objective: String,
    pub confidence: f32,
    pub priority: f32,
    pub risk: f32,
    pub expected_gain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationProposal {
    pub suggestion: Suggestion,
    pub hypothesis: String,
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
pub struct Evaluation {
    pub verdict: Verdict,
    pub score: f32,
    pub metrics: Vec<String>,
    pub fitness_delta: Option<f64>,
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
pub struct EvolutionContext {
    pub world_state: WorldState,
    pub current_genome: Option<GenomeNode>,
    pub goals: Vec<String>,
    pub recent_experiments: Vec<ExperimentRecord>,
    pub knowledge_base: Vec<KnowledgeItem>,
    pub resource_state: ResourceState,
    pub active_tasks: ActiveTasks,
    pub self_assessment: SelfAssessment,
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
        }
    }
}
