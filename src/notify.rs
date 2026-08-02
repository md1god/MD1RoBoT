use ureq;

pub struct Notifier {
    bot_token: String,
    chat_id: String,
}

impl Notifier {
    pub fn from_env() -> Option<Self> {
        let bot_token = std::env::var("TG_BOT_TOKEN").ok()?;
        let chat_id = std::env::var("TG_CHAT_ID").ok()?;
        Some(Notifier { bot_token, chat_id })
    }

    pub fn send(&self, text: &str) {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let _ = ureq::post(&url).send_json(ureq::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML"
        }));
    }
}
