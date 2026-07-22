use super::Tool;
use serde_json::{json, Value};

pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern (e.g. **/*.rs, src/**/*.ts). \
         Returns matching file paths."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. **/*.rs, src/**/*.ts)"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search from (defaults to current directory)"
                }
            },
            "required": ["pattern"]
        })
    }

    fn execute(&self, args: &Value) -> Result<String, String> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or("Missing 'pattern' argument")?;
        let base = args["path"]
            .as_str()
            .unwrap_or(".")
            .to_string();
        let base = super::read_file::expand_tilde(&base);

        let full_pattern = if pattern.starts_with('/') || pattern.starts_with('~') {
            super::read_file::expand_tilde(pattern)
        } else {
            format!("{base}/{pattern}")
        };

        let mut matches: Vec<String> = glob::glob(&full_pattern)
            .map_err(|e| format!("Invalid glob pattern: {e}"))?
            .filter_map(|entry| entry.ok())
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                s.replace('\\', "/")
            })
            .filter(|p| !is_in_excluded_dir(p))
            .collect();

        matches.sort();

        if matches.is_empty() {
            Ok("No files matched.".to_string())
        } else {
            let count = matches.len();
            let mut output = matches.join("\n");
            output.push_str(&format!("\n\n{count} file(s) matched."));
            Ok(output)
        }
    }
}

fn is_in_excluded_dir(path: &str) -> bool {
    let excluded = [
        "/.git/", "/node_modules/", "/target/", "/__pycache__/", "/.next/", "/dist/", "/build/",
    ];
    excluded.iter().any(|e| path.contains(e))
}
