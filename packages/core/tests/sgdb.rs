use serde_json::json;

use super::*;

#[test]
fn assets_are_parsed_from_the_api_shape() {
    let parsed = Asset::parse(&json!({
        "url": "https://cdn2.steamgriddb.com/grid/a.png",
        "thumb": "https://cdn2.steamgriddb.com/thumb/a.jpg",
        "width": 600,
        "height": 900,
        "style": "alternate",
        "author": { "name": "someone" },
    }))
    .unwrap();

    assert_eq!(parsed.url.as_str(), "https://cdn2.steamgriddb.com/grid/a.png");
    assert_eq!(
        parsed.preview.unwrap().as_str(),
        "https://cdn2.steamgriddb.com/thumb/a.jpg"
    );
    assert_eq!((parsed.width, parsed.height), (600, 900));
    assert_eq!(parsed.style.as_deref(), Some("alternate"));
    assert_eq!(parsed.author.as_deref(), Some("someone"));
}

#[test]
fn animated_assets_have_no_preview_and_foreign_urls_are_dropped() {
    let animated = Asset::parse(&json!({
        "url": "https://cdn2.steamgriddb.com/grid/a.webp",
        "thumb": "https://cdn2.steamgriddb.com/thumb/a.webm",
    }))
    .unwrap();
    assert_eq!(animated.preview, None);
    assert_eq!((animated.width, animated.height), (0, 0));

    assert!(Asset::parse(&json!({ "url": "https://example.com/a.png" })).is_none());
    assert!(Asset::parse(&json!({ "thumb": "https://cdn2.steamgriddb.com/thumb/a.jpg" })).is_none());
}

#[test]
fn only_a_full_page_promises_more() {
    let item = json!({ "url": "https://cdn2.steamgriddb.com/grid/a.png" });

    let full = Page::parse(&json!({ "limit": 2, "total": 9, "data": [item, item] }));
    assert_eq!(full.items.len(), 2);
    assert!(full.more);

    // "total" also counts what the filters hide, so it is ignored
    let short = Page::parse(&json!({ "limit": 2, "total": 9, "data": [item] }));
    assert!(!short.more);

    let empty = Page::parse(&json!({}));
    assert_eq!(empty.items, []);
    assert!(!empty.more);
}

#[test]
fn each_query_has_its_own_cache_key() {
    let grid = page_key("/grids/steam/440", &[("types", "static"), ("page", "0")]);
    let next = page_key("/grids/steam/440", &[("types", "static"), ("page", "1")]);
    assert_eq!(grid, "/grids/steam/440?types=static&page=0");
    assert_ne!(grid, next);
}

#[test]
fn every_kind_has_an_endpoint() {
    assert_eq!(endpoint(ArtKind::Grid).0, "grids");
    assert_eq!(endpoint(ArtKind::Wide).0, "grids");
    assert_ne!(endpoint(ArtKind::Grid).1, endpoint(ArtKind::Wide).1);
    assert_eq!(endpoint(ArtKind::Icon), ("icons", None));
}

#[test]
fn matches_are_parsed_from_the_search_shape() {
    let found = Match::parse(&json!({ "id": 5248, "name": "Celeste", "release_date": 1_516_838_400 })).unwrap();
    assert_eq!(
        (found.id, found.name.as_str(), found.year),
        (5248.into(), "Celeste", Some(2018))
    );

    assert_eq!(Match::parse(&json!({ "id": 1, "name": "Undated" })).unwrap().year, None);
    assert!(Match::parse(&json!({ "id": 1, "name": "" })).is_none());
    assert!(Match::parse(&json!({ "id": -1, "name": "Negative" })).is_none());
    assert!(Match::parse(&json!({ "id": 4_294_967_296_u64, "name": "Too big" })).is_none());
}

#[test]
fn years_turn_over_at_midnight_utc() {
    assert_eq!(year_of(0), 1970);
    assert_eq!(year_of(-1), 1969);
    assert_eq!(year_of(951_782_400), 2000, "29 february 2000");
    assert_eq!(year_of(1_704_067_199), 2023);
    assert_eq!(year_of(1_704_067_200), 2024);
    assert_eq!(year_of(i64::MAX), 292_277_026_596);
}

#[test]
fn a_search_term_cannot_leave_its_path_segment() {
    assert_eq!(path_segment("Half-Life 2"), "Half-Life%202");
    assert_eq!(path_segment("../games?x=1#y"), "..%2Fgames%3Fx%3D1%23y");
    assert_eq!(path_segment("Pokémon"), "Pok%C3%A9mon");
}
