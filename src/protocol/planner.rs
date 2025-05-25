// src/protocol/planner.rs

use crate::context::Context;
use crate::protocol::{Plan, PlanStep};
use crate::tools::Tool;
use crate::tools::llm::LLMTool;

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
        // Step 1: Serialize memory
        let memory_dump = context
            .memory()
            .entries
            .iter()
            .map(|(label, content)| format!("[{}] {}", label, content))
            .collect::<Vec<_>>()
            .join("\n");

        // Step 2: Compose planning prompt
        let prompt = format!(
            r#"You are an agentic planner. Your job is to create a list of steps to reach the following goal.

Goal:
{}

Based on prior memory:
{}

Use only these available tools:
- git_status
- reflect
- echo

Output format:
1. git_status
2. reflect
3. Info: Optional message here
"#,
            goal, memory_dump
        );

        // Step 3: Call LLMTool
        let result = self.llm.execute(&prompt);

        // Step 4: Parse result into a Plan
        match result.success {
            true => parse_plan_from_response(result.output.unwrap_or_default().as_str()),
            false => {
                context.log(
                    "planner",
                    &format!("Planning failed: {}", result.output.unwrap_or_default()),
                );
                Plan {
                    steps: vec![PlanStep::Info("Planning failed.".into())],
                }
            }
        }
    }
}

fn parse_plan_from_response(response: &str) -> Plan {
    let mut steps = vec![];
    let known_tools = ["git_status", "reflect", "echo"];

    for line in response.lines() {
        let line = line.trim();

        if let Some(stripped) = line.strip_prefix("Info:") {
            steps.push(PlanStep::Info(stripped.trim().to_string()));
        } else if let Some((_n, rest)) = line.split_once('.') {
            let mut content = rest.trim().to_string();

            // Normalize potential tool name
            content = content
                .trim_matches('`') // remove backticks
                .replace(' ', "_") // "git status" → "git_status"
                .to_lowercase();

            if known_tools.contains(&content.as_str()) {
                steps.push(PlanStep::ToolCall(content));
            } else {
                // Fallback: treat as info step
                steps.push(PlanStep::Info(rest.trim().to_string()));
            }
        }
    }

    Plan { steps }
}
