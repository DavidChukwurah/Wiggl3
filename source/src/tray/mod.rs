//! Tray / menu bar icon: Open, Pause / Resume, Quit; the icon changes between on and off.
//!
//! Windows and macOS use `tray-icon`; Linux uses the StatusNotifierItem protocol (`ksni`, pure D-Bus,
//! no GTK), which needs a tray host (KDE, XFCE, Cinnamon, MATE, Budgie and others have one; GNOME needs
//! the AppIndicator extension). `init` returns false when there is no tray, and the window then behaves
//! like an ordinary window (closing it quits).
//!
//! Menu clicks are handled in callbacks, not by polling from the window, so they work while the window
//! is hidden.

use crate::controller::Handle;
use crate::shell::Shell;
use chrono::Local;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

#[cfg(any(windows, target_os = "macos"))]
#[path = "desktop.rs"]
mod imp;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
mod imp {
    use super::{Handle, Shell};
    pub fn init(_shell: Shell, _controller: Handle) -> bool {
        false
    }
    pub fn apply(_online: bool, _toggle_on: bool) {}
    pub fn remove() {}
}

static EXISTS: AtomicBool = AtomicBool::new(false);
static CONTROLLER: OnceLock<Handle> = OnceLock::new();

pub(crate) const ICON_PX: u32 = 64;

/// Create the tray icon. Call once, on the main thread, after the window exists. True if there is a tray.
pub fn init(shell: Shell, controller: Handle) -> bool {
    let _ = CONTROLLER.set(controller.clone());
    let ok = imp::init(shell, controller);
    EXISTS.store(ok, Ordering::Relaxed);
    if ok {
        refresh();
    }
    ok
}

pub fn exists() -> bool {
    EXISTS.load(Ordering::Relaxed)
}

/// Bring the icon and the menu label in line with the controller's current state. Cheap; skips work when
/// nothing changed. On Windows and macOS only the main thread can touch the icon, so calls from other
/// threads do nothing there (the main thread calls it too).
pub fn refresh() {
    if !exists() {
        return;
    }
    if let Some(controller) = CONTROLLER.get() {
        let view = controller.snapshot().engine.view(Local::now());
        imp::apply(view.online, view.toggle_on);
    }
}

/// Take the icon away (main thread; elsewhere it does nothing).
pub fn remove() {
    imp::remove();
}

/// 64 x 64 RGBA icon data, built into the program.
pub(crate) fn icon_bytes(kind: &str, on: bool) -> &'static [u8] {
    macro_rules! icon {
        ($k:literal) => {
            if on {
                include_bytes!(concat!("../../assets/tray_", $k, "_on_64.rgba")) as &[u8]
            } else {
                include_bytes!(concat!("../../assets/tray_", $k, "_off_64.rgba")) as &[u8]
            }
        };
    }
    match kind {
        "light" => icon!("light"),
        "dark" => icon!("dark"),
        "template" => icon!("template"),
        _ => icon!("mid"),
    }
}
