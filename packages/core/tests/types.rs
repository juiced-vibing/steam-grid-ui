use super::*;

fn url(text: &str) -> Result<AssetUrl, Error> {
    AssetUrl::try_from(text.to_string())
}

#[test]
fn ids_are_numbers_and_nothing_else() {
    assert_eq!(AppId::try_from("440".to_string()).unwrap(), AppId::from(440));
    assert_eq!(AppId::from(440).to_string(), "440");

    for bad in ["", "abc", "../x", "44 0", "-1", "99999999999"] {
        assert!(UserId::try_from(bad.to_string()).is_err(), "{bad:?}");
    }

    // in JSON they are strings, both ways
    let id: AppId = serde_json::from_str("\"42700\"").unwrap();
    assert_eq!(serde_json::to_string(&id).unwrap(), "\"42700\"");
    assert!(serde_json::from_str::<AppId>("\"../x\"").is_err());
}

#[test]
fn api_key_is_never_empty() {
    assert!(ApiKey::new("").is_none());
    assert!(ApiKey::new("   ").is_none());
    assert_eq!(ApiKey::new(" abc\n").unwrap().as_str(), "abc");
}

#[test]
fn slots_map_to_steam_file_names() {
    let app = AppId::from(440);
    assert_eq!(Slot::Grid.file_stem(app), "440p");
    assert_eq!(Slot::Wide.file_stem(app), "440");
    assert_eq!(Slot::Hero.file_stem(app), "440_hero");
    assert_eq!(Slot::Logo.file_stem(app), "440_logo");
}

#[test]
fn icons_are_a_kind_but_not_a_slot() {
    assert_eq!(serde_json::from_str::<ArtKind>("\"icon\"").unwrap(), ArtKind::Icon);
    assert!(serde_json::from_str::<Slot>("\"icon\"").is_err());
    assert_eq!(serde_json::from_str::<Slot>("\"hero\"").unwrap(), Slot::Hero);
}

#[test]
fn only_steamgriddb_urls_exist() {
    assert!(url("https://cdn2.steamgriddb.com/grid/abc.png").is_ok());
    assert!(url("https://steamgriddb.com/x").is_ok());

    for bad in [
        "https://evilsteamgriddb.com/x.png",
        "https://steamgriddb.com.evil.io/x.png",
        "http://cdn2.steamgriddb.com/x.png",
        "file:///c:/x.png",
        "",
    ] {
        assert!(matches!(url(bad), Err(Error::UntrustedUrl)), "{bad:?}");
    }
}

#[test]
fn url_parts() {
    let png = url("https://cdn2.steamgriddb.com/grid/abc.PNG?x=1").unwrap();
    assert_eq!(png.ext(), Some("PNG"));
    assert_eq!(png.format(), Some(ImageFormat::Png));
    assert_eq!(png.file_name().as_deref(), Some("abc.PNG"));

    let video = url("https://cdn2.steamgriddb.com/thumb/abc.webm").unwrap();
    assert_eq!(video.ext(), Some("webm"));
    assert_eq!(video.format(), None);

    let bare = url("https://cdn2.steamgriddb.com/grid/abc").unwrap();
    assert_eq!(bare.ext(), None);

    let odd = url("https://cdn2.steamgriddb.com/thumb/a..b/../c d.jpg").unwrap();
    assert_eq!(odd.file_name().as_deref(), Some("cd.jpg"));
    assert_eq!(url("https://cdn2.steamgriddb.com/thumb/").unwrap().file_name(), None);
    assert_eq!(
        url("https://cdn2.steamgriddb.com/thumb/.hidden").unwrap().file_name(),
        None
    );
}

#[test]
fn image_formats() {
    assert_eq!(ImageFormat::of(Path::new("440p.JPG")), Some(ImageFormat::Jpg));
    assert_eq!(ImageFormat::of(Path::new("440.json")), None);
    assert_eq!(ImageFormat::of(Path::new("440")), None);
    assert_eq!(ImageFormat::Webp.ext(), "webp");
}
