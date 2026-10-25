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
}
