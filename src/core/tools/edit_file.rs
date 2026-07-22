use super::Tool;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Mutex, LazyLock};
use std::collections::HashMap;
use std::time::SystemTime;

/// Track file mtimes for staleness detection.
static FILE_CACHE: LazyLock<Mutex<HashMap<String, SystemTime>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct EditFileTool;

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing exact text. The old text must appear exactly once in the file. \
         You must read the file with read_file before editing it."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_text": {
                    "type": "string",
                    "description": "The exact text to find in the file (must be unique)"
                },
                "new_text": {
                    "type": "string",
                    "description": "The replacement text"
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    fn execute(&self, args: &Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .ok_or("Missing 'path' argument")?;
        let old_text = args["old_text"]
            .as_str()
            .ok_or("Missing 'old_text' argument")?;
        let new_text = args["new_text"]
            .as_str()
            .ok_or("Missing 'new_text' argument")?;

        let path = super::read_file::expand_tilde(path);

        if !Path::new(&path).exists() {
            return Err(format!("File not found: {path}"));
        }

        // Staleness check
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(mtime) = metadata.modified() {
                let cache = FILE_CACHE.lock().unwrap();
                if let Some(last_known) = cache.get(&path) {
                    if mtime != *last_known {
                        return Err(format!(
                            "Stale file: {path} was modified externally since last read. \
                             Please read the file again before editing."
                        ));
                    }
                }
            }
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {path}: {e}"))?;

        // Uniqueness check
        let matches: Vec<usize> = content.match_indices(old_text).map(|(i, _)| i).collect();

        match matches.len() {
            0 => {
                // Try quote normalization as a hint
                let normalized_old = normalize_quotes(old_text);
                let normalized_content = normalize_quotes(&content);
                let norm_matches = normalized_content.match_indices(&normalized_old).count();
                if norm_matches == 1 {
                    return Err(format!(
                        "No exact match found, but a match exists with different quote styles. \
                         Check curly vs straight quotes. Original text not found in {path}."
                    ));
                }
                Err(format!("old_text not found in {path}"))
            }
            1 => {
                let new_content = content.replacen(old_text, new_text, 1);
                std::fs::write(&path, &new_content)
                    .map_err(|e| format!("Failed to write {path}: {e}"))?;

                // Update cache
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if let Ok(mtime) = metadata.modified() {
                        FILE_CACHE.lock().unwrap().insert(path.clone(), mtime);
                    }
                }

                Ok(format!("Successfully edited {path}"))
            }
            n => {
                Err(format!(
                    "old_text appears {n} times in {path}. \
                     Provide a longer, more unique string to match exactly once."
                ))
            }
        }
    }
}

fn normalize_quotes(s: &str) -> String {
    s.replace('\u{2018}', "'")
        .replace('\u{2019}', "'")
        .replace('\u{201C}', "\"")
        .replace('\u{201D}', "\"")
}

/// Record that a file was read, for staleness tracking.
pub fn track_read(path: &str) {
    let expanded = super::read_file::expand_tilde(path);
    if let Ok(metadata) = std::fs::metadata(&expanded) {
        if let Ok(mtime) = metadata.modified() {
            FILE_CACHE.lock().unwrap().insert(expanded, mtime);
        }
    }
}
