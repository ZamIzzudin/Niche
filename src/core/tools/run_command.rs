use super::Tool;
use serde_json::{json, Value};
use std::process::Command;

pub struct RunCommandTool;

impl Tool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return stdout and stderr."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            },
            "required": ["command"]
        })
    }

    fn execute(&self, args: &Value) -> Result<String, String> {
        let command = args["command"]
            .as_str()
            .ok_or("Missing 'command' argument")?;

        let (program, flag) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let output = Command::new(program)
            .arg(flag)
            .arg(command)
            .output()
            .map_err(|e| format!("Failed to execute command: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n");
            }
            result.push_str("[stderr]\n");
            result.push_str(&stderr);
        }
        if result.is_empty() {
            result = "(no output)".to_string();
        }

        if !output.status.success() {
            result = format!("[exit code: {}]\n{}", output.status.code().unwrap_or(-1), result);
        }

        Ok(result)
    }
}
