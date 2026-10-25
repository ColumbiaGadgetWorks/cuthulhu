// SPDX-License-Identifier: GPL-3.0-or-later
//! Every path a browser names. The desktop's commands take filesystem paths because its native
//! picker chose them; here a path comes from whoever can reach the port, so it is only ever a bare
//! file name inside the server's own files directory. A name with a separator, a `..`, or anything
//! else outside a short safe alphabet is refused rather than cleaned up into something else.

use std::path::{Path, PathBuf};

const MAX_NAME: usize = 128;

pub fn safe_name(name: &str) -> Result<&str, String> {
    let ok = !name.is_empty()
        && name.len() <= MAX_NAME
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' '));
    if ok { Ok(name) } else { Err(format!("{name:?} is not a file name this server accepts")) }
}

/// `name`, resolved in `dir`. A browser can only reach the files this server holds.
pub fn resolve(dir: &Path, name: &str) -> Result<PathBuf, String> {
    Ok(dir.join(safe_name(name)?))
}

/// One file the server holds, as the page lists it.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Listed {
    pub name: String,
    pub size: u64,
    /// Seconds since the Unix epoch.
    pub modified: u64,
}

/// Every file in `dir` a browser could name, newest first. Anything `safe_name` would refuse (a
/// save's temporary file, say) is left out, since the page could not open it anyway.
pub fn list(dir: &Path) -> Result<Vec<Listed>, String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))? {
        let Ok(entry) = entry else { continue };
        let Ok(meta) = entry.metadata() else { continue };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else { continue };
        if !meta.is_file() || safe_name(&name).is_err() {
            continue;
        }
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        out.push(Listed { name, size: meta.len(), modified });
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_bare_names_inside_the_directory_are_accepted() {
        let dir = Path::new("/data/files");
        assert_eq!(resolve(dir, "cuthulhu-project.cut").unwrap(), dir.join("cuthulhu-project.cut"));
        assert_eq!(resolve(dir, "My design 2.svg").unwrap(), dir.join("My design 2.svg"));
        for bad in ["", "../etc/passwd", "/etc/passwd", "a/b.cut", "a\\b.cut", ".hidden", "..", "x\0y"] {
            assert!(resolve(dir, bad).is_err(), "{bad:?} was accepted");
        }
        assert!(resolve(dir, &"a".repeat(MAX_NAME + 1)).is_err());
    }

    #[test]
    fn the_listing_holds_only_files_a_browser_could_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("design.cut"), b"x").unwrap();
        std::fs::write(dir.path().join(".tmp-save"), b"x").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let names: Vec<String> = list(dir.path()).unwrap().into_iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["design.cut".to_string()]);
    }
}
