// src/tools/mod.rs

/// The result of executing a tool.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(output: &str) -> Self {
        Self {
            success: true,
            output: Some(output.to_string()),
            error: None,
        }
    }

    pub fn failure(error: &str) -> Self {
        Self {
            success: false,
            output: None,
            error: Some(error.to_string()),
        }
    }
}

/// Trait that defines a pluggable tool usable by an agent.
pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, input: &str) -> ToolResult;
}

pub mod fake_echo;
pub use fake_echo::FakeEchoTool;
