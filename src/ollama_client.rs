use serde_json::{json, Value};
use std::env;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::sync::{Mutex, OnceLock};

#[derive(Clone)]
pub struct OllamaClient {
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

struct RateLimiter {
    window_start: Instant,
    used: u64,
    window_secs: u64,
    limit: u64,
}

impl RateLimiter {
    fn new(limit: u64, window_secs: u64) -> Self {
        RateLimiter {
            window_start: Instant::now(),
            used: 0,
            window_secs,
            limit,
        }
    }

    // Try to reserve `need` tokens. Returns Ok(()) if reserved, Err(wait_duration) if not.
    fn try_reserve(&mut self, need: u64) -> Result<(), Duration> {
        let elapsed = self.window_start.elapsed().as_secs_f64();
        if elapsed >= (self.window_secs as f64) {
            // reset window
            self.window_start = Instant::now();
            self.used = 0;
        }
        if need > self.limit {
            // single request would exceed limit: signal long wait (caller may reduce tokens)
            return Err(Duration::from_secs(self.window_secs));
        }
        if self.used + need <= self.limit {
            self.used += need;
            return Ok(());
        }
        // not enough tokens left in this window; compute wait until reset
        let wait_secs = self.window_secs.saturating_sub(self.window_start.elapsed().as_secs());
        Err(Duration::from_secs(wait_secs + 1)) // add 1s safety margin
    }

    // release previously reserved tokens (e.g., on 429 or failure)
    fn release(&mut self, qty: u64) {
        if self.used >= qty {
            self.used -= qty;
        } else {
            self.used = 0;
        }
    }

    fn time_until_reset(&self) -> Duration {
        let elapsed = self.window_start.elapsed().as_secs();
        if elapsed >= self.window_secs {
            Duration::ZERO
        } else {
            Duration::from_secs(self.window_secs - elapsed + 1)
        }
    }
}

static GLOBAL_RATE_LIMITER: OnceLock<Mutex<RateLimiter>> = OnceLock::new();

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

        // ensure global rate limiter exists (only matters if using Groq)
        if groq_key.is_some() {
            let tpm_limit = env::var("GROQ_TPM_LIMIT")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(6000);
            let window_secs = env::var("GROQ_RATE_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);
            GLOBAL_RATE_LIMITER.get_or_init(|| Mutex::new(RateLimiter::new(tpm_limit, window_secs)));
            // Note: we ignore the returned lock here; it will be used at request time.
        }

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

        // If using Groq, add a small proactive pause to avoid tight bursts from same instance
        if self.api_key.is_some() {
            // default short pause between sequential requests from same thread
            sleep(Duration::from_millis(500));
        }

        let max_attempts = 6;
        for attempt in 0..max_attempts {
            // estimate tokens needed for this request
            // rough estimate: tokens in prompt ~ chars/4, plus response tokens = GROQ_MAX_TOKENS
            let prompt_tokens_est = (prompt.len() as u64 / 4).max(1);
            let max_resp_tokens = env::var("GROQ_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(900);
            let need_tokens = prompt_tokens_est.saturating_add(max_resp_tokens);

            // If using Groq, attempt to reserve tokens globally before sending
            let mut reserved = false;
            if self.api_key.is_some() {
                if let Some(lock) = GLOBAL_RATE_LIMITER.get() {
                    loop {
                        let mut rl = lock.lock().unwrap();
                        match rl.try_reserve(need_tokens) {
                            Ok(()) => {
                                reserved = true;
                                break;
                            }
                            Err(wait) => {
                                let wait = wait;
                                eprintln!("Global TPM exhausted, waiting {:.1}s before trying to send...", wait.as_secs_f64());
                                drop(rl);
                                sleep(wait);
                                // then loop and try again
                            }
                        }
                    }
                }
            }

            let (url, body) = if let Some(ref _api_key) = self.api_key {
                let u = self.endpoint.clone();
                let b = json!({
                    "model": self.model,
                    "messages": [
                        {"role": "user", "content": prompt}
                    ],
                    "temperature": 0.7,
                    "max_tokens": max_resp_tokens
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
                        let txt = parsed["choices"][0]["message"]["content"]
                            .as_str()
                            .map(|s| s.trim().to_string())
                            .ok_or_else(|| "Groq response contains no content text.".to_string());
                        return txt;
                    } else {
                        let txt = parsed["response"]
                            .as_str()
                            .map(|s| s.trim().to_string())
                            .ok_or_else(|| "Ollama response contains no response field.".to_string());
                        return txt;
                    }
                }
                Err(ureq::Error::Status(429, resp)) => {
                    // free reserved tokens, if any
                    if reserved {
                        if let Some(lock) = GLOBAL_RATE_LIMITER.get() {
                            let mut rl = lock.lock().unwrap();
                            rl.release(need_tokens);
                        }
                    }
                    let text = resp.into_string().unwrap_or_default();
                    // نستنى المدة اللي مزود الخدمة نفسه طلبها، زائد هامش أمان
                    // بيكبر مع كل محاولة فاشلة، عشان مانضربش نفس نافذة الدقيقة
                    // بمحاولات متلاحقة كل كام ثانية.
                    let provider_wait = parse_retry_seconds(&text).unwrap_or(8.0);
                    let backoff_margin = 5.0 * (attempt as f64 + 1.0);
                    let wait_secs = provider_wait + backoff_margin;
                    eprintln!(
                        "Rate limited by provider (attempt {}). Waiting {:.1}s...",
                        attempt + 1,
                        wait_secs
                    );
                    sleep(Duration::from_secs_f64(wait_secs));
                    if attempt == max_attempts - 1 {
                        return Err(format!("Rate limit exceeded after {} attempts", max_attempts));
                    }
                    // continue to next attempt (backoff loop)
                }
                Err(other) => {
                    // on other errors, if we reserved tokens, release them
                    if reserved {
                        if let Some(lock) = GLOBAL_RATE_LIMITER.get() {
                            let mut rl = lock.lock().unwrap();
                            rl.release(need_tokens);
                        }
                    }
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
