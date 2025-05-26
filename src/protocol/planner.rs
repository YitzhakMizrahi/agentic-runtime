use crate::context::Context;
use crate::protocol::{Plan, PlanStep};
use crate::tools::Tool;
use crate::tools::goal_analyzer::GoalAnalyzerTool;
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
    goal_analyzer: GoalAnalyzerTool,
}

impl LLMPlanner {
    pub fn new(llm: LLMTool) -> Self {
        let goal_analyzer = GoalAnalyzerTool::new(llm.clone());
        Self { llm, goal_analyzer }
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

        // 🎯 DYNAMIC INTELLIGENCE: Use GoalAnalyzerTool to generate context-aware examples
        context.log("planner", "Using dynamic LLM planner");

        let (examples_text, output_format, critical_rules) = match self
            .goal_analyzer
            .analyze_context(goal, &memory_dump, false)
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
                    "planner",
                    &format!(
                        "⚠️ GoalAnalyzer failed: {}, falling back to hardcoded examples",
                        e
                    ),
                );

                // Fallback to hardcoded examples
                let examples = r#"// Complete git workflow example
{"plan": [{"type": "tool", "name": "run_command", "input": "git status --porcelain"}, {"type": "tool", "name": "reflect", "input": "$output[run_command]"}, {"type": "tool", "name": "run_command", "input": "git add ."}, {"type": "tool", "name": "run_command", "input": "git commit -m 'Update files'"}, {"type": "info", "message": "Goal completed"}]}"#.to_string();

                (examples, "Standard JSON plan format with linear steps".to_string(), "- Use only linear sequences, no conditionals\n- Complete the entire git workflow\n- Use proper JSON format".to_string())
            }
        };

        let prompt = format!(
            r#"You are an autonomous planning agent. Think through the problem step by step, then generate ONLY valid JSON.

GOAL: {}

MEMORY LOG:
{}

AVAILABLE TOOLS:
- run_command: Execute shell commands (e.g. "git status", "git add .", "git commit -m 'message'")  
- reflect: Analyze text or tool outputs (input: text or "$output[tool_name]")
- analyze_error: Analyze errors and suggest fixes (input: error message)

DYNAMIC EXAMPLES FOR THIS GOAL TYPE:
{}

OUTPUT FORMAT: {}

CRITICAL RULES:
{}

🚨🚨🚨 CRITICAL FORMAT REQUIREMENTS 🚨🚨🚨
EVERY SINGLE STEP MUST USE THE CORRECT FORMAT!

❌❌❌ THESE ARE WRONG AND WILL CAUSE ERRORS ❌❌❌
{{"type": "reflect"}} 
{{"type": "run_command"}}
{{"type": "analyze_error"}}

✅✅✅ THESE ARE THE ONLY CORRECT FORMATS ✅✅✅
{{"type": "tool", "name": "reflect"}}
{{"type": "tool", "name": "run_command"}}  
{{"type": "tool", "name": "analyze_error"}}
{{"type": "info", "message": "text"}}

🔥 MANDATORY RULES FOR EVERY STEP 🔥
- EVERY tool step MUST have: "type": "tool", "name": "tool_name"
- NEVER use "type": "tool_name" - this is WRONG
- NEVER mix formats - be consistent throughout
- Only "tool" and "info" are valid types
- Tool names: ONLY "run_command", "reflect", or "analyze_error"

TEMPLATE TO COPY EXACTLY:
{{
  "plan": [
    {{"type": "tool", "name": "run_command", "input": "git status --porcelain"}},
    {{"type": "tool", "name": "reflect", "input": "$output[run_command]"}},
    {{"type": "tool", "name": "run_command", "input": "git add ."}},
    {{"type": "tool", "name": "run_command", "input": "git commit -m 'Update'"}},
    {{"type": "info", "message": "Goal completed"}}
  ]
}}

🚨 EVERY STEP MUST FOLLOW THIS EXACT PATTERN 🚨
STOP after outputting the JSON. NO other format is acceptable.
"#,
            goal, memory_dump, examples_text, output_format, critical_rules
        );

        let result = self.llm.execute(&prompt);
        let raw = result.output.unwrap_or_default();

        context.log("planner", "--- DEBUG: Raw planner output ---");
        context.log("planner", &raw);

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
        let mut json_str = Regex::new(r#"(?s)\{\s*"plan"\s*:\s*\[.*?\]\s*\}"#)
            .unwrap()
            .find(&cleaned)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        // 🎯 DYNAMIC INTELLIGENCE: Auto-fix common LLM format mistakes
        // Convert {"type": "tool_name"} to {"type": "tool", "name": "tool_name"}
        json_str = json_str
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
        json_str = comment_regex.replace_all(&json_str, "").to_string();

        // Remove invalid step types (condition, etc.) - replace with info
        let invalid_types = ["condition", "check", "validate", "if", "when"];
        for invalid_type in invalid_types {
            let pattern = format!(r#""type": "{}""#, invalid_type);
            json_str = json_str.replace(&pattern, r#""type": "info""#);
        }

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

        let registered_tools = ["run_command", "reflect", "analyze_error"];
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
