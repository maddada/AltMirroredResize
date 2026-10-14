//! Applies symmetric resizes on its own thread, so a slow app never stalls the
//! mouse hook. Pointer moves that pile up while the app is still redrawing are
//! coalesced: only the latest one is applied.

use std::sync::mpsc::{self, Receiver, Sender};

use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
};

use super::window;
use crate::shared::geometry::{self, Edges, Limits, Rect};

/// Tag on the key presses we inject.
const MARKER: usize = 0x414C_5452;

/// A virtual-key code with no key assigned to it.
const VK_UNASSIGNED: u16 = 0xE8;

pub enum Event {
    /// The user grabbed `edges` of `hwnd` with the pointer at `cursor`.
    Start { hwnd: isize, edges: Edges, cursor: POINT },
    Move(POINT),
    End,
}

#[derive(Clone)]
pub struct Resizer(Sender<Event>);

impl Resizer {
    pub fn send(&self, event: Event) {
        let _ = self.0.send(event);
    }
}

pub fn spawn() -> Resizer {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("resizer".into())
        .spawn(move || run(rx))
        .expect("failed to spawn resizer thread");
    Resizer(tx)
}

fn run(events: Receiver<Event>) {
    let mut session: Option<Session> = None;
    while let Ok(first) = events.recv() {
        let mut latest_move = None;
        for event in std::iter::once(first).chain(events.try_iter()) {
            match event {
                Event::Move(pt) => latest_move = Some(pt),
                Event::Start { hwnd, edges, cursor } => {
                    session = Session::begin(hwnd as HWND, edges, cursor);
                    latest_move = None;
                }
                Event::End => {
                    if let (Some(s), Some(pt)) = (session.as_mut(), latest_move.take()) {
                        s.follow(pt);
                    }
                    session = None;
                }
            }
        }
        if let (Some(s), Some(pt)) = (session.as_mut(), latest_move) {
            s.follow(pt);
        }
    }
}

struct Session {
    hwnd: HWND,
    edges: Edges,
    /// Window rectangle and pointer position when the drag started.
    start: Rect,
    cursor: POINT,
    limits: Limits,
    last: Rect,
}

impl Session {
    fn begin(hwnd: HWND, edges: Edges, cursor: POINT) -> Option<Self> {
        let start = window::rect(hwnd)?;
        mask_alt_release();
        Some(Self { hwnd, edges, start, cursor, limits: window::size_limits(hwnd), last: start })
    }

    /// Resizes the window for the pointer being at `pt`.
    fn follow(&mut self, pt: POINT) {
        let (dx, dy) = (pt.x - self.cursor.x, pt.y - self.cursor.y);
        let target = geometry::symmetric(self.start, self.edges, dx, dy, self.limits);
        if target == self.last {
            return;
        }
        self.last = target;
        set_rect(self.hwnd, target, 0);

        // Some apps enforce limits they didn't report; keep the window centred anyway.
        if let Some(actual) = window::rect(self.hwnd)
            && (actual.width(), actual.height()) != (target.width(), target.height())
        {
            let centred = geometry::around_centre(self.start, self.edges, actual.width(), actual.height());
            if centred != actual {
                set_rect(self.hwnd, centred, SWP_NOSIZE);
            }
        }
    }
}

fn set_rect(hwnd: HWND, r: Rect, extra: SET_WINDOW_POS_FLAGS) {
    let flags = SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE | extra;
    unsafe { SetWindowPos(hwnd, std::ptr::null_mut(), r.left, r.top, r.width(), r.height(), flags) };
}

/// Taps an unassigned key so that letting go of Alt afterwards doesn't open the
/// focused app's menu bar (Windows only does that when Alt was pressed alone).
fn mask_alt_release() {
    let key = |flags: KEYBD_EVENT_FLAGS| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VK_UNASSIGNED, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: MARKER },
        },
    };
    let inputs = [key(0), key(KEYEVENTF_KEYUP)];
    unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32) };
}
