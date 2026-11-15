use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::error::Error;
use crate::error::Result;
use crate::types::AppId;
use crate::types::ImageFormat;
use crate::types::Slot;

const BACKUPS_PER_SLOT: usize = 3;

const DEFAULT_LOGO_POSITION: &str =
    r#"{"nVersion":1,"logoPosition":{"pinnedPosition":"BottomLeft","nWidthPct":50,"nHeightPct":50}}"#;

#[derive(Debug)]
pub struct Staged {
    path: PathBuf,
}

impl Staged {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        // already gone when it was moved into its slot
        let _ = fs::remove_file(&self.path);
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Placed {
    Applied { path: PathBuf, replaced: usize },
    Unchanged { path: PathBuf },
}

#[derive(Debug)]
pub struct GridDir {
    dir: PathBuf,
    backups: PathBuf,
}

impl GridDir {
    pub const fn new(dir: PathBuf, backups: PathBuf) -> Self {
        Self { dir, backups }
    }

    fn art(&self) -> impl Iterator<Item = (String, PathBuf)> {
        fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| ImageFormat::of(path).is_some())
            .filter_map(|path| Some((path.file_stem()?.to_str()?.to_lowercase(), path)))
    }

    pub fn art_stems(&self) -> HashSet<String> {
        self.art().map(|(stem, _)| stem).collect()
    }

    fn occupants(&self, stem: &str) -> Vec<PathBuf> {
        self.art()
            .filter(|(found, _)| found.eq_ignore_ascii_case(stem))
            .map(|(_, path)| path)
            .collect()
    }

    pub fn stage(&self, app: AppId, slot: Slot) -> Result<Staged> {
        fs::create_dir_all(&self.dir).map_err(Error::io(format!("creating {}", self.dir.display())))?;

        Ok(Staged {
            path: self.dir.join(format!("{}.incoming", slot.file_stem(app))),
        })
    }

    #[expect(clippy::needless_pass_by_value, reason = "placing uses up the staged download")]
    pub fn place(&self, staged: Staged, app: AppId, slot: Slot, format: ImageFormat) -> Result<Placed> {
        let stem = slot.file_stem(app);
        let path = self.dir.join(format!("{stem}.{}", format.ext()));

        if slot == Slot::Logo {
            self.ensure_logo_position(app)?;
        }

        if self.occupants(&stem) == [path.clone()] && same_content(staged.path(), &path) {
            return Ok(Placed::Unchanged { path });
        }

        let replaced = self.clear(app, slot)?;
        fs::rename(staged.path(), &path).map_err(Error::io(format!("writing {}", path.display())))?;

        Ok(Placed::Applied { path, replaced })
    }

    // steam only draws a custom logo where <appid>.json says to; a game without an official logo has no position
    fn ensure_logo_position(&self, app: AppId) -> Result<()> {
        let path = self.dir.join(format!("{app}.json"));
        if path.exists() {
            return Ok(());
        }

        fs::write(&path, DEFAULT_LOGO_POSITION).map_err(Error::io(format!("writing {}", path.display())))
    }

    pub fn clear(&self, app: AppId, slot: Slot) -> Result<usize> {
        let stem = slot.file_stem(app);
        let files = self.occupants(&stem);
        if files.is_empty() {
            return Ok(0);
        }

        fs::create_dir_all(&self.backups).map_err(Error::io(format!("creating {}", self.backups.display())))?;

        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());

        for file in &files {
            let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("art");
            let dest = self.backups.join(format!("{stamp}_{name}"));

            // a plain move fails when the backup folder is on another drive
            if fs::rename(file, &dest).is_err() {
                fs::copy(file, &dest).map_err(Error::io(format!("backing up {name}")))?;
                fs::remove_file(file).map_err(Error::io(format!("replacing {name}")))?;
            }
        }

        prune_backups(&self.backups, &stem, BACKUPS_PER_SLOT);

        Ok(files.len())
    }
}

fn prune_backups(dir: &Path, stem: &str, keep: usize) -> usize {
    let Ok(entries) = fs::read_dir(dir) else { return 0 };

    let mut backups: Vec<(u64, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let (stamp, slot) = path.file_stem()?.to_str()?.split_once('_')?;
            let stamp = stamp.parse::<u64>().ok()?;
            slot.eq_ignore_ascii_case(stem).then_some((stamp, path))
        })
        .collect();

    backups.sort_unstable_by_key(|(stamp, _)| std::cmp::Reverse(*stamp));
    backups
        .into_iter()
        .skip(keep)
        .filter(|(_, path)| fs::remove_file(path).is_ok())
        .count()
}

fn same_content(a: &Path, b: &Path) -> bool {
    let len = |path: &Path| fs::metadata(path).map(|m| m.len()).ok();
    len(a).is_some() && len(a) == len(b) && fs::read(a).ok() == fs::read(b).ok()
}

#[cfg(test)]
#[path = "../tests/grid.rs"]
mod tests;
