use serde_json::{json, Value};
use std::env;

#[derive(Clone)]
pub struct OllamaClient {
    endpoint: String,
    model: String,
}

impl OllamaClient {
    pub fn new(model_override: Option<&str>) -> Self {
        let endpoint = env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = model_override
            .map(|m| m.to_string())
            .unwrap_or_else(|| {
                env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:14b".to_string())
            });
        OllamaClient { endpoint, model }
    }

    pub fn generate(&self, prompt: &str) -> Result<String, String> {
        let url = format!("{}/api/generate", self.endpoint);
        let body = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false
        });

        let response = ureq::post(&url)
            .set("Bypass-Tunnel-Reminder", "true")  // هذا يتجاوز صفحة LocalTunnel
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
                Err(format!("Ollama error ({code}): {text}"))
            }
            Err(e) => Err(format!("Failed to connect to Ollama: {e}")),
        }
    }
}
