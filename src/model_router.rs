use crate::ollama_client::OllamaClient;
use std::collections::HashMap;

pub struct ModelRouter {
    clients: HashMap<String, OllamaClient>,
}

impl ModelRouter {
    pub fn new() -> Self {
        let mut clients = HashMap::new();
        // إضافة عدة نماذج محلية متخصصة (يجب أن تكون موجودة في Ollama)
        clients.insert("rust".into(), OllamaClient::new(Some("deepseek-coder:6.7b")));
        clients.insert("python".into(), OllamaClient::new(Some("qwen2.5-coder:7b")));
        clients.insert("javascript".into(), OllamaClient::new(Some("qwen2.5-coder:7b")));
        clients.insert("default".into(), OllamaClient::new(None)); // استخدام المتغير البيئي أو النموذج الافتراضي
        ModelRouter { clients }
    }

    pub fn route(&self, language: &str, prompt: &str) -> Result<String, String> {
        let client = self.clients.get(language).unwrap_or_else(|| self.clients.get("default").unwrap());
        client.generate(prompt)
    }
}
