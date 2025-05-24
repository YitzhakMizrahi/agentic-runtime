// src/agent/mod.rs

use crate::context::Context;
use crate::model::TaskModel;
use crate::protocol::{ExecutionResult, Feedback, Plan, PlanStep, SimulationResult};

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
                PlanStep::Info(format!("Understand goal: {}", self.model.goal)),
                PlanStep::ToolCall("git_status".into()),
                PlanStep::ToolCall("echo".into()),
                PlanStep::ToolCall("reflect".into()),
                PlanStep::Info("Generate output".into()),
            ],
        }
    }

    fn simulate(&self, plan: &Plan) -> SimulationResult {
        let mut warnings = vec![];
        let mut tools_used = vec![];

        for step in &plan.steps {
            if let PlanStep::ToolCall(name) = step {
                if let Some(tool) = self.context.get_tool(name) {
                    let spec = tool.spec();
                    tools_used.push(format!(
                        "[TOOL] {} - {} (hint: {})",
                        spec.name, spec.description, spec.input_hint
                    ));
                } else {
                    warnings.push(format!("Tool '{}' not registered", name));
                }
            }
        }

        let predicted = format!(
            "Plan contains {} step(s) and will attempt {} tool call(s).",
            plan.steps.len(),
            tools_used.len()
        );

        warnings.extend(tools_used);

        SimulationResult {
            predicted_outcome: predicted,
            warnings,
        }
    }

    fn execute(&mut self, plan: &Plan) -> ExecutionResult {
        let mut combined_output = String::new();
        let mut errors = vec![];
        let mut success = true;
        let mut latest_output = self.model.goal.clone();

        for step in &plan.steps {
            match step {
                PlanStep::ToolCall(tool_name) => match self.context.get_tool(tool_name) {
                    Some(tool) => {
                        let result = tool.execute(&latest_output);

                        self.context.log(
                            &format!("tool: {}", tool_name),
                            &format!(
                                "input: {}\noutput: {}",
                                latest_output,
                                result.output.clone().unwrap_or_default()
                            ),
                        );

                        if result.success {
                            if let Some(output) = result.output.clone() {
                                combined_output.push_str(&output);
                                combined_output.push('\n');
                                latest_output = output;
                            }
                        } else {
                            success = false;
                            if let Some(err) = result.error.clone() {
                                errors.push(err);
                            }
                        }
                    }
                    None => {
                        success = false;
                        errors.push(format!("Tool not found: {}", tool_name));
                    }
                },
                PlanStep::Info(message) => {
                    combined_output.push_str(&format!("[INFO] {}\n", message));
                    self.context.log("info", message);
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
