use serde_json::{json, Value};
use std::env;

#[derive(Clone)]
pub struct OllamaClient {
    endpoint: String,
    model: String,
    api_key: Option<String>, // سيُستعمل فقط مع Hugging Face
}

impl OllamaClient {
    pub fn new(model_override: Option<&str>) -> Self {
        // إذا كان مفتاح Hugging Face موجوداً، نفعّله
        if let Ok(hf_key) = env::var("HF_API_KEY") {
            let model_id = model_override.unwrap_or("Qwen/Qwen2.5-Coder-1.5B");
            let endpoint = format!("https://api-inference.huggingface.co/models/{}", model_id);
            return OllamaClient {
                endpoint,
                model: model_id.to_string(),
                api_key: Some(hf_key),
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
        // إذا كان لدينا مفتاح Hugging Face، استخدم واجهة HF
        if let Some(api_key) = &self.api_key {
            let body = json!({
                "inputs": prompt,
                "parameters": {
                    "max_new_tokens": 1024,
                    "temperature": 0.7
                }
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
                        .map_err(|e| format!("Failed to parse HF response: {e}"))?;
                    // HF يرجع مصفوفة فيها كائن generated_text
                    if let Some(arr) = parsed.as_array() {
                        if let Some(first) = arr.first() {
                            first["generated_text"]
                                .as_str()
                                .map(|s| s.trim().to_string())
                                .ok_or_else(|| "No generated_text in HF response".to_string())
                        } else {
                            Err("Empty response array from HF".to_string())
                        }
                    } else {
                        // أحياناً يرجع كائناً مباشراً (حسب النموذج)
                        parsed["generated_text"]
                            .as_str()
                            .map(|s| s.trim().to_string())
                            .ok_or_else(|| "HF response missing generated_text".to_string())
                    }
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let text = resp.into_string().unwrap_or_default();
                    Err(format!("HF error ({}): {}", code, text))
                }
                Err(e) => Err(format!("Failed to connect to HF: {}", e)),
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
