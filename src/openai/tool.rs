use std::sync::Arc;
use async_openai::types::{ChatCompletionTool, ChatCompletionToolType, FunctionObject};
use serde_json::{Value, Map};
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
    pub parameters: ToolParameters,  // JSON Schema wrapper
    pub strict: bool,
    handler: ToolHandler,
}

impl std::fmt::Debug for ToolDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolDefinition")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("parameters", &self.parameters.as_value())
            .field("strict", &self.strict)
            .finish()
    }
}

impl ToolDefinition {
    /// 新規作成
    pub fn new(
        name: &'static str,
        description: &'static str,
        parameters: impl Into<ToolParameters>,
        handler: ToolHandler,
    ) -> Self {
        Self { name, description, parameters: parameters.into(), strict: false, handler }
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
            parameters: Some(self.parameters.as_value().clone()),
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

/* -------------------------------------------------------------------------- */
/* Tool Parameters Wrapper & Builder                                          */
/* -------------------------------------------------------------------------- */

/// JSON Schema (object) を表現する薄いラッパ。内部は `serde_json::Value`。
/// 直接 `Value` を扱うより、ビルダーで型安全寄りに構築できるようにする目的。
#[derive(Clone, Debug)]
pub struct ToolParameters(Value);

impl ToolParameters {
    pub fn as_value(&self) -> &Value { &self.0 }
    pub fn into_value(self) -> Value { self.0 }

    /// 空の object スキーマ ( `{ "type": "object", "properties": {}, "required": [] }` )
    pub fn empty_object() -> Self {
        ToolParameters(json!({
            "type": "object",
            "properties": {},
            "required": []
        }))
    }
}

impl From<Value> for ToolParameters {
    fn from(v: Value) -> Self { ToolParameters(v) }
}

/// ツール引数用 JSON Schema (object) を段階的に構築するビルダー。
pub struct ToolParametersBuilder {
    properties: Map<String, Value>,
    required: Vec<String>,
    additional_properties: Option<bool>,
}

impl ToolParametersBuilder {
    pub fn new_object() -> Self {
        Self { properties: Map::new(), required: Vec::new(), additional_properties: None }
    }

    pub fn add_string(mut self, name: &str, description: Option<&str>) -> Self {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("string".to_string()));
        if let Some(d) = description { schema.insert("description".to_string(), Value::String(d.to_string())); }
        self.properties.insert(name.to_string(), Value::Object(schema));
        self
    }

    pub fn add_string_enum(mut self, name: &str, description: Option<&str>, values: &[&str]) -> Self {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("string".to_string()));
        if let Some(d) = description { schema.insert("description".to_string(), Value::String(d.to_string())); }
        schema.insert("enum".to_string(), Value::Array(values.iter().map(|v| Value::String((*v).to_string())).collect()));
        self.properties.insert(name.to_string(), Value::Object(schema));
        self
    }

    pub fn add_integer(mut self, name: &str, description: Option<&str>, minimum: Option<i64>, maximum: Option<i64>) -> Self {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("integer".to_string()));
        if let Some(d) = description { schema.insert("description".to_string(), Value::String(d.to_string())); }
        if let Some(min) = minimum { schema.insert("minimum".to_string(), Value::Number(min.into())); }
        if let Some(max) = maximum { schema.insert("maximum".to_string(), Value::Number(max.into())); }
        self.properties.insert(name.to_string(), Value::Object(schema));
        self
    }

    pub fn required(mut self, name: &str) -> Self {
        if !self.required.iter().any(|r| r == name) { self.required.push(name.to_string()); }
        self
    }

    pub fn additional_properties(mut self, allow: bool) -> Self {
        self.additional_properties = Some(allow);
        self
    }

    pub fn build(self) -> ToolParameters {
        let mut obj = Map::new();
        obj.insert("type".to_string(), Value::String("object".to_string()));
        obj.insert("properties".to_string(), Value::Object(self.properties));
        if !self.required.is_empty() {
            obj.insert("required".to_string(), Value::Array(self.required.into_iter().map(Value::String).collect()));
        }
        if let Some(ap) = self.additional_properties { obj.insert("additionalProperties".to_string(), Value::Bool(ap)); }
        ToolParameters(Value::Object(obj))
    }
}

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

    #[test]
    fn tool_definition_executes_closure() -> Result<()> {
        let tool = ToolDefinition::new(
            "echo_keys",
            "Return number of keys in object",
            ToolParametersBuilder::new_object()
                .add_string("payload", Some("Arbitrary JSON object (will count keys)")) // Placeholder type (not enforcing object deeply)
                .required("payload")
                .build(),
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

    #[test]
    fn builder_creates_empty_object() {
        let p = ToolParameters::empty_object();
        let v = p.as_value();
        assert_eq!(v["type"], "object");
        assert!(v["properties"].as_object().unwrap().is_empty());
    }

    #[test]
    fn builder_adds_fields_and_required() {
        let p = ToolParametersBuilder::new_object()
            .add_string("q", Some("query desc"))
            .add_integer("n", Some("count"), Some(1), Some(10))
            .required("q")
            .additional_properties(false)
            .build();
        let v = p.as_value();
        assert_eq!(v["properties"].as_object().unwrap().len(), 2);
        assert!(v["required"].as_array().unwrap().iter().any(|x| x == "q"));
        assert_eq!(v["additionalProperties"], false);
        println!("Built schema: {}", v);
    }
}
