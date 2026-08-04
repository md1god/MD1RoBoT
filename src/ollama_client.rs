use serde_json::{json, Value};
use std::env;
use std::thread::sleep;
use std::time::Duration;

#[derive(Clone)]
pub struct OllamaClient {
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

impl OllamaClient {
    pub fn new(model_override: Option<&str>) -> Self {
        // إذا وُجد مفتاح Groq، نفعّل واجهة Groq
        if let Ok(groq_key) = env::var("GROQ_API_KEY") {
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
        // إذا كان لدينا مفتاح Groq، استخدم Groq API مع إعادة المحاولة عند تجاوز الحد
        if let Some(api_key) = &self.api_key {
            let body = json!({
                "model": self.model,
                "messages": [
                    {"role": "user", "content": prompt}
                ],
                "temperature": 0.7,
                "max_tokens": 1024
            });

            // محاولات متعددة مع انتظار عند خطأ 429
            let max_attempts = 5;
            for attempt in 0..max_attempts {
                let response = ureq::post(&self.endpoint)
                    .set("Authorization", &format!("Bearer {}", api_key))
                    .set("Content-Type", "application/json")
                    .timeout(std::time::Duration::from_secs(300))
                    .send_json(body.clone());

                match response {
                    Ok(resp) => {
                        let parsed: Value = resp
                            .into_json()
                            .map_err(|e| format!("Failed to parse Groq response: {e}"))?;
                        return parsed["choices"][0]["message"]["content"]
                            .as_str()
                            .map(|s| s.trim().to_string())
                            .ok_or_else(|| "No content in Groq response".to_string());
                    }
                    Err(ureq::Error::Status(429, resp)) => {
                        let text = resp.into_string().unwrap_or_default();
                        // استخرج وقت الانتظار من رسالة الخطأ (مثلاً: "Please try again in 3.96s.")
                        let wait_secs = parse_retry_seconds(&text).unwrap_or(5.0);
                        eprintln!(
                            "Groq rate limited (attempt {}). Waiting {:.1}s...",
                            attempt + 1,
                            wait_secs
                        );
                        sleep(Duration::from_secs_f64(wait_secs));
                        if attempt == max_attempts - 1 {
                            return Err(format!("Groq rate limit exceeded after {} attempts", max_attempts));
                        }
                        // وإلا حاول مرة أخرى
                    }
                    Err(other) => {
                        return Err(format!("Groq request failed: {}", other));
                    }
                }
            }
            unreachable!();
        }

        // السلوك الأصلي لـ Ollama (بدون تغيير)
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

/// استخراج الثواني من رسالة خطأ Groq التي تحتوي "Please try again in X.XXs."
fn parse_retry_seconds(error_text: &str) -> Option<f64> {
    let prefix = "Please try again in ";
    if let Some(start) = error_text.find(prefix) {
        let rest = &error_text[start + prefix.len()..];
        if let Some(end) = rest.find('s') {
            let num_str = &rest[..end];
            return num_str.parse::<f64>().ok();
        }
    }
    None
}
