use crate::brain::Brain;
use crate::db::{Db, Fitness};
use crate::goal_manager::GoalManager;
use crate::evolution::EvolutionController;
use crate::context_builder::ContextBuilder;
use crate::crazy::Crazy;
use crate::kreza::Kreza;
use crate::evolution_lab::EvolutionLab;
use crate::protocol::{Verdict, Evaluation, Suggestion};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct Planner {
    brain: Brain,
    db: Db,
    goal_manager: GoalManager,
    pub evo: EvolutionController,
}

impl Planner {
    pub fn new(brain: Brain, db: Db, goal_manager: GoalManager, evo: EvolutionController) -> Self {
        Planner { brain, db, goal_manager, evo }
    }

    pub fn run_cycle(&mut self) -> Result<(), String> {
        let ctx_builder = ContextBuilder::new(self.db.clone(), self.goal_manager.clone());
        let ctx = ctx_builder.build();

        let default_lang = ctx.current_genome.as_ref()
            .and_then(|g| g.files_changed.first().map(|f| detect_language(f)))
            .unwrap_or_else(|| "rust".to_string());

        let proposals = Crazy::propose_mutations(&mut self.brain, &ctx, 5)?;

        let mut best_score = 0.0;
        let mut best_plan: Option<(Suggestion, Evaluation, crate::db::Phenotype)> = None;

        for prop in proposals {
            let sug = &prop.suggestion;
            let (lab_ok, pheno_opt, errors) = EvolutionLab::run_experiment(".", sug)?;
            let evaluation = Kreza::evaluate(&mut self.brain, &prop, &ctx, pheno_opt.as_ref(), &errors);
            let error_hash = EvolutionController::hash_mutation(&sug.file_path, &sug.original_snippet, &sug.new_snippet);

            match &evaluation.verdict {
                Verdict::Approve => {
                    if evaluation.score > best_score {
                        if let Some(pheno) = pheno_opt {
                            best_score = evaluation.score;
                            best_plan = Some((sug.clone(), evaluation, pheno));
                        }
                    }
                }
                Verdict::Reject { reason } => {
                    self.evo.record_rejection(
                        &sug.id, self.evo.current_generation(), &sug.file_path,
                        &sug.reason, &sug.objective, sug.confidence,
                        &error_hash, &errors,
                    )?;
                    let (_, oscillating) = self.evo.check_oscillation(&error_hash);
                    if oscillating {
                        println!("Oscillation detected: {}. Consider changing the objective.", reason);
                    }
                }
                Verdict::Modify { suggestion } => {
                    println!("Modification suggested by Kreza: {}", suggestion);
                    self.evo.record_rejection(&sug.id, self.evo.current_generation(), &sug.file_path, &sug.reason, &sug.objective, sug.confidence, &error_hash, &errors)?;
                }
                Verdict::NeedsMoreResearch { reason } => {
                    println!("Kreza requests more research: {}", reason);
                }
                Verdict::NeedsExperiment { reason } => {
                    println!("Kreza requests additional experiments: {}", reason);
                }
                Verdict::Rollback { reason } => {
                    println!("Kreza requests rollback: {}", reason);
                }
            }
        }

        if let Some((sug, eval, pheno)) = best_plan {
            let target_file = &sug.file_path;
            let original_content = std::fs::read_to_string(target_file)
                .map_err(|e| format!("Failed to read original file: {e}"))?;
            let new_content = original_content.replace(&sug.original_snippet, &sug.new_snippet);
            let backup = format!("{}.bak", target_file);
            std::fs::copy(target_file, &backup).ok();
            std::fs::write(target_file, new_content).map_err(|e| format!("Failed to write mutation: {e}"))?;

            let new_gen = self.evo.increment_generation()?;
            let fitness = Fitness {
                performance: if pheno.error_rate == 0.0 { 0.9 } else { 0.5 },
                memory: 0.8,
                reliability: if pheno.error_rate == 0.0 { 1.0 } else { 0.0 },
                maintainability: 0.7,
            };
            self.evo.update_best_fitness(new_gen, fitness.overall())?;

            let error_hash = EvolutionController::hash_mutation(&sug.file_path, &sug.original_snippet, &sug.new_snippet);
            self.evo.record_success(
                &sug.id, new_gen, &sug.file_path,
                &sug.reason, &sug.objective, sug.confidence,
                &fitness, &pheno, &error_hash,
            )?;

            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let genome_id = Uuid::new_v4().to_string();
            let genome_hash = hex::encode(sha2::Sha256::digest(new_content.as_bytes()));
            self.db.insert_genome_node(
                &genome_id, &genome_hash, None, new_gen, &sug.objective,
                &[sug.file_path.clone()], &error_hash, "", &fitness, &pheno,
                &vec!["experiment".to_string()],
                now, "MERGED",
            ).map_err(|e| e.to_string())?;

            println!("Mutation merged. Generation: {}", new_gen);
        } else {
            println!("No acceptable mutation this cycle.");
        }

        Ok(())
    }
}

fn detect_language(file_path: &str) -> String {
    if file_path.ends_with(".rs") { "rust".into() }
    else if file_path.ends_with(".py") { "python".into() }
    else if file_path.ends_with(".js") || file_path.ends_with(".ts") { "javascript".into() }
    else if file_path.ends_with(".c") || file_path.ends_with(".h") { "c".into() }
    else if file_path.ends_with(".cpp") || file_path.ends_with(".hpp") { "cpp".into() }
    else { "unknown".into() }
}
