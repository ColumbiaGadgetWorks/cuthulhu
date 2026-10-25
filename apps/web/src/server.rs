// SPDX-License-Identifier: GPL-3.0-or-later
//! Routes:
//!
//! - `POST /api/<command>`: a JSON object of arguments in, the command's JSON answer out (200), or
//!   its error (400) shaped as the desktop's command errs.
//! - `POST /upload?name=<file>`: the body saved as that file in the files directory, for Open,
//!   Trace and anything else the desktop reads through a native picker.
//! - `GET /files/<file>`: a saved file, as a download, for Save and the exports.
//! - `GET /?token=<token>`: sets the access cookie and redirects to `/`.
//! - Anything else: the UI's static files, with `index.html` for a path that is not one.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use desktop::device::DeviceManagerHandle;
use desktop::state::AppState;
use driver_registry::HardwareBackendFactory;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::api::{dispatch, Ctx};
use crate::Config;

const COOKIE: &str = "cuthulhu_token";
/// An SVG or project of a few MB is large; an image for tracing can be tens.
const MAX_UPLOAD: usize = 64 * 1024 * 1024;
/// Enough for `import_svg`, whose bytes travel as a JSON array of numbers (about four characters
/// a byte).
const MAX_API_BODY: usize = 4 * MAX_UPLOAD;

pub fn run(cfg: Config) -> Result<(), String> {
    let files = cfg.data.join("files");
    std::fs::create_dir_all(&files).map_err(|e| format!("cannot create {}: {e}", files.display()))?;
    if !cfg.ui.join("index.html").is_file() {
        return Err(format!("no UI at {} (index.html is missing); pass --ui", cfg.ui.display()));
    }

    let (dev, events) = DeviceManagerHandle::new(Arc::new(HardwareBackendFactory));
    let dev = dev.with_usage_log(desktop::usage::default_usage_path());
    let ctx = Arc::new(Ctx { state: std::sync::Mutex::new(AppState::new()), dev, files });

    // The desktop's event bridge forwards these to the webview. A browser polls
    // `get_device_state` instead, so all that is left to do with each event is the usage log's.
    let observer = ctx.clone();
    std::thread::spawn(move || {
        for event in events {
            observer.dev.observe_event(&event);
        }
    });

    let server = Server::http(&cfg.bind).map_err(|e| format!("cannot listen on {}: {e}", cfg.bind))?;
    eprintln!(
        "cuthulhu-web: serving on http://{}{}",
        cfg.bind,
        if cfg.token.is_some() { " (open it once as /?token=<your token>)" } else { " with no token: anyone who can reach it can cut" }
    );
    let shared = Arc::new((cfg.ui, cfg.token));
    for request in server.incoming_requests() {
        let ctx = ctx.clone();
        let shared = shared.clone();
        // A request per thread: `cut` blocks while the worker sends, and a browser polling the
        // device state meanwhile must still be answered.
        std::thread::spawn(move || handle(&ctx, &shared.0, shared.1.as_deref(), request));
    }
    Ok(())
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("static header")
}

fn json(status: u16, body: &serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(serde_json::to_vec(body).unwrap_or_default())
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", "application/json"))
}

fn text(status: u16, body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body).with_status_code(StatusCode(status))
}

/// Compared in time independent of where the strings differ, so the token cannot be found a
/// character at a time.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn query_param<'a>(url: &'a str, key: &str) -> Option<&'a str> {
    url.split_once('?')?.1.split('&').find_map(|kv| kv.strip_prefix(key)?.strip_prefix('='))
}

fn cookie<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers()
        .iter()
        .filter(|h| h.field.equiv("Cookie"))
        .flat_map(|h| h.value.as_str().split(';'))
        .find_map(|kv| kv.trim().strip_prefix(name)?.strip_prefix('='))
}

/// `%XX` and `+` decoded, for file names in a URL. Invalid escapes are kept as typed; the file
/// name check refuses whatever that leaves.
fn decode(s: &str) -> String {
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match (b[i], b.get(i + 1).copied().and_then(hex), b.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(hi), Some(lo)) => {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
            (b'+', _, _) => out.push(b' '),
            (c, _, _) => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_body(req: &mut Request, limit: usize) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    req.as_reader().take(limit as u64 + 1).read_to_end(&mut body).map_err(|e| e.to_string())?;
    if body.len() > limit { Err(format!("body larger than {limit} bytes")) } else { Ok(body) }
}

fn handle(ctx: &Ctx, ui: &Path, token: Option<&str>, mut req: Request) {
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or("/").to_string();

    if let Some(token) = token {
        if let Some(given) = query_param(&url, "token") {
            let reply = if same(&decode(given), token) {
                // SameSite=Strict: another site's page cannot make the browser send it, so a link
                // elsewhere cannot start a cut.
                Response::from_string("")
                    .with_status_code(StatusCode(303))
                    .with_header(header("Location", "/"))
                    .with_header(header("Set-Cookie", &format!("{COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=31536000")))
            } else {
                text(403, "wrong token")
            };
            let _ = req.respond(reply);
            return;
        }
        if !cookie(&req, COOKIE).is_some_and(|c| same(c, token)) {
            let _ = req.respond(text(401, "Open this page once as /?token=<your token> to sign in."));
            return;
        }
    }

    let reply = match (req.method(), path.as_str()) {
        (Method::Post, p) if p.starts_with("/api/") => {
            let cmd = p.trim_start_matches("/api/").to_string();
            match read_body(&mut req, MAX_API_BODY) {
                Err(e) => json(413, &serde_json::Value::String(e)),
                Ok(body) => {
                    let args: serde_json::Value = if body.is_empty() {
                        serde_json::json!({})
                    } else {
                        match serde_json::from_slice(&body) {
                            Ok(v) => v,
                            Err(e) => {
                                let _ = req.respond(json(400, &serde_json::Value::String(format!("arguments are not JSON: {e}"))));
                                return;
                            }
                        }
                    };
                    match dispatch(ctx, &cmd, &args) {
                        Ok(v) => json(200, &v),
                        Err(e) => json(400, &e),
                    }
                }
            }
        }
        (Method::Post, "/upload") => {
            let name = decode(query_param(&url, "name").unwrap_or(""));
            match crate::files::resolve(&ctx.files, &name) {
                Err(e) => json(400, &serde_json::Value::String(e)),
                Ok(dest) => match read_body(&mut req, MAX_UPLOAD).and_then(|b| std::fs::write(&dest, b).map_err(|e| e.to_string())) {
                    Ok(()) => json(200, &serde_json::Value::String(name)),
                    Err(e) => json(400, &serde_json::Value::String(e)),
                },
            }
        }
        (Method::Get, p) if p.starts_with("/files/") => {
            let name = decode(p.trim_start_matches("/files/"));
            match crate::files::resolve(&ctx.files, &name).and_then(|f| std::fs::read(f).map_err(|e| e.to_string())) {
                Ok(bytes) => Response::from_data(bytes)
                    .with_header(header("Content-Type", "application/octet-stream"))
                    .with_header(header("Content-Disposition", &format!("attachment; filename=\"{name}\""))),
                Err(e) => text(404, &e),
            }
        }
        (Method::Get, p) => static_file(ui, p),
        _ => text(405, "method not allowed"),
    };
    let _ = req.respond(reply);
}

fn content_type(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("json") => "application/json",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// A file under `ui`, or `index.html`. Only plain path segments are followed, so `..` cannot step
/// out of the UI directory.
fn static_file(ui: &Path, url_path: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut file = PathBuf::from(ui);
    for seg in url_path.split('/').filter(|s| !s.is_empty()) {
        if seg == ".." || seg == "." || seg.contains('\\') {
            return text(400, "bad path");
        }
        file.push(seg);
    }
    if !file.is_file() {
        file = ui.join("index.html");
    }
    match std::fs::read(&file) {
        Ok(bytes) if file.ends_with("index.html") => Response::from_data(mark_web(&bytes))
            .with_header(header("Content-Type", content_type(&file)))
            .with_header(header("Cache-Control", "no-cache")),
        Ok(bytes) => Response::from_data(bytes).with_header(header("Content-Type", content_type(&file))),
        Err(e) => text(500, &e.to_string()),
    }
}

/// The page is the desktop's own bundle; this flag, set before its scripts run, is what sends its
/// IPC over HTTP (`ui/src/transport.ts`).
fn mark_web(index_html: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(index_html)
        .replacen("<head>", "<head><script>window.__CUTHULHU_WEB__ = true;</script>", 1)
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_and_escapes_read_as_a_browser_writes_them() {
        assert_eq!(query_param("/upload?name=a%20b.svg&x=1", "name"), Some("a%20b.svg"));
        assert_eq!(query_param("/upload?names=x&name=y", "name"), Some("y"));
        assert_eq!(query_param("/upload", "name"), None);
        assert_eq!(decode("a%20b+c.svg"), "a b c.svg");
        assert_eq!(decode("%2e%2e%2fx"), "../x"); // decoded, then refused by the name check
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("é%zz%"), "é%zz%");
    }

    #[test]
    fn the_served_page_is_marked_before_its_scripts() {
        let page = String::from_utf8(mark_web(b"<html><head><script src=a.js></script></head></html>")).unwrap();
        assert!(page.find("__CUTHULHU_WEB__").unwrap() < page.find("a.js").unwrap());
    }

    #[test]
    fn the_token_compare_needs_the_whole_token() {
        assert!(same("abc", "abc"));
        assert!(!same("abc", "abd"));
        assert!(!same("ab", "abc"));
    }
}
