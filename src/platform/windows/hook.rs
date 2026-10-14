//! Low-level mouse hook: Alt + left-drag on a window's resize border starts a
//! symmetric resize. Only that button press and its release are swallowed (so
//! the app doesn't start its own one-sided resize); pointer motion passes
//! through, so the cursor stays on the edge it grabbed.
//!
//! Injected input counts too: with mouse-sharing tools (Deskflow, Mouse
//! Without Borders) or remote desktop apps, every click is injected.

use std::cell::Cell;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_MENU};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, HC_ACTION, MSG, MSLLHOOKSTRUCT, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
};

use super::resizer::{Event, Resizer};
use super::window;
use crate::shared::{log, settings};

static RESIZER: OnceLock<Resizer> = OnceLock::new();

thread_local! {
    /// A resize is in progress (the hook thread's own state).
    static RESIZING: Cell<bool> = const { Cell::new(false) };
}

/// Installs the hook on a dedicated high-priority thread.
pub fn start(resizer: Resizer) {
    let _ = RESIZER.set(resizer);
    std::thread::Builder::new()
        .name("mouse-hook".into())
        .spawn(|| unsafe {
            // Windows silently drops low-level hooks that respond too slowly, so
            // this thread does nothing else and runs at the highest priority.
            SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
            let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), GetModuleHandleW(std::ptr::null()), 0);
            if hook.is_null() {
                log::write("SetWindowsHookExW(WH_MOUSE_LL) failed");
                return;
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                DispatchMessageW(&msg);
            }
            UnhookWindowsHookEx(hook);
        })
        .expect("failed to spawn mouse hook thread");
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        if swallow(wparam as u32, info) {
            return 1;
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

/// Handles one mouse event; true if the app under the pointer shouldn't see it.
fn swallow(msg: u32, info: &MSLLHOOKSTRUCT) -> bool {
    let Some(resizer) = RESIZER.get() else { return false };
    let resizing = RESIZING.get();
    match msg {
        WM_MOUSEMOVE if resizing => {
            resizer.send(Event::Move(info.pt));
            false
        }
        WM_LBUTTONUP if resizing => {
            RESIZING.set(false);
            resizer.send(Event::Move(info.pt));
            resizer.send(Event::End);
            true
        }
        WM_LBUTTONDOWN => {
            if resizing {
                // The release never reached us (e.g. it happened on the secure desktop).
                RESIZING.set(false);
                resizer.send(Event::End);
            }
            let alt = unsafe { GetAsyncKeyState(VK_MENU as i32) } < 0;
            if !alt || !settings::current().enabled {
                return false;
            }
            let Some((hwnd, edges)) = window::border_under(info.pt) else { return false };
            resizer.send(Event::Start { hwnd: hwnd as isize, edges, cursor: info.pt });
            RESIZING.set(true);
            true
        }
        _ => false,
    }
}
