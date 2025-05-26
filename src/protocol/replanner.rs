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

Before proposing a new plan, compare the reflection with the original goal.

🛑 If the reflection clearly shows the goal was achieved, respond with:
{{ "plan": [ {{ "type": "info", "message": "Goal achieved. No further action required." }} ] }}


---

### 🔄 Reflection

{reflection}

---

### 🧠 Memory Log

{memory_dump}

---

### 🎯 Goal

{goal}

---

### 🧪 Output Format (STRICT JSON)

{{
  "plan": [
    {{ "type": "tool", "name": "run_command", "input": "cargo check" }},
    {{ "type": "tool", "name": "reflect", "input": "$output[run_command]" }},
    {{ "type": "info", "message": "Check complete and reflected." }}
  ]
}}

### 🚫 Constraints

- Only use `tool` and `info` as valid step types.
- Never use `loop`, `shell`, `comment`, or other custom formats.
- All tools must use the format: {{ "type": "tool", "name": "<tool>", "input": "<input>" }}
- Supported tools: `run_command`, `reflect`
- Respond with JSON ONLY — do not explain anything outside the JSON block.
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

        let json = Regex::new(r#"(?s)\{\s*\"plan\"\s*:\s*\[.*?\]\s*\}"#)
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

        let registered_tools = ["run_command", "reflect"];
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
