// src/agent/mod.rs

use crate::context::Context;
use crate::model::TaskModel;
use crate::protocol::planner::Planner;
use crate::protocol::replanner::Replanner;
use crate::protocol::{ExecutionResult, Feedback, Plan, PlanStep, SimulationResult};

use std::io::{Write, stdin, stdout};

pub trait Agent {
    fn plan(&mut self) -> Plan;
    fn simulate(&self, plan: &Plan) -> SimulationResult;
    fn execute(&mut self, plan: &Plan) -> ExecutionResult;
    fn evaluate(&self, result: &ExecutionResult) -> Feedback;
    fn replan(&mut self, reflection: &str) -> Option<Plan>;
}

pub struct BasicAgent {
    pub model: TaskModel,
    pub context: Context,
    pub planner: Option<Box<dyn Planner>>,
    pub replanner: Option<Box<dyn Replanner>>,
}

impl BasicAgent {
    pub fn new(
        model: TaskModel,
        context: Context,
        planner: Option<Box<dyn Planner>>,
        replanner: Option<Box<dyn Replanner>>,
    ) -> Self {
        Self {
            model,
            context,
            planner,
            replanner,
        }
    }
}

impl Agent for BasicAgent {
    fn plan(&mut self) -> Plan {
        if let Some(planner) = &self.planner {
            self.context.log("planning", "Using dynamic LLM planner");
            planner.generate_plan(&mut self.context, &self.model.goal)
        } else {
            self.context.log("planning", "Using static hardcoded plan");
            Plan {
                steps: vec![
                    PlanStep::Info(format!("Understand goal: {}", self.model.goal)),
                    PlanStep::ToolCall {
                        name: "git_status".into(),
                        input: "Check repo state".into(),
                    },
                    PlanStep::ToolCall {
                        name: "reflect".into(),
                        input: "Summarize changes".into(),
                    },
                    PlanStep::ToolCall {
                        name: "echo".into(),
                        input: "Task complete.".into(),
                    },
                    PlanStep::Info("Generate output".into()),
                ],
            }
        }
    }

    fn simulate(&self, plan: &Plan) -> SimulationResult {
        let mut warnings = vec![];
        let mut tools_used = vec![];

        for step in &plan.steps {
            if let PlanStep::ToolCall { name, .. } = step {
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
        println!("--- PLAN ---\n{:#?}", plan);
        let simulation = self.simulate(plan);
        println!("--- SIMULATION ---\n{:#?}", simulation);

        let mut combined_output = String::new();
        let mut errors = vec![];
        let mut critical_failures = 0;
        let mut previous_outputs = std::collections::HashMap::new();

        for step in &plan.steps {
            match step {
                PlanStep::ToolCall { name, input } => {
                    let resolved_input = if input.starts_with("$output[") && input.ends_with("]") {
                        let key = &input[8..input.len() - 1];
                        previous_outputs
                            .get(key)
                            .cloned()
                            .unwrap_or_else(|| format!("(missing output for '{}')", key))
                    } else {
                        input.clone()
                    };

                    print!("Execute {}: `{}`? (Y/n): ", name, resolved_input);
                    stdout().flush().unwrap();
                    let mut line = String::new();
                    stdin().read_line(&mut line).unwrap();
                    let line = line.trim();
                    if line == "n" || line == "N" {
                        println!("Skipped {}\n", name);
                        continue;
                    }

                    match self.context.get_tool(name) {
                        Some(tool) => {
                            let result = tool.execute(&resolved_input);

                            self.context.log(
                                &format!("tool: {}", name),
                                &format!(
                                    "[input] {}\n[output] {}",
                                    resolved_input,
                                    result.output.clone().unwrap_or_default()
                                ),
                            );

                            if result.success {
                                if let Some(output) = result.output.clone() {
                                    previous_outputs.insert(name.clone(), output.clone());
                                    combined_output.push_str(&output);
                                    combined_output.push('\n');
                                }
                            } else {
                                let error_msg =
                                    result.error.clone().unwrap_or("Unknown error".to_string());
                                errors.push(error_msg.clone());

                                // 🎯 DYNAMIC INTELLIGENCE: Classify tool failures by criticality
                                // Core tools (run_command) are critical, auxiliary tools (reflect) are not
                                let is_critical = match name.as_str() {
                                    "run_command" => true,    // Core execution tool
                                    "reflect" => false,       // Auxiliary analysis tool
                                    "analyze_error" => false, // Auxiliary analysis tool
                                    _ => true, // Default to critical for unknown tools
                                };

                                if is_critical {
                                    critical_failures += 1;
                                }

                                // Log detailed error for replanner to see
                                self.context.log(
                                    "execution_error",
                                    &format!("Tool '{}' failed: {}", name, error_msg),
                                );

                                // Use AI to analyze the error and suggest fixes (only for critical failures)
                                if is_critical {
                                    if let Some(analyzer) = self.context.get_tool("analyze_error") {
                                        let analysis_result = analyzer.execute(&error_msg);
                                        if analysis_result.success {
                                            if let Some(analysis) = analysis_result.output {
                                                self.context.log("error_analysis", &analysis);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            critical_failures += 1;
                            errors.push(format!("Tool not found: {}", name));
                        }
                    }
                }
                PlanStep::Info(message) => {
                    combined_output.push_str(&format!("[INFO] {}\n", message));
                    self.context.log("info", message);
                }
            }
        }

        self.model.set_output(combined_output.trim().to_string());

        // 🎯 DYNAMIC INTELLIGENCE: Success based on critical tool performance
        // If core tools succeeded, the plan succeeded even if auxiliary tools failed
        let success = critical_failures == 0;

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

    fn replan(&mut self, reflection: &str) -> Option<Plan> {
        if let Some(replanner) = &self.replanner {
            self.context
                .log("replanner", "Using reflection-based replanning");
            let plan =
                replanner.generate_followup_plan(&mut self.context, &self.model.goal, reflection);
            if !plan.steps.is_empty() {
                Some(plan)
            } else {
                None
            }
        } else {
            None
        }
    }
}
