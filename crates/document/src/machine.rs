// SPDX-License-Identifier: GPL-3.0-or-later
use serde::{Serialize, Deserialize};
use geometry::Rect;
use crate::history::Editor;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MachineProfile {
    pub id: String,
    pub name: String,
    pub width_mm: f64,
    pub height_mm: f64,
}

pub fn builtin_profiles() -> Vec<MachineProfile> {
    vec![
        MachineProfile {
            id: "cameo5".into(),
            name: "Silhouette Cameo 5 Alpha".into(),
            width_mm: 330.0,
            height_mm: 3000.0,
        },
        MachineProfile {
            id: "puma".into(),
            name: "GCC Puma IV".into(),
            width_mm: 600.0,
            height_mm: 5000.0,
        },
        // The reachable area: 304 × 3000 mm less the 9 mm left and 1 mm top margins. Pinned to
        // `SilhouetteDriver::cameo1()`'s profile by the desktop's profile-agreement test.
        MachineProfile {
            id: "cameo1".into(),
            name: "Silhouette Cameo".into(),
            width_mm: 295.0,
            height_mm: 2999.0,
        },
    ]
}

/// The artboard a machine starts on: a 12×12 in mat, clamped to what the machine reaches. The full
/// reach of a roll cutter is metres long, and fitting that to a window left a design the width of
/// a matchstick; the media picker sizes it to anything else, a roll included.
pub const DEFAULT_MEDIA_MM: f64 = 304.8;

pub fn default_artboard(p: &MachineProfile) -> Rect {
    Rect { x: 0.0, y: 0.0, w: p.width_mm.min(DEFAULT_MEDIA_MM), h: p.height_mm.min(DEFAULT_MEDIA_MM) }
}

impl Editor {
    pub fn set_machine(&mut self, p: MachineProfile) {
        self.doc.artboard = default_artboard(&p);
        self.doc.machine = Some(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_ids_are_canonical() {
        let ids: Vec<String> = builtin_profiles().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, vec!["cameo5", "puma", "cameo1"]);
    }

    #[test]
    fn set_machine_resizes_artboard() {
        let mut ed = Editor::new();
        let puma = builtin_profiles()
            .into_iter()
            .find(|p| p.id == "puma")
            .unwrap();
        ed.set_machine(puma);
        assert_eq!((ed.doc.artboard.w, ed.doc.artboard.h), (DEFAULT_MEDIA_MM, DEFAULT_MEDIA_MM));
    }
}
