use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct KrezaConfig {
    pub use_ensemble: bool,
    pub ensemble_models: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub models: HashMap<String, String>,
    #[serde(default)]
    pub kreza: KrezaConfig,
    #[serde(default = "default_evolution_cycles")]
    pub evolution_cycles: u64,
    #[serde(default)]
    pub lite_mode: bool,
    #[serde(default)]
    pub energy_costs: HashMap<String, u32>,
}

fn default_evolution_cycles() -> u64 { 5 }

impl Default for AppConfig {
    fn default() -> Self {
        let mut models = HashMap::new();
        models.insert("rust".into(), "qwen2.5-coder:7b".into());
        models.insert("python".into(), "qwen2.5-coder:7b".into());
        models.insert("default".into(), "qwen2.5-coder:7b".into());
        
        AppConfig {
            models,
            kreza: KrezaConfig {
                use_ensemble: false,
                ensemble_models: vec![],
            },
            evolution_cycles: 5,
            lite_mode: false,
            energy_costs: HashMap::new(),
        }
    }
}

pub fn load_config(path: &str) -> AppConfig {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read config file from {path}: {e}. Using defaults.");
            return AppConfig::default();
        }
    };
    
    match toml::from_str(&content) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error parsing config.toml: {e}. Using defaults.");
            AppConfig::default()
        }
    }
}
