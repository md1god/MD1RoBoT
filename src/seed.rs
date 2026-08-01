use crate::brain::{Brain, TaskType, ThoughtRequest};
use crate::db::Db;
use crate::search;
use crate::protocol::{AgentRole, EvolutionContext};
use std::fs;
use std::path::Path;

pub struct Seed {
    pub db: Db,
    brain: Brain,
}

impl Seed {
    pub fn new(db: Db, brain: Brain) -> Self {
        Seed { db, brain }
    }

    pub fn cycle(&mut self) {
        let (mut generation, mut age, mut curiosity) = self.db.load_state();
        generation += 1;
        age += 1;
        curiosity += 0.05;

        let past_topics = self.db.recent_topics(15);
        let _past_context = if past_topics.is_empty() {
            "لا توجد معرفة متراكمة بعد.".to_string()
        } else {
            format!("مواضيع سبق استكشافها: {}", past_topics.join("، "))
        };

        let ctx = EvolutionContext::minimal();
        let query_request = ThoughtRequest {
            task_type: TaskType::ResearchTopic,
            goal: "اختيار موضوع بحث جديد".to_string(),
            context: ctx,
            constraints: vec!["3-6 كلمات".to_string()],
            agent: AgentRole::Researcher,
            language_hint: None,
        };

        let raw_query = match self.brain.think(query_request) {
            Ok(q) => q,
            Err(_) => return,
        };
        let query = raw_query.lines().next().unwrap_or("").trim().to_string();
        if query.is_empty() { return; }

        let results = search::web_search(&query);
        let raw_findings = if results.is_empty() {
            "لم تظهر أي نتائج.".to_string()
        } else {
            results.iter().map(|r| format!("- {}: {} (نص: {})", r.title, r.snippet, r.full_text.as_deref().unwrap_or("لا يوجد"))).collect::<Vec<_>>().join("\n")
        };

        let summary_request = ThoughtRequest {
            task_type: TaskType::Summarize,
            goal: format!("تلخيص نتائج البحث عن {}", query),
            context: EvolutionContext::minimal(),
            constraints: vec!["3-5 جمل".to_string()],
            agent: AgentRole::Researcher,
            language_hint: None,
        };
        let summary = match self.brain.think(summary_request) {
            Ok(s) => s,
            Err(_) => return,
        };

        self.db.add_knowledge(generation, &query, &summary).ok();
        let _ = fs::create_dir_all("thoughts");
        let filename = format!("thoughts/discovery_{}.txt", generation);
        let content = format!(
            "الجيل: {}\nالموضوع: {}\nالملخص: {}\nعدد المعرفة: {}",
            generation, query, summary, self.db.knowledge_count()
        );
        fs::write(filename, content).ok();
        self.db.save_state(generation, age, curiosity).ok();
    }
}
