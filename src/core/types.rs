use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }
}

#[derive(Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Streaming SSE chunk from the API.
#[derive(Deserialize)]
pub struct StreamChunk {
    pub choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
pub struct StreamChoice {
    pub delta: Delta,
}

#[derive(Deserialize)]
pub struct Delta {
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug)]
pub enum ApiError {
    Request(String),
    Status(u16, String),
    #[allow(dead_code)]
    Parse(String, String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Request(e) => write!(f, "Request failed: {e}"),
            ApiError::Status(code, body) => write!(f, "API error {code}: {body}"),
            ApiError::Parse(e, raw) => write!(f, "Parse error: {e}\nRaw: {raw}"),
        }
    }
}
