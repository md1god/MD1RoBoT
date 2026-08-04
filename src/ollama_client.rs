use serde_json::{json, Value};
use std::env;

#[derive(Clone)]
pub struct OllamaClient {
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

impl OllamaClient {
    pub fn new(model_override: Option<&str>) -> Self {
        // التحقق مما إذا كان مفتاح Groq موجوداً
        let groq_key = env::var("GROQ_API_KEY").ok();
        
        let (endpoint, model) = if let Some(ref _key) = groq_key {
            // استخدام Groq API
            let ep = "https://api.groq.com/openai/v1/chat/completions".to_string();
            let mdl = model_override
                .map(|m| m.to_string())
                .unwrap_or_else(|| {
                    env::var("GROQ_MODEL").unwrap_or_else(|| "llama-3.1-8b-instant".to_string())
                });
            (ep, mdl)
        } else {
            // استخدام Ollama المحلي الافتراضي
            let ep = env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string());
            let mdl = model_override
                .map(|m| m.to_string())
                .unwrap_or_else(|| {
                    env::var("OLLAMA_MODEL").unwrap_or_else(|| "qwen2.5-coder:7b".to_string())
                });
            (ep, mdl)
        };

        OllamaClient {
            endpoint,
            model,
            api_key: groq_key,
        }
    }

    pub fn generate(&self, prompt: &str) -> Result<String, String> {
        let client = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(300))
            .build();

        let (url, body) = if let Some(ref api_key) = self.api_key {
            // تجهيز طلب Groq (متوافق مع OpenAI API)
            let u = self.endpoint.clone();
            let b = json!({
                "model": self.model,
                "messages": [
                    {"role": "user", "content": prompt}
                ],
                "temperature": 0.7,
                "max_tokens": 2048
            });
            (u, b)
        } else {
            // تجهيز طلب Ollama المحلي
            let u = format!("{}/api/generate", self.endpoint);
            let b = json!({
                "model": self.model,
                "prompt": prompt,
                "stream": false
            });
            (u, b)
        };

        let mut request = client.post(&url);

        // إضافة الهيدرز حسب الخدمة المستخدمة
        if let Some(ref api_key) = self.api_key {
            request = request
                .set("Authorization", &format!("Bearer {}", api_key))
                .set("Content-Type", "application/json");
        } else {
            request = request
                .set("Bypass-Tunnel-Reminder", "true")
                .set("Content-Type", "application/json");
        }

        let response = request.send_json(body);

        match response {
            Ok(resp) => {
                let parsed: Value = resp
                    .into_json()
                    .map_err(|e| format!("Failed to parse response JSON: {e}"))?;

                if self.api_key.is_some() {
                    // استخراج النص من استجابة Groq / OpenAI format
                    parsed["choices"][0]["message"]["content"]
                        .as_str()
                        .map(|s| s.trim().to_string())
                        .ok_or_else(|| "Groq response contains no content text.".to_string())
                } else {
                    // استخراج النص من استجابة Ollama format
                    parsed["response"]
                        .as_str()
                        .map(|s| s.trim().to_string())
                        .ok_or_else(|| "Ollama response contains no response field.".to_string())
                }
            }
            Err(ureq::Error::Status(code, resp)) => {
                let text = resp.into_string().unwrap_or_default();
                Err(format!("API error ({code}): {text}"))
            }
            Err(e) => Err(format!("Failed to connect to API endpoint: {e}")),
        }
    }
}
