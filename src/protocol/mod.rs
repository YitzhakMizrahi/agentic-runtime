// src/protocol/mod.rs

#[derive(Clone, Debug)]
pub enum PlanStep {
    Info(String),
    ToolCall(String),
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub steps: Vec<PlanStep>,
}

#[derive(Clone, Debug)]
pub struct SimulationResult {
    pub predicted_outcome: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Feedback {
    pub score: u8, // 0–100 scale for now
    pub notes: String,
}
