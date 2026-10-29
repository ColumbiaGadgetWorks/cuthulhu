// SPDX-License-Identifier: GPL-3.0-or-later

//! The daemon's configuration: where it listens. There is no sign-in: anything that can reach
//! the port may cut, which is why the bind defaults to loopback and refuses a public address.

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug)]
pub enum ConfigError {
    Unreadable(String),
    Malformed(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Unreadable(m) => write!(f, "the configuration could not be read ({m})"),
            ConfigError::Malformed(m) => write!(f, "the configuration could not be understood ({m})"),
        }
    }
}
impl std::error::Error for ConfigError {}

/// Fields it does not know are ignored rather than refused, so a `cutd.toml` from before sign-in
/// was removed, with its `[tokens]` table, still starts the daemon.
#[derive(Deserialize)]
struct ConfigFile {
    bind: Option<String>,
    max_frame: Option<usize>,
    cert_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Config {
    pub bind: SocketAddr,
    pub max_frame: usize,
    pub cert_dir: PathBuf,
}

/// `toml::de::Error`'s own `Display` renders the offending source line verbatim, so a stray
/// quote on a line would print it straight to the operator's log (the daemon prints load errors
/// to stderr, which under the unit is the journal). `span()` only gives a byte range, not the line/column an operator can act on, so keep the first line of `Display`
/// instead — "TOML parse error at line N, column M" — and drop the snippet that follows it.
//
// ponytail: that fixed header is only the first line when the underlying `toml_edit` error
// carries a span; a span-less error (from `serde::de::Error::custom`, e.g. a required field,
// an enum, or `deny_unknown_fields`) puts its raw message on the first line instead, which can
// be the field content itself. This holds today because `ConfigFile` is all-`Option` fields
// with no custom `Deserialize` — every error toml can produce here does carry a span. Add any
// of those to `ConfigFile` and this needs to filter on content, not position.
fn describe(error: toml::de::Error) -> String {
    error.to_string().lines().next().unwrap_or_default().to_string()
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Unreadable(e.to_string()))?;
        let file: ConfigFile = toml::from_str(&text).map_err(|e| ConfigError::Malformed(describe(e)))?;

        let bind = file
            .bind
            .unwrap_or_else(|| "127.0.0.1:7878".into())
            .parse()
            .map_err(|e| ConfigError::Malformed(format!("bind: {e}")))?;

        Ok(Config {
            bind,
            max_frame: file.max_frame.unwrap_or(crate::frame::DEFAULT_MAX_FRAME),
            cert_dir: file.cert_dir.unwrap_or_else(|| PathBuf::from("/var/lib/cuthulhu")),
        })
    }

    /// Whether this address is on a private network. The daemon refuses a public
    /// bind unless told otherwise, because the thing on the other end of it can be
    /// made to move a blade.
    pub fn is_private_bind(&self) -> bool {
        match self.bind.ip() {
            IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
            IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("cutd.toml"), body).unwrap();
        dir
    }

    #[test]
    fn a_config_reads_its_bind() {
        let dir = write_config("bind = \"192.168.1.10:7878\"\n");
        let config = Config::load(&dir.path().join("cutd.toml")).unwrap();
        assert_eq!(config.bind.port(), 7878);
        assert_eq!(config.max_frame, crate::frame::DEFAULT_MAX_FRAME, "an unset cap takes the default");
    }

    /// A Pi set up while sign-in existed keeps its file; the daemon must still start on it.
    #[test]
    fn a_config_from_before_sign_in_was_removed_still_loads() {
        let dir = write_config("bind = \"192.168.1.10:7878\"\ntoken = \"x\"\n\n[tokens]\nlaptop = \"s3cret\"\n");
        assert_eq!(Config::load(&dir.path().join("cutd.toml")).unwrap().bind.port(), 7878);
    }

    /// The daemon binds LAN-only by default, and this is the predicate that
    /// decides. A machine that can be told to move must not be reachable from
    /// the internet because a config file said `0.0.0.0`.
    #[test]
    fn only_private_addresses_count_as_private() {
        for private in ["127.0.0.1:1", "192.168.1.10:1", "10.0.0.4:1", "172.16.5.5:1"] {
            let dir = write_config(&format!("bind = \"{private}\"\n"));
            assert!(Config::load(&dir.path().join("cutd.toml")).unwrap().is_private_bind(), "{private}");
        }
        for public in ["0.0.0.0:1", "8.8.8.8:1", "172.32.0.1:1"] {
            let dir = write_config(&format!("bind = \"{public}\"\n"));
            assert!(!Config::load(&dir.path().join("cutd.toml")).unwrap().is_private_bind(), "{public}");
        }
    }

    #[test]
    fn a_malformed_config_reports_where_without_quoting_the_line() {
        let dir = write_config("bind = \"127.0.0.1:7878\"\nmax_frame = \"unterminated\n");
        let message = match Config::load(&dir.path().join("cutd.toml")) {
            Err(e) => e.to_string(),
            Ok(_) => panic!("an unterminated quote must not parse"),
        };
        assert!(!message.contains("unterminated"), "the line reached the error: {message}");
        assert!(message.contains("line"), "an operator still needs to be told where: {message}");
    }
}
