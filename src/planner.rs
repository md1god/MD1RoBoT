use crate::brain::Brain;
use crate::db::{Db, Fitness};
use crate::goal_manager::GoalManager;
use crate::evolution::EvolutionController;
use crate::context_builder::ContextBuilder;
use crate::crazy::Crazy;
use crate::kreza::Kreza;
use crate::evolution_lab::EvolutionLab;
use crate::protocol::{Verdict, Evaluation, Suggestion, Hypothesis};
use crate::config_loader::AppConfig;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use sha2::Digest;

pub struct Planner {
    brain: Brain,
    db: Db,
    goal_manager: GoalManager,
    pub evo: EvolutionController,
    config: AppConfig,
}

impl Planner {
    pub fn new(brain: Brain, db: Db, goal_manager: GoalManager, evo: EvolutionController, config: AppConfig) -> Self {
        Planner { brain, db, goal_manager, evo, config }
    }

    pub fn run_cycle(&mut self) -> Result<(), String> {
        let ctx_builder = ContextBuilder::new(self.db.clone(), self.goal_manager.clone());
        let mut ctx = ctx_builder.build();

        // 1. توليد طفرات بفرضيات (Crazy الجديد يرجع مع كل طفرة فرضية)
        let proposals = Crazy::propose_mutations(&mut self.brain, &ctx, 5)?;

        let mut best_score = 0.0;
        let mut best_plan: Option<(Suggestion, Evaluation, crate::db::Phenotype, Hypothesis)> = None;

        let kreza = Kreza::new(self.config.clone());

        for prop in proposals {
            let sug = &prop.suggestion;
            let (_lab_ok, pheno_opt, errors) = EvolutionLab::run_experiment(".", sug)?;
            let evaluation = kreza.evaluate(&mut self.brain, &prop, &ctx, pheno_opt.as_ref(), &errors);
            let error_hash = EvolutionController::hash_mutation(&sug.file_path, &sug.original_snippet, &sug.new_snippet);

            // تسجيل التجربة في القاعدة (مع الفرضية)
            self.db.record_experiment(
                &sug.id,
                self.evo.current_generation(),
                &sug.file_path,
                &sug.reason,
                &sug.objective,
                sug.confidence,
                &format!("{:?}", evaluation.verdict), // نص مبسط
                None, // سنضيف phenotype لاحقاً
                pheno_opt.as_ref(),
                0,
                &error_hash,
                &errors,
                Some(&prop.hypothesis.id),
                None, // theory_id سيُحسب بعد قليل
            ).map_err(|e| e.to_string())?;

            match &evaluation.verdict {
                Verdict::Approve => {
                    if evaluation.score > best_score {
                        if let Some(pheno) = pheno_opt {
                            best_score = evaluation.score;
                            best_plan = Some((sug.clone(), evaluation, pheno, prop.hypothesis.clone()));
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
                _ => {
                    println!("Kreza verdict: {:?}", evaluation.verdict);
                }
            }
        }

        // 2. تطبيق أفضل طفرة ثم تحويل فرضيتها إلى نظرية (إن أمكن)
        if let Some((sug, _eval, pheno, hyp)) = best_plan {
            let target_file = &sug.file_path;
            let original_content = std::fs::read_to_string(target_file)
                .map_err(|e| format!("Failed to read original file: {e}"))?;
            let new_content = original_content.replace(&sug.original_snippet, &sug.new_snippet);
            let backup = format!("{}.bak", target_file);
            std::fs::copy(target_file, &backup).ok();
            std::fs::write(target_file, &new_content)
                .map_err(|e| format!("Failed to write mutation: {e}"))?;

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

            // --- 🧠 إدارة المعرفة: تسجيل الفرضية ونقلها لنظرية ---
            // تأكد من تخزين الفرضية في DB
            self.db.insert_hypothesis(&hyp.id, &hyp.statement, &hyp.context_tags, hyp.confidence, new_gen)
                .map_err(|e| e.to_string())?;

            // ابحث عن نظريات مشابهة بناءً على الوسوم
            let matching_theories = self.db.find_matching_theories(&hyp.context_tags);
            let theory_id = if let Some((existing_id, existing_statement, old_conf, ev)) = matching_theories.first() {
                // رفع ثقة النظرية الموجودة
                let new_conf = (old_conf * 0.9 + 0.6 * 0.1).min(1.0); // نجاح جديد يضيف 0.6 ثقة
                self.db.upsert_theory(existing_id, existing_statement, &hyp.id, new_conf, &["rust".to_string()], new_gen)
                    .map_err(|e| e.to_string())?;
                existing_id.clone()
            } else {
                // إنشاء نظرية جديدة
                let new_theory_id = Uuid::new_v4().to_string();
                let statement = format!("Hypothesis: {} (auto-generated theory)", hyp.statement);
                self.db.upsert_theory(&new_theory_id, &statement, &hyp.id, 0.6, &["rust".to_string()], new_gen)
                    .map_err(|e| e.to_string())?;
                new_theory_id
            };

            // تحديث التجربة المسجلة بربطها بـ theory_id
            // (للتبسيط، يمكننا تجاهل ربط التجربة الآن، أو تنفيذه لاحقاً)

            // تخزين الجينوم الجديد
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let genome_id = Uuid::new_v4().to_string();
            let genome_hash = hex::encode(sha2::Sha256::digest(new_content.as_bytes()));
            self.db.insert_genome_node(
                &genome_id, &genome_hash, None, new_gen, &sug.objective,
                &[sug.file_path.clone()], &error_hash, "", &fitness, &pheno,
                &vec!["experiment".to_string()],
                now, "MERGED",
            ).map_err(|e| e.to_string())?;

            println!("Mutation merged. Generation: {}, Theory bank updated.", new_gen);
        } else {
            println!("No acceptable mutation this cycle.");
        }

        // 3. إعادة بناء السياق بعد التحديث (للدورة القادمة)
        Ok(())
    }
}
