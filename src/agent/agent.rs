// src/agent.rs

pub trait Agent {
    fn plan(&mut self) -> Plan;
    fn simulate(&self, plan: &Plan) -> SimulationResult;
    fn execute(&mut self, plan: &Plan) -> ExecutionResult;
    fn evaluate(&self, result: &ExecutionResult) -> Feedback;
}

// Later we will define:
// - Plan
// - SimulationResult
// - ExecutionResult
// - Feedback

// We'll also make this generic over Model and Context when needed.
