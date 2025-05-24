use crate::agent::agent::Agent;
use crate::context::Context;
use crate::model::{TaskModel};
use crate::protocol::{ExecutionResult, Feedback, Plan, SimulationResult};

pub struct BasicAgent {
    pub model: TaskModel,
    pub context: Context,
}

impl Agent for BasicAgent {
    fn plan(&mut self) -> Plan {
        Plan {
            steps: vec![
                format!("Understand goal: {}", self.model.goal),
                "Query tools".into(),
                "Generate output".into(),
            ],
        }
    }

    fn simulate(&self, _plan: &Plan) -> SimulationResult {
        SimulationResult {
            predicted_outcome: format!("Will likely achieve: {}", self.model.goal),
            warnings: vec!["No real tools used yet.".into()],
        }
    }

    fn execute(&mut self, plan: &Plan) -> ExecutionResult {
        let output = format!("Executed plan with {} steps", plan.steps.len());
        self.model.set_output(output.clone());
        ExecutionResult {
            success: true,
            output: Some(output),
            errors: vec![],
        }
    }

    fn evaluate(&self, result: &ExecutionResult) -> Feedback {
        Feedback {
            score: if result.success { 90 } else { 30 },
            notes: "Initial fake execution complete.".into(),
        }
    }
}
