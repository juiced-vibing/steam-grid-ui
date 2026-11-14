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

fn grid_in(root: &Path) -> GridDir {
    GridDir {
        dir: root.join("grid"),
        backups: root.join("backups"),
    }
}

fn apply(grid: &GridDir, slot: Slot, format: ImageFormat, content: &[u8]) -> Placed {
    let app = AppId::from(440);
    let staged = grid.stage(app, slot).unwrap();
    fs::write(staged.path(), content).unwrap();
    grid.place(staged, app, slot, format).unwrap()
}

#[test]
fn placing_backs_up_what_was_there() {
    let root = scratch("place");
    let grid = grid_in(&root);
    let png = root.join("grid/440_hero.png");
    let jpg = root.join("grid/440_hero.jpg");

    let first = apply(&grid, Slot::Hero, ImageFormat::Png, b"first");
    assert_eq!(
        first,
        Placed::Applied {
            path: png.clone(),
            replaced: 0
        }
    );
    assert_eq!(fs::read(&png).unwrap(), b"first");
    assert!(!root.join("backups").exists(), "nothing was replaced");

    // a different format still replaces the same slot
    let second = apply(&grid, Slot::Hero, ImageFormat::Jpg, b"second");
    assert_eq!(second, Placed::Applied { path: jpg, replaced: 1 });
    assert!(!png.exists());
    assert_eq!(fs::read_dir(root.join("backups")).unwrap().count(), 1);

    assert!(!root.join("grid/440_hero.incoming").exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn placing_the_same_image_again_changes_nothing() {
    let root = scratch("unchanged");
    let grid = grid_in(&root);
    let path = root.join("grid/440p.png");

    apply(&grid, Slot::Grid, ImageFormat::Png, b"same");
    let again = apply(&grid, Slot::Grid, ImageFormat::Png, b"same");
    assert_eq!(again, Placed::Unchanged { path: path.clone() });

    assert_eq!(fs::read(&path).unwrap(), b"same");
    assert!(!root.join("backups").exists(), "an identical image is not backed up");
    assert!(!root.join("grid/440p.incoming").exists());

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn an_abandoned_download_is_removed() {
    let root = scratch("staged");
    let grid = grid_in(&root);

    let staged = grid.stage(AppId::from(440), Slot::Logo).unwrap();
    let path = staged.path().to_path_buf();
    fs::write(&path, b"half a file").unwrap();
    drop(staged);

    assert!(!path.exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn clearing_keeps_the_newest_three_backups_of_a_slot() {
    let root = scratch("clear");
    let grid = grid_in(&root);
    let backups = root.join("backups");

    for stamp in [10, 20, 30] {
        write(&backups.join(format!("{stamp}_440_hero.png")), "old");
    }
    write(&backups.join("10_440p.png"), "another slot");
    write(&root.join("grid/440_hero.jpg"), "current");
    write(&root.join("grid/440_hero.json"), "not art");
    write(&root.join("grid/440p.png"), "another slot");

    assert_eq!(grid.clear(AppId::from(440), Slot::Hero).unwrap(), 1);

    assert!(!root.join("grid/440_hero.jpg").exists());
    assert!(root.join("grid/440_hero.json").exists());
    assert!(root.join("grid/440p.png").exists());

    let kept: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("_440_hero."))
        .collect();
    assert_eq!(kept.len(), 3, "{kept:?}");
    assert!(
        kept.iter().all(|name| name != "10_440_hero.png"),
        "the oldest is dropped"
    );
    assert!(backups.join("10_440p.png").exists());

    let again = grid.clear(AppId::from(440), Slot::Hero).unwrap();
    assert_eq!(again, 0, "an empty slot is a no-op");

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_logo_gets_a_position_unless_steam_already_has_one() {
    let root = scratch("logo_position");
    let grid = grid_in(&root);
    let position = root.join("grid/440.json");

    apply(&grid, Slot::Hero, ImageFormat::Png, b"hero");
    assert!(!position.exists());

    apply(&grid, Slot::Logo, ImageFormat::Png, b"logo");
    assert_eq!(fs::read_to_string(&position).unwrap(), DEFAULT_LOGO_POSITION);
    assert!(serde_json::from_str::<serde_json::Value>(DEFAULT_LOGO_POSITION).is_ok());

    fs::write(&position, "set by the user in steam").unwrap();
    apply(&grid, Slot::Logo, ImageFormat::Png, b"another logo");
    assert_eq!(fs::read_to_string(&position).unwrap(), "set by the user in steam");

    fs::remove_file(&position).unwrap();
    let again = apply(&grid, Slot::Logo, ImageFormat::Png, b"another logo");
    assert!(matches!(again, Placed::Unchanged { .. }));
    assert!(position.exists());

    fs::remove_dir_all(&root).unwrap();
}
