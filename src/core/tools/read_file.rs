use super::Tool;
use serde_json::{json, Value};
use std::path::Path;

pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Returns the file content with line numbers."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, args: &Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .ok_or("Missing 'path' argument")?;

        let path = expand_tilde(path);

        if !Path::new(&path).exists() {
            return Err(format!("File not found: {path}"));
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {path}: {e}"))?;

        // Track file for staleness detection
        super::edit_file::track_read(&path);

        let numbered: String = content
            .lines()
            .enumerate()
            .map(|(i, line)| format!("{:>4} | {}", i + 1, line))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(numbered)
    }
}

pub(crate) fn expand_tilde(input: &str) -> String {
    if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
        {
            return format!("{}/{}", home.to_string_lossy(), rest);
        }
    }
    input.to_string()
}
