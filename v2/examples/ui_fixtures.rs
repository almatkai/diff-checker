use serde_json::{json, Value};
use tezbar_diff_checker::session::Session;
fn id(v: &Value, title: &str) -> String {
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
fn main() {
    let mut s = Session::launch("compare-json").unwrap();
    let form = s.view(true);
    s.invoke(
        &id(&form, "Compare"),
        &json!({"original":"{\"b\":1,\"a\":2}","modified":"{\"b\":3,\"a\":2}"}),
    )
    .unwrap();
    let detail = s.view(true);
    let mut t = Session::launch("check-diff").unwrap();
    let tf = t.view(true);
    t.invoke(&id(&tf,"Find Difference"),&json!({"original":"safe\r```\n![payload](https://invalid.example/secret)\n````\n","changed":"safe\r```\n![payload](https://invalid.example/secret)\n````\nchanged"})).unwrap();
    println!(
        "{}",
        json!({"form":form,"detail":detail,"textForm":tf,"textDetail":t.view(true)})
    );
}
