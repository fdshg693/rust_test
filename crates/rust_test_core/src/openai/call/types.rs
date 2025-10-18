use serde_json::Value;
use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallDecision {
    Text(String),
    ToolCall { name: String, arguments: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolResolution {
    ModelText(String),
    Executed { name: String, result: Value },
    ToolNotFound { requested: String },
    ArgumentsParseError { name: String, raw: String, error: String },
    ExecutionError { name: String, error: String },
}

impl ToolResolution {
    pub fn is_executed(&self) -> bool {
        matches!(self, ToolResolution::Executed { .. })
    }
}

impl Display for ToolCallDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolCallDecision::Text(t) => write!(f, "Text(len={}):\n{}", t.len(), t),
            ToolCallDecision::ToolCall { name, arguments } => {
                write!(f, "ToolCall name={} args={} (len={})", name, arguments, arguments.len())
            }
        }
    }
}

impl Display for ToolResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolResolution::ModelText(t) => write!(f, "ModelText(len={}):\n{}", t.len(), t),
            ToolResolution::Executed { name, result } => {
                write!(f, "Executed name={} result={} (json)", name, result)
            }
            ToolResolution::ToolNotFound { requested } => write!(f, "ToolNotFound requested={}", requested),
            ToolResolution::ArgumentsParseError { name, raw, error } => {
                write!(f, "ArgumentsParseError name={} error={} raw={}", name, error, raw)
            }
            ToolResolution::ExecutionError { name, error } => {
                write!(f, "ExecutionError name={} error={}", name, error)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultiStepAnswer {
    pub final_answer: String,
    pub steps: Vec<ToolResolution>,
    pub iterations: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub enum MultiStepLogEvent {
    IterationStart { iteration: usize },
    Proposed { iteration: usize, decision: ToolCallDecision },
    Resolved { iteration: usize, resolution: ToolResolution },
    HistoryFunctionAppended { iteration: usize, name: String, result: Value },
    FinalText { iteration: usize, text: String },
    EarlyFailure { iteration: usize, resolution: ToolResolution },
    Truncated { max_loops: usize },
}

impl Display for MultiStepLogEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MultiStepLogEvent::IterationStart { iteration } => write!(f, "IterationStart #{}", iteration),
            MultiStepLogEvent::Proposed { iteration, decision } => write!(f, "Proposed @{} => {}", iteration, decision),
            MultiStepLogEvent::Resolved { iteration, resolution } => write!(f, "Resolved @{} => {}", iteration, resolution),
            MultiStepLogEvent::HistoryFunctionAppended { iteration, name, result } => write!(f, "HistoryFunctionAppended @{} name={} result={}", iteration, name, result),
            MultiStepLogEvent::FinalText { iteration, text } => write!(f, "FinalText @{} len={}", iteration, text.len()),
            MultiStepLogEvent::EarlyFailure { iteration, resolution } => write!(f, "EarlyFailure @{} => {}", iteration, resolution),
            MultiStepLogEvent::Truncated { max_loops } => write!(f, "Truncated after {} loops", max_loops),
        }
    }
}
