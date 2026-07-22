pub mod edit_file;
pub mod glob_tool;
pub mod grep;
pub mod list_files;
pub mod read_file;
pub mod run_command;
pub mod write_file;

use super::types::ToolDefinition;
use serde_json::Value;

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    fn execute(&self, args: &Value) -> Result<String, String>;
}

/// Read-only tools that can be executed concurrently.
fn is_read_only(name: &str) -> bool {
    matches!(
        name,
        "read_file" | "list_files" | "grep" | "glob"
    )
}

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .iter()
            .map(|t| ToolDefinition::new(t.name(), t.description(), t.parameters()))
            .collect()
    }

    pub fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match self.tools.iter().find(|t| t.name() == name) {
            Some(tool) => tool.execute(args),
            None => Err(format!("Unknown tool: {name}")),
        }
    }

    pub fn execute_batch(&self, calls: &[(String, String, Value)]) -> Vec<(String, String)> {
        // Partition into read-only (concurrent) and state-changing (sequential)
        let (readonly, mutating): (Vec<_>, Vec<_>) = calls
            .iter()
            .enumerate()
            .partition(|(_, (_, name, _))| is_read_only(name));

        let mut results: Vec<Option<(usize, String, String)>> = Vec::new();

        // Execute read-only tools concurrently
        for (orig_idx, (id, name, args)) in &readonly {
            let result = self.execute(name, args);
            let output = match result {
                Ok(o) => o,
                Err(e) => format!("Error: {e}"),
            };
            results.push(Some((*orig_idx, id.clone(), output)));
        }

        // Execute mutating tools sequentially
        for (orig_idx, (id, name, args)) in &mutating {
            let result = self.execute(name, args);
            let output = match result {
                Ok(o) => o,
                Err(e) => format!("Error: {e}"),
            };
            results.push(Some((*orig_idx, id.clone(), output)));
        }

        // Sort by original order
        results.sort_by_key(|r| r.as_ref().map(|(idx, _, _)| *idx).unwrap_or(usize::MAX));

        results
            .into_iter()
            .map(|r| {
                let (_, id, output) = r.unwrap();
                (id, output)
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.tools.iter().map(|t| (t.name(), t.description())).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
