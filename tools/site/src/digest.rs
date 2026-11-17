use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use sha2::{Digest, Sha256};

/// A short name for what `parts` are, which changes when any of them does.
pub fn of<'a>(parts: impl IntoIterator<Item = &'a [u8]>) -> String {
    let mut hasher = Sha256::new();

    // Each part is length-prefixed so that moving bytes from one part to the
    // next is a different digest.
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }

    hasher.finalize()[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The digest of every file under `dir`, by path and contents.
pub fn of_tree(dir: &Path) -> Result<String> {
    let mut files = Vec::new();
    let mut queue = vec![dir.to_owned()];

    while let Some(dir) = queue.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();

            if path.is_dir() {
                queue.push(path);
            } else {
                files.push(path);
            }
        }
    }

    files.sort();

    let mut parts = Vec::new();

    for file in &files {
        let relative: PathBuf = file.strip_prefix(dir)?.into();
        parts.push(relative.to_string_lossy().into_owned().into_bytes());
        parts.push(fs::read(file)?);
    }

    Ok(of(parts.iter().map(Vec::as_slice)))
}
