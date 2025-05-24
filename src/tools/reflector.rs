// src/tools/reflector.rs

use crate::tools::{Tool, ToolResult, ToolSpec};

pub struct ReflectorTool;

impl ReflectorTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReflectorTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ReflectorTool {
    fn name(&self) -> &str {
        "reflect"
    }

    fn description(&self) -> &str {
        "Analyzes a memory log (as input text) and provides a summary reflection."
    }

    fn execute(&self, input: &str) -> ToolResult {
        let summary = input
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| format!("- {}", line))
            .collect::<Vec<_>>()
            .join("\n");

        let output = format!("Reflection Summary:\n{}", summary);
        ToolResult::success(&output)
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().into(),
            description: self.description().into(),
            input_hint: "Pass memory log as plain text.".into(),
            tags: vec!["introspection".into(), "meta".into()],
        }
    }
}
