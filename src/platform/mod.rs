//! One backend per OS. Each provides `run()`: install the input hook, resize
//! windows, show the tray icon and run until the user quits.
//!
//! macOS already does this natively (Option-drag a window edge), so only
//! Windows needs it. A Linux port would hook the pointer (X11: XInput2,
//! Wayland: a compositor plugin) and reuse `shared::geometry`.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::run;

#[cfg(not(windows))]
pub fn run() {
    eprintln!("{} only supports Windows for now.", crate::shared::settings::APP_NAME);
}
