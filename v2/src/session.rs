//! Session-owned capabilities. Action payloads never supply committed copy content.
use crate::{diff, input, json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    command: String,
    token: String,
    detail: bool,
    active: bool,
    original: String,
    modified: String,
    original_error: Option<String>,
    modified_error: Option<String>,
    notice: Option<String>,
    committed: Option<(String, String)>,
}
#[derive(Debug)]
pub enum Effect {
    ReadOriginal,
    ReadModified,
    Copy(String),
    None,
}
impl Session {
    pub fn launch(command: &str) -> Result<Self, String> {
        if !commands().contains(&command) {
            return Err("Unknown Diff Checker command".into());
        }
        Ok(Self {
            command: command.into(),
            token: token()?,
            detail: false,
            active: true,
            original: String::new(),
            modified: String::new(),
            original_error: None,
            modified_error: None,
            notice: None,
            committed: None,
        })
    }
    pub fn matches_command(&self, command: &str) -> bool {
        self.command == command
    }
    fn id(&self, name: &str) -> String {
        format!("dc:{}:{}:{name}", self.command, self.token)
    }
    fn rotate(&mut self) -> Result<(), String> {
        self.token = token()?;
        Ok(())
    }
    pub fn invalidate(&mut self) {
        self.active = false;
        self.committed = None;
        self.original.clear();
        self.modified.clear();
    }
    pub fn search(&mut self) -> Result<Value, String> {
        self.rotate()?;
        Ok(self.view(false))
    }
    pub fn invoke(&mut self, id: &str, values: &Value) -> Result<Effect, String> {
        if !self.active {
            return Err("Diff Checker session is disposed".into());
        }
        let names: &[&str] = if self.detail {
            if self.command == "compare-json" {
                &[
                    "copy-full",
                    "copy-compact",
                    "copy-original",
                    "copy-modified",
                    "back",
                ]
            } else {
                &["back"]
            }
        } else if self.command == "compare-json" {
            &[
                "original",
                "modified",
                "submit",
                "format",
                "swap",
                "paste-original",
                "paste-modified",
            ]
        } else {
            &["original", "modified", "submit"]
        };
        let name = names
            .iter()
            .find(|n| self.id(n) == id)
            .copied()
            .ok_or("Stale or unknown Diff Checker action")?;
        // Capability and phase validated before interpreting any event values/effects.
        if self.detail {
            if name == "back" {
                self.rotate()?;
                self.detail = false;
                self.committed = None;
                self.notice = None;
                return Ok(Effect::None);
            }
            let (a, b) = self
                .committed
                .as_ref()
                .ok_or("Missing committed comparison")?;
            let result = diff::json(a, b)?; // recompute exclusively from committed source inputs
            return Ok(Effect::Copy(match name {
                "copy-full" => result.full_diff,
                "copy-compact" => result.markdown,
                "copy-original" => result.original_formatted,
                "copy-modified" => result.modified_formatted,
                _ => unreachable!(),
            }));
        }
        if matches!(name, "original" | "modified") {
            let text = values["value"]
                .as_str()
                .ok_or("Text field requires a string")?;
            input(text)?;
            if name == "original" {
                self.original = text.into();
                if self.command == "compare-json" {
                    self.original_error = live_error(text);
                }
            } else {
                self.modified = text.into();
                if self.command == "compare-json" {
                    self.modified_error = live_error(text);
                }
            }
            self.notice = None;
            return Ok(Effect::None);
        }
        // Beta sends a complete current draft on submit. Other actions use serialized
        // field callbacks; optional full drafts are accepted atomically when provided.
        let second = if self.command == "check-diff" {
            "changed"
        } else {
            "modified"
        };
        if values.get("original").is_some() || values.get(second).is_some() {
            let a = values["original"]
                .as_str()
                .ok_or("Original field requires a string")?;
            let b = values[second]
                .as_str()
                .ok_or("Modified field requires a string")?;
            input(a)?;
            input(b)?;
            self.original = a.into();
            self.modified = b.into();
        }
        self.notice = None;
        match name {
            "submit" => {
                if self.command == "check-diff" {
                    if self.original == self.modified {
                        self.notice = Some("Files are identical".into());
                        return Ok(Effect::None);
                    }
                    if let Err(e) = diff::text(&self.original, &self.modified).and_then(|text| {
                        diff::render_markdown(&format!("\n  ## Diff\n\n  ```\n{text}\n  ```"))
                    }) {
                        self.notice = Some(e);
                        return Ok(Effect::None);
                    }
                } else {
                    if let Err(e) = json::format(&self.original) {
                        self.original_error = Some(e);
                        return Ok(Effect::None);
                    }
                    if let Err(e) = json::format(&self.modified) {
                        self.modified_error = Some(e);
                        return Ok(Effect::None);
                    }
                    if let Err(e) = diff::json(&self.original, &self.modified)
                        .and_then(|result| diff::render_markdown(&result.markdown))
                    {
                        self.notice = Some(e);
                        return Ok(Effect::None);
                    }
                }
                self.rotate()?;
                self.committed = Some((self.original.clone(), self.modified.clone()));
                self.detail = true;
            }
            "format" => {
                let bounded_format = |text: &str| -> Result<String, String> {
                    let formatted = json::format(text)?;
                    if formatted.len() > crate::INPUT_LIMIT {
                        return Err("Formatted JSON exceeds the 1 MiB field input limit".into());
                    }
                    Ok(formatted)
                };
                let a = bounded_format(&self.original);
                let b = bounded_format(&self.modified);
                let mut limits = Vec::new();
                if let Err(e) = &a {
                    if e.contains("limit") {
                        limits.push(format!("Original: {e}"));
                    }
                }
                if let Err(e) = &b {
                    if e.contains("limit") {
                        limits.push(format!("Modified: {e}"));
                    }
                }
                let (ok_a, ok_b) = (a.is_ok(), b.is_ok());
                if let Ok(a) = a {
                    self.original = a;
                    self.original_error = None;
                }
                if let Ok(b) = b {
                    self.modified = b;
                    self.modified_error = None;
                }
                self.notice = Some(
                    match (ok_a, ok_b) {
                        (true, true) => "Formatted Both JSONs",
                        (true, false) => "Formatted Original JSON",
                        (false, true) => "Formatted Modified JSON",
                        _ => "Nothing to Format — Both fields are empty or contain invalid JSON",
                    }
                    .into(),
                );
                if !limits.is_empty() {
                    self.notice = Some(format!(
                        "{}\n{}",
                        self.notice.as_deref().unwrap_or_default(),
                        limits.join("\n")
                    ));
                }
            }
            "swap" => {
                std::mem::swap(&mut self.original, &mut self.modified); /* source deliberately retains existing errors */
            }
            "paste-original" => return Ok(Effect::ReadOriginal),
            "paste-modified" => return Ok(Effect::ReadModified),
            _ => unreachable!(),
        }
        Ok(Effect::None)
    }
    pub fn pasted(&mut self, original: bool, response: Result<Value, String>) {
        match response {
            Ok(Value::String(s)) if s.is_empty() => {}
            Ok(Value::String(s)) => {
                if let Err(e) = input(&s) {
                    self.notice = Some(e);
                    return;
                }
                if original {
                    self.original = s;
                } else {
                    self.modified = s;
                }
                self.notice = Some(format!(
                    "Pasted into {} JSON",
                    if original { "Original" } else { "Modified" }
                ));
                // As in useJSONForm, paste does not run the live validation setters.
            }
            _ => self.notice = Some("Could not read clipboard".into()),
        }
    }
    pub fn copied(&mut self, response: Result<Value, String>) {
        self.notice = Some(match response {
            Ok(v)
                if v.is_object()
                    && v["ok"] == true
                    && v.get("error").is_none_or(Value::is_null)
                    && v.get("success")
                        .is_none_or(|value| value == &Value::Bool(true)) =>
            {
                "Copied".into()
            }
            _ => "Could not copy. Try again.".into(),
        });
    }
    pub fn is_field_action(&self, id: &str) -> bool {
        !self.detail && (id == self.id("original") || id == self.id("modified"))
    }
    pub fn view(&self, include_values: bool) -> Value {
        if self.detail {
            return self.detail_view();
        }
        let json_cmd = self.command == "compare-json";
        let mut children = Vec::new();
        if json_cmd {
            children.push(node(
                "Form.Description",
                json!({"text":"Paste two JSON snippets and compare them."}),
            ));
        }
        if let Some(n) = &self.notice {
            children.push(node("Form.Description", json!({"text":n})));
        }
        for (i, (name, title, text, error)) in [
            (
                "original",
                if json_cmd {
                    "Original JSON"
                } else {
                    "Original Text"
                },
                &self.original,
                &self.original_error,
            ),
            (
                if json_cmd { "modified" } else { "changed" },
                if json_cmd {
                    "Modified JSON"
                } else {
                    "Changed Text"
                },
                &self.modified,
                &self.modified_error,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            if json_cmd && i == 1 {
                children.push(node("Form.Separator", json!({})));
            }
            let mut props = json!({"id":name,"title":title,"actionId":self.id(if i==0 {"original"} else {"modified"})});
            if include_values {
                props["value"] = json!(text);
            }
            if let Some(e) = error {
                props["error"] = json!(e);
            }
            if json_cmd {
                props["placeholder"] = json!("{\"key\": \"value\"}");
            }
            children.push(node("Form.TextArea", props));
        }
        let mut actions = vec![self.action(
            "submit",
            if json_cmd {
                "Compare"
            } else {
                "Find Difference"
            },
            "submit-form",
            None,
        )];
        if json_cmd {
            for (name, title, mods, key) in [
                ("format", "Format JSONs", vec!["cmd", "shift"], "f"),
                ("swap", "Swap JSONs", vec!["cmd", "shift"], "s"),
                (
                    "paste-original",
                    "Paste Clipboard to Original",
                    vec!["cmd", "shift"],
                    "1",
                ),
                (
                    "paste-modified",
                    "Paste Clipboard to Modified",
                    vec!["cmd", "shift"],
                    "2",
                ),
            ] {
                actions.push(self.action(name, title, "action", Some((mods, key))));
            }
        }
        json!({"ok":true,"mode":"view","title":if json_cmd {"Compare JSON"} else {"Check Diff"},"root":{"type":"Form","props":{},"children":children},"actions":actions})
    }
    fn action(
        &self,
        name: &str,
        title: &str,
        kind: &str,
        shortcut: Option<(Vec<&str>, &str)>,
    ) -> Value {
        let mut a = json!({"id":self.id(name),"title":title,"kind":kind});
        if let Some((mods, key)) = shortcut {
            a["shortcut"] = json!({"modifiers":mods,"key":key});
        }
        a
    }
    fn detail_view(&self) -> Value {
        let (a, b) = self.committed.as_ref().expect("committed detail");
        let (raw, metadata, mut actions) = if self.command == "check-diff" {
            (
                format!(
                    "\n  ## Diff\n\n  ```\n{}\n  ```",
                    diff::text(a, b).expect("bounded committed text")
                ),
                Value::Null,
                vec![],
            )
        } else {
            let r = diff::json(a, b).expect("bounded committed JSON");
            let tag = |title: &str, text: String, color: &str| json!({"type":"Detail.Metadata.TagList","props":{"title":title},"children":[node("Detail.Metadata.TagList.Item",json!({"text":text,"color":color}))]});
            let meta = json!({"type":"Detail.Metadata","children":[tag("Added Lines",format!("+{}",r.additions),"success"),tag("Removed Lines",format!("-{}",r.removals),"error"),node("Detail.Metadata.Separator",json!({})),tag("Total Changed",format!("{} lines",r.additions+r.removals),if r.additions+r.removals==0 {"success"} else {"warning"})]});
            (
                r.markdown,
                meta,
                vec![
                    self.action("copy-full", "Copy Full Diff", "action", None),
                    self.action(
                        "copy-compact",
                        "Copy Compact Diff",
                        "action",
                        Some((vec!["cmd", "shift"], "c")),
                    ),
                    self.action(
                        "copy-original",
                        "Copy Formatted Original",
                        "action",
                        Some((vec!["cmd"], "1")),
                    ),
                    self.action(
                        "copy-modified",
                        "Copy Formatted Modified",
                        "action",
                        Some((vec!["cmd"], "2")),
                    ),
                ],
            )
        };
        let back = self.id("back");
        actions.push(self.action(
            "back",
            if self.command == "compare-json" {
                "Compare Again"
            } else {
                "Back"
            },
            "action",
            if self.command == "compare-json" {
                Some((vec!["cmd"], "r"))
            } else {
                None
            },
        ));
        let mut children = Vec::new();
        if let Some(n) = &self.notice {
            children.push(node(
                "Detail.Metadata.Label",
                json!({"title":"Clipboard","text":n}),
            ));
        }
        let mut meta = metadata;
        if !children.is_empty() {
            if meta.is_null() {
                meta = json!({"type":"Detail.Metadata","children":[]});
            }
            meta["children"].as_array_mut().unwrap().extend(children);
        }
        json!({"ok":true,"mode":"view","title":if self.command=="compare-json" {"Compare JSON"} else {"Check Diff"},"root":{"type":"Detail","props":{"markdown":diff::render_markdown(&raw).expect("bounded committed rendering"),"rawMarkdown":raw,"backActionId":back},"metadata":meta},"actions":actions})
    }
}
fn live_error(s: &str) -> Option<String> {
    if json::trim_js(s).is_empty() {
        None
    } else {
        json::validate(s)
    }
}
fn node(kind: &str, props: Value) -> Value {
    json!({"type":kind,"props":props})
}
fn token() -> Result<String, String> {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).map_err(|_| "Could not create action capabilities")?;
    const HEX: &[u8] = b"0123456789abcdef";
    let mut text = String::with_capacity(32);
    for byte in b {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 15) as usize] as char);
    }
    Ok(text)
}
pub fn commands() -> &'static [&'static str] {
    &["check-diff", "compare-json"]
}
