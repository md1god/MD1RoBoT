use serde_json::{json, Value};
use std::env;

#[derive(Clone)]
pub struct OllamaClient {
    endpoint: String,
    model: String,
    api_key: Option<String>,   // يُستخدم مع Groq فقط
}

impl OllamaClient {
    pub fn new(model_override: Option<&str>) -> Self {
        // إذا وُجد مفتاح Groq، نفعّل واجهة Groq
        if let Ok(groq_key) = env::var("GROQ_API_KEY") {
            // يمكن اختيار النموذج من متغير بيئة اختياري، وإلا نستخدم llama-3.1-8b-instant (سريع ومجاني)
            let model = env::var("GROQ_MODEL")
                .unwrap_or_else(|_| "llama-3.1-8b-instant".to_string());
            return OllamaClient {
                endpoint: "https://api.groq.com/openai/v1/chat/completions".to_string(),
                model,
                api_key: Some(groq_key),
            };
        }

        // وإلا نعود للإعدادات الأصلية (Ollama)
        let endpoint = env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = model_override
            .map(|m| m.to_string())
            .unwrap_or_else(|| {
                env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:14b".to_string())
            });
        OllamaClient {
            endpoint,
            model,
            api_key: None,
        }
    }

    pub fn generate(&self, prompt: &str) -> Result<String, String> {
        // إذا كان لدينا مفتاح Groq، استخدم Groq API (بصيغة Chat Completions)
        if let Some(api_key) = &self.api_key {
            let body = json!({
                "model": self.model,
                "messages": [
                    {"role": "user", "content": prompt}
                ],
                "temperature": 0.7,
                "max_tokens": 1024
            });

            let response = ureq::post(&self.endpoint)
                .set("Authorization", &format!("Bearer {}", api_key))
                .set("Content-Type", "application/json")
                .timeout(std::time::Duration::from_secs(300))
                .send_json(body);

            return match response {
                Ok(resp) => {
                    let parsed: Value = resp
                        .into_json()
                        .map_err(|e| format!("Failed to parse Groq response: {e}"))?;
                    parsed["choices"][0]["message"]["content"]
                        .as_str()
                        .map(|s| s.trim().to_string())
                        .ok_or_else(|| "No content in Groq response".to_string())
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let text = resp.into_string().unwrap_or_default();
                    Err(format!("Groq error ({}): {}", code, text))
                }
                Err(e) => Err(format!("Failed to connect to Groq: {}", e)),
            };
        }

        // السلوك الأصلي لـ Ollama
        let url = format!("{}/api/generate", self.endpoint);
        let body = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false
        });

        let response = ureq::post(&url)
            .set("Bypass-Tunnel-Reminder", "true")
            .timeout(std::time::Duration::from_secs(300))
            .send_json(body);

        match response {
            Ok(resp) => {
                let parsed: Value = resp
                    .into_json()
                    .map_err(|e| format!("Failed to parse Ollama response: {e}"))?;
                parsed["response"]
                    .as_str()
                    .map(|s| s.trim().to_string())
                    .ok_or_else(|| "Response contains no text.".to_string())
            }
            Err(ureq::Error::Status(code, resp)) => {
                let text = resp.into_string().unwrap_or_default();
                Err(format!("Ollama error ({}): {}", code, text))
            }
            Err(e) => Err(format!("Failed to connect to Ollama: {}", e)),
        }
    }
}
