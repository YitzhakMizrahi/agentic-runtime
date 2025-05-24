use crate::tools::{Tool, ToolResult};

pub struct FakeEchoTool;

impl Tool for FakeEchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes the input back with a prefix"
    }

    fn execute(&self, input: &str) -> ToolResult {
        let output = format!("Echoed: {}", input);
        ToolResult::success(&output)
    }
}
