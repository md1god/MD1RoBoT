use crate::db::Db;
use crate::db::Fitness;
use crate::db::Phenotype;
use sha2::{Sha256, Digest};
use std::fs;
use std::path::Path;

#[derive(Clone)]
pub struct EvolutionController {
    db: Db,
    lock_file: String,
}

impl EvolutionController {
    pub fn new(db: Db, lock_file: &str) -> Result<Self, String> {
        Ok(EvolutionController { db, lock_file: lock_file.to_string() })
    }

    pub fn acquire_lock(&self) -> bool {
        let lock = Path::new(&self.lock_file);
        if lock.exists() {
            if let Ok(content) = fs::read_to_string(lock) {
                if let Ok(info) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(_pid) = info["pid"].as_u64() {
                        #[cfg(unix)]
                        {
                            if unsafe { libc::kill(_pid as i32, 0) == 0 } { return false; }
                        }
                    }
                }
            }
            let _ = fs::remove_file(lock);
        }
        fs::write(lock, serde_json::json!({"pid": std::process::id()}).to_string()).is_ok()
    }

    pub fn release_lock(&self) {
        let _ = fs::remove_file(&self.lock_file);
    }

    pub fn current_generation(&self) -> u64 {
        self.db.get_evolution_state().map(|(g, _, _)| g).unwrap_or(0)
    }

    pub fn increment_generation(&self) -> Result<u64, String> {
        let gen = self.current_generation() + 1;
        self.db.update_evolution_state(gen, 0.0, 0).map_err(|e| e.to_string())?;
        Ok(gen)
    }

    pub fn update_best_fitness(&self, generation: u64, fitness: f64) -> Result<(), String> {
        let (_, current_best, _) = self.db.get_evolution_state().map_err(|e| e.to_string())?;
        if fitness > current_best {
            self.db.update_evolution_state(self.current_generation(), fitness, generation).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn hash_mutation(file_path: &str, original: &str, new: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(file_path.as_bytes());
        hasher.update(original.as_bytes());
        hasher.update(new.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn record_success(
        &self,
        experiment_id: &str,
        generation: u64,
        file_path: &str,
        reason: &str,
        objective: &str,
        confidence: f32,
        fitness: &Fitness,
        phenotype: &Phenotype,
        error_hash: &str,
    ) -> Result<(), String> {
        self.db.record_experiment(experiment_id, generation, file_path, reason, objective, confidence, "MERGED", Some(fitness), Some(phenotype), 0, error_hash, &[]).map_err(|e| e.to_string())?;
        self.db.clear_rejection(error_hash).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn record_rejection(
        &self,
        experiment_id: &str,
        generation: u64,
        file_path: &str,
        reason: &str,
        objective: &str,
        confidence: f32,
        error_hash: &str,
        errors: &[String],
    ) -> Result<(), String> {
        self.db.record_experiment(experiment_id, generation, file_path, reason, objective, confidence, "REJECTED", None, None, 0, error_hash, errors).map_err(|e| e.to_string())?;
        self.db.increment_rejection(error_hash).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn check_oscillation(&self, error_hash: &str) -> (u32, bool) {
        self.db.check_oscillation(error_hash)
    }
}
