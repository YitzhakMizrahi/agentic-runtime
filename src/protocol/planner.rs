use crate::context::Context;
use crate::protocol::{Plan, PlanStep};
use crate::tools::Tool;
use crate::tools::llm::LLMTool;
use crate::validation::plan::validate_plan;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

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
                "You are an autonomous planning agent.
            
            Your job is to generate a minimal, valid action plan in strict JSON format to achieve the given goal.
            
            ---
            
            ### 🧪 Output Format (STRICT JSON)
            
            Only emit a JSON object using this structure:
            
            {{
              \"plan\": [
                {{ \"type\": \"tool\", \"name\": \"run_command\", \"input\": \"cargo check\" }},
                {{ \"type\": \"tool\", \"name\": \"reflect\", \"input\": \"$output[run_command]\" }},
                {{ \"type\": \"info\", \"message\": \"Compilation check complete.\" }}
              ]
            }}
            
            ---
            
            ### 🚫 Hard Constraints
            
            - `type` must be only `tool` or `info`.
            - `tool` steps must specify a `name` and `input`.
            - ❗ Do NOT use `type: \"reflect\"`. Instead, use `type: \"tool\", name: \"reflect\"`.
            - Never use: `loop`, `shell`, `comment`, or other made-up types.
            - Do not include markdown, commentary, or explanations.
            
            ---
            
            ### 🛠️ Available Tools
            
            - run_command: Executes a shell command (e.g. \"cargo check\", \"ls -la\").
            - reflect: Summarizes memory logs. Input can be text or a reference like `$output[...]`.
            
            ---
            
            ### 🤩 Context: Memory Log
            
            {}
            
            ---
            
            ### 🛍️ Goal
            
            \"{}\"
            ",
                memory_dump,
                goal
            );

        let result = self.llm.execute(&prompt);
        let raw = result.output.unwrap_or_default();

        context.log("planner", "--- DEBUG: Raw planner output ---");
        context.log("planner", &raw);

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

        let json_str = Regex::new(r#"(?s)\{\s*"plan"\s*:\s*\[.*?\]\s*\}"#)
            .unwrap()
            .find(&cleaned)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        context.log("planner", "--- DEBUG: Extracted JSON block ---");
        context.log("planner", &json_str);

        if !result.success {
            context.log("planner", &format!("❌ Planner LLM failed: {}", raw));
            return Plan {
                steps: vec![PlanStep::Info("Planner LLM failed.".into())],
            };
        }

        let parsed_json: Value = match serde_json::from_str(&json_str) {
            Ok(val) => val,
            Err(e) => {
                context.log(
                    "planner",
                    &format!(
                        "❌ Failed to parse raw JSON:\n{}\n\n[raw]: {}\n\n[cleaned]: {}",
                        e, raw, json_str
                    ),
                );
                return Plan {
                    steps: vec![PlanStep::Info("Failed to parse structured plan.".into())],
                };
            }
        };

        let plan_steps_json = parsed_json
            .get("plan")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let registered_tools = ["run_command", "reflect"];
        let validation_errors = validate_plan(&plan_steps_json, &registered_tools);

        for error in validation_errors.iter() {
            let (msg, maybe_hint) = error.hint();
            context.log("planner", &format!("⚠️ Validation warning: {}", msg));
            if let Some(hint) = maybe_hint {
                context.log("planner", &format!("→ Hint: {}", hint));
            }
        }

        let response = serde_json::from_str::<PlannerResponse>(&json_str);
        match response {
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
                        "❌ Failed to parse into PlannerResponse:\n{}\n\n[raw]: {}\n\n[json]: {}",
                        e, raw, json_str
                    ),
                );
                Plan {
                    steps: vec![PlanStep::Info("Planner JSON parse error.".into())],
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
