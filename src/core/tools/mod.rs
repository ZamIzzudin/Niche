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
