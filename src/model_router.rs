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

        // إذا كان هناك مفتاح Hugging Face في متغيرات البيئة، نستخدمه لكل اللغات
        if let Ok(hf_key) = std::env::var("HF_API_KEY") {
            let client = OllamaClient::new(None, Some(&hf_key));
            clients.insert("rust".into(), client.clone());
            clients.insert("python".into(), client.clone());
            clients.insert("javascript".into(), client.clone());
            clients.insert("c".into(), client.clone());
            clients.insert("cpp".into(), client.clone());
            clients.insert("default".into(), client);
        } else {
            // استخدم Ollama المحلي مع النماذج المعرفة في config.toml
            for (lang, model) in &config.models {
                clients.insert(lang.clone(), OllamaClient::new(Some(model), None));
            }
            if !clients.contains_key("default") {
                clients.insert("default".into(), OllamaClient::new(Some("qwen2.5-coder:7b"), None));
            }
        }

        ModelRouter { clients }
    }

    pub fn route(&self, language: &str, prompt: &str) -> Result<String, String> {
        let client = self.clients.get(language)
            .or_else(|| self.clients.get("default"))
            .ok_or("No client available for this language")?;
        client.generate(prompt)
    }
}
