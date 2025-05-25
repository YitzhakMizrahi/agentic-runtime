use crate::context::Context;
use crate::protocol::{Plan, PlanStep};
use crate::tools::Tool;
use crate::tools::llm::LLMTool;
use crate::validation::plan::validate_plan;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

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

                - "type": "tool" with "name" being one of:
                - "run_command" — executes a shell command like "git add ." or "git push"
                - "git_status" — runs `git status` (no input required)
                - "reflect" — summarizes previous output or memory (e.g. "input": "$output[git_status]")
                - "echo" — returns the input string as-is (for debug/info)

                - "type": "info" — includes a "message" string for progress narration

                Do **not invent** other types like "shell_command", "log", or "comment" — only `tool` and `info` are accepted.

                ---

                ### ✅ Output Format

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

        let cleaned = raw
            .lines()
            .filter(|line| {
                !line.trim_start().starts_with("```")
                    && !line.trim_start().starts_with("<think>")
                    && !line.trim_start().starts_with("</think>")
                    && !line.trim_start().starts_with("---")
                    && !line.trim_start().starts_with("### ")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let json = Regex::new(r#"(?s)\{\s*"plan"\s*:\s*\[.*?\]\s*\}"#)
            .unwrap()
            .find(&cleaned)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        context.log("replanner", "--- DEBUG: Extracted JSON block ---");
        context.log("replanner", &json);

        if !result.success {
            context.log("replanner", &format!("❌ Replanner LLM failed: {}", raw));
            return Plan {
                steps: vec![PlanStep::Info("Replanner LLM failed.".into())],
            };
        }

        let parsed_json: Value = match serde_json::from_str(&json) {
            Ok(val) => val,
            Err(e) => {
                context.log(
                    "replanner",
                    &format!(
                        "❌ Failed to parse raw JSON:\n{}\n\n[raw]: {}\n\n[cleaned]: {}",
                        e, raw, json
                    ),
                );
                return Plan {
                    steps: vec![PlanStep::Info("Failed to parse replanned output.".into())],
                };
            }
        };

        let plan_steps_json = parsed_json
            .get("plan")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let registered_tools = ["run_command", "git_status", "reflect", "echo"];
        let validation_errors = validate_plan(&plan_steps_json, &registered_tools);

        for error in validation_errors.iter() {
            let (msg, maybe_hint) = error.hint();
            context.log("replanner", &format!("⚠️ Validation warning: {}", msg));
            if let Some(hint) = maybe_hint {
                context.log("replanner", &format!("→ Hint: {}", hint));
            }
        }

        let response = serde_json::from_str::<ReplannerResponse>(&json);
        match response {
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
                        "❌ Failed to parse into ReplannerResponse:\n{}\n\n[raw]: {}\n\n[json]: {}",
                        e, raw, json
                    ),
                );
                Plan {
                    steps: vec![PlanStep::Info("Replanner JSON parse error.".into())],
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
