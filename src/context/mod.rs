// src/context/mod.rs

use crate::tools::Tool;
use std::collections::HashMap;

/// Basic runtime context for an agent — gives access to tools and config.
pub struct Context {
    pub dry_run: bool,
    pub llm_provider: Option<String>,
    #[allow(clippy::type_complexity)]
    pub tools: HashMap<String, Box<dyn Tool + Send + Sync>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            dry_run: false,
            llm_provider: None,
        }
    }

    pub fn register_tool<T: Tool + Send + Sync + 'static>(mut self, tool: T) -> Self {
        self.tools.insert(tool.name().into(), Box::new(tool));
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

    pub fn get_tool(&self, name: &str) -> Option<&(dyn Tool + Send + Sync)> {
        self.tools.get(name).map(|boxed| boxed.as_ref())
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
