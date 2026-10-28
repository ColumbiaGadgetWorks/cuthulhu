// SPDX-License-Identifier: GPL-3.0-or-later
//! `cuthulhu-web`: the cuthulhu editor served to a browser from the machine the cutter is plugged
//! into (a Raspberry Pi, typically). The page is the desktop's own UI; its IPC calls arrive here as
//! `POST /api/<command>` and run against the same `AppState` and `DeviceManagerHandle` the desktop
//! shell drives, so the cut path (`plan_cut`, preflight, the device worker) is the one chokepoint
//! it always was.

mod api;
mod files;
mod server;
mod update;

use std::path::PathBuf;

const USAGE: &str = "\
usage: cuthulhu-web [--bind ADDR:PORT] [--ui DIR] [--data DIR]

  --bind        address to listen on (default 0.0.0.0:8080)
  --ui          the built UI to serve (default: ./ui beside the binary)
  --data        where uploaded and saved files live (default /var/lib/cuthulhu-web)

Anyone who can reach the port can use the cutter: keep it on a network you trust.";

pub struct Config {
    pub bind: String,
    pub ui: PathBuf,
    pub data: PathBuf,
}

fn parse_args() -> Result<Config, String> {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let mut cfg = Config {
        bind: "0.0.0.0:8080".into(),
        ui: exe_dir.map(|d| d.join("ui")).unwrap_or_else(|| PathBuf::from("ui")),
        data: PathBuf::from("/var/lib/cuthulhu-web"),
    };
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{flag} needs a value\n\n{USAGE}"));
        match flag.as_str() {
            "--bind" => cfg.bind = value()?,
            "--ui" => cfg.ui = PathBuf::from(value()?),
            "--data" => cfg.data = PathBuf::from(value()?),
            // Sign-in is gone. A unit file from an older install still passes this, and refusing it
            // would leave the server down until someone reinstalls over SSH.
            "--token-file" => {
                value()?;
            }
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }
    Ok(cfg)
}

fn main() {
    let cfg = match parse_args() {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };
    if let Err(e) = server::run(cfg) {
        eprintln!("cuthulhu-web: {e}");
        std::process::exit(1);
    }
}
