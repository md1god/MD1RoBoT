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
        let groq_key = env::var("GROQ_API_KEY").ok();

        let (endpoint, model) = if groq_key.is_some() {
            let ep = "https://api.groq.com/openai/v1/chat/completions".to_string();
            let mdl = model_override
                .map(|m| m.to_string())
                .unwrap_or_else(|| {
                    env::var("GROQ_MODEL").unwrap_or_else(|_| "llama-3.1-8b-instant".to_string())
                });
            (ep, mdl)
        } else {
            let ep = env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string());
            let mdl = model_override
                .map(|m| m.to_string())
                .unwrap_or_else(|| {
                    env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:7b".to_string())
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

        // تباعد استباقي بين الطلبات (فقط عند استخدام Groq، الذي له حد صارم
        // للتوكنات في الدقيقة). هذا يمنع تكدّس عدة طلبات متتالية في نفس
        // الدقيقة ويقلل الاصطدام بحد الـ rate limit من الأساس، بدل الاعتماد
        // فقط على إعادة المحاولة بعد الفشل.
        if self.api_key.is_some() {
            sleep(Duration::from_secs(3));
        }

        let max_attempts = 6;
        for attempt in 0..max_attempts {
            let (url, body) = if let Some(ref api_key) = self.api_key {
                let _ = api_key;
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
                let u = format!("{}/api/generate", self.endpoint);
                let b = json!({
                    "model": self.model,
                    "prompt": prompt,
                    "stream": false
                });
                (u, b)
            };

            let mut request = client.post(&url);
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
                        return parsed["choices"][0]["message"]["content"]
                            .as_str()
                            .map(|s| s.trim().to_string())
                            .ok_or_else(|| "Groq response contains no content text.".to_string());
                    } else {
                        return parsed["response"]
                            .as_str()
                            .map(|s| s.trim().to_string())
                            .ok_or_else(|| "Ollama response contains no response field.".to_string());
                    }
                }
                Err(ureq::Error::Status(429, resp)) => {
                    let text = resp.into_string().unwrap_or_default();
                    // نضيف ثانية إضافية كهامش أمان فوق ما تطلبه Groq بالضبط،
                    // لأن الالتزام الحرفي بالرقم أحياناً غير كافٍ إذا كانت
                    // ساعة الخادم والعميل غير متطابقتين تماماً.
                    let wait_secs = parse_retry_seconds(&text).unwrap_or(8.0) + 1.0;
                    eprintln!(
                        "Rate limited (attempt {}). Waiting {:.1}s...",
                        attempt + 1,
                        wait_secs
                    );
                    sleep(Duration::from_secs_f64(wait_secs));
                    if attempt == max_attempts - 1 {
                        return Err(format!("Rate limit exceeded after {} attempts", max_attempts));
                    }
                }
                Err(other) => {
                    return Err(format!("API request failed: {}", other));
                }
            }
        }
        unreachable!();
    }
}

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
