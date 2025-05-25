// src/protocol/planner.rs

use crate::context::Context;
use crate::protocol::{Plan, PlanStep};
use crate::tools::Tool;
use crate::tools::llm::LLMTool;
use serde::Deserialize;

/// Trait for generating a Plan from a goal + current context.
pub trait Planner: Send + Sync {
    fn generate_plan(&self, context: &mut Context, goal: &str) -> Plan;
}

/// Implementation using a local LLMTool + structured prompt.
pub struct LLMPlanner {
    llm: LLMTool,
}

impl LLMPlanner {
    pub fn new(llm: LLMTool) -> Self {
        Self { llm }
    }
}

impl Planner for LLMPlanner {
    fn generate_plan(&self, context: &mut Context, goal: &str) -> Plan {
        // Step 1: Serialize memory for planning context
        let memory_dump = context
            .memory()
            .entries
            .iter()
            .map(|(label, content)| format!("[{}] {}", label, content))
            .collect::<Vec<_>>()
            .join("\n");

        // Step 2: Prompt the LLM with JSON instruction
        let prompt = format!(
            r#"You are an autonomous planning agent.

Your job is to generate a structured JSON plan using only the following tools:

- git_status
- reflect
- echo

Respond only with a JSON object in this format:

{{
  "plan": [
    {{ "type": "tool", "name": "git_status" }},
    {{ "type": "tool", "name": "reflect" }},
    {{ "type": "info", "message": "Reflect on git output." }}
  ]
}}

Goal: {}
Memory:
{}
"#,
            goal, memory_dump
        );

        // Step 3: Call the LLM
        let result = self.llm.execute(&prompt);

        // Step 4: Parse structured plan from JSON
        match result.success {
            true => {
                let json = result.output.unwrap_or_default();
                match serde_json::from_str::<PlannerResponse>(&json) {
                    Ok(parsed) => Plan {
                        steps: parsed
                            .plan
                            .into_iter()
                            .map(|step| match step {
                                PlannerStep::Tool { name } => PlanStep::ToolCall(name),
                                PlannerStep::Info { message } => PlanStep::Info(message),
                            })
                            .collect(),
                    },
                    Err(e) => {
                        context.log(
                            "planner",
                            &format!("❌ Failed to parse plan: {}\n{}", e, json),
                        );
                        Plan {
                            steps: vec![PlanStep::Info("Failed to parse structured plan.".into())],
                        }
                    }
                }
            }
            false => {
                context.log(
                    "planner",
                    &format!(
                        "❌ LLM execution failed: {}",
                        result.output.unwrap_or_default()
                    ),
                );
                Plan {
                    steps: vec![PlanStep::Info("LLM call failed.".into())],
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct PlannerResponse {
    plan: Vec<PlannerStep>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum PlannerStep {
    #[serde(rename = "tool")]
    Tool { name: String },
    #[serde(rename = "info")]
    Info { message: String },
}
