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
                "tool:git_status".into(),
                "Generate output".into(),
            ],
        }
    }

    fn simulate(&self, _plan: &Plan) -> SimulationResult {
        SimulationResult {
            predicted_outcome: format!("Will likely achieve: {}", self.model.goal),
            warnings: vec!["Plan contains tool execution steps.".into()],
        }
    }

    fn execute(&mut self, plan: &Plan) -> ExecutionResult {
        let mut combined_output = String::new();
        let mut errors = vec![];
        let mut success = true;

        for step in &plan.steps {
            if let Some(tool_name) = step.strip_prefix("tool:") {
                match self.context.get_tool(tool_name) {
                    Some(tool) => {
                        let result = tool.execute(&self.model.goal);
                        if result.success {
                            if let Some(output) = result.output {
                                combined_output.push_str(&output);
                                combined_output.push('\n');
                            }
                        } else {
                            success = false;
                            if let Some(err) = result.error {
                                errors.push(err);
                            }
                        }
                    }
                    None => {
                        success = false;
                        errors.push(format!("Tool not found: {}", tool_name));
                    }
                }
            }
        }

        self.model.set_output(combined_output.trim().to_string());

        ExecutionResult {
            success,
            output: Some(self.model.output.clone().unwrap_or_default()),
            errors,
        }
    }

    fn evaluate(&self, result: &ExecutionResult) -> Feedback {
        Feedback {
            score: if result.success { 90 } else { 30 },
            notes: "Dynamic tool execution complete.".into(),
        }
    }
}
