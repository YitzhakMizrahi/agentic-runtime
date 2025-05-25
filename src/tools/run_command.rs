use crate::tools::{Tool, ToolResult, ToolSpec};
use std::process::Command;

pub struct RunCommandTool;

impl Tool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Runs a shell command and returns its stdout/stderr output."
    }

    fn execute(&self, input: &str) -> ToolResult {
        let output = Command::new("sh").arg("-c").arg(input).output();

        match output {
            Ok(out) => {
                let mut result = String::new();
                result.push_str(&String::from_utf8_lossy(&out.stdout));
                result.push_str(&String::from_utf8_lossy(&out.stderr));
                ToolResult::success(result.trim())
            }
            Err(e) => ToolResult::failure(&format!("Command execution failed: {e}")),
        }
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().into(),
            description: self.description().into(),
            input_hint: "Shell command to run (e.g. 'cargo check')".into(),
            tags: vec!["shell".into(), "command".into(), "execution".into()],
        }
    }
}
