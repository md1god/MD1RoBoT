use config::{Config, File};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub max_cycles: usize,
    pub health_threshold: f64,
    pub fitness_weights: FitnessWeights,
    pub models: HashMap<String, String>,
    pub kreza_use_ensemble: bool,
    pub kreza_ensemble_models: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FitnessWeights {
    pub performance: f64,
    pub memory: f64,
    pub reliability: f64,
    pub maintainability: f64,
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let settings = Config::builder()
            .add_source(File::with_name("config.toml").required(false))
            .build()?;

        let max_cycles = settings.get_int("evolution.max_cycles").unwrap_or(5) as usize;
        let health_threshold = settings.get_float("evolution.health_threshold").unwrap_or(0.7);

        let perf = settings.get_float("fitness_weights.performance").unwrap_or(0.35);
        let mem = settings.get_float("fitness_weights.memory").unwrap_or(0.20);
        let rel = settings.get_float("fitness_weights.reliability").unwrap_or(0.30);
        let maint = settings.get_float("fitness_weights.maintainability").unwrap_or(0.15);
        let fitness_weights = FitnessWeights {
            performance: perf,
            memory: mem,
            reliability: rel,
            maintainability: maint,
        };

        let mut models = HashMap::new();
        let model_table = settings.get_table("models").unwrap_or_default();
        for (k, v) in model_table {
            models.insert(k, v.into_string().unwrap_or_default());
        }
        if models.is_empty() {
            models.insert("default".into(), "qwen2.5-coder:7b".into());
        }

        let kreza_use_ensemble = settings.get_bool("kreza.use_ensemble").unwrap_or(false);
        let mut ensemble_models = vec![];
        if let Ok(arr) = settings.get_array("kreza.ensemble_models") {
            for val in arr {
                if let Ok(s) = val.into_string() {
                    ensemble_models.push(s);
                }
            }
        }

        Ok(AppConfig {
            max_cycles,
            health_threshold,
            fitness_weights,
            models,
            kreza_use_ensemble,
            kreza_ensemble_models: ensemble_models,
        })
    }
}
