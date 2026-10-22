# Supported platforms

**wiggl3** is a small desktop app that keeps a computer looking active: when you are idle it glides the cursor
across the screens at an interval you choose, so chat apps do not show "Away" and the screen does not lock or sleep.
It has a tray / menu bar icon, notifications, a schedule, and runs one copy at a time.

## What it runs on

| System | Requirement | Built as |
|---|---|---|
| Windows | Windows 10 or 11, 64-bit. Windows 11 on ARM runs it through emulation. | `wiggl3-windows-x64.exe` |
| macOS | macOS 11 or newer; Apple silicon and Intel in one app | `wiggl3-mac-universal.dmg` |
| Linux Intel/AMD | 64-bit, glibc 2.28+, any desktop distribution, X11 or Wayland | `wiggl3-linux-x86_64.tar.gz` |
| Linux ARM | 64-bit ARM, glibc 2.28+, same desktops | `wiggl3-linux-aarch64.tar.gz` |

Linux files are built inside a glibc 2.28 container, so they run on anything at least as new as Debian 10, Ubuntu 18.10,
RHEL/Rocky/Alma 8, Fedora 29 and their relatives. The program carries no graphics libraries of its own: it loads
OpenGL, X11, Wayland and `libxkbcommon` from the system when it starts, which every normal desktop has.

**Not supported:** 32-bit systems, Windows 7/8/8.1, macOS 10.x, Alpine / musl Linux, FreeBSD and other BSDs, ChromeOS
(except through its Linux container), Android, iOS, servers with no screen.

## How each system does the work

| | Idle time | Cursor movement | Stay awake | Tray / menu bar | Notifications |
|---|---|---|---|---|---|
| Windows | `GetLastInputInfo` | `SendInput` across the virtual desktop | `SetThreadExecutionState` | Notification area icon (follows light/dark taskbar) | Windows toast |
| macOS | CoreGraphics idle time | CoreGraphics mouse event (needs Accessibility permission) | IOKit power assertion | Menu bar item (adapts to light/dark) | Notification Centre |
| Linux X11 | XScreenSaver | XTest fake input | Inhibit portal | StatusNotifierItem (D-Bus) | freedesktop notifications |
| Linux Wayland | GNOME or KDE over D-Bus; otherwise unknown (then it moves every interval) | RemoteDesktop portal (the user approves once) | Inhibit portal | StatusNotifierItem (D-Bus) | freedesktop notifications |

## Behaviour that depends on the desktop

- **GNOME:** shows no tray icons unless the "AppIndicator and KStatusNotifierItem Support" extension is installed.
  Without a tray, closing the window quits wiggl3 (it says so in the window).
- **Wayland compositors without the RemoteDesktop portal** (some Sway / Hyprland setups): wiggl3 cannot move the cursor
  and tells the user so instead of silently doing nothing.
- **Wayland window:** a native Wayland application cannot hide its own window, so wiggl3 draws its window through
  XWayland when that is available. Without XWayland, closing the window quits.
- **macOS:** the Accessibility permission is required. The Dock icon does not bring back a closed window; the
  menu bar icon does.
- **Windows:** unsigned, so SmartScreen shows a one-time warning. Managed computers may block unsigned programs
  entirely; wiggl3 does not try to bypass that.

## What has actually been tested

This is the honest list. "Verified" means run and observed in this project's test setup; "compiled" means the
code builds for that system but has never been run on it.

| Area | Status |
|---|---|
| Linux x86_64, X11 (virtual desktop): window, toggle, cursor movement when idle, notification, tray registration, close-to-tray, second copy brings window back | **Verified** |
| Linux, Wayland-style session with XWayland: window hides and returns; clear notice when the portal is missing | **Verified** (no real Wayland compositor) |
| Unit tests (engine, schedule, settings, movement, single-instance token) | **Verified**, 22 passing |
| Windows build (type-check against the Windows API) | **Compiled** (not run) |
| macOS build, Apple silicon and Intel (type-check, with the notification library left out) | **Compiled** (not run) |
| Linux ARM | **Compiled only through CI** (not run) |
| GitHub build workflow | **Syntax-checked**, never run until you push it |
| Real GNOME, KDE, XFCE trays and their icon appearance | Not tested |
| Real Wayland compositors (GNOME, KDE, Sway) and the RemoteDesktop portal | Not tested |
| Windows and macOS: tray, notifications, permissions, signing warnings | Not tested |
| Old distributions (glibc 2.28 floor) | Not tested; guaranteed only by the build container |

The first run on a real Windows or Mac is the real test. If anything is wrong there, it is reported against this list.
