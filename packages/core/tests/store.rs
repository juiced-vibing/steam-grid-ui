use super::*;

#[test]
fn reads_keys_from_env_text() {
    let text = "# comment\nOTHER=1\nSGDB_API_KEY = \"abc123\"\n";
    assert_eq!(parse_env(text, "SGDB_API_KEY").as_deref(), Some("abc123"));
    assert_eq!(parse_env("SGDB_API_KEY=\n", "SGDB_API_KEY"), None);
    assert_eq!(parse_env("#SGDB_API_KEY=x\n", "SGDB_API_KEY"), None);
}

#[test]
fn prune_dir_drops_the_oldest_files_first() {
    let dir = std::env::temp_dir().join(format!("sgdb_core_prune_dir_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // four files of 100 bytes, a.bin the oldest
    for (age, name) in ["d.bin", "c.bin", "b.bin", "a.bin"].into_iter().enumerate() {
        let path = dir.join(name);
        fs::write(&path, [0u8; 100]).unwrap();
        let modified = SystemTime::now() - std::time::Duration::from_secs(60 * age as u64);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(modified)
            .unwrap();
    }

    assert_eq!(prune_dir(&dir, 400), 0, "at the limit nothing is removed");

    // over a 300 byte limit it prunes down to 225, so two files go
    assert_eq!(prune_dir(&dir, 300), 2);
    assert!(!dir.join("a.bin").exists());
    assert!(!dir.join("b.bin").exists());
    assert!(dir.join("c.bin").exists());
    assert!(dir.join("d.bin").exists());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn remembered_values_survive_and_are_replaced() {
    let path = std::env::temp_dir().join(format!("sgdb_core_remember_{}.json", std::process::id()));
    let _ = fs::remove_file(&path);
    let read = || read_json::<HashMap<AppId, String>>(&path);

    remember_in(&path, 70.into(), "Half-Life".to_string());
    remember_in(&path, 80.into(), "Condition Zero".to_string());
    remember_in(&path, 70.into(), "Half-Life (1998)".to_string());
    assert_eq!(read().len(), 2);
    assert_eq!(read()[&AppId::from(70)], "Half-Life (1998)");

    // a file that cannot be read is started over, not an error
    fs::write(&path, "{ not json").unwrap();
    assert!(read().is_empty());
    remember_in(&path, 70.into(), "Half-Life".to_string());
    assert_eq!(read().len(), 1);

    fs::remove_file(&path).unwrap();
}
