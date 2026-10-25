// SPDX-License-Identifier: GPL-3.0-or-later
fn main() {
    #[cfg(feature = "app")]
    tauri_build::build()
}
