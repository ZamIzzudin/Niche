use super::Tool;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct ListFilesTool;

impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files and directories at the given path. Returns a tree-like listing."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list (defaults to current directory)"
                }
            }
        })
    }

    fn execute(&self, args: &Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .unwrap_or(".")
            .to_string();

        let path = super::read_file::expand_tilde(&path);

        if !Path::new(&path).exists() {
            return Err(format!("Path not found: {path}"));
        }

        let mut result = Vec::new();
        list_recursive(&path, "", &mut result, 0)?;

        if result.is_empty() {
            Ok("(empty directory)".into())
        } else {
            Ok(result.join("\n"))
        }
    }
}

fn list_recursive(
    dir: &str,
    prefix: &str,
    result: &mut Vec<String>,
    depth: usize,
) -> Result<(), String> {
    if depth > 3 {
        return Ok(());
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();

        if should_skip(&name) {
            continue;
        }

        let path = entry.path();
        let is_dir = path.is_dir();

        if is_dir {
            result.push(format!("{prefix}{name}/"));
            let new_dir = entry.path().to_string_lossy().to_string();
            let new_prefix = format!("{prefix}  ");
            list_recursive(&new_dir, &new_prefix, result, depth + 1)?;
        } else {
            result.push(format!("{prefix}{name}"));
        }
    }

    Ok(())
}

fn should_skip(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | "__pycache__" | ".next" | "dist" | "build"
    ) || name.starts_with(".niche")
}
