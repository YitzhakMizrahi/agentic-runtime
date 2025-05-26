use crate::context::Context;
use crate::protocol::{Plan, PlanStep};
use crate::tools::Tool;
use crate::tools::goal_analyzer::GoalAnalyzerTool;
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
    goal_analyzer: GoalAnalyzerTool,
}

impl LLMReplanner {
    pub fn new(llm: LLMTool) -> Self {
        let goal_analyzer = GoalAnalyzerTool::new(llm.clone());
        Self { llm, goal_analyzer }
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

        // 🎯 DYNAMIC INTELLIGENCE: Use GoalAnalyzerTool for context-aware recovery planning
        context.log("replanner", "Using dynamic LLM replanner");

        let (examples_text, output_format, critical_rules) = match self
            .goal_analyzer
            .analyze_context(goal, &memory_dump, true)
        {
            Ok(analysis) => {
                let examples = analysis
                    .examples
                    .iter()
                    .map(|ex| format!("// {}\n{}", ex.description, ex.json_plan))
                    .collect::<Vec<_>>()
                    .join("\n\n");

                (
                    examples,
                    analysis.output_format,
                    analysis.critical_rules.join("\n"),
                )
            }
            Err(e) => {
                context.log(
                    "replanner",
                    &format!(
                        "⚠️ GoalAnalyzer failed: {}, falling back to hardcoded examples",
                        e
                    ),
                );

                // Fallback to hardcoded examples
                let examples = r#"// Error recovery with fix commands
{"plan": [{"type": "tool", "name": "run_command", "input": "cargo fmt"}, {"type": "tool", "name": "run_command", "input": "git commit -m 'Fix formatting and commit changes'"}, {"type": "info", "message": "Goal completed"}]}"#.to_string();

                (examples, "Standard JSON recovery plan format".to_string(), "- Extract fix_commands from error analysis JSON\n- Apply fixes then retry original operation\n- Use linear sequences only".to_string())
            }
        };

        let prompt = format!(
            r#"You are an autonomous replanning agent. Analyze what went wrong and create a plan to complete the goal.

GOAL: {}

REFLECTION FROM PREVIOUS ATTEMPT:
{}

MEMORY LOG:
{}

DYNAMIC RECOVERY EXAMPLES FOR THIS CONTEXT:
{}

OUTPUT FORMAT: {}

CRITICAL RULES:
{}

🚨 CRITICAL FORMAT REQUIREMENTS 🚨
NEVER EVER use these INVALID formats:
❌ {{"type": "reflect"}} 
❌ {{"type": "run_command"}}
❌ {{"type": "analyze_error"}}

ALWAYS use these VALID formats:
✅ {{"type": "tool", "name": "reflect"}}
✅ {{"type": "tool", "name": "run_command"}}  
✅ {{"type": "tool", "name": "analyze_error"}}
✅ {{"type": "info", "message": "text"}}

UNIVERSAL RULES:
- If the reflection contains JSON with "fix_commands" array, use those EXACT commands first
- Then ALWAYS retry the original failed operation to complete the goal
- If reflection is plain text, analyze what failed and create appropriate recovery steps
- Complete the ENTIRE goal, not just fix the immediate problem
- For git commit failures: run fix commands, then ALWAYS retry git commit with proper message
- NEVER stop after just running the fix - ALWAYS complete the original goal
- Only "tool" and "info" are valid types
- Tool names: ONLY "run_command", "reflect", or "analyze_error"
- Plan ALL steps needed to complete the goal
- NO conditional logic (if/else) in JSON - create complete linear plan
- NO pseudo-code - ONLY valid JSON
- If goal is achieved, use: {{"type": "info", "message": "Goal achieved"}}
- NO markdown, NO explanations after JSON

⚠️ CRITICAL: Do NOT include any reflection analysis JSON in your response. Only output the plan JSON.

🚨 CRITICAL EXAMPLE: If reflection contains error analysis JSON like:
{{"analysis": "Command failed due to formatting", "fix_commands": ["cargo fmt"], "explanation": "..."}}

Then output (MUST include BOTH fix AND retry):
{{
  "plan": [
    {{"type": "tool", "name": "run_command", "input": "cargo fmt"}},
    {{"type": "tool", "name": "run_command", "input": "git commit -m 'Fix formatting and commit changes'"}},
    {{"type": "info", "message": "Goal completed"}}
  ]
}}

🚨 NEVER stop after just the fix command - ALWAYS retry the original operation!

OUTPUT ONLY this exact JSON structure (ignore any other formats in examples):
{{
  "plan": [
    {{"type": "tool", "name": "run_command", "input": "your_command_here"}},
    {{"type": "tool", "name": "reflect", "input": "$output[run_command]"}},
    {{"type": "info", "message": "Goal completed"}}
  ]
}}

STOP after outputting the JSON. NO other format is acceptable.
"#,
            goal, reflection, memory_dump, examples_text, output_format, critical_rules
        );

        let result = self.llm.execute(&prompt);
        let raw = result.output.unwrap_or_default();

        context.log("replanner", "--- DEBUG: Raw replanner output ---");
        context.log("replanner", &raw);

        // Extract everything after </think> tag if present, otherwise use full response
        let post_think = if raw.contains("</think>") {
            raw.split("</think>").last().unwrap_or(&raw)
        } else {
            &raw
        };

        let cleaned = post_think
            .lines()
            .filter(|line| {
                !line.trim_start().starts_with("```")
                    && !line.trim_start().starts_with("---")
                    && !line.trim_start().starts_with("### ")
                    && !line.trim().is_empty()
            })
            .collect::<Vec<_>>()
            .join("\n");

        // More robust JSON extraction - find the complete JSON object
        let mut json = Regex::new(r#"(?s)\{\s*\"plan\"\s*:\s*\[.*?\]\s*\}"#)
            .unwrap()
            .find(&cleaned)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        // 🎯 DYNAMIC INTELLIGENCE: Auto-fix common LLM format mistakes
        // Convert {"type": "tool_name"} to {"type": "tool", "name": "tool_name"}
        json = json
            .replace(
                r#""type": "run_command""#,
                r#""type": "tool", "name": "run_command""#,
            )
            .replace(
                r#""type": "reflect""#,
                r#""type": "tool", "name": "reflect""#,
            )
            .replace(
                r#""type": "analyze_error""#,
                r#""type": "tool", "name": "analyze_error""#,
            );

        // Remove JSON comments (// comments)
        let comment_regex = Regex::new(r#",?\s*//[^\n\r]*"#).unwrap();
        json = comment_regex.replace_all(&json, "").to_string();

        // Remove invalid step types (condition, etc.) - replace with info
        let invalid_types = ["condition", "check", "validate", "if", "when"];
        for invalid_type in invalid_types {
            let pattern = format!(r#""type": "{}""#, invalid_type);
            json = json.replace(&pattern, r#""type": "info""#);
        }

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

        let registered_tools = ["run_command", "reflect", "analyze_error"];
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
