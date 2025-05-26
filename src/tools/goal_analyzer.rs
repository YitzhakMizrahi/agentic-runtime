use crate::tools::llm::LLMTool;
use crate::tools::{Tool, ToolResult, ToolSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalAnalysis {
    pub goal_type: String,
    pub context_type: String, // "initial_planning" | "error_recovery" | "continuation"
    pub tool_sequence: Vec<String>,
    pub examples: Vec<PlanExample>,
    pub output_format: String,
    pub critical_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExample {
    pub description: String,
    pub json_plan: String,
}

pub struct GoalAnalyzerTool {
    llm: LLMTool,
}

impl GoalAnalyzerTool {
    pub fn new(llm: LLMTool) -> Self {
        Self { llm }
    }

    pub fn analyze_context(
        &self,
        goal: &str,
        memory_log: &str,
        is_replanning: bool,
    ) -> Result<GoalAnalysis, String> {
        let context_type = if is_replanning {
            if memory_log.contains("error_analysis")
                || memory_log.contains("execution_error")
                || memory_log.contains("Command failed")
            {
                "error_recovery"
            } else {
                "continuation"
            }
        } else {
            "initial_planning"
        };

        let prompt = format!(
            r#"You are a meta-planning agent that analyzes goals and generates appropriate planning patterns.

GOAL: {}
CONTEXT_TYPE: {}
MEMORY_LOG:
{}

TASK: Analyze this goal and context to generate:
1. Goal type (git_operations, file_management, error_recovery, api_calls, etc.)
2. Appropriate tool sequence for this goal type (SIMPLE STRING ARRAY)
3. 2-3 concrete examples in JSON format
4. Custom output format instructions
5. Context-specific critical rules

AVAILABLE_TOOLS: ["run_command", "reflect", "analyze_error"]

OUTPUT ONLY this JSON structure:
{{
  "goal_type": "descriptive_goal_type",
  "context_type": "{}",
  "tool_sequence": ["run_command", "reflect", "run_command"],
  "examples": [
    {{
      "description": "Example description",
      "json_plan": "{{\\\"plan\\\": [{{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"run_command\\\", \\\"input\\\": \\\"git status\\\"}}, {{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"reflect\\\", \\\"input\\\": \\\"$output[run_command]\\\"}}, {{\\\"type\\\": \\\"info\\\", \\\"message\\\": \\\"Goal completed\\\"}}]}}"
    }}
  ],
  "output_format": "Specific instructions for JSON output format",
  "critical_rules": ["Rule 1", "Rule 2", "Rule 3"]
}}

🚨 CRITICAL FORMAT REQUIREMENTS FOR EXAMPLES 🚨
The json_plan field in examples MUST use this EXACT format:

INVALID (NEVER USE):
❌ {{\"type\": \"run_command\"}}
❌ {{\"type\": \"reflect\"}}  
❌ {{\"type\": \"analyze_error\"}}

VALID (ALWAYS USE):
✅ {{\"type\": \"tool\", \"name\": \"run_command\"}}
✅ {{\"type\": \"tool\", \"name\": \"reflect\"}}
✅ {{\"type\": \"tool\", \"name\": \"analyze_error\"}}
✅ {{\"type\": \"info\", \"message\": \"text\"}}

EXAMPLE TEMPLATE (copy this format exactly):
"json_plan": "{{\\\"plan\\\": [{{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"run_command\\\", \\\"input\\\": \\\"git status\\\"}}, {{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"reflect\\\", \\\"input\\\": \\\"$output[run_command]\\\"}}, {{\\\"type\\\": \\\"info\\\", \\\"message\\\": \\\"Goal completed\\\"}}]}}"

CONCRETE GIT EXAMPLE:
"json_plan": "{{\\\"plan\\\": [{{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"run_command\\\", \\\"input\\\": \\\"git status --porcelain\\\"}}, {{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"reflect\\\", \\\"input\\\": \\\"$output[run_command]\\\"}}, {{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"run_command\\\", \\\"input\\\": \\\"git add .\\\"}}, {{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"run_command\\\", \\\"input\\\": \\\"git commit -m 'Update files'\\\"}}, {{\\\"type\\\": \\\"info\\\", \\\"message\\\": \\\"Goal completed\\\"}}]}}"

🚨 ABSOLUTELY FORBIDDEN IN EXAMPLES 🚨
❌ NEVER use: "type": "conditional"
❌ NEVER use: "if", "then", "else" 
❌ NEVER use: "test", "when", "check"
❌ NEVER use: pseudo-code or variables like $output[reflect]

✅ ONLY ALLOWED TYPES:
- "type": "tool" (with "name" and "input")
- "type": "info" (with "message")

ADDITIONAL REQUIREMENTS:
- tool_sequence MUST be simple string array: ["run_command", "reflect", "analyze_error"]
- For error_recovery context, focus on fix_commands from error_analysis AND retry original operation
- For git operations, include complete workflow (status, add, commit)
- For file operations, include validation steps
- Always include linear sequences, no conditionals
- Examples must be valid JSON strings (escaped quotes)
- Only \"tool\" and \"info\" are valid types in examples
- Each example must be a complete, executable linear plan

🚨 ERROR RECOVERY PATTERN 🚨
For error_recovery context, examples should follow this pattern:
1. Extract fix commands from error_analysis JSON in memory_log
2. Run each fix command
3. ALWAYS retry the original failed operation
4. Complete the goal

Example error recovery pattern:
"json_plan": "{{\\\"plan\\\": [{{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"run_command\\\", \\\"input\\\": \\\"cargo fmt\\\"}}, {{\\\"type\\\": \\\"tool\\\", \\\"name\\\": \\\"run_command\\\", \\\"input\\\": \\\"git commit -m 'Fix formatting and commit changes'\\\"}}, {{\\\"type\\\": \\\"info\\\", \\\"message\\\": \\\"Goal completed\\\"}}]}}"
"#,
            goal, context_type, memory_log, context_type
        );

        let result = self.llm.execute(&prompt);

        if !result.success {
            return Err(format!("LLM execution failed: {:?}", result.error));
        }

        let response = result.output.unwrap_or_default();

        // Extract JSON from response (handle thinking model output)
        let post_think = if response.contains("</think>") {
            response.split("</think>").last().unwrap_or(&response)
        } else {
            &response
        };

        let json_start = post_think.find('{').unwrap_or(0);
        let json_end = post_think
            .rfind('}')
            .map(|i| i + 1)
            .unwrap_or(post_think.len());
        let json_str = &post_think[json_start..json_end];

        if json_str.trim().is_empty() {
            return Err(format!("No JSON found in response: {}", response));
        }

        match serde_json::from_str::<GoalAnalysis>(json_str) {
            Ok(analysis) => Ok(analysis),
            Err(e) => Err(format!(
                "Failed to parse goal analysis JSON: {} | JSON: {}",
                e, json_str
            )),
        }
    }
}

impl Tool for GoalAnalyzerTool {
    fn name(&self) -> &str {
        "analyze_goal"
    }

    fn description(&self) -> &str {
        "Analyzes goals and generates appropriate planning patterns, examples, and output formats dynamically."
    }

    fn execute(&self, input: &str) -> ToolResult {
        // Input format: "goal|memory_log|is_replanning"
        let parts: Vec<&str> = input.split('|').collect();
        if parts.len() != 3 {
            return ToolResult::failure("Input must be: goal|memory_log|is_replanning");
        }

        let goal = parts[0];
        let memory_log = parts[1];
        let is_replanning = parts[2] == "true";

        match self.analyze_context(goal, memory_log, is_replanning) {
            Ok(analysis) => match serde_json::to_string_pretty(&analysis) {
                Ok(json) => ToolResult::success(&json),
                Err(e) => ToolResult::failure(&format!("Failed to serialize analysis: {}", e)),
            },
            Err(e) => ToolResult::failure(&e),
        }
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().into(),
            description: self.description().into(),
            input_hint: "goal|memory_log|is_replanning (e.g., 'commit changes|[memory]|false')"
                .into(),
            tags: vec!["meta".into(), "planning".into(), "analysis".into()],
        }
    }
}
