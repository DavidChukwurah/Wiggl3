//! One copy only.
//!
//! The first copy holds an exclusive lock on a file in the config directory for as long as it runs (the OS
//! drops the lock if the app crashes, so a crash never blocks the next start). A second copy fails to get
//! the lock, writes a fresh token into a "show" file next to it and exits; a small watcher thread in the
//! first copy notices the new token and brings the window forward.
//!
//! The watcher is its own thread, not part of the window's update loop, because a hidden window does not
//! always get redrawn (Windows). No sockets, no network.

use crate::settings::config_dir;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Instance {
    _lock: Option<File>, // held until the process exits
    show_file: Option<PathBuf>,
}

pub enum Start {
    /// We are the first copy.
    First(Instance),
    /// Another copy is already running and has been asked to show its window.
    Second,
}

pub fn acquire() -> Start {
    let Some(dir) = config_dir() else {
        // No config directory: nothing to coordinate through, so just run.
        return Start::First(Instance { _lock: None, show_file: None });
    };
    let _ = std::fs::create_dir_all(&dir);
    let show_file = dir.join("show");
    let opened = OpenOptions::new().create(true).write(true).truncate(false).open(dir.join("instance.lock"));
    let Ok(file) = opened else {
        return Start::First(Instance { _lock: None, show_file: Some(show_file) });
    };
    match file.try_lock() {
        Ok(()) => Start::First(Instance { _lock: Some(file), show_file: Some(show_file) }),
        Err(TryLockError::WouldBlock) => {
            allow_foreground();
            let _ = std::fs::write(&show_file, token());
            Start::Second
        }
        // Locking is unsupported here or failed for another reason: never stop the user from running.
        Err(TryLockError::Error(_)) => Start::First(Instance { _lock: Some(file), show_file: Some(show_file) }),
    }
}

/// A value that differs every time it is called (process id plus the clock in nanoseconds).
fn token() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{} {}", std::process::id(), nanos)
}

/// Windows only lets the foreground process raise a window; the second copy is the one the user just
/// launched, so it hands that right to the first copy.
fn allow_foreground() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};
        AllowSetForegroundWindow(ASFW_ANY);
    }
}

impl Instance {
    /// Start the watcher: `on_show` runs (on the watcher thread) each time a second copy asks us to show up.
    pub fn watch(&self, on_show: impl Fn() + Send + 'static) {
        let Some(path) = self.show_file.clone() else { return };
        let mut seen = std::fs::read_to_string(&path).ok(); // anything left over from an earlier run is old news
        let _ = std::thread::Builder::new().name("wiggl3-single".into()).spawn(move || loop {
            std::thread::sleep(Duration::from_millis(400));
            let now = std::fs::read_to_string(&path).ok();
            if now.is_some() && now != seen {
                seen = now;
                on_show();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_differ_between_calls() {
        let a = token();
        std::thread::sleep(Duration::from_millis(2));
        assert_ne!(a, token());
    }
}
