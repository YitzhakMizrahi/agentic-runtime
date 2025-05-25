// src/protocol/planner.rs

use crate::context::Context;
use crate::protocol::{Plan, PlanStep};
use crate::tools::Tool;
use crate::tools::llm::LLMTool;
use regex::Regex;
use serde::Deserialize;

pub trait Planner: Send + Sync {
    fn generate_plan(&self, context: &mut Context, goal: &str) -> Plan;
}

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
        let memory_dump = context
            .memory()
            .entries
            .iter()
            .map(|(label, content)| format!("[{}] {}", label, content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"You are an autonomous planning agent.
                
                Your task is to produce a precise, minimal action plan in **strict JSON format** to accomplish the goal below.
                
                ---
                
                ### ❗ Allowed Step Types
                
                You may only use:
                
                - `"type": "tool"` with one of the following tool names:
                  - `"run_command"` – to execute a shell command (e.g. `"ls -la"`, `"git push"`)
                  - `"git_status"` – runs `git status` (no input needed)
                  - `"reflect"` – summarizes memory log (e.g. `"input": "$output[run_command]"`)
                  - `"echo"` – returns the string in `input` (for logging/debug)
                
                - `"type": "info"` with a `message` field to narrate or annotate progress
                
                ⚠️ Do **not** use other types like `"shell_command"`, `"log"`, `"reflect"` as a separate type, or any invented variant.
                
                ---
                
                ### ✅ JSON Output Format

                Each step must be an object with:

                - `"type": "tool"` — for any tool invocation
                - `"name": "<tool_name>"` — the registered tool name (e.g., `run_command`, `git_status`)
                - `"input": "<string>"` — what to pass as input to the tool

                For example:
                
                ```json
                {{
                  "plan": [
                    {{ "type": "tool", "name": "git_status" }},
                    {{ "type": "tool", "name": "run_command", "input": "git add ." }},
                    {{ "type": "tool", "name": "run_command", "input": "git push" }},
                    {{ "type": "info", "message": "Changes staged and pushed." }}
                  ]
                }}
             
                ❗ Do not use "type": "run_command" — always use "type": "tool" with "name": "run_command".
                ---
                
                ### 🧩 Context: Memory Log
                
                {memory_dump}
                
                ---
                
                ### 🧭 Goal
                
                "{goal}"
                "#
        );

        let result = self.llm.execute(&prompt);
        let raw = result.output.unwrap_or_default();

        context.log("planner", "--- DEBUG: Raw planner output ---");
        context.log("planner", &raw);

        let json = Regex::new(r"\{[\s\S]*\}")
            .unwrap()
            .find(&raw)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        context.log("planner", "--- DEBUG: Extracted JSON block ---");
        context.log("planner", &json);

        match result.success {
            true => match serde_json::from_str::<PlannerResponse>(&json) {
                Ok(parsed) => Plan {
                    steps: parsed
                        .plan
                        .into_iter()
                        .map(|step| match step {
                            PlannerStep::Tool { name, input } => PlanStep::ToolCall {
                                name,
                                input: input.unwrap_or_default(),
                            },
                            PlannerStep::Info { message } => PlanStep::Info(message),
                        })
                        .collect(),
                },
                Err(e) => {
                    context.log(
                        "planner",
                        &format!(
                            "❌ Failed to parse plan:\n{}\n\n[raw]: {}\n\n[cleaned]: {}",
                            e, raw, json
                        ),
                    );
                    Plan {
                        steps: vec![PlanStep::Info("Failed to parse structured plan.".into())],
                    }
                }
            },
            false => {
                context.log("planner", &format!("❌ Planner LLM failed: {}", raw));
                Plan {
                    steps: vec![PlanStep::Info("Planner LLM failed.".into())],
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct PlannerResponse {
    #[serde(default)]
    plan: Vec<PlannerStep>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum PlannerStep {
    #[serde(rename = "tool")]
    Tool {
        name: String,
        #[serde(default)]
        input: Option<String>,
    },
    #[serde(rename = "info")]
    Info { message: String },
}
