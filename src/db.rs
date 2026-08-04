use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use crate::genome::{GenomeNode, GenomeStatus, KnowledgeType, KnowledgeReference};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Fitness {
    pub performance: f64,
    pub memory: f64,
    pub reliability: f64,
    pub maintainability: f64,
}

impl Fitness {
    pub fn overall(&self) -> f64 {
        self.performance * 0.35 + self.memory * 0.20 + self.reliability * 0.30 + self.maintainability * 0.15
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phenotype {
    pub search_speed_ms: u64,
    pub memory_usage_mb: u64,
    pub error_rate: f64,
    pub build_time_ms: u64,
}

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                generation INTEGER NOT NULL,
                age INTEGER NOT NULL,
                curiosity REAL NOT NULL
            );
            CREATE TABLE IF NOT EXISTS knowledge (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                generation INTEGER NOT NULL,
                topic TEXT NOT NULL,
                summary TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS evolution_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                current_generation INTEGER NOT NULL DEFAULT 0,
                best_fitness REAL DEFAULT 0.0,
                best_generation INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS mutation_history (
                experiment_id TEXT PRIMARY KEY,
                generation INTEGER NOT NULL,
                target_file TEXT NOT NULL,
                reason TEXT,
                objective TEXT,
                confidence REAL,
                verdict TEXT NOT NULL,
                fitness_json TEXT,
                phenotype_json TEXT,
                retry_count INTEGER,
                error_hash TEXT,
                errors_json TEXT,
                timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                hypothesis_id TEXT,
                theory_id TEXT
            );
            CREATE TABLE IF NOT EXISTS genome_nodes (
                id TEXT PRIMARY KEY,
                genome_hash TEXT NOT NULL,
                parent_id TEXT,
                generation INTEGER NOT NULL,
                objective TEXT,
                files_changed TEXT,
                patch_hash TEXT,
                patch_path TEXT,
                fitness_json TEXT,
                phenotype_json TEXT,
                knowledge_sources TEXT,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS oscillation_log (
                error_hash TEXT PRIMARY KEY,
                reject_count INTEGER DEFAULT 1,
                last_rejection_time TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS hypotheses (
                id TEXT PRIMARY KEY,
                statement TEXT NOT NULL,
                context_tags TEXT,
                confidence REAL NOT NULL,
                generation INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS theories (
                id TEXT PRIMARY KEY,
                statement TEXT NOT NULL,
                hypotheses_ids TEXT,
                confidence REAL NOT NULL,
                evidence_experiments INTEGER NOT NULL DEFAULT 0,
                applicable_languages TEXT,
                related_genes TEXT,
                created_generation INTEGER NOT NULL,
                last_validated_generation INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS decision_genes (
                id TEXT PRIMARY KEY,
                description TEXT NOT NULL,
                language TEXT NOT NULL,
                applicability_tags TEXT,
                benefits TEXT,
                risks TEXT,
                evidence_count INTEGER NOT NULL DEFAULT 0,
                success_rate REAL NOT NULL DEFAULT 0.0,
                last_used_generation INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS council_votes (
                vote_id TEXT PRIMARY KEY,
                experiment_id TEXT NOT NULL,
                agent_role TEXT NOT NULL,
                benefit REAL NOT NULL,
                novelty REAL NOT NULL,
                risk REAL NOT NULL,
                cost REAL NOT NULL,
                confidence REAL NOT NULL,
                timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            INSERT OR IGNORE INTO state (id, generation, age, curiosity) VALUES (1, 0, 1, 1.0);
            INSERT OR IGNORE INTO evolution_state (id, current_generation, best_fitness, best_generation) VALUES (1, 0, 0.0, 0);
            ",
        )?;
        Ok(Db { conn: Arc::new(Mutex::new(conn)) })
    }

    // --- دوال الحالة الأساسية ---
    pub fn load_state(&self) -> (u64, u64, f64) {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT generation, age, curiosity FROM state WHERE id = 1", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap_or((0,1,1.0))
    }
    pub fn save_state(&self, gen: u64, age: u64, cur: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO state (id, generation, age, curiosity) VALUES (1, ?1, ?2, ?3) ON CONFLICT(id) DO UPDATE SET generation=excluded.generation, age=excluded.age, curiosity=excluded.curiosity", params![gen, age, cur])?;
        Ok(())
    }
    pub fn add_knowledge(&self, gen: u64, topic: &str, summary: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO knowledge (generation, topic, summary) VALUES (?1, ?2, ?3)", params![gen, topic, summary])?;
        Ok(())
    }
    pub fn recent_topics(&self, limit: u32) -> Vec<String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT topic FROM knowledge ORDER BY id DESC LIMIT ?1").unwrap();
        stmt.query_map(params![limit], |row| row.get::<_,String>(0)).unwrap().filter_map(|r| r.ok()).collect()
    }
    pub fn knowledge_count(&self) -> u64 {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM knowledge", [], |row| row.get(0)).unwrap_or(0)
    }
    pub fn get_evolution_state(&self) -> Result<(u64, f64, u64)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT current_generation, best_fitness, best_generation FROM evolution_state WHERE id = 1", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
    }
    pub fn update_evolution_state(&self, gen: u64, best_fit: f64, best_gen: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE evolution_state SET current_generation = ?1, best_fitness = ?2, best_generation = ?3 WHERE id = 1", params![gen, best_fit, best_gen])?;
        Ok(())
    }

    // --- تسجيل التجارب (مع الفرضيات والنظريات) ---
    pub fn record_experiment(
        &self,
        experiment_id: &str,
        generation: u64,
        target_file: &str,
        reason: &str,
        objective: &str,
        confidence: f32,
        verdict: &str,
        fitness: Option<&Fitness>,
        phenotype: Option<&Phenotype>,
        retry_count: u32,
        error_hash: &str,
        errors: &[String],
        hypothesis_id: Option<&str>,
        theory_id: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let f_json = fitness.map(|f| serde_json::to_string(f).unwrap_or_default());
        let p_json = phenotype.map(|p| serde_json::to_string(p).unwrap_or_default());
        let e_json = serde_json::to_string(errors).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO mutation_history (experiment_id, generation, target_file, reason, objective, confidence, verdict, fitness_json, phenotype_json, retry_count, error_hash, errors_json, hypothesis_id, theory_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![experiment_id, generation, target_file, reason, objective, confidence, verdict, f_json, p_json, retry_count, error_hash, e_json, hypothesis_id, theory_id],
        )?;
        Ok(())
    }

    // --- دوال الجينوم ---
    pub fn insert_genome_node(
        &self, id: &str, genome_hash: &str, parent_id: Option<&str>, generation: u64, objective: &str,
        files_changed: &[String], patch_hash: &str, patch_path: &str,
        fitness: &Fitness, phenotype: &Phenotype, knowledge_sources: &[String],
        created_at: u64, status: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let files_json = serde_json::to_string(files_changed).unwrap();
        let f_json = serde_json::to_string(fitness).unwrap();
        let p_json = serde_json::to_string(phenotype).unwrap();
        let k_json = serde_json::to_string(knowledge_sources).unwrap();
        conn.execute("INSERT OR REPLACE INTO genome_nodes (id, genome_hash, parent_id, generation, objective, files_changed, patch_hash, patch_path, fitness_json, phenotype_json, knowledge_sources, created_at, status) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)", params![id, genome_hash, parent_id, generation, objective, files_json, patch_hash, patch_path, f_json, p_json, k_json, created_at, status])?;
        Ok(())
    }
    pub fn get_latest_genome(&self) -> Option<GenomeNode> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, genome_hash, parent_id, generation, objective, files_changed, patch_hash, patch_path, fitness_json, phenotype_json, knowledge_sources, created_at, status FROM genome_nodes WHERE status = 'MERGED' ORDER BY generation DESC LIMIT 1").ok()?;
        let row = stmt.query_row([], |row| {
            Ok((
                row.get::<_,String>(0)?,
                row.get::<_,String>(1)?,
                row.get::<_,Option<String>>(2)?,
                row.get::<_,u64>(3)? as u32,
                row.get::<_,String>(4)?,
                row.get::<_,String>(5)?,
                row.get::<_,String>(6)?,
                row.get::<_,String>(7)?,
                row.get::<_,String>(8)?,
                row.get::<_,String>(9)?,
                row.get::<_,String>(10)?,
                row.get::<_,u64>(11)?,
                row.get::<_,String>(12)?,
            ))
        }).ok()?;
        let files: Vec<String> = serde_json::from_str(&row.5).ok()?;
        let fitness: Fitness = serde_json::from_str(&row.8).ok()?;
        let phenotype: Phenotype = serde_json::from_str(&row.9).ok()?;
        let ks: Vec<String> = serde_json::from_str(&row.10).unwrap_or_default();
        let status = match row.12.as_str() {
            "MERGED" => GenomeStatus::Merged,
            "ACTIVE" => GenomeStatus::Active,
            "REJECTED" => GenomeStatus::Rejected,
            _ => GenomeStatus::Experimental,
        };
        Some(GenomeNode {
            id: row.0,
            genome_hash: row.1,
            parent_id: row.2,
            generation: row.3,
            objective: row.4,
            files_changed: files,
            patch_hash: row.6,
            patch_path: row.7,
            fitness,
            phenotype,
            knowledge_sources: ks.into_iter().map(|s| KnowledgeReference { source_id: s.clone(), source_type: KnowledgeType::Unknown, confidence: 0.5 }).collect(),
            created_at: row.11,
            status,
        })
    }

    // --- دوال التذبذب والرفض (كانت ناقصة) ---
    pub fn check_oscillation(&self, error_hash: &str) -> (u32, bool) {
        let conn = self.conn.lock().unwrap();
        let count: u32 = conn.query_row("SELECT reject_count FROM oscillation_log WHERE error_hash = ?1", params![error_hash], |row| row.get(0)).unwrap_or(0);
        (count, count >= 3)
    }
    pub fn increment_rejection(&self, error_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO oscillation_log (error_hash, reject_count) VALUES (?1, 1) ON CONFLICT(error_hash) DO UPDATE SET reject_count = reject_count + 1", params![error_hash])?;
        Ok(())
    }
    pub fn clear_rejection(&self, error_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM oscillation_log WHERE error_hash = ?1", params![error_hash])?;
        Ok(())
    }

    // --- استرجاع التجارب الحديثة (لـ context_builder) ---
    pub fn get_recent_experiments(&self, limit: u32) -> Vec<crate::protocol::ExperimentRecord> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT experiment_id, generation, target_file, verdict, fitness_json, phenotype_json, error_hash, timestamp FROM mutation_history ORDER BY timestamp DESC LIMIT ?1").unwrap();
        let rows = stmt.query_map(params![limit], |row| {
            let f_str: Option<String> = row.get(4)?;
            let p_str: Option<String> = row.get(5)?;
            let fitness = f_str.and_then(|s| serde_json::from_str(&s).ok());
            let phenotype = p_str.and_then(|s| serde_json::from_str(&s).ok());
            Ok(crate::protocol::ExperimentRecord {
                experiment_id: row.get(0)?,
                generation: row.get(1)?,
                file_path: row.get(2)?,
                verdict: row.get(3)?,
                fitness,
                phenotype,
                error_hash: row.get(6)?,
                timestamp: row.get(7)?,
            })
        }).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }
    pub fn get_recent_knowledge(&self, limit: u32) -> Vec<(String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT topic, summary FROM knowledge ORDER BY id DESC LIMIT ?1").unwrap();
        let rows = stmt.query_map(params![limit], |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?))).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    // --- دوال الفرضيات والنظريات ---
    pub fn insert_hypothesis(&self, id: &str, statement: &str, context_tags: &[String], confidence: f64, generation: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tags_json = serde_json::to_string(context_tags).unwrap();
        conn.execute("INSERT INTO hypotheses (id, statement, context_tags, confidence, generation) VALUES (?1,?2,?3,?4,?5)", params![id, statement, tags_json, confidence, generation])?;
        Ok(())
    }
    pub fn find_matching_theories(&self, tags: &[String]) -> Vec<(String, String, f64, u32)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, statement, confidence, evidence_experiments FROM theories").unwrap();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,f64>(2)?, row.get::<_,u32>(3)?))
        }).unwrap();
        let mut results = Vec::new();
        for row in rows {
            if let Ok((id, statement, conf, ev)) = row {
                if tags.iter().any(|t| statement.contains(t)) {
                    results.push((id, statement, conf, ev));
                }
            }
        }
        results.sort_by(|a,b| b.2.partial_cmp(&a.2).unwrap());
        results
    }
    pub fn upsert_theory(
        &self,
        id: &str,
        statement: &str,
        hypotheses_id: &str,
        confidence: f64,
        applicable_languages: &[String],
        generation: u64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let existing = conn.query_row("SELECT confidence, evidence_experiments, hypotheses_ids FROM theories WHERE id=?1", params![id], |row| {
            Ok((row.get::<_,f64>(0)?, row.get::<_,u32>(1)?, row.get::<_,String>(2)?))
        }).ok();
        if let Some((old_conf, old_ev, old_hyps)) = existing {
            let new_ev = old_ev + 1;
            let new_conf = (old_conf * 0.9 + confidence * 0.1).min(1.0);
            let mut hyps: Vec<String> = serde_json::from_str(&old_hyps).unwrap_or_default();
            if !hyps.contains(&hypotheses_id.to_string()) { hyps.push(hypotheses_id.to_string()); }
            let hyps_json = serde_json::to_string(&hyps).unwrap();
            conn.execute("UPDATE theories SET confidence=?1, evidence_experiments=?2, hypotheses_ids=?3, last_validated_generation=?4 WHERE id=?5", params![new_conf, new_ev, hyps_json, generation, id])?;
        } else {
            let hyps_json = serde_json::to_string(&vec![hypotheses_id]).unwrap();
            let langs_json = serde_json::to_string(applicable_languages).unwrap();
            conn.execute("INSERT INTO theories (id, statement, hypotheses_ids, confidence, evidence_experiments, applicable_languages, related_genes, created_generation, last_validated_generation) VALUES (?1,?2,?3,?4,1,?5,'[]',?6,?6)", params![id, statement, hyps_json, confidence, langs_json, generation])?;
        }
        Ok(())
    }
    pub fn record_council_vote(&self, vote: &crate::protocol::CouncilVote, experiment_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let vote_id = format!("{}_{}", experiment_id, vote.agent.agent_role()); // الإصلاح هنا
        conn.execute(
            "INSERT OR REPLACE INTO council_votes (vote_id, experiment_id, agent_role, benefit, novelty, risk, cost, confidence) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![vote_id, experiment_id, vote.agent.agent_role(), vote.benefit, vote.novelty, vote.risk, vote.cost, vote.confidence],
        )?;
        Ok(())
    }
}
