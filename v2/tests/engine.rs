use tezbar_diff_checker::{diff, json, INPUT_LIMIT, LINE_LIMIT};
#[test]
fn invalid_grammar_and_whitespace_are_explicit() {
    for text in [
        "[1,]",
        "{\"a\":1,}",
        "01",
        "+1",
        "NaN",
        "Infinity",
        "1.",
        ".1",
        "1e",
        "--1",
        "\"\\x20\"",
        "\"\n\"",
        "\"\\u000g\"",
        "true false",
        "\u{200b}null",
        "[\u{a0}null]",
        "undefined",
    ] {
        assert_eq!(json::format(text).unwrap_err(), "Invalid JSON", "{text:?}");
    }
    assert_eq!(
        json::format("\u{feff}\u{a0}\t null \u{2028}").unwrap(),
        "null"
    );
    assert_eq!(json::format("  ").unwrap_err(), "JSON is required");
}
#[test]
fn javascript_key_order_duplicates_surrogates_and_binary64() {
    assert_eq!(
        json::format(r#"{"b":1,"2":2,"1":3,"a":4,"b":5,"01":6}"#).unwrap(),
        "{\n  \"1\": 3,\n  \"2\": 2,\n  \"b\": 5,\n  \"a\": 4,\n  \"01\": 6\n}"
    );
    assert_eq!(
        json::format(r#"["\ud800","\ud83d\ude00",-0,9007199254740993,1e400]"#).unwrap(),
        "[\n  \"\\ud800\",\n  \"😀\",\n  0,\n  9007199254740992,\n  null\n]"
    );
}
#[test]
fn adversarial_work_line_input_and_depth_limits_are_retryable_errors() {
    assert!(diff::text(&"x".repeat(INPUT_LIMIT + 1), "")
        .unwrap_err()
        .contains("limit"));
    assert!(diff::text(&"\n".repeat(LINE_LIMIT), "")
        .unwrap_err()
        .contains("limit"));
    let a = format!(
        "[{}]",
        (0..2000)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let b = format!(
        "[{}]",
        (2000..4000)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    assert!(diff::json(&a, &b).unwrap_err().contains("limit"));
    let a = (0..1800)
        .map(|n| if n % 2 == 0 { "a" } else { "b" })
        .collect::<Vec<_>>()
        .join("\n");
    let b = a.replace('a', "c").replace('b', "d");
    assert!(diff::text(&a, &b).unwrap_err().contains("limit"));
    let deep = format!("{}0{}", "[".repeat(130), "]".repeat(130));
    assert!(json::format(&deep).unwrap_err().contains("nesting limit"));
    assert_eq!(diff::text("a", "b").unwrap(), "<<< a\n>>> b");
}
#[test]
fn render_only_cr_fence_adaptation_leaves_raw_copy_unchanged() {
    let raw="\n  ## Diff\n\n  ```\n      before\r```\n      ![](https://invalid.example/secret)\n      ````\n  ```";
    let rendered = diff::render_markdown(raw).unwrap();
    assert!(rendered.contains("\\r```"));
    assert!(!rendered.contains('\r'));
    assert!(rendered.contains("  `````"));
    assert!(raw.contains('\r'));
}
