// src/memory/mod.rs

/// A trait for agent memory to log steps, tool results, and thoughts.
pub trait Memory {
    fn log(&mut self, label: &str, content: &str);
    fn read_all(&self) -> Vec<(String, String)>;
}

/// In-memory implementation of the Memory trait.
#[derive(Default, Debug)]
pub struct InMemoryLog {
    pub entries: Vec<(String, String)>,
}

impl InMemoryLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl Memory for InMemoryLog {
    fn log(&mut self, label: &str, content: &str) {
        self.entries.push((label.to_string(), content.to_string()));
    }

    fn read_all(&self) -> Vec<(String, String)> {
        self.entries.clone()
    }
}
