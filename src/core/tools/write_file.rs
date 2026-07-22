use super::Tool;
use serde_json::{json, Value};
use std::path::Path;

pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates the file if it does not exist, or overwrites if it does."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn execute(&self, args: &Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .ok_or("Missing 'path' argument")?;
        let content = args["content"]
            .as_str()
            .ok_or("Missing 'content' argument")?;

        let path = super::read_file::expand_tilde(path);
        let path_obj = Path::new(&path);

        if let Some(parent) = path_obj.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directories: {e}"))?;
            }
        }

        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write {path}: {e}"))?;

        Ok(format!("Wrote {} bytes to {path}", content.len()))
    }
}
