//! Include unchanged inside native_core::extensions::v2_diff_checker.
//! Depends on the vendored tezbar-diff-checker crate and beta common/v2_sdk modules.
use super::v2_sdk;
use crate::native_core::common::{CoreContext, CoreResult};
use serde_json::Value;
use tezbar_diff_checker::session::{Effect, Session};

pub fn commands() -> &'static [&'static str] {
    tezbar_diff_checker::session::commands()
}

pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    if !commands().contains(&command) {
        return Err("Unknown Diff Checker command".into());
    }
    if operation == "extension:run-command" {
        let session = Session::launch(command)?;
        let view = session.view(true);
        *state = serde_json::to_value(session).map_err(|_| "Could not encode session")?;
        return Ok(view);
    }
    let mut session: Session =
        serde_json::from_value(state.clone()).map_err(|_| "Diff Checker session is not active")?;
    if !session.matches_command(command) {
        return Err("Cross-command session rejected".into());
    }
    if operation == "extension:dispose-session" {
        session.invalidate();
        *state = Value::Null;
        return Ok(v2_sdk::unchanged());
    }
    let view = match operation {
        "extension:search-text-changed" => session.search()?,
        "extension:refresh-session" => return Ok(v2_sdk::unchanged()),
        "extension:invoke-action" => {
            let id = payload["actionId"].as_str().ok_or("Missing actionId")?;
            let typing = session.is_field_action(id);
            let effect = session.invoke(id, &payload["formValues"])?;
            match effect {
                Effect::ReadOriginal => {
                    session.pasted(true, ctx.request("clipboard:read", vec![]).await)
                }
                Effect::ReadModified => {
                    session.pasted(false, ctx.request("clipboard:read", vec![]).await)
                }
                Effect::Copy(text) => session.copied(v2_sdk::copy(ctx, &text).await),
                Effect::None => {}
            }
            session.view(!typing)
        }
        _ => return Err("Unsupported Diff Checker operation".into()),
    };
    *state = serde_json::to_value(session).map_err(|_| "Could not encode session")?;
    // Use the production SDK envelope rather than a JSON pretend dispatcher.
    Ok(v2_sdk::view(
        view["title"].as_str().unwrap_or("Diff Checker"),
        view["root"].clone(),
        view["actions"].as_array().cloned().unwrap_or_default(),
    ))
}
