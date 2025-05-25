// src/protocol/replanner.rs

use crate::context::Context;
use crate::protocol::{Plan, PlanStep};
use crate::tools::Tool;
use crate::tools::llm::LLMTool;
use regex::Regex;
use serde::Deserialize;

pub trait Replanner: Send + Sync {
    fn generate_followup_plan(&self, context: &mut Context, goal: &str, reflection: &str) -> Plan;
}

pub struct LLMReplanner {
    llm: LLMTool,
}

impl LLMReplanner {
    pub fn new(llm: LLMTool) -> Self {
        Self { llm }
    }
}

impl Replanner for LLMReplanner {
    fn generate_followup_plan(&self, context: &mut Context, goal: &str, reflection: &str) -> Plan {
        let memory_dump = context
            .memory()
            .entries
            .iter()
            .map(|(label, content)| format!("[{}] {}", label, content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"You are an autonomous replanning agent.
                
                Your job is to revise or extend the plan in response to the reflection summary. Output a **minimal, valid** plan in **strict JSON format**.
                
                ---
                
                ### 🔄 Context: Reflection
                
                {reflection}
                
                ---

                ### ❗ Allowed Step Types

                Only use:

                - `"type": "tool"` with `"name"` being one of:
                - `"run_command"` — executes a shell command like `"git add ."` or `"git push"`
                - `"git_status"` — runs `git status` (no input required)
                - `"reflect"` — summarizes previous output or memory (e.g. `"input": "$output[git_status]"`)
                - `"echo"` — returns the input string as-is (for debug/info)

                - `"type": "info"` — includes a `"message"` string for progress narration

                Do **not invent** other types like `"shell_command"`, `"log"`, or `"comment"` — only `tool` and `info` are accepted.

                ---

                ### ✅ Output Format

                Each step must be an object with:

                - `"type": "tool"` — for any tool invocation
                - `"name": "<tool_name>"` — the registered tool name (e.g., `run_command`, `git_status`)
                - `"input": "<string>"` — what to pass as input to the tool

                For example:
                
                ```json
                {{
                "plan": [
                    {{ "type": "tool", "name": "run_command", "input": "ls -la" }},
                    {{ "type": "tool", "name": "reflect", "input": "$output[run_command]" }},
                    {{ "type": "info", "message": "Listed directory and reflected on it." }}
                ]
                }}

                ❗ Do not use "type": "run_command" — always use "type": "tool" with "name": "run_command".
                ---
                
                ### 🧠 Memory Log
                
                {memory_dump}
                
                ---
                
                ### 🎯 Goal
                
                "{goal}"
                "#
        );

        let result = self.llm.execute(&prompt);
        let raw = result.output.unwrap_or_default();

        context.log("replanner", "--- DEBUG: Raw replanner output ---");
        context.log("replanner", &raw);

        let json = Regex::new(r"\{[\s\S]*\}")
            .unwrap()
            .find(&raw)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        context.log("replanner", "--- DEBUG: Extracted JSON block ---");
        context.log("replanner", &json);

        match result.success {
            true => match serde_json::from_str::<ReplannerResponse>(&json) {
                Ok(parsed) => Plan {
                    steps: parsed
                        .plan
                        .into_iter()
                        .map(|step| match step {
                            ReplannerStep::Tool { name, input } => PlanStep::ToolCall {
                                name,
                                input: input.unwrap_or_default(),
                            },
                            ReplannerStep::Info { message } => PlanStep::Info(message),
                        })
                        .collect(),
                },
                Err(e) => {
                    context.log(
                        "replanner",
                        &format!(
                            "❌ Failed to parse replanned output:\n{}\n\n[raw]: {}\n\n[cleaned]: {}",
                            e, raw, json
                        ),
                    );
                    Plan {
                        steps: vec![PlanStep::Info("Failed to parse replanned output.".into())],
                    }
                }
            },
            false => {
                context.log("replanner", &format!("❌ Replanner LLM failed: {}", raw));
                Plan {
                    steps: vec![PlanStep::Info("Replanner LLM failed.".into())],
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct ReplannerResponse {
    #[serde(default)]
    plan: Vec<ReplannerStep>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ReplannerStep {
    #[serde(rename = "tool")]
    Tool {
        name: String,
        #[serde(default)]
        input: Option<String>,
    },
    #[serde(rename = "info")]
    Info { message: String },
}
