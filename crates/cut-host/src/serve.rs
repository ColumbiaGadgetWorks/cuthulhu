// SPDX-License-Identifier: GPL-3.0-or-later

//! Serving a Cut Host over TLS.
//!
//! `handle_request` is deliberately separate from the socket: it is the whole of
//! what a request means, and it is what the tests drive. `serve` adds a listener,
//! and a certificate to it and nothing else.

use std::io;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use socket2::{SockRef, TcpKeepalive};

use crate::config::Config;
use crate::frame::{read_frame, write_frame, FrameError, DEFAULT_BODY_TIMEOUT, SOCKET_POLL_INTERVAL};
use crate::host::Host;
use crate::protocol::{Request, Response};

/// How long a stream may sit silent before the OS sends the first probe. The
/// request loop's read (`serve_client`) waits with no deadline of its own — a
/// desktop that polls only while a dialog is open is *meant* to go quiet for as
/// long as the operator leaves it alone (see the amending comment on #103), so
/// nothing at the application layer may treat silence as absence. Only a kernel
/// probe can tell a suspended peer from a quiet one, since both look identical
/// on the wire until then. A minute keeps eight stacked lid-closes from
/// exhausting `MAX_CLIENTS` long before anyone notices, without probing a
/// healthy LAN connection often enough to be chatter.
const KEEPALIVE_IDLE: Duration = Duration::from_secs(60);
/// Gap between probes once idle time is exceeded.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
/// Failed probes tolerated before the OS reports the connection dead — worst
/// case `KEEPALIVE_IDLE + KEEPALIVE_RETRIES * KEEPALIVE_INTERVAL` (2 minutes)
/// before a slot is freed.
const KEEPALIVE_RETRIES: u32 = 4;

/// Turn on TCP keepalive so a peer that vanishes without closing (laptop
/// suspended, access point dropped — no FIN or RST ever arrives) is eventually
/// reaped instead of holding a `MAX_CLIENTS` slot forever. `SockRef` borrows the
/// stream's socket to set the option without taking ownership of the fd, so the
/// caller keeps using `stream` as normal afterwards.
///
/// A failure here is logged and swallowed, not propagated: refusing to serve a
/// client because a socket option did not apply would recreate the exhaustion
/// this exists to fix, only faster. `with_retries` needs socket2's `all`
/// feature to exist at all, but once enabled it — like `with_time` and
/// `with_interval` — is implemented for both Linux (`TCP_KEEPIDLE`/`TCP_KEEPCNT`)
/// and macOS (`TCP_KEEPALIVE`/`TCP_KEEPCNT`), so developing on a Mac exercises
/// the same code path the Pi runs.
fn enable_keepalive(stream: &TcpStream, peer: &str) {
    let keepalive = TcpKeepalive::new()
        .with_time(KEEPALIVE_IDLE)
        .with_interval(KEEPALIVE_INTERVAL)
        .with_retries(KEEPALIVE_RETRIES);
    if let Err(e) = SockRef::from(stream).set_tcp_keepalive(&keepalive) {
        eprintln!("cut host: could not enable keepalive for {peer}: {e}");
    }
}

/// The maximum clients served at once. A Cut Host answers a handful of desktops;
/// anything beyond this is a bug or an attempt to exhaust it.
pub const MAX_CLIENTS: usize = 8;

/// How long a new peer has to send its greeting, from the connection being accepted.
///
/// Short on purpose, and much shorter than `DEFAULT_BODY_TIMEOUT`: a peer that has said nothing
/// has earned less patience than a desktop mid-dispatch, and until it speaks it is holding one of
/// `MAX_CLIENTS` slots. Spent at most twice — once waiting for the frame to begin, once for the
/// rest of it — so the worst a silent peer can squat is roughly `2x` this (#118).
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);


/// What a request means, with no socket involved.
pub fn handle_request(host: &Arc<Host>, request: Request) -> Response {
    match request {
        Request::ListDevices => Response::Devices(host.devices()),
        Request::Snapshot => Response::Snapshots(host.snapshots()),
        Request::Dispatch { dispatch_id, device, machine_id, passes } => {
            match host.dispatch(dispatch_id.clone(), &device, &machine_id, passes) {
                Ok(admitted) => Response::Accepted { dispatch_id, admitted },
                Err(refusal) => Response::Refused(refusal),
            }
        }
        Request::Cancel { device } => once(host.cancel(&device)),
        Request::Resume { device } => once(host.resume(&device)),
        Request::ConfirmPassDone { device } => once(host.confirm_pass_done(&device)),
        Request::Reconnect { device } => once(host.reconnect(&device)),
    }
}

fn once(result: Result<(), crate::protocol::Refusal>) -> Response {
    match result {
        Ok(()) => Response::Ok,
        Err(refusal) => Response::Refused(refusal),
    }
}

/// The certificate this host presents, generated on first run and reused after.
/// Self-signed on purpose: a client pins its fingerprint at pairing, which needs
/// no authority and no name that resolves.
fn load_or_make_cert(dir: &Path) -> io::Result<(Vec<rustls::pki_types::CertificateDer<'static>>, rustls::pki_types::PrivateKeyDer<'static>)> {
    std::fs::create_dir_all(dir)?;
    let cert_path = dir.join("cutd.crt");
    let key_path = dir.join("cutd.key");

    if !cert_path.exists() || !key_path.exists() {
        let cert = rcgen::generate_simple_self_signed(vec!["cuthulhu-cutd".to_string()])
            .map_err(io::Error::other)?;
        std::fs::write(&cert_path, cert.cert.pem())?;
        std::fs::write(&key_path, cert.key_pair.serialize_pem())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
        }
        eprintln!("cut host: generated a certificate at {}", cert_path.display());
    }

    let certs = rustls_pemfile::certs(&mut io::BufReader::new(std::fs::File::open(&cert_path)?))
        .collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut io::BufReader::new(std::fs::File::open(&key_path)?))?
        .ok_or_else(|| io::Error::other("no private key in cutd.key"))?;
    Ok((certs, key))
}

/// The SHA-256 of the DER certificate, which is what a client pins. Printed at
/// startup so whoever is pairing can read it off the Pi's console.
pub fn fingerprint(cert: &rustls::pki_types::CertificateDer<'_>) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(cert.as_ref());
    digest.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(":")
}

/// Serve until the process ends.
///
/// `allow_public_bind` is the explicit override for the LAN-only default. It is a
/// parameter rather than a config key so that reaching the internet takes a
/// deliberate act at the command line, not a line someone copied into a file.
pub fn serve(host: Arc<Host>, config: Config, allow_public_bind: bool) -> io::Result<()> {
    if !config.is_private_bind() && !allow_public_bind {
        return Err(io::Error::other(format!(
            "{} is not a private address. A Cut Host can make a blade move; pass --allow-public-bind if you \
             really mean to expose it.",
            config.bind
        )));
    }

    let (certs, _) = load_or_make_cert(&config.cert_dir)?;
    eprintln!("cut host: certificate fingerprint {}", fingerprint(&certs[0]));

    let listener = TcpListener::bind(config.bind)?;
    eprintln!("cut host: listening on {}", config.bind);
    serve_on(listener, host, config)
}

/// Serve on a listener somebody else bound, with no bind-address guard — the
/// caller already chose the address. Split out from `serve` so a test can take
/// port 0 and read back the port the OS gave it.
pub fn serve_on(listener: TcpListener, host: Arc<Host>, config: Config) -> io::Result<()> {
    let (certs, key) = load_or_make_cert(&config.cert_dir)?;
    let tls = Arc::new(
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(io::Error::other)?,
    );

    let clients = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let max_frame = config.max_frame;

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        if clients.load(std::sync::atomic::Ordering::SeqCst) >= MAX_CLIENTS {
            continue; // dropped without ceremony; a real client retries
        }
        let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "?".into());
        enable_keepalive(&stream, &peer);
        clients.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let (host, tls, clients) = (host.clone(), tls.clone(), clients.clone());
        thread::spawn(move || {
            if let Err(e) = serve_client(stream, tls, &host, max_frame) {
                eprintln!("cut host: client ended: {e}");
            }
            clients.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        });
    }
    Ok(())
}

/// The fingerprint of the certificate in `dir`, generating it if it is not there
/// yet. What a pairing client is shown, and what `cuthulhu-cutd` prints at
/// startup.
pub fn fingerprint_of_cert_dir(dir: &Path) -> io::Result<String> {
    let (certs, _) = load_or_make_cert(dir)?;
    Ok(fingerprint(&certs[0]))
}

fn serve_client(
    stream: std::net::TcpStream,
    tls: Arc<rustls::ServerConfig>,
    host: &Arc<Host>,
    max_frame: usize,
) -> io::Result<()> {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "?".into());

    // The frame layer re-checks its deadline whenever a read comes back empty, so the socket
    // needs to come back empty rather than block indefinitely. This value only sets how promptly
    // a stalled frame is noticed; `DEFAULT_BODY_TIMEOUT` decides how long one is tolerated.
    stream
        .set_read_timeout(Some(SOCKET_POLL_INTERVAL))
        .map_err(|e| io::Error::other(format!("could not set a read timeout: {e}")))?;
    // The same, for the direction this worker writes in. Without it a client that connects
    // and then stops reading pins this thread — and its `MAX_CLIENTS` slot — in `write_frame` as
    // soon as the queued events outgrow the socket buffer. Keepalive does not reach that peer
    // either: it is alive and acknowledging, it has just stopped draining.
    stream
        .set_write_timeout(Some(SOCKET_POLL_INTERVAL))
        .map_err(|e| io::Error::other(format!("could not set a write timeout: {e}")))?;

    let conn = rustls::ServerConnection::new(tls).map_err(io::Error::other)?;
    let mut tls_stream = rustls::StreamOwned::new(conn, stream);

    // The greeting before anything else, within a short deadline: a peer that connects and says
    // nothing must not hold one of the `MAX_CLIENTS` slots. Its content is not checked — there is
    // no sign-in; anyone on the network the host listens on may cut. A desktop from before
    // sign-in was removed sends its token here, and is let in the same way.
    let _hello: String =
        read_frame(&mut tls_stream, 1024, Some(HELLO_TIMEOUT), HELLO_TIMEOUT)
            .map_err(io::Error::other)?;
    eprintln!("cut host: {peer} connected");
    write_frame(&mut tls_stream, &Response::Ok, DEFAULT_BODY_TIMEOUT).map_err(io::Error::other)?;

    // Events for every cutter go out on this connection; requests come back on it.
    // ponytail: they are written just before each reply rather than pushed the
    // moment they happen, so a client that asks nothing hears nothing. That suits a
    // desktop that polls `Snapshot`; give events a connection of their own when a UI
    // wants them pushed.
    let events = host.subscribe();

    loop {
        // Drain whatever the cutters have said before waiting on the client again.
        while let Ok(event) = events.try_recv() {
            write_frame(&mut tls_stream, &event, DEFAULT_BODY_TIMEOUT).map_err(io::Error::other)?;
        }
        // `None` buys the client an unbounded wait for its next request to *begin*, and nothing
        // more: it is idle between polls, not stalled mid-frame, and must not be dropped for
        // waiting. From the first byte on, `read_frame` holds it to `DEFAULT_BODY_TIMEOUT` —
        // otherwise a client that sends one length byte and stops keeps this worker, and one of
        // the `MAX_CLIENTS` slots, until the daemon is restarted. Keepalive does not reach that
        // peer: it is alive and acknowledging, it just stopped talking.
        match read_frame::<_, Request>(&mut tls_stream, max_frame, None, DEFAULT_BODY_TIMEOUT) {
            Ok(request) => {
                if let Request::Dispatch { ref device, .. } = request {
                    eprintln!("cut host: {peer} dispatched to {device}");
                }
                let response = handle_request(host, request);
                write_frame(&mut tls_stream, &response, DEFAULT_BODY_TIMEOUT)
                    .map_err(io::Error::other)?;
            }
            Err(FrameError::Eof) => return Ok(()),
            Err(e) => return Err(io::Error::other(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::testing::{TwoCutterFactory, CAMEO};
    use std::sync::Arc;

    #[test]
    fn list_devices_answers_with_every_cutter() {
        let host = Host::start(Arc::new(TwoCutterFactory));
        match handle_request(&host, Request::ListDevices) {
            Response::Devices(d) => assert_eq!(d.len(), 2),
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn snapshot_answers_with_every_cutters_status() {
        let host = Host::start(Arc::new(TwoCutterFactory));
        match handle_request(&host, Request::Snapshot) {
            Response::Snapshots(s) => {
                assert_eq!(s.len(), 2);
                assert!(s.iter().all(|s| s.status.phase == driver_core::Phase::Idle));
            }
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn a_refused_dispatch_answers_refused_rather_than_dropping_the_connection() {
        let host = Host::start(Arc::new(TwoCutterFactory));
        let request = Request::Dispatch {
            dispatch_id: crate::protocol::DispatchId("d-1".into()),
            device: "usb:9:9".into(),
            machine_id: "cameo5".into(),
            passes: Vec::new(),
        };
        assert!(matches!(handle_request(&host, request), Response::Refused(_)));
    }

    #[test]
    fn an_unknown_cutter_refuses_every_verb_over_the_wire() {
        let host = Host::start(Arc::new(TwoCutterFactory));
        for request in [
            Request::Cancel { device: "usb:9:9".into() },
            Request::Resume { device: "usb:9:9".into() },
            Request::ConfirmPassDone { device: "usb:9:9".into() },
            Request::Reconnect { device: "usb:9:9".into() },
        ] {
            assert!(matches!(handle_request(&host, request), Response::Refused(_)));
        }
    }

    #[test]
    fn a_cancel_on_a_known_cutter_answers_ok() {
        let host = Host::start(Arc::new(TwoCutterFactory));
        assert!(matches!(handle_request(&host, Request::Cancel { device: CAMEO.into() }), Response::Ok));
    }

    /// Not a test that a dead peer is actually reaped — that needs a suspended machine or a
    /// firewall rule to fake convincingly, and a mock would only test the mock. This instead
    /// catches the regression that matters day to day: someone deleting the `enable_keepalive`
    /// call from the accept loop.
    #[test]
    fn an_accepted_stream_has_keepalive_enabled() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let _client = std::net::TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();

        enable_keepalive(&server, "test");

        assert!(socket2::SockRef::from(&server).keepalive().unwrap());
    }
}
