use crate::db::Db;
use crate::goals::GoalManager;
use crate::protocol::{EvolutionContext, WorldState, ResourceState, ActiveTasks, SelfAssessment, KnowledgeItem, ExperimentRecord};
use crate::genome::GenomeNode;

pub struct ContextBuilder {
    db: Db,
    goal_manager: GoalManager,
}

impl ContextBuilder {
    pub fn new(db: Db, goal_manager: GoalManager) -> Self {
        ContextBuilder { db, goal_manager }
    }

    pub fn build(&self) -> EvolutionContext {
        let (gen, best_fitness, _) = self.db.get_evolution_state().unwrap_or((0, 0.0, 0));

        let world = WorldState {
            total_files: 15, // يمكن حسابه ديناميكياً
            modules: 7,
            lines_of_code: 0,
            test_coverage: 0.0,
            current_generation: gen,
            active_branch: "main".into(),
            health_score: self.calculate_health(),
        };

        let current_genome = self.db.get_latest_genome();

        let goals = self.goal_manager.current_goals();

        let experiments: Vec<ExperimentRecord> = self.db.get_recent_experiments(5);

        let knowledge: Vec<KnowledgeItem> = self.db.get_recent_knowledge(10)
            .into_iter()
            .map(|(topic, summary)| KnowledgeItem {
                id: format!("k_{}", topic),
                topic,
                summary,
                source_type: "search".into(),
                confidence: 0.8,
            })
            .collect();

        let resources = ResourceState {
            cpu_usage_percent: 0.0,
            memory_available_mb: 0,
            disk_free_gb: 0,
            network_connected: true,
        };

        let tasks = ActiveTasks {
            searching: false,
            evolving: true,
            testing: false,
            waiting: false,
        };

        let assessment = SelfAssessment {
            weakest_point: "جودة الاقتراحات".to_string(),
            improvement_score: best_fitness,
        };

        EvolutionContext {
            world_state: world,
            current_genome,
            goals,
            recent_experiments: experiments,
            knowledge_base: knowledge,
            resource_state: resources,
            active_tasks: tasks,
            self_assessment: assessment,
        }
    }

    fn calculate_health(&self) -> f64 {
        let (_, fit, _) = self.db.get_evolution_state().unwrap_or((0, 0.0, 0));
        fit
    }
}
