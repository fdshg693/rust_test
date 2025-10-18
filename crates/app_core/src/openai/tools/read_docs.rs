use std::sync::Arc;
use serde_json::{json, Value};
use crate::openai::tools::{ToolDefinition, ToolParametersBuilder};
use std::path::PathBuf;

/// docs フォルダ配下（サブフォルダ含む）の Markdown / テキストを返すツール。
/// - 入力は `path`（docs からの相対パス）。例: "benches.md", "guides/intro.md"
/// - `.md` と `.txt` のみ許可。
/// - `std::fs::canonicalize` でパストラバーサルを防止し、`docs` 配下であることを検証。
/// 返却形式: { "path": string, "filename": string, "content": string, ("truncated": bool)? } または { "error": string }
pub fn build_read_doc_tool() -> ToolDefinition {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = PathBuf::from(manifest_dir).parent().unwrap().parent().unwrap().to_path_buf();
    let docs_root = workspace_root.join("docs");
    build_read_doc_tool_with_root(docs_root)
}

/// Internal version that allows specifying a custom docs root (useful for testing)
fn build_read_doc_tool_with_root(docs_root: PathBuf) -> ToolDefinition {
    let parameters = ToolParametersBuilder::new_object()
        .add_string("path", Some("Relative path under docs/ to a .md or .txt file (subfolders allowed)"))
        .required("path")
        .additional_properties(false)
        .build();

    ToolDefinition::new(
        "read_docs_file",
        "Read a .md or .txt file from the local docs directory (including subfolders) and return its text content.",
        parameters,
        Arc::new(move |args: &Value| read_docs_file_impl(args, &docs_root))
    )
}

/// Implementation separated from the closure to allow easier testing / reuse.
fn read_docs_file_impl(args: &Value, docs_root: &std::path::Path) -> color_eyre::Result<Value> {
    use std::path::{Path, PathBuf};
    const MAX_BYTES: usize = 16 * 1024; // 16KB safety limit

    // 1) Parse input path
    let rel_path_str = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) if !p.trim().is_empty() => p,
        _ => return Ok(json!({"error": "path is required"})),
    };

    // 2) Reject absolute paths early
    let rel_path = Path::new(rel_path_str);
    if rel_path.is_absolute() {
        return Ok(json!({"error": "absolute path not allowed"}));
    }

    // 3) Build candidate under docs and canonicalize
    let candidate: PathBuf = docs_root.join(rel_path);
    let docs_root_canon = match std::fs::canonicalize(docs_root) {
        Ok(p) => p,
        Err(e) => return Ok(json!({"error": format!("docs root not found: {e}")})),
    };
    let candidate_canon = match std::fs::canonicalize(&candidate) {
        Ok(p) => p,
        Err(e) => return Ok(json!({"error": format!("file not found: {e}")})),
    };

    // 4) Ensure candidate stays within docs root
    if !candidate_canon.starts_with(&docs_root_canon) {
        return Ok(json!({"error": "path escapes docs root"}));
    }

    // 5) Allow only .md or .txt
    let allowed_ext = ["md", "markdown", "txt"]; // allow a couple of markdown variants
    let ext_ok = candidate_canon
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| allowed_ext.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false);
    if !ext_ok {
        return Ok(json!({"error": "unsupported extension (allowed: .md, .markdown, .txt)"}));
    }

    // 6) Read file content
    let content = match std::fs::read_to_string(&candidate_canon) {
        Ok(c) => c,
        Err(e) => return Ok(json!({"error": format!("read error: {e}")})),
    };

    // 7) Truncate if needed
    let mut truncated = false;
    let output = if content.len() > MAX_BYTES {
        truncated = true;
        let mut s = content.clone();
        s.truncate(MAX_BYTES);
        s
    } else { content };

    // 8) Prepare response: keep backward-compatible "filename" and add "path"
    let filename = candidate_canon.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let rel_display = rel_path.to_string_lossy().to_string();
    if truncated {
        Ok(json!({
            "path": rel_display,
            "filename": filename,
            "content": output,
            "truncated": true,
            "max_bytes": MAX_BYTES
        }))
    } else {
        Ok(json!({ "path": rel_display, "filename": filename, "content": output }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::Result;
    use std::path::PathBuf;

    fn get_workspace_root() -> PathBuf {
        // CARGO_MANIFEST_DIR points to crates/app_core
        // We need to go up two levels to reach the workspace root
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir).parent().unwrap().parent().unwrap().to_path_buf()
    }

    #[test]
    fn read_doc_tool_valid_file() -> Result<()> {
        let workspace_root = get_workspace_root();
        let docs_root = workspace_root.join("docs");
        let tool = build_read_doc_tool_with_root(docs_root);
        let out = tool.execute(&json!({"path": "benches.md"}))?;
        assert_eq!(out["filename"], "benches.md");
        assert!(out["content"].as_str().unwrap_or("").len() > 0);
        Ok(())
    }

    #[test]
    fn read_doc_impl_invalid_filename() -> Result<()> {
        let workspace_root = get_workspace_root();
        let docs_root = workspace_root.join("docs");
        let val = read_docs_file_impl(&json!({"path": "../secret"}), &docs_root)?;
        let err = val["error"].as_str().unwrap().to_string();
        assert!(err.contains("escapes docs root") || err.contains("absolute path not allowed") || err.contains("file not found"));
        Ok(())
    }
}
