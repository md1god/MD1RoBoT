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

        // إذا وُجد مفتاح Groq، نستخدم عميل واحد لجميع اللغات
        if std::env::var("GROQ_API_KEY").is_ok() {
            let client = OllamaClient::new(None); // سيختار Groq تلقائياً عند وجود المفتاح
            clients.insert("rust".into(), client.clone());
            clients.insert("python".into(), client.clone());
            clients.insert("javascript".into(), client.clone());
            clients.insert("c".into(), client.clone());
            clients.insert("cpp".into(), client.clone());
            clients.insert("default".into(), client);
        } else {
            // السلوك الأصلي: Ollama لكل لغة
            for (lang, model) in &config.models {
                clients.insert(lang.clone(), OllamaClient::new(Some(model)));
            }
            if !clients.contains_key("default") {
                clients.insert("default".into(), OllamaClient::new(Some("qwen2.5-coder:7b")));
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
