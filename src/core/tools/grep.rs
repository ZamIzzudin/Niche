use super::Tool;
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;

pub struct GrepTool;

impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search file contents using a regex pattern. Returns matching lines with file paths \
         and line numbers."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (defaults to current directory)"
                }
            },
            "required": ["pattern"]
        })
    }

    fn execute(&self, args: &Value) -> Result<String, String> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or("Missing 'pattern' argument")?;
        let path = args["path"]
            .as_str()
            .unwrap_or(".")
            .to_string();
        let path = super::read_file::expand_tilde(&path);

        let re = Regex::new(pattern)
            .map_err(|e| format!("Invalid regex: {e}"))?;

        let mut results = Vec::new();
        let mut match_count = 0;
        const MAX_RESULTS: usize = 100;

        if Path::new(&path).is_file() {
            search_file(&path, &re, &mut results, &mut match_count, MAX_RESULTS);
        } else {
            search_dir(&path, &re, &mut results, &mut match_count, MAX_RESULTS);
        }

        if results.is_empty() {
            Ok("No matches found.".to_string())
        } else {
            let mut output = results.join("\n");
            if match_count >= MAX_RESULTS {
                output.push_str(&format!(
                    "\n\n... showing first {MAX_RESULTS} matches ({match_count} total)"
                ));
            }
            Ok(output)
        }
    }
}

fn search_dir(
    dir: &str,
    re: &Regex,
    results: &mut Vec<String>,
    match_count: &mut usize,
    max: usize,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if *match_count >= max {
            return;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip(&name) {
            continue;
        }

        let path = entry.path();
        if path.is_dir() {
            search_dir(&path.to_string_lossy(), re, results, match_count, max);
        } else if path.is_file() {
            search_file(&path.to_string_lossy(), re, results, match_count, max);
        }
    }
}

fn search_file(
    file_path: &str,
    re: &Regex,
    results: &mut Vec<String>,
    match_count: &mut usize,
    max: usize,
) {
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for (line_num, line) in content.lines().enumerate() {
        if *match_count >= max {
            return;
        }
        if re.is_match(line) {
            results.push(format!("{file_path}:{}: {line}", line_num + 1));
            *match_count += 1;
        }
    }
}

fn should_skip(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | "__pycache__" | ".next" | "dist" | "build"
    ) || name.starts_with(".niche")
}
