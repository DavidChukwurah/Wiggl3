//! Windows: `GetLastInputInfo` tells us when the user was last active; `SendInput` with absolute,
//! virtual-desktop coordinates glides the cursor across every monitor and that input resets the
//! idle timer Teams, Slack and Office read. `SetThreadExecutionState` keeps the machine and display
//! awake while Online.
//! NOTE: the execution state is per-thread, so `KeepAwake::set` must always be called from the
//! same thread (the controller thread does this).

use super::{glide, next_target, Backend, Bounds, KeepAwake, Nudger};
use std::mem::size_of;
use std::time::Duration;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetLastInputInfo, SendInput, INPUT, INPUT_0, INPUT_MOUSE, LASTINPUTINFO, MOUSEEVENTF_ABSOLUTE,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN,
};

pub fn backend() -> Backend {
    Backend { nudger: Box::new(WinNudger { leg: 0 }), awake: Box::new(WinAwake) }
}

fn virtual_screen() -> Bounds {
    unsafe {
        Bounds {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN) as f64,
            y: GetSystemMetrics(SM_YVIRTUALSCREEN) as f64,
            w: GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1) as f64,
            h: GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1) as f64,
        }
    }
}

/// Move the cursor to a pixel position on the virtual desktop (all monitors).
fn move_to(b: Bounds, x: f64, y: f64) -> Result<(), String> {
    // Absolute mouse input is normalised to 0..=65535 across the whole virtual desktop.
    let nx = (((x - b.x) / (b.w - 1.0).max(1.0)) * 65535.0).round().clamp(0.0, 65535.0) as i32;
    let ny = (((y - b.y) / (b.h - 1.0).max(1.0)) * 65535.0).round().clamp(0.0, 65535.0) as i32;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: nx,
                dy: ny,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let sent = unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
    if sent == 1 {
        Ok(())
    } else {
        // 0 usually means another process (UIPI) or a secure desktop is blocking input.
        Err("Windows blocked the cursor movement.".into())
    }
}

struct WinNudger {
    leg: u32,
}

impl Nudger for WinNudger {
    fn idle_for(&mut self) -> Option<Duration> {
        let mut info = LASTINPUTINFO { cbSize: size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        if unsafe { GetLastInputInfo(&mut info) } == 0 {
            return None;
        }
        let now = unsafe { GetTickCount() };
        Some(Duration::from_millis(now.wrapping_sub(info.dwTime) as u64))
    }

    fn sweep(&mut self) -> Result<(), String> {
        let b = virtual_screen();
        let mut p = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut p) } == 0 {
            return Err("Could not read the cursor position.".into());
        }
        let target = next_target(&mut self.leg, b);
        glide((p.x as f64, p.y as f64), target, |x, y| move_to(b, x, y))
    }
}

struct WinAwake;

impl KeepAwake for WinAwake {
    fn set(&mut self, on: bool) -> Result<(), String> {
        let flags = if on {
            ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
        } else {
            ES_CONTINUOUS
        };
        if unsafe { SetThreadExecutionState(flags) } == 0 {
            return Err("Could not keep Windows awake.".into());
        }
        Ok(())
    }
}
