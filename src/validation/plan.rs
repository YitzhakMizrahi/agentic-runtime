// src/validation/plan.rs

use serde_json::{Value, json};

#[derive(Debug)]
pub enum PlanValidationError {
    UnknownType(String),
    DuplicateKey(&'static str),
    MissingField(&'static str),
    InvalidTool(String),
    InvalidReference(String),
    ToolInputMismatch { tool: String, reason: String },
    RegexError(String),
    StyleWarning(String),
}

impl PlanValidationError {
    pub fn hint(&self) -> (String, Option<Value>) {
        match self {
            PlanValidationError::UnknownType(_) => (
                "Unknown step type. Only 'tool' or 'info' are valid.".to_string(),
                Some(json!({ "type": "tool", "name": "example_tool", "input": "..." })),
            ),
            PlanValidationError::DuplicateKey(_) => (
                "Duplicate key in step. Only one of each key is allowed.".to_string(),
                None,
            ),
            PlanValidationError::MissingField(field) => (
                "Missing required field.".to_string(),
                Some(json!({ field.to_string(): "<required>" })),
            ),
            PlanValidationError::InvalidTool(name) => (
                "Unknown tool used. Make sure it's registered.".to_string(),
                Some(json!({ "name": name, "input": "..." })),
            ),
            PlanValidationError::InvalidReference(var) => (
                "Reference to output of nonexistent step.".to_string(),
                Some(json!({ "reference": var })),
            ),
            PlanValidationError::ToolInputMismatch { tool, reason } => (
                "Tool input is invalid or unsafe.".to_string(),
                Some(json!({ "tool": tool, "reason": reason })),
            ),
            PlanValidationError::RegexError(desc) => (
                "Regex error in condition.".to_string(),
                Some(json!({ "error": desc })),
            ),
            PlanValidationError::StyleWarning(msg) => (msg.clone(), None),
        }
    }
}

pub fn validate_plan(plan: &[Value], registered_tools: &[&str]) -> Vec<PlanValidationError> {
    let mut errors = Vec::new();

    for step in plan {
        let Some(step_type) = step.get("type") else {
            errors.push(PlanValidationError::MissingField("type"));
            continue;
        };
        let Some(step_type_str) = step_type.as_str() else {
            errors.push(PlanValidationError::ToolInputMismatch {
                tool: "<unknown>".to_string(),
                reason: "Field 'type' must be a string".to_string(),
            });
            continue;
        };

        match step_type_str {
            "tool" => {
                let Some(name) = step.get("name").and_then(|v| v.as_str()) else {
                    errors.push(PlanValidationError::MissingField("name"));
                    continue;
                };

                if !registered_tools.contains(&name) {
                    errors.push(PlanValidationError::InvalidTool(name.to_string()));
                }

                if name != "git_status" && step.get("input").is_none() {
                    errors.push(PlanValidationError::MissingField("input"));
                }

                if let Some(input) = step.get("input").and_then(|v| v.as_str()) {
                    if input.contains('<') && input.contains('>') {
                        errors.push(PlanValidationError::ToolInputMismatch {
                            tool: name.to_string(),
                            reason: "Input contains placeholder like <file>".to_string(),
                        });
                    }
                }
            }
            "info" => {
                if step.get("message").is_none() {
                    errors.push(PlanValidationError::MissingField("message"));
                }
            }
            unknown => {
                errors.push(PlanValidationError::UnknownType(unknown.to_string()));
            }
        }
    }

    errors
}
