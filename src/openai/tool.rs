use std::sync::Arc;
use async_openai::types::{ChatCompletionTool, ChatCompletionToolType, FunctionObject};
use serde_json::Value;
use serde_json::json;
use color_eyre::Result;

/// ランタイムで実行するツール関数の型。
/// 引数(JSON)を受け取り、結果(JSON)を返す。
pub type ToolHandler = Arc<dyn Fn(&Value) -> Result<Value> + Send + Sync + 'static>;

/// OpenAI function calling に渡すメタデータと実行ハンドラをまとめた定義。
/// 非同期が必要になったら `ToolHandler` を futures を返す型に差し替える拡張が可能。
#[derive(Clone)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,  // JSON Schema
    pub strict: bool,
    handler: ToolHandler,
}

impl std::fmt::Debug for ToolDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolDefinition")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("parameters", &self.parameters)
            .field("strict", &self.strict)
            .finish()
    }
}

impl ToolDefinition {
    /// 新規作成
    pub fn new(
        name: &'static str,
        description: &'static str,
        parameters: Value,
        handler: ToolHandler,
    ) -> Self {
        Self { name, description, parameters, strict: false, handler }
    }

    /// strict フラグを設定（OpenAI の strict function 呼び出しモード用）
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// OpenAI SDK の `FunctionObject` に変換
    pub fn function_object(&self) -> FunctionObject {
        FunctionObject {
            name: self.name.to_string(),
            description: Some(self.description.to_string()),
            parameters: Some(self.parameters.clone()),
            strict: Some(self.strict),
        }
    }

    /// ChatCompletionTool 形式（APIへ渡す vector 用）
    pub fn as_chat_tool(&self) -> ChatCompletionTool {
        ChatCompletionTool {
            r#type: ChatCompletionToolType::Function,
            function: self.function_object(),
        }
    }

    /// ツールを実行
    pub fn execute(&self, args: &Value) -> Result<Value> {
        (self.handler)(args)
    }
}

/// 便利関数: 既存の定数(X,Y)を返すツール定義を作成
pub fn build_get_constants_tool(x: i32, y: i32) -> ToolDefinition {
    ToolDefinition::new(
        "get_constants",
        "Return constants X and Y as JSON",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        Arc::new(move |_v| Ok(json!({ "X": x, "Y": y }))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definition_executes_closure() -> Result<()> {
        let tool = ToolDefinition::new(
            "echo_keys",
            "Return number of keys in object",
            json!({
                "type": "object",
                "properties": {
                    "payload": { "type": "object" }
                },
                "required": ["payload"]
            }),
            Arc::new(|v| {
                let obj = v.get("payload").and_then(|p| p.as_object()).ok_or_else(|| color_eyre::eyre::eyre!("missing payload object"))?;
                Ok(json!({ "len": obj.len() }))
            })
        );

        let args = json!({"payload": {"a": 1, "b": 2}});
        let out = tool.execute(&args)?;
        assert_eq!(out["len"], 2);

        // Chat tool conversion sanity check
        let chat_tool = tool.as_chat_tool();
        assert_eq!(chat_tool.function.name, "echo_keys");
        Ok(())
    }

    #[test]
    fn build_get_constants_tool_returns_values() -> Result<()> {
        let tool = build_get_constants_tool(42, 7);
        let out = tool.execute(&json!({}))?;
        assert_eq!(out["X"], 42);
        assert_eq!(out["Y"], 7);
        Ok(())
    }
}
