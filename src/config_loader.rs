use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub models: HashMap<String, String>,
    pub kreza_use_ensemble: bool,
    pub kreza_ensemble_models: Vec<String>,
    pub evolution_cycles: u64,
    pub lite_mode: bool,
    pub energy_costs: HashMap<String, u32>,
    #[serde(default)]
    pub groq_api_key: Option<String>, // حقل مفتاح Groq الاختياري
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut models = HashMap::new();
        models.insert("rust".into(), "qwen2.5-coder:7b".into());
        models.insert("python".into(), "qwen2.5-coder:7b".into());
        models.insert("default".into(), "qwen2.5-coder:7b".into());
        let mut energy = HashMap::new();
        energy.insert("qwen2.5-coder:7b".into(), 1);
        AppConfig {
            models,
            kreza_use_ensemble: false,
            kreza_ensemble_models: vec![],
            evolution_cycles: 5,
            lite_mode: false,
            energy_costs: energy,
            groq_api_key: None,
        }
    }
}

pub fn load_config(path: &str) -> AppConfig {
    let content = fs::read_to_string(path).unwrap_or_default();
    toml::from_str(&content).unwrap_or_default()
}
