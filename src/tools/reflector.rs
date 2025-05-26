// src/tools/reflector.rs

use crate::tools::llm::LLMTool;
use crate::tools::{Tool, ToolResult, ToolSpec};

pub struct ReflectorTool {
    pub llm: LLMTool,
}

impl ReflectorTool {
    pub fn new(llm: LLMTool) -> Self {
        Self { llm }
    }
}

impl Tool for ReflectorTool {
    fn name(&self) -> &str {
        "reflect"
    }

    fn description(&self) -> &str {
        "Analyzes a memory log and generates a reflection summary using LLM."
    }

    fn execute(&self, input: &str) -> ToolResult {
        let prompt = format!(
            r#"You are a reflection module embedded in an autonomous agent runtime.

Given the following memory log, produce a structured reflection that summarizes what the agent tried to do, what happened, what failed (if anything), and what could be improved next time.

---

# 🧠 Reflection Summary

## Memory Log
{input}

## Summary (fill in below):

## What was the agent trying to do?
- 

## What steps did the agent take?
- 

## What worked well?
- 

## What failed or could be improved?
- 

## Suggested improvements:
- 
"#
        );

        let result = self.llm.execute(&prompt);
        match result.success {
            true => ToolResult::success(&result.output.unwrap_or_else(|| "(no output)".into())),
            false => ToolResult::failure("LLM failed to generate reflection."),
        }
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().into(),
            description: self.description().into(),
            input_hint: "Pass memory log and goal as plain text.".into(),
            tags: vec!["introspection".into(), "reflection".into(), "llm".into()],
        }
    }
}
