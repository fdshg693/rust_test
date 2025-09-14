//! tavily Search tool integration (reorganized under tools::tavily)

use crate::openai::tools::{ToolDefinition, ToolParametersBuilder};
use serde_json::{json, Value};
use std::sync::Arc;
use color_eyre::{Result, eyre::WrapErr};
use tracing::debug;
use reqwest::blocking::Client;
use std::time::Duration;

pub fn tavily_search(query: &str, max_results: u64) -> Result<String> {
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
    if let Some(answer) = parsed.get("answer").and_then(|v| v.as_str()) {
        Ok(answer.to_string())
    } else {
        Ok(parsed.to_string())
    }
}

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
                Ok(answer) => Ok(json!({"answer": answer})),
                Err(e) => {
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
}
