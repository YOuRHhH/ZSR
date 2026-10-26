use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::SystemTime,
};

pub(super) const LIMIT: usize = 200;

#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct File {
    pub modified: SystemTime,
    pub path: PathBuf,
    pub bytes: u64,
}

type Scan = (PathBuf, Result<Vec<File>, String>);

#[derive(Default)]
pub(super) struct Recordings {
    pub open: bool,
    pub files: Vec<File>,
    pub error: Option<String>,
    directory: Option<PathBuf>,
    pending: Option<Receiver<Scan>>,
    dirty: bool,
}

impl Recordings {
    /// Schedule a refresh, including when a previous directory scan is still running.
    pub fn refresh(&mut self) {
        self.dirty = true;
    }

    /// Report pending work so the UI distinguishes loading from an empty directory.
    pub fn loading(&self) -> bool {
        self.pending.is_some() || self.dirty
    }

    /// Scan off the UI thread and discard results belonging to an old save directory.
    pub fn poll(&mut self, directory: &Path) {
        if self.directory.as_deref() != Some(directory) {
            self.directory = Some(directory.to_owned());
            self.files.clear();
            self.error = None;
            self.dirty = true;
        }
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok((path, result)) => {
                    self.pending = None;
                    if path == directory {
                        match result {
                            Ok(files) => {
                                self.files = files;
                                self.error = None;
                            }
                            Err(error) => {
                                self.files.clear();
                                self.error = Some(error);
                            }
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.error = Some("Recording directory scan stopped unexpectedly".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.pending.is_none() && self.dirty {
            self.dirty = false;
            let path = directory.to_owned();
            let (tx, rx) = mpsc::channel();
            self.pending = Some(rx);
            std::thread::spawn(move || {
                let result = scan(&path).map_err(|error| error.to_string());
                let _ = tx.send((path, result));
            });
        }
    }
}

/// Keep at most the newest 200 completed ZSR MP4s without reading video contents.
fn scan(directory: &Path) -> std::io::Result<Vec<File>> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut newest = BTreeSet::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("ZSR_") || !name.ends_with(".mp4") || name.ends_with(".partial.mp4") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() || metadata.len() == 0 {
            continue;
        }
        newest.insert(File {
            path: entry.path(),
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            bytes: metadata.len(),
        });
        if newest.len() > LIMIT {
            newest.pop_first();
        }
    }
    Ok(newest.into_iter().rev().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A completed scan of the previous folder must never populate the new folder's list.
    #[test]
    fn discards_old_directory_results() {
        let folder = tempfile::tempdir().unwrap();
        let old = folder.path().join("old");
        let new = folder.path().join("new");
        let (tx, rx) = mpsc::channel();
        tx.send((
            old.clone(),
            Ok(vec![File {
                modified: SystemTime::UNIX_EPOCH,
                path: old.join("ZSR_old.mp4"),
                bytes: 1,
            }]),
        ))
        .unwrap();
        let mut recordings = Recordings {
            directory: Some(old),
            pending: Some(rx),
            ..Default::default()
        };
        recordings.poll(&new);
        assert!(recordings.files.is_empty());
        assert_eq!(recordings.directory.as_deref(), Some(new.as_path()));
        assert!(recordings.loading());
    }

    /// In-progress files, unrelated videos and directories must not appear as recordings.
    #[test]
    fn lists_completed_recordings_only() {
        let folder = tempfile::tempdir().unwrap();
        for name in [
            "ZSR_done.mp4",
            "ZSR_中文.mp4",
            "ZSR_active.partial.mp4",
            "other.mp4",
        ] {
            std::fs::write(folder.path().join(name), b"fixture").unwrap();
        }
        std::fs::write(folder.path().join("ZSR_empty.mp4"), b"").unwrap();
        std::fs::create_dir(folder.path().join("ZSR_directory.mp4")).unwrap();
        let files = scan(folder.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|file| file.bytes == 7));
        assert!(scan(&folder.path().join("missing")).unwrap().is_empty());
    }

    /// Large folders stay bounded and retain the newest entries in descending order.
    #[test]
    fn keeps_newest_with_bounded_storage() {
        let folder = tempfile::tempdir().unwrap();
        for index in 0..LIMIT + 5 {
            let file =
                std::fs::File::create(folder.path().join(format!("ZSR_{index}.mp4"))).unwrap();
            file.set_len(1).unwrap();
            file.set_modified(
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(index as u64),
            )
            .unwrap();
        }
        let files = scan(folder.path()).unwrap();
        assert_eq!(files.len(), LIMIT);
        assert!(files[0].path.ends_with("ZSR_204.mp4"));
        assert!(files.last().unwrap().path.ends_with("ZSR_5.mp4"));
    }
}
