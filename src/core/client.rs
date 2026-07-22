use super::config::Config;
use super::types::{ApiError, ChatRequest, ChatResponse, Message};

pub struct Client {
    http: reqwest::blocking::Client,
    config: Config,
}

impl Client {
    pub fn new(config: Config) -> Self {
        Self {
            http: reqwest::blocking::Client::new(),
            config,
        }
    }

    pub fn chat(&self, messages: &[Message]) -> Result<String, ApiError> {
        let request_body = ChatRequest {
            model: self.config.model.clone(),
            messages: messages.to_vec(),
        };

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .timeout(std::time::Duration::from_secs(120))
            .json(&request_body)
            .send()
            .map_err(|e| ApiError::Request(e.to_string()))?;

        let status = response.status();
        let body_text = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(ApiError::Status(status.as_u16(), body_text));
        }

        let chat_resp: ChatResponse =
            serde_json::from_str(&body_text).map_err(|e| ApiError::Parse(e.to_string(), body_text.clone()))?;

        chat_resp
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| ApiError::Parse("empty choices".into(), body_text))
    }
}
