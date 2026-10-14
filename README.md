# AltMirroredResize

Hold **Alt** and drag any window edge or corner: the opposite side moves too, so the window
grows or shrinks around its centre, like Option-resizing on macOS. Lives in the notification area.

## Download

Get `AltMirroredResize.exe` from [Releases](https://github.com/maddada/AltMirroredResize/releases/latest)
and run it; there's nothing to install. The exe isn't code-signed, so SmartScreen may warn the
first time: click **More info → Run anyway**.

## Using it

| You do | What happens |
|---|---|
| Alt + drag the right edge outwards | Right edge follows the pointer, left edge moves out by the same amount |
| Alt + drag a corner | Both axes resize around the centre |
| Drag an edge without Alt | Normal Windows resize |
| Alt + click inside a window | Untouched (only resize borders are taken over) |

Tray icon: **left-click** toggles on/off, **right-click** for the menu (Start with Windows,
Restart as administrator, Quit). Launching the exe again shows a reminder of how to use it.
It works with mouse-sharing and remote-control tools like Deskflow too.

Settings live in `%APPDATA%\AltMirroredResize\settings.ini`, next to `log.txt`.

### Administrator mode

Windows doesn't let regular apps resize windows that run as administrator (Task Manager,
admin terminals…). Those keep the normal one-sided resize. Pick **Restart as administrator**
in the tray menu to cover them too. That's remembered, and "Start with Windows" then uses a
scheduled task with highest privileges, so there's no UAC prompt at sign-in.

## Building

```
cargo build --release
target\release\AltMirroredResize.exe
```

Needs the MSVC toolchain and the Windows SDK (`rc.exe` embeds the icon, manifest and version
info). `cargo test` runs the geometry and settings tests. `AltMirroredResize.exe --replace` takes over
from a copy that's already running, which is handy after rebuilding.

## Code layout

```
src/
  main.rs              entry point
  shared/              OS-independent
    geometry.rs        symmetric resize maths (pure, tested)
    settings.rs        enabled / run-as-admin, persisted as key = value
    icon.rs            procedural icon (also used by build.rs to make the .ico)
    log.rs             log file + panic hook
  platform/
    mod.rs             what each OS provides: run()
    windows/
      hook.rs          WH_MOUSE_LL hook: Alt + left-press on a resize border starts a resize
      window.rs        hit test (WM_NCHITTEST), window rect, min/max size (WM_GETMINMAXINFO)
      resizer.rs       worker thread: SetWindowPos with the symmetric rect, coalescing moves
      tray.rs          notification icon, menu, main message loop
      elevation.rs     admin detection, elevated relaunch
      autostart.rs     Run key / elevated scheduled task
      instance.rs      single-instance lock
build.rs               Windows resources (icon, manifest, version info)
```

How it works: the hook sees Alt + left-press, asks the window under the pointer what's there
(the same `WM_NCHITTEST` question Windows asks on a click) and, if the answer is a resize
border, swallows the press so the app doesn't start its own resize. Pointer moves pass through
untouched; the resizer thread turns each one into a window rectangle mirrored around the
starting centre and applies it with `SetWindowPos`. The release is swallowed too.

macOS has this built in. A Linux port would need its own pointer hook and window moves
(X11, or a compositor extension on Wayland) and could reuse `shared/geometry.rs`.

## Limitations

- Windows running as administrator need AltMirroredResize elevated too (see above).
- Apps that snap their size to a grid during a normal resize (some terminals) don't snap
  during an Alt resize. They only do that inside Windows' own resize loop.
- The app doesn't get the "resize started/finished" notifications of a normal resize, so apps
  that only save their window size at the end of a resize may not remember an Alt resize.
- Child windows with their own borders (MDI documents inside an app) resize normally.

## See also

- [MiddleClickDragScroll](https://github.com/maddada/MiddleClickDragScroll): hold the middle
  mouse button and drag to scroll, with trackpad-style flick momentum.
- [Ghostex](https://ghostex.dev) ([GitHub](https://github.com/maddada/Ghostex)): a native
  workspace for coding agents, built in Rust with GPUI.
