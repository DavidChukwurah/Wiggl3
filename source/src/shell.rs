//! Showing and hiding the window from anywhere: the close button, the tray menu, a second copy of the app.
//!
//! Why this is its own module: a hidden window is not redrawn on every system, so these actions must not
//! depend on the window's own update loop.
//!   * Windows: a hidden window gets no paint messages, so show and hide go straight to Win32 `ShowWindow`.
//!   * macOS and X11: the ordinary egui viewport commands work, and the window keeps updating while hidden.
//!   * Wayland (native): the protocol does not allow an application to hide or restore its own window, so
//!     `can_hide` is false and closing the window quits instead. See `ui::prefer_x11_window`.

use eframe::egui;
use std::sync::atomic::AtomicIsize;
#[cfg(windows)]
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Clone)]
pub struct Shell(Arc<Inner>);

struct Inner {
    ctx: egui::Context,
    /// Windows window handle (HWND); 0 everywhere else.
    #[cfg_attr(not(windows), allow(dead_code))]
    native: AtomicIsize,
    can_hide: bool,
}

impl Shell {
    pub fn new(ctx: egui::Context, native_handle: isize, can_hide: bool) -> Self {
        Self(Arc::new(Inner {
            ctx,
            native: AtomicIsize::new(native_handle),
            can_hide,
        }))
    }

    /// Whether this system lets the app hide its own window (see the module notes).
    pub fn can_hide(&self) -> bool {
        self.0.can_hide
    }

    pub fn show(&self) {
        #[cfg(windows)]
        native::show(self.0.native.load(Ordering::Relaxed));
        #[cfg(not(windows))]
        self.0.ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        // Keeps the window library's own idea of the window state in step (it believes the window is
        // minimised after a minimise click, and would otherwise ignore the next one).
        self.0.ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        self.0.ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.0.ctx.request_repaint();
    }

    pub fn hide(&self) {
        if !self.0.can_hide {
            return;
        }
        #[cfg(windows)]
        native::hide(self.0.native.load(Ordering::Relaxed));
        #[cfg(not(windows))]
        self.0.ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.0.ctx.request_repaint();
    }
}

/// Leave right now: take the tray icon away first (Windows keeps a ghost icon otherwise), then exit. The
/// operating system releases the keep-awake request and the settings are saved whenever they change, so
/// nothing is lost by not unwinding.
pub fn quit_now() -> ! {
    crate::tray::remove();
    std::process::exit(0)
}

#[cfg(windows)]
mod native {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsIconic, SetForegroundWindow, ShowWindow, SW_HIDE, SW_RESTORE, SW_SHOW,
    };

    pub fn show(handle: isize) {
        if handle == 0 {
            return;
        }
        let hwnd = handle as HWND;
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            } else {
                ShowWindow(hwnd, SW_SHOW);
            }
            SetForegroundWindow(hwnd);
        }
    }

    pub fn hide(handle: isize) {
        if handle != 0 {
            unsafe { ShowWindow(handle as HWND, SW_HIDE) };
        }
    }
}
