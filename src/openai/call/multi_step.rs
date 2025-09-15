use crate::config::Config;
use crate::openai::tools::ToolDefinition;
use crate::openai::ConversationHistory;
use color_eyre::Result;
use tokio::runtime::Runtime;
use tracing::{debug, info, instrument};

use super::proposer::propose_tool_call;
use super::resolver::resolve_and_execute_tool_call;
use super::types::{MultiStepAnswer, MultiStepLogEvent, ToolCallDecision, ToolResolution};

#[instrument(name = "multi_step_tool_answer", skip(tools, config))]
pub async fn multi_step_tool_answer(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
) -> Result<MultiStepAnswer> {
    multi_step_tool_answer_with_logger_internal(
        original_user_prompt,
        tools,
        config,
        max_loops,
        None,
    ).await
}

#[instrument(name = "multi_step_tool_answer_with_logger", skip(tools, config, logger))]
pub async fn multi_step_tool_answer_with_logger(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
    logger: impl FnMut(&MultiStepLogEvent),
) -> Result<MultiStepAnswer> {
    let mut user_logger = logger;
    let mut log_and_forward = |ev: &MultiStepLogEvent| {
        debug!(target: "openai", event = %ev, "multi_step_event");
        user_logger(ev);
    };
    let mut opt_logger: Option<&mut dyn FnMut(&MultiStepLogEvent)> = Some(&mut log_and_forward);
    multi_step_tool_answer_with_logger_internal(
        original_user_prompt,
        tools,
        config,
        max_loops,
        opt_logger.as_deref_mut(),
    ).await
}

async fn multi_step_tool_answer_with_logger_internal(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
    mut logger: Option<&mut dyn FnMut(&MultiStepLogEvent)>,
) -> Result<MultiStepAnswer> {
    let max_loops = max_loops.unwrap_or(5);
    let mut steps: Vec<ToolResolution> = Vec::new();
    let mut truncated = false;
    let mut history = ConversationHistory::new();
    history.add_user(original_user_prompt);

    for iteration in 1..=max_loops {
        debug!(target: "openai", iteration, "multi_step_iteration_start");
        if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::IterationStart { iteration }); }
        let decision = propose_tool_call(history.as_slice(), "", tools, config).await?;
        if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::Proposed { iteration, decision: decision.clone() }); }
        match decision {
            ToolCallDecision::Text(text) => {
                debug!(target: "openai", iteration, "multi_step_text_final");
                if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::FinalText { iteration, text: text.clone() }); }
                return Ok(MultiStepAnswer { final_answer: text, steps, iterations: iteration, truncated });
            }
            ToolCallDecision::ToolCall { name, arguments } => {
                debug!(target: "openai", iteration, tool = %name, "multi_step_tool_call" );
                let resolution = resolve_and_execute_tool_call(
                    ToolCallDecision::ToolCall { name, arguments },
                    tools,
                );
                if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::Resolved { iteration, resolution: resolution.clone() }); }
                let executed_json_for_next = match &resolution {
                    ToolResolution::Executed { name, result } => {
                        format!("ツール {name} の結果 JSON: {result}")
                    }
                    ToolResolution::ModelText(t) => format!("モデルテキスト: {t}"),
                    ToolResolution::ToolNotFound { requested } => {
                        format!("要求されたツール {requested} は存在しません。")
                    }
                    ToolResolution::ArgumentsParseError { name, raw, error } => {
                        format!("ツール {name} の引数パース失敗: {error}. RAW: {raw}")
                    }
                    ToolResolution::ExecutionError { name, error } => {
                        format!("ツール {name} 実行エラー: {error}")
                    }
                };
                steps.push(resolution.clone());

                if !resolution.is_executed() {
                    if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::EarlyFailure { iteration, resolution: resolution.clone() }); }
                    let final_answer = format!(
                        "途中でツール実行に失敗したためここまでの情報で回答します。\n元の質問: {original_user_prompt}\n{executed_json_for_next}"
                    );
                    return Ok(MultiStepAnswer { final_answer, steps, iterations: iteration, truncated });
                }

                if let ToolResolution::Executed { name, result } = &resolution {
                    history.add_function(name, result.to_string());
                    if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::HistoryFunctionAppended { iteration, name: name.clone(), result: result.clone() }); }
                }
            }
        }
    }

    truncated = true;
    if let Some(cb) = logger.as_deref_mut() { cb(&MultiStepLogEvent::Truncated { max_loops }); }
    let final_answer = format!(
        "最大ループ回数({})に達したため打ち切りました。これまでの function 結果(JSON)を参考に最終回答をまとめてください。",
        max_loops
    );
    Ok(MultiStepAnswer { final_answer, steps, iterations: max_loops, truncated })
}

#[instrument(name = "multi_step_tool_answer_blocking", skip(tools, config))]
pub fn multi_step_tool_answer_blocking(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
) -> Result<MultiStepAnswer> {
    let rt = Runtime::new()?;
    rt.block_on(multi_step_tool_answer(original_user_prompt, tools, config, max_loops))
}

#[instrument(name = "multi_step_tool_answer_blocking_with_logger", skip(tools, config, logger))]
pub fn multi_step_tool_answer_blocking_with_logger(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &Config,
    max_loops: Option<usize>,
    logger: impl FnMut(&MultiStepLogEvent),
) -> Result<MultiStepAnswer> {
    let max_loops_val = max_loops.unwrap_or(5);
    info!(target: "openai", model = %config.model, max_tokens = config.max_tokens, max_loops = max_loops_val, "multi_step_blocking_request");

    let mut user_logger = logger;
    let mut log_and_forward = |ev: &MultiStepLogEvent| {
        debug!(target: "openai", event = %ev, "multi_step_event");
        user_logger(ev);
    };

    let mut opt_logger: Option<&mut dyn FnMut(&MultiStepLogEvent)> = Some(&mut log_and_forward);
    let rt = Runtime::new()?;
    let result = rt.block_on(multi_step_tool_answer_with_logger_internal(
        original_user_prompt,
        tools,
        config,
        max_loops,
        opt_logger.as_deref_mut(),
    ));

    let result = result?;
    info!(target: "openai", iterations = result.iterations, truncated = result.truncated, steps = result.steps.len(), final_len = result.final_answer.len(), "multi_step_blocking_done");
    Ok(result)
}
