use serde_json::{json, Value};
use std::io::{self, BufRead};
fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let q: Value = serde_json::from_str(&line).unwrap();
        let a = q["a"].as_str().unwrap();
        let b = q["b"].as_str().unwrap_or("");
        let r = match q["kind"].as_str().unwrap() {
            "text" => tezbar_diff_checker::diff::text(a, b).map(|v| json!(v)),
            "json" => {
                tezbar_diff_checker::diff::json(a, b).map(|v| serde_json::to_value(v).unwrap())
            }
            "format" => tezbar_diff_checker::json::format(a).map(|v| json!(v)),
            _ => Err("unknown".into()),
        };
        println!(
            "{}",
            match r {
                Ok(v) => json!({"ok":true,"value":v}),
                Err(e) => json!({"ok":false,"error":e}),
            }
        );
    }
}
