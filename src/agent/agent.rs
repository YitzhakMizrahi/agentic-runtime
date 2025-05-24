use crate::protocol::{Plan, SimulationResult, ExecutionResult, Feedback};

pub trait Agent {
    fn plan(&mut self) -> Plan;
    fn simulate(&self, plan: &Plan) -> SimulationResult;
    fn execute(&mut self, plan: &Plan) -> ExecutionResult;
    fn evaluate(&self, result: &ExecutionResult) -> Feedback;
}
