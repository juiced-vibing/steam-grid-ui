use super::*;

#[test]
fn parses_nested_blocks_and_escapes() {
    let v = parse(
        r#"
        // comment
        "libraryfolders"
        {
            "0" { "path" "C:\\Program Files (x86)\\Steam" "apps" { "228980" "0" } }
            "1" { "path" "G:\\SteamLibrary" }
        }"#,
    )
    .unwrap();
    let libs = v.get("LibraryFolders").unwrap();
    assert_eq!(libs.entries().len(), 2);
    assert_eq!(
        libs.path(&["0", "path"]).unwrap().as_str(),
        Some(r"C:\Program Files (x86)\Steam")
    );
    assert_eq!(libs.path(&["0", "apps", "228980"]).unwrap().as_str(), Some("0"));
}

#[test]
fn rejects_unterminated_block() {
    assert!(parse(r#""a" { "b" "c""#).is_err());
}

const SHORTCUTS: &[u8] = b"\0shortcuts\0\x000\0\x02appid\0\x01\0\0\x80\x01AppName\0Celeste\0\0tags\0\x08\x08\x08\x08";

#[test]
fn parses_the_binary_form() {
    let v = parse_binary(SHORTCUTS).unwrap();
    let shortcut = v.path(&["shortcuts", "0"]).unwrap();

    // steam names the art after the unsigned value
    assert_eq!(shortcut.text("appid"), Some("2147483649"));
    assert_eq!(shortcut.text("appname"), Some("Celeste"));
    assert_eq!(shortcut.get("tags").unwrap().entries().len(), 0);
}

#[test]
fn rejects_binary_it_cannot_read() {
    for cut in [SHORTCUTS.len() - 3, 20, 14] {
        assert!(parse_binary(&SHORTCUTS[..cut]).is_err(), "cut at {cut}");
    }

    assert!(parse_binary(b"\x07wide\0\0\0\0\0\0\0\0\0\x08").is_err());

    // a file of nothing but opening blocks must not exhaust the stack
    assert!(parse_binary(&b"\0a\0".repeat(100_000)).is_err());
}
