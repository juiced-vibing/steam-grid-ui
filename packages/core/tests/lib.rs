use super::*;

fn error_code(response: &str) -> String {
    let value: Value = serde_json::from_str(response).unwrap();
    assert_eq!(value["ok"], false, "{response}");
    assert!(value["error"]["message"].as_str().is_some_and(|m| !m.is_empty()));
    value["error"]["code"].as_str().unwrap().to_string()
}

#[test]
fn failures_reach_the_frontend_as_a_code_and_a_message() {
    assert_eq!(error_code(&respond("nope", "{}")), "bad_request");
    assert_eq!(error_code(&respond("init", "not json")), "bad_request");
    assert_eq!(
        error_code(&respond("thumb", r#"{"url":"https://example.com/a.png"}"#)),
        "bad_request"
    );
    assert_eq!(error_code(&respond("set_api_key", r#"{"key":"  "}"#)), "no_api_key");
}

#[test]
fn replies_put_the_tag_first() {
    let known = reply(&AssetPage::Known {
        items: Vec::new(),
        more: false,
        game_name: None,
    })
    .unwrap();
    assert!(known.starts_with(r#"{"state":"known","#), "{known}");

    let placed = reply(&grid::Placed::Unchanged { path: "a.png".into() }).unwrap();
    assert!(placed.starts_with(r#"{"outcome":"unchanged","#), "{placed}");
}
