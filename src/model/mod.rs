// src/model/mod.rs

#[derive(Clone, Debug)]
pub struct TaskModel {
    pub goal: String,
    pub current_state: String,
    pub output: Option<String>,
}

impl TaskModel {
    pub fn new(goal: &str) -> Self {
        Self {
            goal: goal.to_string(),
            current_state: "Not started".into(),
            output: None,
        }
    }

    pub fn set_output(&mut self, result: String) {
        self.output = Some(result);
        self.current_state = "Completed".into();
    }
}

pub trait Model: Clone {
    fn is_complete(&self) -> bool;
    fn summary(&self) -> String;
}

impl Model for TaskModel {
    fn is_complete(&self) -> bool {
        self.output.is_some()
    }

    fn summary(&self) -> String {
        format!("Goal: {}\nStatus: {}", self.goal, self.current_state)
    }
}
