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
            r#"You are an autonomous agent replanner.

Your job is to generate a follow-up plan in strict JSON format based on:

- The original goal: "{goal}"
- The reflection: "{reflection}"

### Constraints:
- Only include steps that continue or improve on the previous plan.
- If the task is complete, return an empty plan: {{ "plan": [] }}
- Valid step types: "tool" or "info"
- You may reference prior tool outputs using $output[tool_name]

### Memory:
{memory_dump}

### Output format:
{{
  "plan": [
    {{ "type": "info", "message": "Next action: ..." }}
  ]
}}
"#
        );

        let result = self.llm.execute(&prompt);
        let raw = result.output.unwrap_or_default();

        let json = Regex::new(r"\{[\s\S]*\}")
            .unwrap()
            .find(&raw)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        match serde_json::from_str::<ReplannerResponse>(&json) {
            Ok(parsed) => Plan {
                steps: parsed
                    .plan
                    .into_iter()
                    .map(|step| match step {
                        ReplannerStep::Tool { name, input } => PlanStep::ToolCall { name, input },
                        ReplannerStep::Info { message } => PlanStep::Info(message),
                    })
                    .collect(),
            },
            Err(e) => {
                context.log(
                    "replanner",
                    &format!("\u{274c} Failed to parse replanned steps:\n{}\n{}", e, raw),
                );
                Plan { steps: vec![] }
            }
        }
    }
}

#[derive(Deserialize)]
struct ReplannerResponse {
    plan: Vec<ReplannerStep>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ReplannerStep {
    #[serde(rename = "tool")]
    Tool { name: String, input: String },
    #[serde(rename = "info")]
    Info { message: String },
}
