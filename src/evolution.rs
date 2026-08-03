use crate::db::{Db, Fitness, Phenotype};
use sha2::Digest;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct EvolutionController {
    db: Db,
    generation: u64,
    lock_file: String,
}

impl EvolutionController {
    pub fn new(db: Db, lock_file: &str) -> Result<Self, String> {
        let (gen, _, _) = db.get_evolution_state().unwrap_or((0, 0.0, 0));
        Ok(EvolutionController {
            db,
            generation: gen,
            lock_file: lock_file.to_string(),
        })
    }

    pub fn current_generation(&self) -> u64 {
        self.generation
    }

    pub fn increment_generation(&mut self) -> Result<u64, String> {
        self.generation += 1;
        self.db.update_evolution_state(self.generation, 0.0, 0).map_err(|e| e.to_string())?;
        Ok(self.generation)
    }

    pub fn update_best_fitness(&self, gen: u64, fitness: f64) -> Result<(), String> {
        self.db.update_evolution_state(gen, fitness, gen).map_err(|e| e.to_string())
    }

    pub fn hash_mutation(file: &str, old: &str, new: &str) -> String {
        let mut hasher = sha2::Sha256::new();
        hasher.update(file);
        hasher.update(old);
        hasher.update(new);
        hex::encode(hasher.finalize())
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
        self.db.increment_rejection(error_hash).map_err(|e| e.to_string())?;
        self.db.record_experiment(
            experiment_id,
            generation,
            file_path,
            reason,
            objective,
            confidence,
            "REJECTED",
            None,
            None,
            0,
            error_hash,
            errors,
            None,  // hypothesis_id
            None,  // theory_id
        ).map_err(|e| e.to_string())?;
        Ok(())
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
        self.db.clear_rejection(error_hash).map_err(|e| e.to_string())?;
        self.db.record_experiment(
            experiment_id,
            generation,
            file_path,
            reason,
            objective,
            confidence,
            "MERGED",
            Some(fitness),
            Some(phenotype),
            0,
            error_hash,
            &[],
            None,  // hypothesis_id
            None,  // theory_id
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn check_oscillation(&self, error_hash: &str) -> (u32, bool) {
        self.db.check_oscillation(error_hash)
    }
}
