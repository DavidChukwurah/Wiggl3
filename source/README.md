# wiggl3 source (for developers and reviewers)

**wiggl3** keeps a computer looking active by gliding the cursor when the user is idle. It runs on Windows 10/11,
macOS 11+ and desktop Linux (X11 and Wayland, Intel/AMD and ARM). Ordinary users download a finished file from the
Releases page; this folder is only for people who want to read or build the code. See `../README.md` and
`../SUPPORTED-PLATFORMS.md`.

## Build

Needs only **Rust 1.89 or newer** (CI pins 1.98.1) and a C compiler/linker (Windows: MSVC Build Tools; macOS: Xcode
Command Line Tools; Linux: gcc or clang). No X11, Wayland, GTK or GL development packages are needed.

```
cargo test --locked
cargo build --release --locked        # target/release/wiggl3
```

From the repository root: `make` (Linux/macOS) or `cmake -S . -B build && cmake --build build` (all systems). Both only wrap
`cargo`; they install nothing and use no sudo. `sh source/tools/doctor.sh` reports what is missing.

## Layout

| Path | What |
|---|---|
| `src/engine.rs` | Pure state machine (on/off, schedule, pause/resume). All time passed in; unit-tested. |
| `src/controller.rs` | Background thread: owns engine and OS backend, runs timers, so it works with the window hidden. |
| `src/nudge/` | Per-OS idle time, cursor glide, keep-awake (`windows.rs`, `macos.rs`, `linux.rs`). |
| `src/shell.rs` | Show / hide the window from anywhere (Win32 on Windows, egui commands elsewhere). |
| `src/tray/` | Tray icon: `desktop.rs` (Windows, macOS via tray-icon), `linux.rs` (StatusNotifierItem). |
| `src/single.rs` | One copy only: file lock plus a "show" token read by a watcher thread. |
| `src/ui/` | The window (egui). `theme.rs` has the palette: greys only, no accent colour. |
| `tools/make_icon.py` | Regenerates every icon from the Material Symbols glyph (needs cairosvg, pillow). |
| `tools/make_mac_app.sh` | Wraps the binary as `wiggl3.app` with a fixed ad-hoc signature identity. |
| `tools/package_linux.sh`, `linux-install.sh`, `linux-uninstall.sh` | The Linux `.tar.gz` and per-user installer. |
| `tools/ci_linux.sh` | The Linux build inside the glibc 2.28 container (used by CI). |

## Design decisions worth knowing

- **Hidden windows are not redrawn everywhere.** Windows gets no paint messages for a hidden window, and native Wayland
  cannot hide a window at all. So tray clicks are callbacks, not polled from the UI; Windows uses `ShowWindow` directly
  plus a 500 ms `SetTimer` to keep the tray current; the second-copy signal has its own thread; on Wayland the window
  is created through XWayland (`ui::prefer_x11_window`) when possible.
- **Tray Quit exits the process directly** (after removing the tray icon, which avoids Windows' ghost icon). Settings are
  saved on every change and the OS releases the keep-awake request.
- **Keep-awake failures are logged, not shown**: the cursor movement already counts as activity.
- **No hiding from IT:** no disguised process name, no hidden tray, no evasion of software blocks.
- Rust is pinned in CI (`RUST_VERSION` in the workflow) so a new compiler release cannot break builds unannounced.

## Icons and colours

No hue is used anywhere. The window uses greys; the toggle is pale (dark mode) or near-black (light mode). Tray icons are
glyph-only with a transparent background: macOS gets a template image (the system recolours it), Windows follows the
taskbar's light/dark setting, Linux uses an outlined icon that reads on any panel. Only the outline of an invalid date or
time field is red.
