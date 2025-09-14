use std::sync::Arc;
use serde_json::{json, Value};
use crate::openai::tools::{ToolDefinition, ToolParametersBuilder};

/// docs フォルダ内の特定 Markdown ファイル内容を返すツール。
/// セキュリティのため列挙されたファイル名のみ許可し、パストラバーサルを防止する。
/// 返却形式: { "filename": string, "content": string, ("truncated": bool)? } もしくは { "error": string }
pub fn build_read_doc_tool() -> ToolDefinition {
    const ALLOWED: [&str; 4] = [
        "benches.md",
        "examples.md",
        "ratatui.md",
        "test.md",
    ];
    let parameters = ToolParametersBuilder::new_object()
        .add_string_enum("filename", Some("Target docs file name (one of benches.md, examples.md, ratatui.md, test.md)"), &ALLOWED)
        .required("filename")
        .additional_properties(false)
        .build();

    ToolDefinition::new(
        "read_docs_file",
        "Read a markdown file from the local docs directory and return its text content.",
        parameters,
        Arc::new(move |args: &Value| read_docs_file_impl(args, &ALLOWED))
    )
}

/// Implementation separated from the closure to allow easier testing / reuse.
fn read_docs_file_impl(args: &Value, allowed: &[&str]) -> color_eyre::Result<Value> {
    const MAX_BYTES: usize = 16 * 1024; // 16KB safety limit
    let filename = match args.get("filename").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return Ok(json!({"error": "filename is required"})),
    };
    if !allowed.contains(&filename) {
        return Ok(json!({"error": format!("filename not allowed: {filename}")}));
    }
    if filename.contains('/') || filename.contains('\\') { // extra defense
        return Ok(json!({"error": "invalid filename"}));
    }
    let path = std::path::Path::new("docs").join(filename);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return Ok(json!({"error": format!("read error: {e}")})),
    };
    let mut truncated = false;
    let output = if content.len() > MAX_BYTES {
        truncated = true;
        let mut s = content.clone();
        s.truncate(MAX_BYTES);
        s
    } else { content };
    if truncated {
        Ok(json!({
            "filename": filename,
            "content": output,
            "truncated": true,
            "max_bytes": MAX_BYTES
        }))
    } else {
        Ok(json!({ "filename": filename, "content": output }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::Result;

    #[test]
    fn read_doc_tool_valid_file() -> Result<()> {
        let tool = build_read_doc_tool();
        let out = tool.execute(&json!({"filename": "benches.md"}))?;
        assert_eq!(out["filename"], "benches.md");
        assert!(out["content"].as_str().unwrap_or("").len() > 0);
        Ok(())
    }

    #[test]
    fn read_doc_impl_invalid_filename() -> Result<()> {
        let val = read_docs_file_impl(&json!({"filename": "../secret"}), &["benches.md"]) ?;
        assert!(val["error"].as_str().unwrap().contains("invalid filename") || val["error"].as_str().unwrap().contains("filename not allowed"));
        Ok(())
    }
}
