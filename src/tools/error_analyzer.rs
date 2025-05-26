use crate::tools::llm::LLMTool;
use crate::tools::{Tool, ToolResult, ToolSpec};

pub struct ErrorAnalyzerTool {
    llm: LLMTool,
}

impl ErrorAnalyzerTool {
    pub fn new(llm: LLMTool) -> Self {
        Self { llm }
    }
}

impl Tool for ErrorAnalyzerTool {
    fn name(&self) -> &str {
        "analyze_error"
    }

    fn description(&self) -> &str {
        "Analyzes command failures and suggests specific fixes"
    }

    fn execute(&self, input: &str) -> ToolResult {
        let prompt = format!(
            "You are an expert system administrator and developer. Analyze this command failure and suggest the exact commands needed to fix it AND complete the original goal.

ERROR OUTPUT:
{}

🚨 CRITICAL: Your fix_commands should include BOTH:
1. Commands to fix the immediate problem
2. Commands to retry/complete the original operation

For example:
- If git commit fails due to formatting → [\"cargo fmt\", \"git commit -m 'Fix formatting and commit changes'\"]
- If npm install fails → [\"npm cache clean --force\", \"npm install\"]
- If permission denied → [\"chmod +x script.sh\", \"./script.sh\"]

Respond with ONLY a JSON object in this format:
{{
  \"analysis\": \"Brief explanation of what went wrong\",
  \"fix_commands\": [\"fix_command\", \"retry_original_command\"],
  \"explanation\": \"Why these commands will fix the issue AND complete the goal\"
}}

Be specific and actionable. Always include the retry/completion step after the fix.",
            input
        );

        let result = self.llm.execute(&prompt);

        if result.success {
            // Try to extract JSON from the response
            if let Some(output) = result.output {
                // Simple JSON extraction - in production, use proper parsing
                if output.contains("fix_commands") {
                    ToolResult::success(&output)
                } else {
                    ToolResult::failure("LLM did not provide structured fix suggestions")
                }
            } else {
                ToolResult::failure("No output from error analysis")
            }
        } else {
            ToolResult::failure("Failed to analyze error with LLM")
        }
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().into(),
            description: self.description().into(),
            input_hint: "Error message or command output to analyze".into(),
            tags: vec!["error".into(), "analysis".into(), "fix".into()],
        }
    }
}
