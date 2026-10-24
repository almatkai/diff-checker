use diff_checker_beta_host::{
    common::{CoreContext, CorePaths},
    extensions::Host,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
fn action(v: &Value, title: &str) -> String {
    v["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["title"] == title)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
#[tokio::test]
async fn byte_exact_native_session_router_error_envelope_back_dispose_relaunch() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let ctx = CoreContext::new(
        CorePaths {
            data_dir: p.join("data"),
            config_dir: p.join("config"),
            resource_dir: p.join("res"),
            temp_dir: p.join("temp"),
            home_dir: p.into(),
        },
        None,
    );
    let writes = Arc::new(Mutex::new(0));
    let w = writes.clone();
    ctx.set_router(Arc::new(move |ch, args| {
        assert_eq!(ch, "clipboard:write");
        assert_eq!(args.len(), 1);
        *w.lock().unwrap() += 1;
        Box::pin(async { Ok(json!({"ok":true})) })
    }))
    .unwrap();
    let host = Host::default();
    let launch =
        json!({"extensionId":"com.tezbar.native.diff-checker","commandName":"compare-json"});
    let form = host
        .handle(&ctx, "extension:run-command", launch.clone())
        .await;
    assert_eq!(form["ok"], true);
    let sid = form["sessionId"].as_str().unwrap();
    assert!(sid.starts_with("native-extension:"));
    let detail=host.handle(&ctx,"extension:invoke-action",json!({"sessionId":sid,"actionId":action(&form,"Compare"),"formValues":{"original":"{\"z\":1}","modified":"null"}})).await;
    assert_eq!(detail["root"]["type"], "Detail");
    let copied = host
        .handle(
            &ctx,
            "extension:invoke-action",
            json!({"sessionId":sid,"actionId":action(&detail,"Copy Full Diff")}),
        )
        .await;
    assert_eq!(copied["ok"], true);
    assert_eq!(*writes.lock().unwrap(), 1);
    let rejected = host
        .handle(
            &ctx,
            "extension:invoke-action",
            json!({"sessionId":sid,"actionId":action(&form,"Compare")}),
        )
        .await;
    assert_eq!(rejected["ok"], false);
    assert!(rejected["message"].as_str().unwrap().contains("Stale"));
    let back = host
        .handle(
            &ctx,
            "extension:invoke-action",
            json!({"sessionId":sid,"actionId":detail["root"]["props"]["backActionId"]}),
        )
        .await;
    assert_eq!(back["root"]["type"], "Form");
    assert!(back.to_string().contains("z"));
    let changed = host
        .handle(
            &ctx,
            "extension:search-text-changed",
            json!({"sessionId":sid,"searchText":"x"}),
        )
        .await;
    assert_eq!(changed["root"]["type"], "Form");
    let stale = host
        .handle(
            &ctx,
            "extension:invoke-action",
            json!({"sessionId":sid,"actionId":action(&back,"Compare")}),
        )
        .await;
    assert_eq!(stale["ok"], false);
    assert_eq!(
        host.handle(&ctx, "extension:dispose-session", json!({"sessionId":sid}))
            .await,
        true
    );
    assert_eq!(
        host.handle(
            &ctx,
            "extension:invoke-action",
            json!({"sessionId":sid,"actionId":action(&detail,"Copy Full Diff")})
        )
        .await["ok"],
        false
    );
    assert_eq!(*writes.lock().unwrap(), 1);
    let second = host.handle(&ctx, "extension:run-command", launch).await;
    assert_ne!(second["sessionId"], sid);
    assert_ne!(action(&second, "Compare"), action(&form, "Compare"));
    host.clear_slug().await;
    assert_eq!(
        host.handle(
            &ctx,
            "extension:invoke-action",
            json!({"sessionId":second["sessionId"],"actionId":action(&second,"Compare")})
        )
        .await["ok"],
        false
    );
    host.clear().await;
    assert!(!p.join("config").exists());
    assert!(!p.join("data").exists());
}
