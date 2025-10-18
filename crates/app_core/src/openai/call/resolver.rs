use serde_json::Value;

use crate::openai::tools::ToolDefinition;

use super::types::{ToolCallDecision, ToolResolution};

/// Resolve and (if needed) execute the proposed tool call against known tools.
pub fn resolve_and_execute_tool_call(
    decision: ToolCallDecision,
    tools: &[ToolDefinition],
) -> ToolResolution {
    match decision {
        ToolCallDecision::Text(t) => ToolResolution::ModelText(t),
        ToolCallDecision::ToolCall { name, arguments } => {
            let tool = match tools.iter().find(|d| d.name == name) {
                Some(t) => t,
                None => return ToolResolution::ToolNotFound { requested: name },
            };
            let parsed: Value = match serde_json::from_str(&arguments) {
                Ok(v) => v,
                Err(e) => {
                    return ToolResolution::ArgumentsParseError {
                        name: tool.name.to_string(),
                        raw: arguments,
                        error: e.to_string(),
                    };
                }
            };
            match tool.execute(&parsed) {
                Ok(v) => ToolResolution::Executed { name: tool.name.to_string(), result: v },
                Err(e) => ToolResolution::ExecutionError { name: tool.name.to_string(), error: e.to_string() },
            }
        }
    }
}
