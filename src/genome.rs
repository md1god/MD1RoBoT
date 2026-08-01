use serde::{Deserialize, Serialize};
use crate::db::{Fitness, Phenotype};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenomeStatus {
    Active,
    Merged,
    Rejected,
    Experimental,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeType {
    Experiment,
    Document,
    Paper,
    Repository,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeReference {
    pub source_id: String,
    pub source_type: KnowledgeType,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomeNode {
    pub id: String,
    pub genome_hash: String,
    pub parent_id: Option<String>,
    pub generation: u32,
    pub objective: String,
    pub files_changed: Vec<String>,
    pub patch_hash: String,
    pub patch_path: String,
    pub fitness: Fitness,
    pub phenotype: Phenotype,
    pub knowledge_sources: Vec<KnowledgeReference>,
    pub created_at: u64,
    pub status: GenomeStatus,
}

impl Default for GenomeNode {
    fn default() -> Self {
        GenomeNode {
            id: "genesis".into(),
            genome_hash: String::new(),
            parent_id: None,
            generation: 0,
            objective: "init".into(),
            files_changed: vec![],
            patch_hash: String::new(),
            patch_path: String::new(),
            fitness: Fitness::default(),
            phenotype: Phenotype {
                search_speed_ms: 0,
                memory_usage_mb: 0,
                error_rate: 0.0,
                build_time_ms: 0,
            },
            knowledge_sources: vec![],
            created_at: 0,
            status: GenomeStatus::Active,
        }
    }
}
