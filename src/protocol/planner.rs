// src/protocol/planner.rs

use crate::context::Context;
use crate::protocol::{Plan, PlanStep};
use crate::tools::Tool;
use crate::tools::llm::LLMTool;
use regex::Regex;
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
        // Format memory for the LLM context
        let memory_dump = context
            .memory()
            .entries
            .iter()
            .map(|(label, content)| format!("[{}] {}", label, content))
            .collect::<Vec<_>>()
            .join("\n");

        // Planner prompt
        let prompt = format!(
            r#"You are an autonomous planning agent.
        
        Your job is to generate a precise, minimal plan in **strict JSON** format to achieve the given goal.
        
        ---
        
        ### ✅ Constraints
        
        - Only include steps that are **directly** required to accomplish the goal.
        - Avoid redundant or unrelated actions (e.g. DO NOT run `git_status` unless explicitly asked).
        - The `"type"` field must be **either `"tool"` or `"info"`**.
        - Do NOT invent new types like `"reflect"` — use `"type": "tool", "name": "reflect"` instead.
        - Do NOT include commentary — respond with **strict JSON only**.
        
        ---
        
        ### 🧠 Variable Usage Guidelines
        
        - Use `$output[tool_name]` only when referring to the output of a previous step.
        - NEVER use `$output[...]` as a shell command itself unless the output is known to be a valid command.
        - Do NOT construct new shell commands using `$output[...] + some-text`. Instead, provide the full intended command literally (e.g. `"cargo audit"`).
        - If the output is uncertain or messy, pass it to `"reflect"` instead of `"run_command"`.
        
        ---
        
        ### 🧰 Available Tools
        
        - **run_command**: Executes a shell command.
          - Input must be a valid shell command (e.g. `"cargo check"`, `"ls -la"`, `"curl example.com"`).
          - ❗ Do NOT invent new tools like `"cargo audit"` — use `run_command` with `"cargo audit"` as the input.
        
        - **git_status**: Returns the output of `git status`. Use ONLY if the goal explicitly involves Git.
        
        - **reflect**: Summarizes the memory log. Input should be plain text or a previous output (e.g. `$output[run_command]`).
        
        - **echo**: Returns the input string unchanged. Useful for debug, status, or logging.
        
        ---
        
        ### 🧪 Output Format (Strict JSON Only)
        
        {{
          "plan": [
            {{ "type": "tool", "name": "run_command", "input": "cargo check" }},
            {{ "type": "tool", "name": "reflect", "input": "$output[run_command]" }},
            {{ "type": "info", "message": "Now reflecting on results." }}
          ]
        }}
        
        ---
        
        ### 🧠 Goal:
        
        "{goal}"
        
        ---
        
        ### 🗃️ Memory Context:
        
        {memory_dump}
        "#
        );

        let result = self.llm.execute(&prompt);
        let raw = result.output.unwrap_or_default();

        // 🧠 Extract the first valid JSON block using regex
        let json = Regex::new(r"\{[\s\S]*\}")
            .unwrap()
            .find(&raw)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        match result.success {
            true => match serde_json::from_str::<PlannerResponse>(&json) {
                Ok(parsed) => Plan {
                    steps: parsed
                        .plan
                        .into_iter()
                        .map(|step| match step {
                            PlannerStep::Tool { name, input } => PlanStep::ToolCall { name, input },
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
                context.log("planner", &format!("❌ LLM execution failed: {}", raw));
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
    Tool { name: String, input: String },
    #[serde(rename = "info")]
    Info { message: String },
}
