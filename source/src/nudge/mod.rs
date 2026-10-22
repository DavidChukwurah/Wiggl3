//! OS-level input nudge and keep-awake backends. One backend per OS, chosen at compile time
//! (Linux additionally chooses X11 or Wayland portal at run time).
//!
//! Design rule from the brief: the nudge goes through the real OS input path, no stealth, no
//! network, no telemetry.

use std::time::Duration;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

/// An action the window can offer next to a notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeAction {
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    OpenAccessibilitySettings,
}

/// A single plain line shown above the footer (permission needed, Wayland limits, errors).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub text: String,
    pub action: Option<(NoticeAction, &'static str)>,
}

impl Notice {
    pub fn text(text: impl Into<String>) -> Self {
        Self { text: text.into(), action: None }
    }
}

pub trait Nudger: Send {
    /// How long since the user last touched the keyboard or mouse. `None` when this system can't
    /// tell (e.g. Wayland), in which case the controller moves the cursor every interval.
    fn idle_for(&mut self) -> Option<Duration> {
        None
    }
    /// Glide the cursor across the screens and leave it where it ends up, so the movement is
    /// visible. Blocks for about half a second.
    fn sweep(&mut self) -> Result<(), String>;
    /// A standing limitation or missing permission, if any. Polled by the controller.
    fn notice(&mut self) -> Option<Notice> {
        None
    }
    /// Called each time nudging starts, so a backend can retry after an earlier failure.
    fn on_start(&mut self) {}
}

pub trait KeepAwake: Send {
    /// Turn the "don't sleep, don't blank the display" request on or off.
    fn set(&mut self, on: bool) -> Result<(), String>;
}

pub struct Backend {
    pub nudger: Box<dyn Nudger>,
    pub awake: Box<dyn KeepAwake>,
}

/// Open the OS page that lets the user grant a permission (macOS Accessibility today).
pub fn perform(action: NoticeAction) {
    match action {
        NoticeAction::OpenAccessibilitySettings => {
            #[cfg(target_os = "macos")]
            macos::open_accessibility_settings();
        }
    }
}

pub fn platform_backend() -> Backend {
    #[cfg(target_os = "linux")]
    return linux::backend();
    #[cfg(target_os = "macos")]
    return macos::backend();
    #[cfg(windows)]
    return windows::backend();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    return unsupported();
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn unsupported() -> Backend {
    struct Unsupported;
    impl Nudger for Unsupported {
        fn sweep(&mut self) -> Result<(), String> {
            Err("This operating system is not supported.".into())
        }
    }
    impl KeepAwake for Unsupported {
        fn set(&mut self, _: bool) -> Result<(), String> {
            Ok(())
        }
    }
    Backend { nudger: Box::new(Unsupported), awake: Box::new(Unsupported) }
}

/// The area the cursor may roam: the bounding box of all screens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Each sweep heads for the opposite side of the screens from the last one (left, right, left...),
/// so the cursor visibly travels across every monitor.
pub fn next_target(leg: &mut u32, b: Bounds) -> (f64, f64) {
    let left = leg.is_multiple_of(2);
    *leg = leg.wrapping_add(1);
    let (fx, fy) = if left { (0.15, 0.35) } else { (0.85, 0.65) };
    (b.x + b.w * fx, b.y + b.h * fy)
}

const GLIDE_STEPS: u32 = 40;
const GLIDE_STEP_DELAY: Duration = Duration::from_millis(12);

/// Move from `from` to `to` in small eased steps, calling `move_to` with each point.
pub fn glide(
    from: (f64, f64),
    to: (f64, f64),
    mut move_to: impl FnMut(f64, f64) -> Result<(), String>,
) -> Result<(), String> {
    for i in 1..=GLIDE_STEPS {
        let t = i as f64 / GLIDE_STEPS as f64;
        let e = t * t * (3.0 - 2.0 * t); // smoothstep
        move_to(from.0 + (to.0 - from.0) * e, from.1 + (to.1 - from.1) * e)?;
        if i != GLIDE_STEPS {
            std::thread::sleep(GLIDE_STEP_DELAY);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_alternate_sides_and_stay_inside_bounds() {
        let b = Bounds { x: -1920.0, y: 0.0, w: 3840.0, h: 1080.0 };
        let mut leg = 0;
        let (x1, _) = next_target(&mut leg, b);
        let (x2, _) = next_target(&mut leg, b);
        let (x3, _) = next_target(&mut leg, b);
        assert!(x1 < 0.0 && x2 > 0.0 && x3 < 0.0);
        for x in [x1, x2, x3] {
            assert!(x > b.x && x < b.x + b.w);
        }
    }

    #[test]
    fn glide_ends_exactly_on_target_and_never_overshoots() {
        let mut points = Vec::new();
        glide((100.0, 100.0), (900.0, 500.0), |x, y| {
            points.push((x, y));
            Ok(())
        })
        .unwrap();
        assert_eq!(points.len(), GLIDE_STEPS as usize);
        let last = *points.last().unwrap();
        assert!((last.0 - 900.0).abs() < 1e-9 && (last.1 - 500.0).abs() < 1e-9);
        assert!(points.iter().all(|p| p.0 >= 100.0 && p.0 <= 900.0));
        assert!(points.windows(2).all(|w| w[1].0 >= w[0].0));
    }

    #[test]
    fn glide_stops_on_error() {
        let mut calls = 0;
        let r = glide((0.0, 0.0), (10.0, 10.0), |_, _| {
            calls += 1;
            Err("boom".into())
        });
        assert!(r.is_err());
        assert_eq!(calls, 1);
    }
}
