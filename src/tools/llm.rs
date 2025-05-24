// src/tools/llm.rs

use crate::tools::{Tool, ToolResult, ToolSpec};
use serde_json::{Value, json};

pub struct LLMTool {
    pub model: String,
}

impl LLMTool {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
        }
    }
}

impl Default for LLMTool {
    fn default() -> Self {
        Self::new("llama3")
    }
}

impl Tool for LLMTool {
    fn name(&self) -> &str {
        "llm"
    }

    fn description(&self) -> &str {
        "Sends input to a local LLM via Ollama and returns the response."
    }

    fn execute(&self, input: &str) -> ToolResult {
        let client = reqwest::blocking::Client::new();
        let url = "http://localhost:11434/api/generate";

        let payload = json!({
            "model": self.model,
            "prompt": input,
            "stream": false
        });

        let response = client.post(url).json(&payload).send();

        match response {
            Ok(resp) => match resp.json::<Value>() {
                Ok(json) => {
                    if let Some(text) = json.get("response").and_then(|v| v.as_str()) {
                        ToolResult::success(text.trim())
                    } else {
                        ToolResult::failure("LLM response missing 'response' field")
                    }
                }
                Err(err) => ToolResult::failure(&format!("Failed to parse JSON: {err}")),
            },
            Err(err) => ToolResult::failure(&format!("Request failed: {err}")),
        }
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().into(),
            description: self.description().into(),
            input_hint: "Freeform prompt text to send to LLM.".into(),
            tags: vec!["llm".into(), "generation".into(), "reasoning".into()],
        }
    }
}
