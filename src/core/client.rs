use super::config::{ActiveModel, Config};
use super::types::{
    ApiError, ChatRequest, Message, StreamChunk, ToolCall, ToolCallFunction, ToolDefinition,
};
use futures_util::StreamExt;
use std::collections::BTreeMap;
use std::sync::Mutex;

pub struct Client {
    http: reqwest::Client,
    active: Mutex<ActiveModel>,
    config: Config,
}

pub struct StreamResult {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
}

impl Client {
    pub fn new(config: Config) -> Self {
        let active = config.active_model();
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            active: Mutex::new(active),
            config,
        }
    }

    pub fn active_model(&self) -> ActiveModel {
        self.active.lock().unwrap().clone()
    }

    pub fn switch_model(&self, name: &str) -> Result<String, String> {
        let entry = self.config.models.iter().find(|m| m.name == name);
        match entry {
            Some(m) => {
                let resolved = ActiveModel {
                    name: m.name.clone(),
                    base_url: m
                        .base_url
                        .clone()
                        .unwrap_or_else(|| self.config.base_url.clone()),
                    api_key: m
                        .api_key
                        .clone()
                        .unwrap_or_else(|| self.config.api_key.clone()),
                };
                let display = m.display_name.clone();
                *self.active.lock().unwrap() = resolved;
                Ok(display)
            }
            None => Err(format!("Model '{name}' not found in config")),
        }
    }

    pub fn available_models(&self) -> Vec<(&str, &str, bool)> {
        let current = self.active.lock().unwrap().name.clone();
        self.config
            .models
            .iter()
            .map(|m| {
                (
                    m.name.as_str(),
                    m.display_name.as_str(),
                    m.name == current,
                )
            })
            .collect()
    }

    pub async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        mut on_token: impl FnMut(&str),
    ) -> Result<StreamResult, ApiError> {
        let active = self.active.lock().unwrap().clone();

        let request_body = ChatRequest {
            model: active.name.clone(),
            messages: messages.to_vec(),
            tools: tools.to_vec(),
            stream: Some(true),
        };

        let url = format!(
            "{}/chat/completions",
            active.base_url.trim_end_matches('/')
        );

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", active.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ApiError::Request(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(ApiError::Status(status.as_u16(), body_text));
        }

        let mut full_response = String::new();
        let mut finish_reason: Option<String> = None;

        let mut tool_accum: BTreeMap<usize, (String, String, String)> = BTreeMap::new();

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| ApiError::Request(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }

                    match serde_json::from_str::<StreamChunk>(data) {
                        Ok(parsed) => {
                            if let Some(choice) = parsed.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    on_token(content);
                                    full_response.push_str(content);
                                }

                                if let Some(fr) = &choice.finish_reason {
                                    finish_reason = Some(fr.clone());
                                }

                                for tc in &choice.delta.tool_calls {
                                    let entry = tool_accum
                                        .entry(tc.index)
                                        .or_insert_with(|| (String::new(), String::new(), String::new()));

                                    if let Some(id) = &tc.id {
                                        entry.0 = id.clone();
                                    }
                                    if let Some(func) = &tc.function {
                                        if let Some(name) = &func.name {
                                            entry.1 = name.clone();
                                        }
                                        if let Some(args) = &func.arguments {
                                            entry.2.push_str(args);
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
        }

        let tool_calls: Vec<ToolCall> = tool_accum
            .into_iter()
            .map(|(_, (id, name, args))| ToolCall {
                id,
                call_type: "function".into(),
                function: ToolCallFunction {
                    name,
                    arguments: args,
                },
            })
            .collect();

        Ok(StreamResult {
            content: full_response,
            tool_calls,
            finish_reason,
        })
    }
}
