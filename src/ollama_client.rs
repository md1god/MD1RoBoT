use serde_json::{json, Value};
use std::env;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::sync::{Mutex, OnceLock};

/// 🔌 عميل التواصل مع النماذج (OllamaClient) لدعم التشغيل المحلي والسحابي مع إدارة الحصص
#[derive(Clone)]
pub struct OllamaClient {
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

/// ⏱️ منظم معدل الطلبات وحصص الرموز (Rate Limiter) لمنع تجاوز الحد المسموح للخدمات
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

    /// 🔒 محاولة حجز رموز جديدة للطلب الحالي
    fn try_reserve(&mut self, need: u64) -> Result<(), Duration> {
        let elapsed = self.window_start.elapsed().as_secs_f64();
        if elapsed >= (self.window_secs as f64) {
            self.window_start = Instant::now();
            self.used = 0;
        }
        if need > self.limit {
            return Err(Duration::from_secs(self.window_secs));
        }
        if self.used + need <= self.limit {
            self.used += need;
            return Ok(());
        }
        let wait_secs = self.window_secs.saturating_sub(self.window_start.elapsed().as_secs());
        Err(Duration::from_secs(wait_secs + 1))
    }

    /// 🔓 تحرير الرموز المحجوزة في حالة الفشل أو حدوث خطأ
    fn release(&mut self, qty: u64) {
        if self.used >= qty {
            self.used -= qty;
        } else {
            self.used = 0;
        }
    }
}

static GLOBAL_RATE_LIMITER: OnceLock<Mutex<RateLimiter>> = OnceLock::new();

impl OllamaClient {
    /// 🏗️ إنشاء مثيل جديد وتحديد ما إذا كان سيعمل محلياً (Ollama) أو سحابياً (Groq)
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
                    env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:1.5b".to_string())
                });
            (ep, mdl)
        };

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
        }

        OllamaClient {
            endpoint,
            model,
            api_key: groq_key,
        }
    }

    /// 🚀 إرسال الطلب إلى النموذج واستلام النص المولد مع إدارة إعادة المحاولة عند الحظر
    pub fn generate(&self, prompt: &str) -> Result<String, String> {
        let client = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(300))
            .build();

        if self.api_key.is_some() {
            sleep(Duration::from_millis(500));
        }

        let max_attempts = 6;
        for attempt in 0..max_attempts {
            let prompt_tokens_est = (prompt.len() as u64 / 4).max(1);
            let max_resp_tokens = env::var("GROQ_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(900);
            let need_tokens = prompt_tokens_est.saturating_add(max_resp_tokens);

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
                                eprintln!("Global TPM exhausted, waiting {:.1}s before trying to send...", wait.as_secs_f64());
                                drop(rl);
                                sleep(wait);
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
                    if reserved {
                        if let Some(lock) = GLOBAL_RATE_LIMITER.get() {
                            let mut rl = lock.lock().unwrap();
                            rl.release(need_tokens);
                        }
                    }
                    let text = resp.into_string().unwrap_or_default();
                    // نستنى المدة اللي مزود الخدمة نفسه طلبها + هامش أمان يكبر مع
                    // كل محاولة فاشلة، عشان مانضربش نفس النافذة الزمنية بمحاولات
                    // متلاحقة كل ثواني قليلة وتفضل الحصة "مشغولة" باستمرار.
                    let provider_wait = parse_retry_seconds(&text).unwrap_or(8.0);
                    // لو مزود الخدمة طالب انتظار طويل (أكتر من دقيقة)، ده مؤشر حد
                    // يومي/تراكمي مش حد الدقيقة العادي — إعادة المحاولة هنا مفيدش
                    // ومبتعملش غير استهلاك زيادة من الحصة، فنوقف فوراً بدل ما نلف.
                    if provider_wait > 60.0 {
                        eprintln!(
                            "Provider requested a long wait ({:.1}s) — likely a daily/account limit, not retrying.",
                            provider_wait
                        );
                        return Err(format!(
                            "Rate limit likely daily/account-level (provider asked to wait {:.1}s)",
                            provider_wait
                        ));
                    }
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
                }
                Err(other) => {
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
