use std::sync::Arc;
use serde_json::json;
use crate::openai::tools::{ToolDefinition, ToolParametersBuilder};

/// 便利関数: 既存の定数(X,Y)を返すツール定義を作成
pub fn build_get_constants_tool(x: i32, y: i32) -> ToolDefinition {
    let params = ToolParametersBuilder::new_object().build();
    ToolDefinition::new(
        "get_constants",
        "Return constants X and Y as JSON",
        params,
        Arc::new(move |_v| Ok(json!({ "X": x, "Y": y }))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_constants_tool_executes() {
        let t = build_get_constants_tool(1, 2);
        let out = t.execute(&json!({})).unwrap();
        assert_eq!(out["X"], 1);
        assert_eq!(out["Y"], 2);
    }
}