use crate::model_router::ModelRouter;
use crate::protocol::{EvolutionContext, AgentRole};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    GenerateMutation,
    EvaluateMutation,
    ResearchTopic,
    PlanGoal,
    ReasonAboutCode,
    Summarize,
}

#[derive(Debug, Clone)]
pub struct ThoughtRequest {
    pub task_type: TaskType,
    pub goal: String,
    pub context: EvolutionContext,
    pub constraints: Vec<String>,
    pub agent: AgentRole,
    pub language_hint: Option<String>,  // لغة الملف المستهدف (مثلاً "python")
}

#[derive(Clone)]
pub struct Brain {
    router: ModelRouter,
    context_memory: Vec<String>,
}

impl Brain {
    pub fn new() -> Self {
        Brain {
            router: ModelRouter::new(),
            context_memory: Vec::new(),
        }
    }

    pub fn think(&mut self, request: ThoughtRequest) -> Result<String, String> {
        let prompt = self.build_prompt(&request);
        let lang = request.language_hint.as_deref().unwrap_or("default");
        let raw_response = self.router.route(lang, &prompt)?;
        let validated = self.validate_response(&raw_response, &request.task_type)?;
        self.context_memory.push(validated.clone());
        if self.context_memory.len() > 10 {
            self.context_memory.remove(0);
        }
        Ok(validated)
    }

    fn build_prompt(&self, req: &ThoughtRequest) -> String {
        let ctx = &req.context;
        let world = &ctx.world_state;
        let assessment = &ctx.self_assessment;

        let state_str = format!(
            "الجيل: {}، صحة: {:.2}، أضعف نقطة: {}\nالأهداف: {}\nموارد: CPU {:.0}%، ذاكرة متاحة {}MB",
            world.current_generation,
            world.health_score,
            assessment.weakest_point,
            ctx.goals.join(", "),
            ctx.resource_state.cpu_usage_percent,
            ctx.resource_state.memory_available_mb,
        );

        let memory = self.context_memory.join("\n");
        let lang_info = req.language_hint.as_ref().map(|l| format!("اللغة المستهدفة: {}", l)).unwrap_or_default();
        format!(
            "{}\n{}\nالمعرفة السابقة:\n{}\nالقيود: {}\nنفذ المهمة ({:?}) بدقة.",
            state_str,
            lang_info,
            memory,
            req.constraints.join(", "),
            req.task_type
        )
    }

    fn validate_response(&self, response: &str, task: &TaskType) -> Result<String, String> {
        match task {
            TaskType::GenerateMutation | TaskType::EvaluateMutation => {
                if !response.contains('{') && !response.contains('[') {
                    return Err("الرد لا يحتوي على JSON".into());
                }
            }
            _ => {}
        }
        Ok(response.to_string())
    }
}
