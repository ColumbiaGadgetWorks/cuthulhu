// SPDX-License-Identifier: GPL-3.0-or-later
//! Updating the server from the page. The server runs unprivileged and never installs anything
//! itself: applying an update writes a request file, and a root systemd unit watching for it
//! (`pi/cuthulhu-web-update.path`) runs `pi/update.sh`, which downloads the release and runs its
//! `install.sh`. What crosses that line is two strings, a repository and a tag, which the script
//! checks again against the same shapes as here before using either.
//!
//! The repository updates come from cannot be changed from the page. The page has no sign-in, and an
//! update runs as root, so whoever could choose where it comes from could run anything on the
//! machine. It is changed over SSH, in `update-settings.json`.
//!
//! What the page can do is install the newest release of that one repository — the same thing an
//! SSH user would do, with nothing for a visitor to choose but when.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Where releases come from until someone chooses otherwise: the fork that publishes `web-v*`.
pub const DEFAULT_REPO: &str = "adman234/cuthulhu";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The release package this build came from, as `dpkg --print-architecture` names it.
pub const ARCH: &str = if cfg!(target_arch = "aarch64") {
    "arm64"
} else if cfg!(target_arch = "arm") {
    "armhf"
} else {
    "amd64"
};

/// Written by root's `update.sh`, in a directory of root's (see there for why not `data`).
pub const STATUS_FILE: &str = "/var/lib/cuthulhu-web-update/status.json";

pub struct Updates {
    data: PathBuf,
    status_file: PathBuf,
}

#[derive(Serialize, Deserialize, Default)]
struct Settings {
    repo: Option<String>,
}

/// `owner/name`, as GitHub allows them. Checked here and again by `update.sh`.
pub fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let ok = |s: Option<&str>| {
        s.is_some_and(|s| !s.is_empty() && s.len() <= 100 && !s.starts_with('.') && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
    };
    ok(parts.next()) && ok(parts.next()) && parts.next().is_none()
}

/// `web-v` and a dotted version, the only tags `release-web.yml` publishes.
pub fn valid_tag(tag: &str) -> bool {
    tag.strip_prefix("web-v").is_some_and(|v| !v.is_empty() && v.len() <= 32 && v.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())))
}

fn version_of(tag: &str) -> Vec<u64> {
    tag.trim_start_matches("web-v").trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// The newest `web-v*` release in GitHub's release list that carries a package for `arch`.
pub fn newest_release(releases: &Value, arch: &str) -> Option<(String, String)> {
    let asset = format!("cuthulhu-web-linux-{arch}.tar.gz");
    releases
        .as_array()?
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false) && !r["prerelease"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?;
            let has = r["assets"].as_array()?.iter().any(|a| a["name"].as_str() == Some(asset.as_str()));
            (valid_tag(tag) && has).then(|| (tag.to_string(), r["html_url"].as_str().unwrap_or("").to_string()))
        })
        .max_by(|a, b| version_of(&a.0).cmp(&version_of(&b.0)))
}

impl Updates {
    pub fn new(data: &Path, status_file: &Path) -> Self {
        Updates { data: data.to_path_buf(), status_file: status_file.to_path_buf() }
    }

    fn settings_path(&self) -> PathBuf {
        self.data.join("update-settings.json")
    }

    pub fn repo(&self) -> String {
        std::fs::read(self.settings_path())
            .ok()
            .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
            .and_then(|s| s.repo)
            .filter(|r| valid_repo(r))
            .unwrap_or_else(|| DEFAULT_REPO.to_string())
    }

    /// What the page shows: this build, where updates come from, and how the last one went.
    pub fn status(&self) -> Value {
        let last = std::fs::read(&self.status_file)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .unwrap_or(Value::Null);
        json!({
            "current": VERSION,
            "arch": ARCH,
            "repo": self.repo(),
            "pending": self.data.join("update-request").exists(),
            "last": last,
        })
    }

    /// Asks GitHub for the newest release with a package for this machine. Through `curl`, which
    /// Raspberry Pi OS ships, rather than an HTTP client compiled into a server that otherwise
    /// makes no outbound requests.
    pub fn check(&self) -> Result<Value, Value> {
        let repo = self.repo();
        let url = format!("https://api.github.com/repos/{repo}/releases?per_page=50");
        let out = Command::new("curl")
            .args(["-fsSL", "--max-time", "20", "-H", "Accept: application/vnd.github+json", &url])
            .output()
            .map_err(|e| Value::String(format!("cannot run curl: {e}")))?;
        if !out.status.success() {
            return Err(Value::String(format!(
                "GitHub did not answer for {repo}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        let releases: Value = serde_json::from_slice(&out.stdout).map_err(|e| Value::String(format!("unexpected answer from GitHub: {e}")))?;
        let latest = newest_release(&releases, ARCH);
        let available = latest.as_ref().is_some_and(|(tag, _)| version_of(tag) > version_of(VERSION));
        Ok(json!({
            "status": self.status(),
            "latest": latest.as_ref().map(|l| &l.0),
            "url": latest.as_ref().map(|l| &l.1),
            "updateAvailable": available,
        }))
    }

    /// Hands `tag` to the root updater. The server is restarted by the install, so the page loses
    /// it for a few seconds and then finds the new version.
    pub fn apply(&self, tag: &str) -> Result<Value, Value> {
        if !valid_tag(tag) {
            return Err(Value::String(format!("{tag:?} is not a release tag (web-vX.Y.Z)")));
        }
        let request = format!("{} {tag}\n", self.repo());
        std::fs::write(self.data.join("update-request"), request).map_err(|e| Value::String(format!("cannot ask for the update: {e}")))?;
        Ok(self.status())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repositories_and_tags_are_only_the_shapes_github_and_the_release_use() {
        assert!(valid_repo("adman234/cuthulhu"));
        assert!(valid_repo("some-org/cut.hulhu_2"));
        for bad in ["", "a", "a/b/c", "/b", "a/", "../x", "a/.x", "a b/c", "a/b;rm -rf /", "a/$(x)"] {
            assert!(!valid_repo(bad), "{bad:?}");
        }
        assert!(valid_tag("web-v0.1.1"));
        assert!(valid_tag("web-v10.0"));
        for bad in ["v0.2.1", "web-v", "web-v1..2", "web-v1.2-rc", "web-v1;x", "web-v1.2 "] {
            assert!(!valid_tag(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_newest_release_with_this_machines_package_wins() {
        let releases = json!([
            { "tag_name": "v0.3.0", "draft": false, "prerelease": false, "assets": [{ "name": "cuthulhu-web-linux-armhf.tar.gz" }] },
            { "tag_name": "web-v0.1.10", "draft": false, "prerelease": false, "html_url": "u10", "assets": [{ "name": "cuthulhu-web-linux-armhf.tar.gz" }] },
            { "tag_name": "web-v0.1.9", "draft": false, "prerelease": false, "assets": [{ "name": "cuthulhu-web-linux-armhf.tar.gz" }] },
            { "tag_name": "web-v0.2.0", "draft": false, "prerelease": false, "assets": [{ "name": "cuthulhu-web-linux-arm64.tar.gz" }] },
            { "tag_name": "web-v0.9.0", "draft": true, "prerelease": false, "assets": [{ "name": "cuthulhu-web-linux-armhf.tar.gz" }] },
        ]);
        assert_eq!(newest_release(&releases, "armhf"), Some(("web-v0.1.10".into(), "u10".into())));
        assert_eq!(newest_release(&releases, "arm64").map(|r| r.0), Some("web-v0.2.0".into()));
        assert_eq!(newest_release(&releases, "amd64"), None);
    }

    #[test]
    fn the_repository_is_read_from_the_settings_file_and_a_bad_one_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let u = Updates::new(dir.path(), &dir.path().join("status.json"));
        assert_eq!(u.repo(), DEFAULT_REPO);
        std::fs::write(dir.path().join("update-settings.json"), r#"{"repo":"someone/else"}"#).unwrap();
        assert_eq!(u.repo(), "someone/else");
        std::fs::write(dir.path().join("update-settings.json"), r#"{"repo":"a/b;reboot"}"#).unwrap();
        assert_eq!(u.repo(), DEFAULT_REPO);
    }

    #[test]
    fn applying_writes_the_request_the_root_updater_reads() {
        let dir = tempfile::tempdir().unwrap();
        let u = Updates::new(dir.path(), &dir.path().join("status.json"));
        assert!(u.apply("web-v1.2.3; reboot").is_err());
        assert!(!dir.path().join("update-request").exists());
        u.apply("web-v1.2.3").unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("update-request")).unwrap(), format!("{DEFAULT_REPO} web-v1.2.3\n"));
    }
}
