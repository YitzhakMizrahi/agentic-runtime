// src/agent/mod.rs

use crate::context::Context;
use crate::model::TaskModel;
use crate::protocol::{ExecutionResult, Feedback, Plan, SimulationResult};

pub trait Agent {
    fn plan(&mut self) -> Plan;
    fn simulate(&self, plan: &Plan) -> SimulationResult;
    fn execute(&mut self, plan: &Plan) -> ExecutionResult;
    fn evaluate(&self, result: &ExecutionResult) -> Feedback;
}

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

    fn execute(&mut self, _plan: &Plan) -> ExecutionResult {
        let input = self.model.goal.clone();

        if let Some(tool) = self.context.get_tool("echo") {
            let result = tool.execute(&input);
            let output_str = result.output.as_ref().cloned().unwrap_or_default();
            self.model.set_output(output_str.clone());

            ExecutionResult {
                success: result.success,
                output: Some(output_str),
                errors: result.error.into_iter().collect(),
            }
        } else {
            ExecutionResult {
                success: false,
                output: None,
                errors: vec!["Echo tool not found".into()],
            }
        }
    }

    fn evaluate(&self, result: &ExecutionResult) -> Feedback {
        Feedback {
            score: if result.success { 90 } else { 30 },
            notes: "Initial fake execution complete.".into(),
        }
    }
}
