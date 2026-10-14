//! Questions about other apps' windows: is the pointer on a resize border, how
//! big is the window, and how small or large does it let itself get.
//!
//! This process is per-monitor DPI aware, so window rectangles and the cursor
//! are in physical pixels. Messages carry no DPI translation, so points and
//! sizes sent to or read from DPI-virtualised windows are converted here.

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
use windows_sys::Win32::UI::HiDpi::{
    GetDpiForMonitor, GetDpiForWindow, GetSystemMetricsForDpi, MDT_EFFECTIVE_DPI,
    PhysicalToLogicalPointForPerMonitorDPI,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GA_PARENT, GA_ROOT, GetAncestor, GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTLEFT, HTRIGHT, HTTOP,
    HTTOPLEFT, HTTOPRIGHT, HTTRANSPARENT, IsHungAppWindow, IsZoomed, MINMAXINFO, SEND_MESSAGE_TIMEOUT_FLAGS,
    SM_CXMAXIMIZED, SM_CXMAXTRACK, SM_CXMINTRACK, SM_CYMAXIMIZED, SM_CYMAXTRACK, SM_CYMINTRACK, SMTO_ABORTIFHUNG,
    SMTO_BLOCK, SendMessageTimeoutW, WM_GETMINMAXINFO, WM_NCHITTEST, WindowFromPoint,
};

use crate::shared::geometry::{Edges, Limits, Rect};

/// How long to wait for an app to answer a question. The mouse hook waits for
/// the hit test, so this has to stay well under the system's hook timeout.
const ANSWER_TIMEOUT_MS: u32 = 100;

/// The top-level window whose resize border is under `pt`, and the edges it grabs.
pub fn border_under(pt: POINT) -> Option<(HWND, Edges)> {
    let mut hwnd = unsafe { WindowFromPoint(pt) };
    if hwnd.is_null() || unsafe { IsHungAppWindow(hwnd) } != 0 {
        return None;
    }
    // Like the system: a child that says "not me" hands the click to its parent.
    let hit = loop {
        let hit = hit_test(hwnd, pt)?;
        if hit != HTTRANSPARENT {
            break hit;
        }
        if unsafe { GetAncestor(hwnd, GA_ROOT) } == hwnd {
            return None;
        }
        hwnd = unsafe { GetAncestor(hwnd, GA_PARENT) };
    };
    let edges = edges_for(hit)?;
    // Whole windows only: child windows with their own borders (MDI) resize normally.
    if unsafe { GetAncestor(hwnd, GA_ROOT) } != hwnd || unsafe { IsZoomed(hwnd) } != 0 {
        return None;
    }
    Some((hwnd, edges))
}

/// Asks the window what's at `pt`, the same question the system asks on a click.
/// `None` if it doesn't answer in time or Windows won't let us ask (admin windows).
fn hit_test(hwnd: HWND, pt: POINT) -> Option<i32> {
    let mut local = pt;
    unsafe { PhysicalToLogicalPointForPerMonitorDPI(hwnd, &mut local) };
    let lparam = (((local.y as u16 as u32) << 16) | local.x as u16 as u32) as isize;
    // SMTO_BLOCK: don't run nested hook calls on the hook thread while waiting.
    send(hwnd, WM_NCHITTEST, lparam, SMTO_BLOCK).map(|result| result as i32)
}

fn edges_for(hit: i32) -> Option<Edges> {
    let (left, top, right, bottom) = match u32::try_from(hit).ok()? {
        HTLEFT => (true, false, false, false),
        HTRIGHT => (false, false, true, false),
        HTTOP => (false, true, false, false),
        HTBOTTOM => (false, false, false, true),
        HTTOPLEFT => (true, true, false, false),
        HTTOPRIGHT => (false, true, true, false),
        HTBOTTOMLEFT => (true, false, false, true),
        HTBOTTOMRIGHT => (false, false, true, true),
        _ => return None,
    };
    Some(Edges { left, top, right, bottom })
}

pub fn rect(hwnd: HWND) -> Option<Rect> {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    (unsafe { GetWindowRect(hwnd, &mut r) } != 0).then_some(Rect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    })
}

/// The window's minimum and maximum size, as it would tell the system during
/// a normal resize.
pub fn size_limits(hwnd: HWND) -> Limits {
    unsafe {
        // Pre-fill the system defaults in the window's own DPI, as Windows does.
        let window_dpi = GetDpiForWindow(hwnd).max(1);
        let metric = |index| GetSystemMetricsForDpi(index, window_dpi);
        let mut info = MINMAXINFO {
            ptReserved: POINT { x: 0, y: 0 },
            ptMaxSize: POINT { x: metric(SM_CXMAXIMIZED), y: metric(SM_CYMAXIMIZED) },
            ptMaxPosition: POINT { x: 0, y: 0 },
            ptMinTrackSize: POINT { x: metric(SM_CXMINTRACK), y: metric(SM_CYMINTRACK) },
            ptMaxTrackSize: POINT { x: metric(SM_CXMAXTRACK), y: metric(SM_CYMAXTRACK) },
        };
        send(hwnd, WM_GETMINMAXINFO, (&raw mut info) as isize, 0);

        // DPI-unaware and system-aware windows answer in scaled units.
        let (mut monitor_dpi, mut unused) = (window_dpi, window_dpi);
        GetDpiForMonitor(
            MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST),
            MDT_EFFECTIVE_DPI,
            &mut monitor_dpi,
            &mut unused,
        );
        let physical = |v: i32| (v as i64 * monitor_dpi as i64 / window_dpi as i64) as i32;
        Limits {
            min: (physical(info.ptMinTrackSize.x), physical(info.ptMinTrackSize.y)),
            max: (physical(info.ptMaxTrackSize.x), physical(info.ptMaxTrackSize.y)),
        }
    }
}

fn send(hwnd: HWND, msg: u32, lparam: isize, flags: SEND_MESSAGE_TIMEOUT_FLAGS) -> Option<usize> {
    let mut result = 0;
    let ok = unsafe {
        SendMessageTimeoutW(hwnd, msg, 0, lparam, SMTO_ABORTIFHUNG | flags, ANSWER_TIMEOUT_MS, &mut result)
    };
    (ok != 0).then_some(result)
}
