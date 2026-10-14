//! AltMirroredResize: hold Alt and drag a window edge or corner to resize the window
//! from both sides at once, around its centre (like Option-resizing on macOS).
//!
//! Layout:
//! - `shared/`   OS-independent: resize geometry, settings, icon art, log
//! - `platform/` one backend per OS: mouse hook, window resizing, tray, autostart

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod platform;
mod shared;

fn main() {
    shared::log::install_panic_hook();
    shared::settings::load();
    platform::run();
}
