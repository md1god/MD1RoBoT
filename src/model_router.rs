use crate::ollama_client::OllamaClient;
use crate::config_loader::AppConfig;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ModelRouter {
    clients: HashMap<String, OllamaClient>,
}

impl ModelRouter {
    pub fn new(config: AppConfig) -> Self {
        let mut clients = HashMap::new();
        for (lang, model) in &config.models {
            clients.insert(lang.clone(), OllamaClient::new(Some(model)));
        }
        if !clients.contains_key("default") {
            clients.insert("default".into(), OllamaClient::new(Some("qwen2.5-coder:7b")));
        }
        ModelRouter { clients }
    }

    pub fn route(&self, language: &str, prompt: &str) -> Result<String, String> {
        let client = self.clients.get(language)
            .or_else(|| self.clients.get("default"))
            .ok_or("No client available for this language")?;
        client.generate(prompt)
    }

    // (توسعة مستقبلية: مجلس، اقتصار طاقة...)
}
