use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Plan {
    pub steps: Vec<String>, // For now, a list of action descriptions
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
