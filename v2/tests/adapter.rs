#[rustfmt::skip]
#[path = "../fixtures/beta/src-tauri/src/native_core/common.rs"]
pub mod actual_common;
#[rustfmt::skip]
#[path = "../fixtures/beta/src-tauri/src/native_core/extensions/v2_sdk.rs"]
pub mod v2_sdk;
pub mod native_core {
    pub use crate::actual_common as common;
}
#[path = "../src/tezbar_adapter.rs"]
mod adapter;

use actual_common::{CoreContext, CorePaths};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
fn context() -> (tempfile::TempDir, CoreContext) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path();
    let ctx = CoreContext::headless(CorePaths {
        data_dir: p.join("data"),
        config_dir: p.join("config"),
        resource_dir: p.join("resources"),
        temp_dir: p.join("tmp"),
        home_dir: p.into(),
    });
    (d, ctx)
}
fn action(v: &Value, title: &str) -> String {
    v["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["title"] == title)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn field(v: &Value, id: &str) -> String {
    v["root"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["props"]["id"] == id)
        .unwrap()["props"]["actionId"]
        .as_str()
        .unwrap()
        .into()
}
async fn invoke(ctx: &CoreContext, s: &mut Value, cmd: &str, id: &str, values: Value) -> Value {
    adapter::handle(
        ctx,
        cmd,
        "extension:invoke-action",
        &json!({"actionId":id,"formValues":values}),
        s,
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn all_routes_ack_retry_committed_and_lifecycle() {
    let (_dir, ctx) = context();
    let writes = Arc::new(Mutex::new(Vec::new()));
    let ack = Arc::new(Mutex::new(Ok(json!({"ok":true}))));
    let (w, a) = (writes.clone(), ack.clone());
    ctx.set_router(Arc::new(move |channel, args| {
        assert_eq!(channel, "clipboard:write");
        w.lock().unwrap().push(args);
        let v = a.lock().unwrap().clone();
        Box::pin(async move { v })
    }))
    .unwrap();
    let mut s = json!({});
    let form = adapter::handle(
        &ctx,
        "compare-json",
        "extension:run-command",
        &Value::Null,
        &mut s,
    )
    .await
    .unwrap();
    let submit = action(&form, "Compare");
    let original = field(&form, "original");
    let modified = field(&form, "modified");
    let v = invoke(
        &ctx,
        &mut s,
        "compare-json",
        &original,
        json!({"value":"{\"b\":2,\"a\":1}"}),
    )
    .await;
    assert_eq!(field(&v, "original"), original);
    assert!(v["root"]["children"][1]["props"].get("value").is_none());
    invoke(
        &ctx,
        &mut s,
        "compare-json",
        &modified,
        json!({"value":"[3]"}),
    )
    .await;
    let detail = invoke(&ctx, &mut s, "compare-json", &submit, json!({})).await;
    assert_eq!(detail["root"]["type"], "Detail");
    assert_eq!(
        detail["root"]["metadata"]["children"][0]["props"]["title"],
        "Added Lines"
    );
    let result = tezbar_diff_checker::diff::json("{\"b\":2,\"a\":1}", "[3]").unwrap();
    let expected = [
        result.full_diff,
        result.markdown,
        result.original_formatted,
        result.modified_formatted,
    ];
    for ((title, text), route) in [
        "Copy Full Diff",
        "Copy Compact Diff",
        "Copy Formatted Original",
        "Copy Formatted Modified",
    ]
    .into_iter()
    .zip(expected)
    .zip(0..)
    {
        let id = action(&detail, title);
        for bad in [
            Ok(json!({"ok":false})),
            Ok(json!({"ok":true,"success":false})),
            Ok(json!({"ok":true,"success":null})),
            Ok(json!({"ok":true,"success":"true"})),
            Ok(json!({"ok":true,"success":1})),
            Ok(json!({"ok":true,"error":"bad"})),
            Ok(json!({"ok":true,"error":false})),
            Ok(json!({})),
            Ok(json!(true)),
            Ok(json!(null)),
            Ok(json!({"ok":"true"})),
            Err("transport".into()),
        ] {
            *ack.lock().unwrap() = bad;
            let failed = invoke(
                &ctx,
                &mut s,
                "compare-json",
                &id,
                json!({"original":"hijack","modified":"0","content":"leak"}),
            )
            .await;
            assert_eq!(
                failed["root"]["props"]["rawMarkdown"],
                detail["root"]["props"]["rawMarkdown"]
            );
            assert!(failed.to_string().contains("Could not copy"));
            assert_eq!(writes.lock().unwrap().last().unwrap(), &vec![json!(text)]);
        }
        *ack.lock().unwrap() = Ok(if route % 2 == 0 {
            json!({"ok":true})
        } else {
            json!({"ok":true,"error":null})
        });
        let success = invoke(&ctx, &mut s, "compare-json", &id, Value::Null).await;
        assert!(success.to_string().contains("Copied"));
    }
    let before = writes.lock().unwrap().len();
    for id in [&submit, &original, "fabricated"] {
        assert!(adapter::handle(
            &ctx,
            "compare-json",
            "extension:invoke-action",
            &json!({"actionId":id}),
            &mut s
        )
        .await
        .is_err());
    }
    assert!(adapter::handle(
        &ctx,
        "check-diff",
        "extension:invoke-action",
        &json!({"actionId":action(&detail,"Copy Full Diff")}),
        &mut s
    )
    .await
    .is_err());
    assert_eq!(before, writes.lock().unwrap().len());
    let back = detail["root"]["props"]["backActionId"].as_str().unwrap();
    let restored = invoke(&ctx, &mut s, "compare-json", back, Value::Null).await;
    assert_eq!(
        restored["root"]["children"][1]["props"]["value"],
        "{\"b\":2,\"a\":1}"
    );
    assert_ne!(action(&restored, "Compare"), submit);
    assert!(adapter::handle(
        &ctx,
        "compare-json",
        "extension:invoke-action",
        &json!({"actionId":back}),
        &mut s
    )
    .await
    .is_err());
    let current = action(&restored, "Compare");
    adapter::handle(
        &ctx,
        "compare-json",
        "extension:search-text-changed",
        &json!({"searchText":"q"}),
        &mut s,
    )
    .await
    .unwrap();
    assert!(adapter::handle(
        &ctx,
        "compare-json",
        "extension:invoke-action",
        &json!({"actionId":current}),
        &mut s
    )
    .await
    .is_err());
    adapter::handle(
        &ctx,
        "compare-json",
        "extension:dispose-session",
        &Value::Null,
        &mut s,
    )
    .await
    .unwrap();
    assert!(adapter::handle(
        &ctx,
        "compare-json",
        "extension:invoke-action",
        &json!({"actionId":current}),
        &mut s
    )
    .await
    .is_err());
    let relaunched = adapter::handle(
        &ctx,
        "compare-json",
        "extension:run-command",
        &Value::Null,
        &mut s,
    )
    .await
    .unwrap();
    assert_ne!(action(&relaunched, "Compare"), current);
}
#[tokio::test]
async fn source_form_actions_queued_drafts_validation_and_clipboard() {
    let (_d, ctx) = context();
    let read = Arc::new(Mutex::new(Ok(json!(""))));
    let reads = Arc::new(Mutex::new(0));
    let (r, n) = (read.clone(), reads.clone());
    ctx.set_router(Arc::new(move |ch, args| {
        assert_eq!(ch, "clipboard:read");
        assert!(args.is_empty());
        *n.lock().unwrap() += 1;
        let v = r.lock().unwrap().clone();
        Box::pin(async move { v })
    }))
    .unwrap();
    let mut s = json!({});
    let f = adapter::handle(
        &ctx,
        "compare-json",
        "extension:run-command",
        &Value::Null,
        &mut s,
    )
    .await
    .unwrap();
    let orig = field(&f, "original");
    let modified = field(&f, "modified");
    let format = action(&f, "Format JSONs");
    let swap = action(&f, "Swap JSONs");
    invoke(&ctx, &mut s, "compare-json", &orig, json!({"value":"["})).await;
    invoke(&ctx, &mut s, "compare-json", &orig, json!({"value":"[1]"})).await;
    let bad = invoke(
        &ctx,
        &mut s,
        "compare-json",
        &modified,
        json!({"value":"{"}),
    )
    .await;
    assert!(bad.to_string().contains("Invalid JSON"));
    let formatted = invoke(&ctx, &mut s, "compare-json", &format, Value::Null).await;
    assert!(formatted.to_string().contains("Formatted Original JSON"));
    assert_eq!(
        formatted["root"]["children"][2]["props"]["value"],
        "[\n  1\n]"
    );
    let swapped = invoke(&ctx, &mut s, "compare-json", &swap, Value::Null).await;
    assert_eq!(swapped["root"]["children"][1]["props"]["value"], "{");
    // Swap retains errors in their original fields, matching own useJSONForm.
    assert!(swapped["root"]["children"][3]["props"]["error"].is_string());
    for title in ["Paste Clipboard to Original", "Paste Clipboard to Modified"] {
        let id = action(&f, title);
        let before = s.clone();
        for response in [
            Ok(json!("")),
            Ok(json!({"text":"malformed"})),
            Ok(json!(null)),
            Err("offline".into()),
        ] {
            *read.lock().unwrap() = response;
            invoke(&ctx, &mut s, "compare-json", &id, Value::Null).await;
            assert_eq!(s["original"], before["original"]);
            assert_eq!(s["modified"], before["modified"]);
        }
        *read.lock().unwrap() = Ok(json!("null"));
        let p = invoke(&ctx, &mut s, "compare-json", &id, Value::Null).await;
        assert!(p.to_string().contains("Pasted into"));
    }
    assert_eq!(*reads.lock().unwrap(), 10);
    let detail = invoke(
        &ctx,
        &mut s,
        "compare-json",
        &action(&f, "Compare"),
        json!({"original":"true","modified":"null"}),
    )
    .await;
    assert_eq!(detail["root"]["type"], "Detail");
}
#[tokio::test]
async fn text_identical_back_and_limit_retry() {
    let (_d, ctx) = context();
    let mut s = json!({});
    let f = adapter::handle(
        &ctx,
        "check-diff",
        "extension:run-command",
        &Value::Null,
        &mut s,
    )
    .await
    .unwrap();
    let id = action(&f, "Find Difference");
    let identical = invoke(
        &ctx,
        &mut s,
        "check-diff",
        &id,
        json!({"original":"a","changed":"a"}),
    )
    .await;
    assert_eq!(identical["root"]["type"], "Form");
    assert!(identical.to_string().contains("Files are identical"));
    let a = (0..2200)
        .map(|n| format!("a{n}\n"))
        .collect::<Vec<_>>()
        .join("");
    let b = a.replace('a', "b");
    // npm path budget rejects an adversarial disjoint case, leaving the form retryable.
    let limited = invoke(
        &ctx,
        &mut s,
        "check-diff",
        &id,
        json!({"original":a,"changed":b}),
    )
    .await;
    assert_eq!(limited["root"]["type"], "Form");
    assert!(limited.to_string().contains("limit exceeded"));
    let d = invoke(
        &ctx,
        &mut s,
        "check-diff",
        &id,
        json!({"original":"a\r\n","changed":"b\n"}),
    )
    .await;
    assert_eq!(
        d["root"]["props"]["rawMarkdown"],
        format!(
            "\n  ## Diff\n\n  ```\n{}\n  ```",
            tezbar_diff_checker::diff::text("a\r\n", "b\n").unwrap()
        )
    );
    let restored = invoke(
        &ctx,
        &mut s,
        "check-diff",
        d["root"]["props"]["backActionId"].as_str().unwrap(),
        Value::Null,
    )
    .await;
    assert_eq!(restored["root"]["children"][0]["props"]["value"], "a\r\n");
}

#[tokio::test]
async fn format_feedback_each_field_and_ordered_submit_validation() {
    let (_d, ctx) = context();
    for (a, b, title) in [
        ("", "", "Nothing to Format"),
        ("{", "[", "Nothing to Format"),
        ("null", "true", "Formatted Both JSONs"),
        ("{", "[1]", "Formatted Modified JSON"),
        ("[1]", "{", "Formatted Original JSON"),
    ] {
        let mut state = json!({});
        let f = adapter::handle(
            &ctx,
            "compare-json",
            "extension:run-command",
            &Value::Null,
            &mut state,
        )
        .await
        .unwrap();
        let formatted = invoke(
            &ctx,
            &mut state,
            "compare-json",
            &action(&f, "Format JSONs"),
            json!({"original":a,"modified":b}),
        )
        .await;
        assert!(formatted.to_string().contains(title));
        if a == "{" {
            assert_eq!(state["original"], a);
        }
        if b == "{" || b == "[" {
            assert_eq!(state["modified"], b);
        }
    }
    let mut state = json!({});
    let f = adapter::handle(
        &ctx,
        "compare-json",
        "extension:run-command",
        &Value::Null,
        &mut state,
    )
    .await
    .unwrap();
    let submit = action(&f, "Compare");
    invoke(
        &ctx,
        &mut state,
        "compare-json",
        &submit,
        json!({"original":" ","modified":" "}),
    )
    .await;
    assert_eq!(state["original_error"], "JSON is required");
    assert_eq!(state["modified_error"], Value::Null);
    invoke(
        &ctx,
        &mut state,
        "compare-json",
        &submit,
        json!({"original":"null","modified":" "}),
    )
    .await;
    assert_eq!(state["modified_error"], "JSON is required");
    let blank = invoke(
        &ctx,
        &mut state,
        "compare-json",
        &field(&f, "original"),
        json!({"value":"\u{a0}"}),
    )
    .await;
    assert_eq!(state["original_error"], Value::Null);
    assert_eq!(blank["root"]["type"], "Form");
    let stale_paste = action(&f, "Paste Clipboard to Original");
    adapter::handle(
        &ctx,
        "compare-json",
        "extension:search-text-changed",
        &Value::Null,
        &mut state,
    )
    .await
    .unwrap();
    // No router is installed: a stale clipboard action must fail before requesting it.
    let result = adapter::handle(
        &ctx,
        "compare-json",
        "extension:invoke-action",
        &json!({"actionId":stale_paste}),
        &mut state,
    )
    .await;
    assert!(result.unwrap_err().contains("Stale"));
}

#[tokio::test]
async fn oversized_formatted_expansion_is_explicit_and_other_field_formats() {
    let (_d, ctx) = context();
    let mut state = json!({});
    let form = adapter::handle(
        &ctx,
        "compare-json",
        "extension:run-command",
        &Value::Null,
        &mut state,
    )
    .await
    .unwrap();
    let nested = format!(
        "{}{}{}",
        "[".repeat(100),
        vec!["0"; 6000].join(","),
        "]".repeat(100)
    );
    let v = invoke(
        &ctx,
        &mut state,
        "compare-json",
        &action(&form, "Format JSONs"),
        json!({"original":nested,"modified":"[1]"}),
    )
    .await;
    assert_eq!(state["original"], nested);
    assert_eq!(state["modified"], "[\n  1\n]");
    assert!(v.to_string().contains("Formatted Modified JSON"));
    assert!(v
        .to_string()
        .contains("Formatted JSON exceeds the 1 MiB field input limit"));
}
