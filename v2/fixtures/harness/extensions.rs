pub use crate::v2_sdk;
pub mod registry {
    use crate::common::CoreContext;
    use serde_json::Value;
    pub fn installed_command(_: &CoreContext, id: &str, command: &str) -> Option<()> {
        if super::v2_runtime::supports(id, command) {
            Some(())
        } else {
            None
        }
    }
    pub fn preferences(_: &(), _: Option<&str>) -> Result<Value, String> {
        Ok(serde_json::json!({}))
    }
}
pub mod v2_runtime {
    use crate::common::{CoreContext, CoreResult};
    use serde_json::Value;
    pub fn slug(id: &str) -> Option<&str> {
        match id {
            "com.tezbar.native.diff-checker" | "raycast.diff-checker" | "diff-checker" => {
                Some("diff-checker")
            }
            _ => None,
        }
    }
    pub fn supports(id: &str, c: &str) -> bool {
        slug(id).is_some() && crate::production_adapter::commands().contains(&c)
    }
    pub async fn handle(
        ctx: &CoreContext,
        _slug: &str,
        c: &str,
        op: &str,
        p: &Value,
        s: &mut Value,
    ) -> CoreResult {
        crate::production_adapter::handle(ctx, c, op, p, s).await
    }
}

#[rustfmt::skip]
#[path = "../beta/src-tauri/src/native_core/extensions/native.rs"]
mod native;
#[derive(Clone, Default)]
pub struct Host(native::NativeSessions);
impl Host {
    pub async fn handle(
        &self,
        ctx: &crate::common::CoreContext,
        ch: &str,
        p: serde_json::Value,
    ) -> serde_json::Value {
        self.0
            .handle(
                ctx,
                &std::sync::Arc::new(std::sync::Mutex::new(())),
                ch,
                &[p],
            )
            .await
            .unwrap()
            .unwrap()
    }
    pub async fn clear(&self) {
        self.0.clear().await;
    }
    pub async fn clear_slug(&self) {
        self.0.clear_slug("diff-checker").await;
    }
}
