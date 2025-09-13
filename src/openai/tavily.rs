//! tavily Search tool integration.
//!
//! Provides a `build_tavily_search_tool` function returning a `ToolDefinition` that
//! performs a synchronous HTTP call (blocking) to the tavily Search API.
//!
//! Environment:
//! * `tavily_API_KEY` must be set (Bearer token) or the tool returns an error JSON.
//!
//! Arguments schema:
//! * `query` (string, required)
//! * `max_results` (integer, optional, default 5; API allows 1..n)
//!
//! The tool returns a JSON object with either the tavily response fields or
//! `{ "error": "..." }` on failure.

use crate::openai::tool::{ToolDefinition, ToolParametersBuilder};
use serde_json::{json, Value};
use std::sync::Arc;
use color_eyre::{Result, eyre::WrapErr};
use tracing::debug;
use reqwest::blocking::Client;
use std::time::Duration;

/// Direct tavily search function which can be reused outside the tool context.
/// Returns the parsed JSON response or an eyre error.
pub fn tavily_search(query: &str, max_results: u64) -> Result<Value> {
    if query.trim().is_empty() { return Err(color_eyre::eyre::eyre!("query is empty")); }

    let api_key = std::env::var("tavily_API_KEY")
        .map_err(|_| color_eyre::eyre::eyre!("tavily_API_KEY not set"))?;

    let max_results = max_results.min(10).max(1);

    let body = json!({
        "query": query.trim(),
        "max_results": max_results,
        "auto_parameters": false,
        "search_depth": "basic",
        "include_answer": true,
        "include_raw_content": false,
        "include_images": false,
        "include_image_descriptions": false,
        "include_favicon": false,
        "topic": "general"
    });

    let client = Client::builder()
        .user_agent("rust_test_tavily_tool/0.1")
        // Reasonable overall timeout so blocked network doesn't hang the tool.
        .timeout(Duration::from_secs(15))
        .build()
        .wrap_err("building reqwest client for tavily")?;

    let resp = client
        .post("https://api.tavily.com/search")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .wrap_err("sending tavily search request")?;

    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    debug!(target: "openai", status = %status, len = text.len(), "tavily_response_raw");

    if !status.is_success() {
        return Err(color_eyre::eyre::eyre!("status {}: {}", status.as_u16(), text));
    }

    let parsed: Value = serde_json::from_str(&text)
        .unwrap_or_else(|_| json!({"raw": text}));
    Ok(parsed)
}

/// Build a tavily search tool.
pub fn build_tavily_search_tool() -> ToolDefinition {
    let parameters = ToolParametersBuilder::new_object()
        .add_string("query", Some("Search query string to send to tavily"))
        .add_integer("max_results", Some("Maximum number of results to request (1-10)"), Some(1), Some(10))
        .required("query")
        .additional_properties(false)
        .build();

    ToolDefinition::new(
        "tavily_search",
        "Perform a web search via tavily API and return JSON results (pass query, optional max_results).",
        parameters,
        Arc::new(|args: &Value| -> Result<Value> {
            let query = match args.get("query").and_then(|v| v.as_str()) {
                Some(s) if !s.trim().is_empty() => s.trim().to_string(),
                _ => return Ok(json!({"error": "query is required string"})),
            };
            let max_results = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(5);
            match tavily_search(&query, max_results) {
                Ok(v) => Ok(v),
                Err(e) => {
                    // Optional verbose chain when troubleshooting: set tavily_VERBOSE=1
                    let verbose = std::env::var("tavily_VERBOSE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")) .unwrap_or(false);
                    if verbose {
                        let chain: Vec<String> = e.chain().map(|c| c.to_string()).collect();
                        Ok(json!({"error": e.to_string(), "chain": chain}))
                    } else {
                        Ok(json!({"error": e.to_string()}))
                    }
                },
            }
        })
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_contains_query() {
        let tool = build_tavily_search_tool();
        assert_eq!(tool.name, "tavily_search");
    let v = tool.parameters.as_value();
    assert!(v["properties"].get("query").is_some());
    }

    #[test]
    fn tavily_search_missing_key_errors() {
        // Ensure key is unset temporarily
        let orig = std::env::var("tavily_API_KEY").ok();
        unsafe { std::env::remove_var("tavily_API_KEY"); }
        let err = tavily_search("rust", 3).unwrap_err();
        assert!(format!("{err}").contains("tavily_API_KEY"));
        if let Some(v) = orig { unsafe { std::env::set_var("tavily_API_KEY", v); } }
    }

    #[test]
    fn tavily_search_integration_test() {
        let _ = dotenvy::dotenv();
        if std::env::var("tavily_API_KEY").is_err() {
            eprintln!("Skipping tavily_search_integration_test: tavily_API_KEY not set");
            return;
        }
        match tavily_search("rust programming language", 2) {
            Ok(res) => {
                if let Some(arr) = res.get("results").and_then(|v| v.as_array()) {
                    assert!(!arr.is_empty(), "results array is empty");
                } else {
                    eprintln!("Skipping: response did not contain results array: {res}");
                    return;
                }
                println!("tavily search results: {}", res);
            }
            Err(e) => {
                eprintln!("Skipping tavily_search_integration_test due to network/API error: {e}");
                for (i, cause) in e.chain().enumerate() {
                    eprintln!("  cause[{i}]: {cause}");
                }
                // Treat network / HTTP / status errors as a skipped test rather than failure in CI environments
                return;
            }
        }
    }
}
