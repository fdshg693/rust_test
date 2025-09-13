//! OpenAI連携のモジュール（ファイル分割版）

pub mod worker;
pub mod simple;
pub mod call_tool;
pub mod tool;
pub mod tavily;
pub mod docs_tool;

// 代表的な公開APIを再エクスポート
pub use worker::start_openai_worker;
pub use simple::{
	get_ai_answer_once,
	get_ai_answer_once_blocking,	
};
pub use call_tool::{
	propose_tool_call,
	propose_tool_call_blocking,
	ToolCallDecision,
    ToolResolution,
    resolve_and_execute_tool_call,
};
pub use tool::{ToolDefinition, build_get_constants_tool};
pub use docs_tool::build_read_doc_tool;
pub use tavily::{build_tavily_search_tool, tavily_search};
