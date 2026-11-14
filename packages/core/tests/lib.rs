use super::*;

const ICON: &str = "https://cdn2.steamgriddb.com/icon/a.png";

fn parse(method: &str, args: &Value) -> std::result::Result<Request, serde_json::Error> {
    serde_json::from_value(json!({ "method": method, "args": args }))
}

fn error_code(response: &str) -> String {
    let value: Value = serde_json::from_str(response).unwrap();
    assert_eq!(value["ok"], false, "{response}");
    assert!(value["error"]["message"].as_str().is_some_and(|m| !m.is_empty()));
    value["error"]["code"].as_str().unwrap().to_string()
}

#[test]
fn requests_parse_into_typed_values() {
    let args = json!({ "appid": "440", "kind": "hero", "page": 2, "animated": true });
    let request = parse("list_assets", &args).unwrap();

    let Request::ListAssets {
        appid,
        kind,
        page,
        filters,
        need_name,
    } = request
    else {
        panic!("wrong variant: {request:?}");
    };
    assert_eq!((appid, kind, page), (AppId::from(440), ArtKind::Hero, 2));
    assert!(filters.animated && !filters.nsfw && !filters.humor && !need_name);

    assert!(matches!(parse("init", &json!({})), Ok(Request::Init {})));
}

#[test]
fn invalid_requests_do_not_parse() {
    // steam has no icon slot
    let apply = |slot: &str| {
        parse(
            "apply",
            &json!({ "user": "7", "appid": "440", "slot": slot, "url": ICON }),
        )
    };
    assert!(apply("icon").is_err());
    assert!(apply("hero").is_ok());

    // ids that could escape a path
    assert!(parse("clear", &json!({ "user": "../7", "appid": "440", "slot": "hero" })).is_err());
    assert!(parse("list_games", &json!({ "user": 7 })).is_err());

    // downloads from anywhere else
    assert!(parse("thumb", &json!({ "url": "https://example.com/a.png" })).is_err());
    assert!(parse("save_as", &json!({ "url": "file:///c:/a.png", "dest": "a.png" })).is_err());

    assert!(parse("list_assets", &json!({ "appid": "440", "kind": "poster" })).is_err());
    assert!(parse("nope", &json!({})).is_err());
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

    let placed = reply(&steam::Placed::Unchanged { path: "a.png".into() }).unwrap();
    assert!(placed.starts_with(r#"{"outcome":"unchanged","#), "{placed}");
}
