use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub models: Vec<ModelEntry>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ModelEntry {
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Resolved model config: the actual values to use for an API call.
#[derive(Clone, Debug)]
pub struct ActiveModel {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
}

impl Config {
    pub fn load(path: &str) -> Self {
        let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Failed to read {path}: {e}");
            std::process::exit(1);
        });
        let mut config: Config = serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!("Failed to parse {path}: {e}");
            std::process::exit(1);
        });

        config.api_key = resolve_env(&config.api_key);
        config.base_url = resolve_env(&config.base_url);
        config.model = resolve_env(&config.model);
        if let Some(ref sp) = config.system_prompt {
            config.system_prompt = Some(resolve_env(sp));
        }
        for m in &mut config.models {
            if let Some(ref key) = m.api_key {
                m.api_key = Some(resolve_env(key));
            }
            if let Some(ref url) = m.base_url {
                m.base_url = Some(resolve_env(url));
            }
        }

        config
    }

    /// Resolve the active model config from the current `model` field.
    /// If `model` matches an entry in `models`, use that entry's overrides.
    pub fn active_model(&self) -> ActiveModel {
        let entry = self.models.iter().find(|m| m.name == self.model);
        ActiveModel {
            name: self.model.clone(),
            base_url: entry
                .and_then(|m| m.base_url.clone())
                .unwrap_or_else(|| self.base_url.clone()),
            api_key: entry
                .and_then(|m| m.api_key.clone())
                .unwrap_or_else(|| self.api_key.clone()),
        }
    }
}

fn resolve_env(value: &str) -> String {
    if let Some(var) = value.strip_prefix('$') {
        std::env::var(var).unwrap_or_else(|_| {
            eprintln!("Warning: environment variable '{var}' not set, using literal value");
            value.to_string()
        })
    } else {
        value.to_string()
    }
}
