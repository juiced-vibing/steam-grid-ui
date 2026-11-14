use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sgdb_core_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn a_missing_folder_is_not_a_steam_install() {
    let root = scratch("locate");
    let cfg = |path: &Path| Config {
        api_key: String::new(),
        steam_path: Some(path.to_string_lossy().into_owned()),
    };

    assert_eq!(Steam::locate(&cfg(&root)).unwrap().root(), root);
    assert!(matches!(
        Steam::locate(&cfg(&root.join("nope"))),
        Err(Error::SteamNotFound(_))
    ));

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn games_come_from_manifests_and_the_local_config() {
    let root = scratch("library");
    let other = scratch("library_other");
    let vdf_path = |path: &Path| path.display().to_string().replace('\\', "\\\\");

    write(
        &root.join("steamapps/libraryfolders.vdf"),
        &format!(
            r#""libraryfolders" {{ "0" {{ "path" "{}" }} "1" {{ "path" "{}" }} }}"#,
            vdf_path(&root),
            vdf_path(&other),
        ),
    );
    write(
        &root.join("steamapps/appmanifest_440.acf"),
        r#""AppState" { "appid" "440" "name" "Team Fortress 2" }"#,
    );
    write(
        &root.join("steamapps/appmanifest_228980.acf"),
        r#""AppState" { "appid" "228980" "name" "Steamworks Common Redistributables" }"#,
    );
    write(
        &other.join("steamapps/appmanifest_10.acf"),
        r#""AppState" { "appid" "10" "name" "Counter-Strike" }"#,
    );
    write(
        &root.join("userdata/7/config/localconfig.vdf"),
        r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "440" { } "70" { } "80" { } } } } } }"#,
    );
    write(&root.join("userdata/7/config/grid/440p.png"), "art");
    write(&root.join("userdata/7/config/grid/440_hero.JPG"), "art");
    write(&root.join("userdata/7/config/grid/440_logo.json"), "position, not art");

    let steam = Steam { root: root.clone() };
    let names = HashMap::from([(AppId::from(70), "Half-Life".to_string())]);
    let games = steam.games(UserId::from(7), &names);

    let game = |id: u32, name: Option<&str>, installed: bool, custom: &[Slot]| Game {
        appid: id.into(),
        name: name.map(str::to_string),
        installed,
        custom: custom.to_vec(),
    };
    assert_eq!(
        games,
        [
            game(10, Some("Counter-Strike"), true, &[]),
            game(70, Some("Half-Life"), false, &[]),
            game(440, Some("Team Fortress 2"), true, &[Slot::Grid, Slot::Hero]),
            game(80, None, false, &[]),
        ]
    );

    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&other).unwrap();
}
