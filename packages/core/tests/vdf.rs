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
