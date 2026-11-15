use serde_json::Value;
use serde_json::json;

use super::*;

const ICON: &str = "https://cdn2.steamgriddb.com/icon/a.png";

fn parse(method: &str, args: &Value) -> Result<Request, serde_json::Error> {
    serde_json::from_value(json!({ "method": method, "args": args }))
}

#[test]
fn requests_parse_into_typed_values() {
    let args = json!({ "game": { "by": "steam", "appid": "440" }, "kind": "hero", "page": 2, "animated": true });
    let request = parse("list_assets", &args).unwrap();

    let Request::ListAssets {
        game,
        kind,
        page,
        filters,
        need_name,
    } = request
    else {
        panic!("wrong variant: {request:?}");
    };
    assert_eq!(game.api_path(), "steam/440");
    assert_eq!((kind, page), (ArtKind::Hero, 2));
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

    let list = |game: Value, kind: &str| parse("list_assets", &json!({ "game": game, "kind": kind }));
    assert!(list(json!({ "by": "sgdb", "id": "5248" }), "grid").is_ok());
    assert!(list(json!({ "by": "steam", "appid": "440" }), "poster").is_err());
    assert!(list(json!({ "by": "sgdb", "id": "5248/../1" }), "grid").is_err());
    assert!(list(json!({ "by": "name", "name": "Celeste" }), "grid").is_err());
    assert!(parse("nope", &json!({})).is_err());
}
