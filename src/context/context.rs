use std::collections::HashMap;

/// Basic runtime context for an agent — gives access to tools and config.
#[derive(Clone)]
pub struct Context {
    pub tool_registry: HashMap<String, String>, // Placeholder, will later map to Tool trait
    pub dry_run: bool,
    pub llm_provider: Option<String>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            tool_registry: HashMap::new(),
            dry_run: false,
            llm_provider: None,
        }
    }

    pub fn with_tool(mut self, name: &str, id: &str) -> Self {
        self.tool_registry.insert(name.into(), id.into());
        self
    }

    pub fn with_llm(mut self, provider: &str) -> Self {
        self.llm_provider = Some(provider.into());
        self
    }

    pub fn enable_dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}
