use crate::brain::{Brain, TaskType, ThoughtRequest};
use crate::protocol::{MutationProposal, AgentRole, EvolutionContext, Hypothesis};
use serde_json::{self, Value, Map};
use uuid::Uuid;

pub struct Crazy;

impl Crazy {
    pub fn propose_mutations(
        &self, // <-- أضفنا self هنا
        brain: &mut Brain,
        ctx: &EvolutionContext,
        max_mutations: usize,
    ) -> Result<Vec<MutationProposal>, String> {
        let hypothesis_requests = self.build_hypothesis_requests(ctx, max_mutations);
        let mut hypotheses = Vec::new();
        for hyp_req in hypothesis_requests {
            let response = brain.think(hyp_req)?;
            let statement = response.lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("No hypothesis generated")
                .to_string();
            let hyp = Hypothesis {
                id: Uuid::new_v4().to_string(),
                statement,
                context_tags: self.extract_tags(ctx),
                confidence: 0.5,
                generation: ctx.world_state.current_generation,
            };
            hypotheses.push(hyp);
        }

        let failed_ids = ctx.recent_experiments.iter()
            .filter(|e| e.verdict == "REJECTED")
            .map(|e| e.experiment_id.clone())
            .collect::<Vec<_>>()
            .join(", ");

        let theory_hints = if ctx.active_theories.is_empty() {
            "No prior theories available.".to_string()
        } else {
            ctx.active_theories.iter()
                .map(|t| format!("Theory (conf={:.2}): {}", t.confidence, t.statement))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let mut proposals = Vec::new();
        for hyp in &hypotheses {
            let constraints = vec![
                format!("Generate exactly 1 mutation proposal for this hypothesis: {}", hyp.statement),
                "Output format: JSON object with keys: suggestion (object), hypothesis (string), expected_fitness_gain (number), risk (number).".to_string(),
                format!("Avoid previously failed experiments: {}", failed_ids),
                format!("Relevant theories:\n{}", theory_hints),
                "The suggestion object must contain: id, agent (Crazy), generation, file_path, language, original_snippet, new_snippet, reason, objective, confidence (0.0-1.0), priority (0.0-1.0), risk (0.0-1.0), expected_gain (string).".to_string(),
                "Diversity: Use one of these styles: incremental improvement, structural refactoring, or architectural change.".to_string(),
            ];

            let request = ThoughtRequest {
                task_type: TaskType::GenerateMutation,
                goal: format!("Test hypothesis: {}", hyp.statement),
                context: ctx.clone(),
                constraints,
                agent: AgentRole::Crazy,
                language_hint: None,
            };

            let response = brain.think(request)?;
            let start = response.find('{').ok_or("No JSON object in response")?;
            let end = response.rfind('}').ok_or("JSON object incomplete")?;
            let json_str = &response[start..=end];

            let raw_value: Value = serde_json::from_str(json_str)
                .map_err(|e| format!("Invalid MutationProposal JSON: {e}"))?;
            let sanitized = sanitize_mutation_json(raw_value, hyp);

            let mut proposal: MutationProposal = serde_json::from_value(sanitized)
                .map_err(|e| format!("Invalid MutationProposal: {e}"))?;
            proposal.hypothesis = hyp.clone();
            proposals.push(proposal);
        }

        Ok(proposals)
    }

    fn build_hypothesis_requests(&self, ctx: &EvolutionContext, count: usize) -> Vec<ThoughtRequest> {
        let base_prompt = format!(
            "You are a creative AI scientist. Based on the current system state (generation {}, health {:.2}), suggest a falsifiable hypothesis for improving the code.",
            ctx.world_state.current_generation, ctx.world_state.health_score
        );
        let mut requests = Vec::new();
        let styles = vec![
            "Focus on performance optimization.",
            "Focus on memory reduction.",
            "Focus on code clarity and maintainability.",
            "Focus on reducing technical debt.",
            "Propose a risky, high-reward hypothesis.",
        ];
        for i in 0..count {
            let style = styles[i % styles.len()];
            let goal = format!("{} {}", base_prompt, style);
            let req = ThoughtRequest {
                task_type: TaskType::ReasonAboutCode,
                goal,
                context: ctx.clone(),
                constraints: vec!["Reply with only the hypothesis statement, one sentence.".into()],
                agent: AgentRole::Crazy,
                language_hint: None,
            };
            requests.push(req);
        }
        requests
    }

    fn extract_tags(&self, ctx: &EvolutionContext) -> Vec<String> {
        let mut tags = Vec::new();
        for exp in &ctx.recent_experiments {
            if let Some(ext) = exp.file_path.split('.').last() {
                tags.push(format!("lang:{}", ext));
            }
            tags.push(format!("file:{}", exp.file_path));
        }
        tags.push(format!("health:{}", ctx.world_state.health_score));
        tags.dedup();
        tags
    }
}

/// طبقة تطهير: تأخذ أي JSON خرج من نموذج لغوي وتجعله متوافقًا مع بنية
/// MutationProposal بغض النظر عن نوعية الأخطاء التي يرتكبها النموذج:
/// - حقول ناقصة تمامًا -> تُملأ بقيم افتراضية آمنة.
/// - أرقام مكتوبة كنصوص أو العكس -> تُحوَّل للنوع الصحيح.
/// - "hypothesis" كنص بدل كائن -> يُستبدل بالفرضية الحقيقية التي نملكها بالفعل.
fn sanitize_mutation_json(raw: Value, hyp: &Hypothesis) -> Value {
    let mut root = match raw {
        Value::Object(m) => m,
        _ => Map::new(),
    };

    // suggestion: لازم تكون object؛ لو مش موجودة أو نوعها غلط، نبدأ من فاضي
    let suggestion_val = root.remove("suggestion").unwrap_or(Value::Null);
    let mut suggestion = match suggestion_val {
        Value::Object(m) => m,
        _ => Map::new(),
    };

    coerce_string(&mut suggestion, "id", || Uuid::new_v4().to_string());
    coerce_string(&mut suggestion, "agent", || "Crazy".to_string());
    coerce_u64(&mut suggestion, "generation", hyp.generation);
    coerce_string(&mut suggestion, "file_path", || String::new());
    coerce_string(&mut suggestion, "language", || String::new());
    coerce_string(&mut suggestion, "original_snippet", || String::new());
    coerce_string(&mut suggestion, "new_snippet", || String::new());
    coerce_string(&mut suggestion, "reason", || String::new());
    coerce_string(&mut suggestion, "objective", || String::new());
    coerce_f32(&mut suggestion, "confidence", 0.5);
    coerce_f32(&mut suggestion, "priority", 0.5);
    coerce_f32(&mut suggestion, "risk", 0.5);
    coerce_string(&mut suggestion, "expected_gain", || String::new());

    root.insert("suggestion".to_string(), Value::Object(suggestion));

    // hypothesis: نتجاهل أي شيء أرسله النموذج ونستخدم الفرضية الحقيقية التي
    // بنيناها بالفعل قبل الطلب (proposal.hypothesis يُستبدل بها لاحقًا على أي
    // حال)، لكن لازم يكون هنا object صالح حتى ينجح الـ deserialize.
    let mut hyp_obj = Map::new();
    hyp_obj.insert("id".to_string(), Value::String(hyp.id.clone()));
    hyp_obj.insert("statement".to_string(), Value::String(hyp.statement.clone()));
    hyp_obj.insert(
        "context_tags".to_string(),
        Value::Array(hyp.context_tags.iter().map(|t| Value::String(t.clone())).collect()),
    );
    hyp_obj.insert(
        "confidence".to_string(),
        Value::Number(serde_json::Number::from_f64(hyp.confidence as f64).unwrap_or(0.into())),
    );
    hyp_obj.insert("generation".to_string(), Value::Number(hyp.generation.into()));
    root.insert("hypothesis".to_string(), Value::Object(hyp_obj));

    coerce_f64_root(&mut root, "expected_fitness_gain", 0.0);
    coerce_f64_root(&mut root, "risk", 0.0);

    Value::Object(root)
}

fn coerce_string(map: &mut Map<String, Value>, key: &str, default: impl FnOnce() -> String) {
    let value = match map.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        Some(Value::Bool(b)) => Some(b.to_string()),
        _ => None,
    };
    map.insert(key.to_string(), Value::String(value.unwrap_or_else(default)));
}

fn coerce_u64(map: &mut Map<String, Value>, key: &str, default: u64) {
    let value = match map.get(key) {
        Some(Value::Number(n)) => n.as_u64().or_else(|| n.as_f64().map(|f| f as u64)),
        Some(Value::String(s)) => s.trim().parse::<u64>().ok(),
        _ => None,
    };
    map.insert(key.to_string(), Value::Number(value.unwrap_or(default).into()));
}

fn coerce_f32(map: &mut Map<String, Value>, key: &str, default: f32) {
    let value = match map.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    };
    let f = value.unwrap_or(default as f64);
    map.insert(
        key.to_string(),
        Value::Number(serde_json::Number::from_f64(f).unwrap_or(0.into())),
    );
}

fn coerce_f64_root(map: &mut Map<String, Value>, key: &str, default: f64) {
    let value = match map.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    };
    let f = value.unwrap_or(default);
    map.insert(
        key.to_string(),
        Value::Number(serde_json::Number::from_f64(f).unwrap_or(0.into())),
    );
}
