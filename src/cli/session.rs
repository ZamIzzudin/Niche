use crate::core::types::Message;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SESSIONS_DIR: &str = ".niche_sessions";

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<Message>,
}

pub struct SessionManager {
    pub current: Option<Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self { current: None }
    }

    fn sessions_dir() -> PathBuf {
        PathBuf::from(SESSIONS_DIR)
    }

    fn session_path(id: &str) -> PathBuf {
        Self::sessions_dir().join(format!("{id}.json"))
    }

    fn timestamp() -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{now}")
    }

    fn gen_id() -> String {
        let ts = Self::timestamp();
        let ts_num: u64 = ts.parse().unwrap_or(0);
        let rand_part: u32 = (ts_num % 100000) as u32;
        format!("ses_{ts}_{rand_part:05x}")
    }

    #[allow(dead_code)]
    pub fn current(&self) -> Option<&Session> {
        self.current.as_ref()
    }

    pub fn current_mut(&mut self) -> Option<&mut Session> {
        self.current.as_mut()
    }

    pub fn start_new(&mut self, model: &str, messages: Vec<Message>) {
        let now = Self::timestamp();
        let session = Session {
            id: Self::gen_id(),
            title: "New Session".to_string(),
            model: model.to_string(),
            created_at: now.clone(),
            updated_at: now,
            messages,
        };
        self.current = Some(session);
        self.ensure_dir();
    }

    pub fn save(&mut self) {
        let session = match &mut self.current {
            Some(s) => s,
            None => return,
        };

        // Auto-title from first user message
        if session.title == "New Session" {
            for msg in &session.messages {
                if msg.role == "user" {
                    let title = msg.content.as_deref().unwrap_or("Untitled");
                    let title: String = title.chars().take(60).collect();
                    session.title = title;
                    break;
                }
            }
        }

        session.updated_at = Self::timestamp();

        let path = Self::session_path(&session.id);
        let content = serde_json::to_string_pretty(&session).unwrap_or_default();
        let _ = std::fs::write(&path, content);
    }

    fn ensure_dir(&self) {
        let dir = Self::sessions_dir();
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
        }
    }

    pub fn list_sessions() -> Vec<Session> {
        let dir = Self::sessions_dir();
        if !dir.exists() {
            return Vec::new();
        }

        let mut sessions: Vec<Session> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    let content = std::fs::read_to_string(&path).ok()?;
                    serde_json::from_str::<Session>(&content).ok()
                } else {
                    None
                }
            })
            .collect();

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sessions
    }

    pub fn load(id: &str) -> Option<Session> {
        let path = Self::session_path(id);
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn load_latest() -> Option<Session> {
        Self::list_sessions().into_iter().next()
    }

    pub fn load_into(&mut self, id: &str) -> Result<&Session, String> {
        let session = Self::load(id).ok_or(format!("Session '{id}' not found"))?;
        self.current = Some(session);
        Ok(self.current.as_ref().unwrap())
    }

    pub fn latest_id() -> Option<String> {
        Self::list_sessions().first().map(|s| s.id.clone())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_age(updated_at: &str) -> String {
    let ts: u64 = updated_at.parse().unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let diff = now.saturating_sub(ts);

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

pub fn session_summary(session: &Session) -> String {
    let msg_count = session
        .messages
        .iter()
        .filter(|m| m.role != "system")
        .count();
    format!(
        "{} msgs",
        msg_count
    )
}
