// SPDX-License-Identifier: GPL-3.0-or-later
//! The desktop's IPC commands, by the same names and the same JavaScript argument names
//! (`apps/desktop/ipc-inventory.json`), answered from the same `AppState` and device manager. What
//! each does is `apps/desktop/src/ipc.rs`'s business; this file only unpacks arguments and maps
//! results. A command errs the way its desktop twin does, as a string or as an `IpcError` object,
//! because the UI's `ipcErrorMessage` reads both.
//!
//! What differs is only what a browser cannot do: name a path on the server (every path argument
//! is a bare file name in the server's files directory, see `files.rs`), open a native picker, or
//! pair a Cut Host (this server is the machine the cutter is on).

use std::path::PathBuf;
use std::sync::Mutex;

use desktop::device::{CutRequest, DeviceManagerHandle, IpcError, TravelPassDto};
use desktop::state::AppState;
use document::NodeId;
use geometry::Affine;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(test)]
use serde_json::json;

pub struct Ctx {
    pub state: Mutex<AppState>,
    pub dev: DeviceManagerHandle,
    pub files: PathBuf,
    pub updates: crate::update::Updates,
}

pub type Reply = Result<Value, Value>;

fn ipc_err(code: &str, message: impl Into<String>) -> Value {
    serde_json::to_value(IpcError { code: code.into(), message: message.into() }).unwrap_or(Value::Null)
}

/// One argument, by its JavaScript name. Absent reads as null, which an `Option` accepts and
/// anything else refuses with the argument's name.
fn arg<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, Value> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| Value::String(format!("invalid argument {key}: {e}")))
}

fn ok<T: Serialize>(v: T) -> Reply {
    serde_json::to_value(v).map_err(|e| Value::String(e.to_string()))
}

/// A desktop command that errs with a string.
fn s<T: Serialize, E: ToString>(r: Result<T, E>) -> Reply {
    r.map_err(|e| Value::String(e.to_string())).and_then(ok)
}

/// A desktop command that errs with an `IpcError`.
fn i<T: Serialize>(r: Result<T, IpcError>) -> Reply {
    r.map_err(|e| serde_json::to_value(e).unwrap_or(Value::Null)).and_then(ok)
}

fn not_here(what: &str) -> Reply {
    Err(ipc_err("unsupported", format!("{what} is not available in the web version")))
}

#[derive(Deserialize)]
struct TransformMove {
    ids: Vec<NodeId>,
    m: Affine,
}

#[derive(Deserialize)]
struct TestCutRequest {
    device_instance_id: String,
    x_mm: f64,
    y_mm: f64,
    pass: desktop::device::ConfiguredPassDto,
    #[serde(default)]
    operator: Option<String>,
}

pub fn dispatch(ctx: &Ctx, cmd: &str, a: &Value) -> Reply {
    let st = || ctx.state.lock().unwrap();
    let file = |key: &str| -> Result<PathBuf, Value> {
        let name: String = arg(a, key)?;
        crate::files::resolve(&ctx.files, &name).map_err(Value::String)
    };
    match cmd {
        "new_doc" => ok(st().new_doc()),
        "snapshot" => ok(st().snapshot()),
        "commit_transform" => s(st().commit_transform(arg(a, "ids")?, arg(a, "m")?)),
        "commit_transforms" => {
            let moves: Vec<TransformMove> = arg(a, "moves")?;
            s(st().commit_transforms(moves.into_iter().map(|t| (t.ids, t.m)).collect()))
        }
        "add_primitive" => s(st().add_primitive(arg(a, "parent")?, arg(a, "kind")?, arg(a, "at")?)),
        "boolean_op" => s(st().boolean_op(arg(a, "ids")?, arg(a, "op")?)),
        "add_text" => s(st().add_text(arg(a, "parent")?, arg(a, "family")?, arg(a, "sizeMm")?, arg(a, "text")?)),
        "offset_shapes" => s(st().offset_shapes(arg(a, "ids")?, arg(a, "distanceMm")?, arg(a, "union")?, arg(a, "join")?)),
        "weed_box" => s(st().weed_box(arg(a, "ids")?, arg(a, "marginMm")?, arg(a, "lineSpacingMm")?)),
        "array_copies" => s(st().array_copies(arg(a, "ids")?, arg(a, "cols")?, arg(a, "rows")?, arg(a, "gapXMm")?, arg(a, "gapYMm")?)),
        "update_text" => s(st().update_text(arg(a, "id")?, arg(a, "family")?, arg(a, "sizeMm")?, arg(a, "text")?)),
        "convert_to_path" => s(st().convert_to_path(arg(a, "ids")?)),
        "weld" => s(st().weld(arg(a, "ids")?)),
        "delete" => s(st().delete(arg(a, "ids")?)),
        "reorder" => s(st().reorder(arg(a, "id")?, arg(a, "newIndex")?)),
        "set_cut_line_type" => s(st().set_cut_line_type(arg(a, "ids")?, arg(a, "value")?)),
        "set_stroke_color" => s(st().set_stroke_color(arg(a, "ids")?, arg(a, "rgba")?)),
        "set_pass_settings" => ok(st().set_pass_settings(arg(a, "key")?, arg(a, "value")?)),
        "set_pass_order" => ok(st().set_pass_order(arg(a, "order")?)),
        "set_mirror" => ok(st().set_mirror(arg(a, "on")?)),
        "set_media" => s(st().set_media(arg(a, "wMm")?, arg(a, "hMm")?).map(|_| ())),
        "set_material_preset" => s(st().set_material_preset(arg(a, "ids")?, arg(a, "value")?)),
        "undo" => ok(st().undo()),
        "redo" => ok(st().redo()),
        "import_svg" => s(st().import_svg(arg(a, "bytes")?, arg(a, "parent")?)),
        "save_project" => {
            let path = file("path")?;
            s(st().save_project(&path))
        }
        "load_project" => {
            let path = file("path")?;
            s(st().load_project(&path))
        }
        "set_machine" => {
            let id: String = arg(a, "machineId")?;
            s(st().set_machine(&id))
        }
        "list_machines" => ok(st().list_machines()),
        "registration_area_for_paper" => ok(document::RegistrationArea::for_paper(arg(a, "paper")?)),
        "add_registration_marks" => s(st().add_registration_marks(arg(a, "area")?)),
        "set_registration_enabled" => s(st().set_registration_enabled(arg(a, "on")?)),
        "registration_status" => ok(st().registration_status()),
        "export_print_svg" => {
            let path = file("path")?;
            s(st().export_print_svg(&path, arg(a, "paper")?))
        }

        "list_devices" => ok(ctx.dev.list_devices()),
        "connect_device" => i(ctx.dev.connect(arg(a, "info")?)),
        "disconnect_device" => i(ctx.dev.disconnect()),
        "reconnect_device" => i(ctx.dev.reconnect()),
        "get_device_state" => ok(ctx.dev.status()),
        "get_connected_device" => ok(ctx.dev.connected.lock().unwrap().clone()),
        "plan_cut" => i(desktop::device::plan_cut_response(&st().editor.doc, arg(a, "grouping")?)),
        "travel_for_order" => {
            let rev: String = arg(a, "docRevision")?;
            let passes: Vec<TravelPassDto> = arg(a, "passes")?;
            i(desktop::device::travel_for_order(&st().editor.doc, &rev, arg(a, "grouping")?, &passes))
        }
        "cut" => {
            let request: CutRequest = arg(a, "request")?;
            // The document lock is dropped before the blocking send, as on the desktop: a cut
            // waiting on the worker must not stop another browser from reading the document.
            let prepared = {
                let app = st();
                ctx.dev.prepare_logged_cut(&app, request)
            };
            i(prepared.and_then(|(planned_for, passes, usage)| ctx.dev.execute_logged_cut(planned_for, passes, Some(usage))))
        }
        "test_cut" => test_cut(ctx, a),
        "cancel_cut" => i(ctx.dev.cancel()),
        "resume_cut" => i(ctx.dev.resume()),
        "confirm_pass_done" => i(ctx.dev.confirm_pass_done()),
        "list_presets" => {
            let id: String = arg(a, "machineId")?;
            i(desktop::settings::presets_path().and_then(|p| desktop::device::list_presets(&p, &id)))
        }
        "machine_caps" => {
            let id: String = arg(a, "machineId")?;
            i(ctx.dev.caps_for(&id))
        }
        "settings_ranges" => {
            let id: Option<String> = arg(a, "machineId")?;
            i(match id {
                Some(id) => ctx.dev.caps_for(&id).map(|c| cutplan::preflight::settings_ranges_for(&c)),
                None => Ok(cutplan::preflight::SETTINGS_RANGES),
            })
        }
        "save_preset" => {
            let preset: cutplan::presets::MaterialPreset = arg(a, "p")?;
            i(desktop::settings::presets_path().and_then(|p| desktop::device::save_preset(&p, preset)))
        }
        "delete_preset" => {
            let machine: String = arg(a, "machineId")?;
            let id: String = arg(a, "id")?;
            i(desktop::settings::presets_path().and_then(|p| desktop::device::delete_preset(&p, &machine, &id)))
        }
        "get_presets_location" => i(desktop::settings::location()),
        "set_presets_location" => not_here("Choosing a presets folder"),
        "usage_log" => i(ctx.dev.usage_log(arg(a, "limit")?)),
        "export_usage_csv" => {
            let path = file("path")?;
            i(ctx.dev.export_usage_csv(&path))
        }
        // This server is the machine the cutter is attached to: there is no Cut Host to pair.
        "list_hosts" => ok(Vec::<Value>::new()),
        "probe_host" | "existing_pairing" | "test_host" | "pair_host" | "forget_host" => not_here("Pairing a Cut Host"),
        "trace_controls" => ok(trace::control_specs()),
        "list_fonts" => ok(geometry::list_font_families()),
        "trace_image" => {
            let path = file("path")?;
            let controls: trace::TraceControls = arg(a, "controls")?;
            i(trace::read_image(&path).and_then(|b| trace::trace(&b, &controls)).map_err(trace_error))
        }
        "load_image_preview" => {
            let path = file("path")?;
            let png = trace::read_image(&path).and_then(|b| trace::preview_png(&b)).map_err(|e| Value::String(e.to_string()))?;
            use base64::Engine as _;
            ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
        }
        // A browser uploads the image instead (`POST /upload`), and names it by what it was saved as.
        "pick_image" => not_here("The native file picker"),
        "force_quit" => not_here("Quitting the server from the page"),
        // The server's own commands, which the desktop has no counterpart for (`update.rs`).
        "web_update_status" => Ok(ctx.updates.status()),
        "web_update_check" => ctx.updates.check(),
        "web_update_apply" => {
            let tag: String = arg(a, "tag")?;
            ctx.updates.apply(&tag)
        }
        "web_update_set_repo" => {
            let repo: String = arg(a, "repo")?;
            ctx.updates.set_repo(&repo)
        }
        _ => Err(Value::String(format!("unknown command {cmd}"))),
    }
}

/// The desktop's `test_cut`: the test figure at a point, with one pass's settings, through the same
/// plan and preflight as a real cut.
fn test_cut(ctx: &Ctx, a: &Value) -> Reply {
    let request: TestCutRequest = arg(a, "request")?;
    let prepared = {
        let app = ctx.state.lock().unwrap();
        let scratch = app.test_cut_scratch(request.x_mm, request.y_mm);
        let cut = CutRequest {
            device_instance_id: request.device_instance_id,
            doc_revision: cutplan::doc_revision(&scratch.editor.doc).to_string(),
            grouping: cutplan::Grouping::Single,
            passes: vec![desktop::device::ConfiguredPassDto { key: cutplan::PassKey::All, enabled: true, ..request.pass }],
            operator: request.operator,
        };
        ctx.dev.prepare_logged_cut(&scratch, cut)
    };
    i(prepared.and_then(|(planned_for, passes, usage)| ctx.dev.execute_logged_cut(planned_for, passes, Some(usage))))
}

fn trace_error(e: trace::TraceError) -> IpcError {
    IpcError { code: e.code().into(), message: e.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dir: &std::path::Path) -> Ctx {
        let (dev, _events) = DeviceManagerHandle::new(std::sync::Arc::new(driver_registry::HardwareBackendFactory));
        Ctx { state: Mutex::new(AppState::new()), dev, files: dir.to_path_buf(), updates: crate::update::Updates::new(dir, &dir.join("status.json"), false) }
    }

    #[test]
    fn a_drawn_rect_round_trips_through_save_and_load_by_file_name() {
        let dir = tempfile::tempdir().unwrap();
        let c = ctx(dir.path());
        let snap: String = serde_json::from_value(dispatch(&c, "snapshot", &json!({})).unwrap()).unwrap();
        let root = serde_json::from_str::<Value>(&snap).unwrap()["root"].clone();
        dispatch(&c, "set_machine", &json!({ "machineId": "cameo5" })).unwrap();
        dispatch(&c, "add_primitive", &json!({ "parent": root, "kind": { "Rect": { "w": 5.0, "h": 5.0 } }, "at": [10.0, 20.0] })).unwrap();
        dispatch(&c, "save_project", &json!({ "path": "design.cut" })).unwrap();
        assert!(dir.path().join("design.cut").exists());
        dispatch(&c, "new_doc", &json!({})).unwrap();
        dispatch(&c, "load_project", &json!({ "path": "design.cut" })).unwrap();
        let snap: String = serde_json::from_value(dispatch(&c, "snapshot", &json!({})).unwrap()).unwrap();
        assert!(snap.contains("\"Rect\""));
    }

    #[test]
    fn a_path_outside_the_files_directory_is_refused_before_anything_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let c = ctx(dir.path());
        for path in ["../escape.cut", "/tmp/escape.cut", "sub/escape.cut"] {
            assert!(dispatch(&c, "save_project", &json!({ "path": path })).is_err(), "{path} was accepted");
        }
        assert!(!dir.path().parent().unwrap().join("escape.cut").exists());
    }

    #[test]
    fn an_unknown_command_and_a_missing_argument_are_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let c = ctx(dir.path());
        assert_eq!(dispatch(&c, "rm_rf", &json!({})), Err(Value::String("unknown command rm_rf".into())));
        let err = dispatch(&c, "set_mirror", &json!({})).unwrap_err();
        assert!(err.as_str().unwrap().contains("on"), "{err}");
    }

    #[test]
    fn every_command_the_desktop_registers_is_answered() {
        let inventory: serde_json::Map<String, Value> =
            serde_json::from_str(include_str!("../../desktop/ipc-inventory.json")).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let c = ctx(dir.path());
        for cmd in inventory.keys() {
            // Commands with side effects on hardware or files still answer: with no arguments they
            // refuse on an argument, never as an unknown command.
            if matches!(cmd.as_str(), "list_devices" | "list_fonts") { continue; }
            if let Err(Value::String(msg)) = dispatch(&c, cmd, &json!({})) {
                assert!(!msg.starts_with("unknown command"), "{cmd} is not handled");
            }
        }
    }
}
