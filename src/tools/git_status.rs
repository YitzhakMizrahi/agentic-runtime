// src/tools/git_status.rs

use crate::tools::{Tool, ToolResult};
use std::process::Command;

pub struct GitStatusTool;

impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }

    fn description(&self) -> &str {
        "Runs 'git status' in the current directory"
    }

    fn execute(&self, _input: &str) -> ToolResult {
        match Command::new("git").arg("status").output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    ToolResult::success(&stdout)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    ToolResult::failure(&format!("Git error: {}", stderr.trim()))
                }
            }
            Err(e) => ToolResult::failure(&format!("Failed to run git: {}", e)),
        }
    }
}
