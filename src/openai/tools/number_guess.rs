use std::sync::Arc;
use serde_json::json;

use super::{ToolDefinition, ToolParametersBuilder};

/// 数字あてゲームのツールを作成する。
/// - 作成者が 1..=max の任意の数字を `target` に指定
/// - モデルは `guess`(integer) を渡してツールを呼び出す
/// - 結果は { result: "low"|"high"|"correct"|"out_of_range" } を返す
pub fn build_number_guess_tool(target: u32, max: u32) -> ToolDefinition {
    // 念のため 1..=max に丸める（max が 0 の場合は 1 に矯正）
    let max = max.max(1);
    let target = target.min(max).max(1);

    let params = ToolParametersBuilder::new_object()
        .add_integer(
            "guess",
            Some("Your guessed integer between 1 and MAX (inclusive)"),
            Some(1),
            Some(max as i64),
        )
        .required("guess")
        .additional_properties(false)
        .build();

    ToolDefinition::new(
        "number_guess",
        "Number guessing game: compare provided 'guess' with the hidden target (1..=MAX) and return whether it is low, high, or correct.",
        params,
        Arc::new(move |v| {
            let guess = v
                .get("guess")
                .and_then(|val| val.as_i64())
                .ok_or_else(|| color_eyre::eyre::eyre!("Invalid or missing 'guess' parameter"))?;

            if !(1..=(max as i64)).contains(&(guess as i64)) {
                return Ok(json!({ "result": "out_of_range" }));
            }

            let result = if (guess as u32) < target {
                "low"
            } else if (guess as u32) > target {
                "high"
            } else {
                "correct"
            };

            Ok(json!({ "result": result }))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn number_guess_tool_works() {
        let tool = build_number_guess_tool(42, 100);

        let r1 = tool.execute(&json!({"guess": 10})).unwrap();
        assert_eq!(r1["result"], "low");

        let r2 = tool.execute(&json!({"guess": 77})).unwrap();
        assert_eq!(r2["result"], "high");

        let r3 = tool.execute(&json!({"guess": 42})).unwrap();
        assert_eq!(r3["result"], "correct");

        let r4 = tool.execute(&json!({"guess": 0})).unwrap();
        assert_eq!(r4["result"], "out_of_range");

        // max を 50 にした場合の境界チェック
        let tool2 = build_number_guess_tool(50, 50);
        let r5 = tool2.execute(&json!({"guess": 51})).unwrap();
        assert_eq!(r5["result"], "out_of_range");
    }
}
