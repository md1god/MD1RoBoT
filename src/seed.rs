use crate::brain::{Brain, TaskType, ThoughtRequest};
use crate::db::Db;
use crate::search;
use crate::protocol::{AgentRole, EvolutionContext};
use std::fs;

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
            "No accumulated knowledge yet.".to_string()
        } else {
            format!("Previously explored topics: {}", past_topics.join(", "))
        };

        let ctx = EvolutionContext::minimal();
        let query_request = ThoughtRequest {
            task_type: TaskType::ResearchTopic,
            goal: "Select a new research topic".to_string(),
            context: ctx,
            constraints: vec!["3-6 words".to_string()],
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
        let _raw_findings = if results.is_empty() {
            "No results found.".to_string()
        } else {
            results.iter().map(|r| format!("- {}: {} (full text: {})", r.title, r.snippet, r.full_text.as_deref().unwrap_or("none"))).collect::<Vec<_>>().join("\n")
        };

        let summary_request = ThoughtRequest {
            task_type: TaskType::Summarize,
            goal: format!("Summarize search results for {}", query),
            context: EvolutionContext::minimal(),
            constraints: vec!["3-5 sentences".to_string()],
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
            "Generation: {}\nTopic: {}\nSummary: {}\nKnowledge count: {}",
            generation, query, summary, self.db.knowledge_count()
        );
        fs::write(filename, content).ok();
        self.db.save_state(generation, age, curiosity).ok();
    }
}
