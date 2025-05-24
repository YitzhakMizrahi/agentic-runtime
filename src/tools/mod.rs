// src/tools/mod.rs

pub mod fake_echo;
pub mod git_status;
pub mod llm;
pub mod reflector;

/// Tool metadata for discoverability and planning.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_hint: String,
    pub tags: Vec<String>,
}

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
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_hint: "Freeform string input".to_string(),
            tags: vec!["generic".into()],
        }
    }
}

pub use fake_echo::FakeEchoTool;
pub use git_status::GitStatusTool;
pub use llm::LLMTool;
pub use reflector::ReflectorTool;
