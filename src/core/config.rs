use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
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

        config
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
