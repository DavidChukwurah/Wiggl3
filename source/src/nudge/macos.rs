//! macOS: `CGEventSourceSecondsSinceLastEventType` gives the idle time; synthetic mouse-moved events
//! posted with `CGEventPost` at the HID tap glide the cursor across all displays and reset it.
//! Needs the Accessibility permission (`AXIsProcessTrusted`). Display sleep is held off with an
//! IOPMAssertion. Plain FFI, no extra crates.

use super::{glide, next_target, Backend, Bounds, KeepAwake, Notice, NoticeAction, Nudger};
use std::time::Duration;
use std::ffi::{c_char, c_void, CString};

type CGEventRef = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

const K_CG_HID_EVENT_TAP: u32 = 0;
const K_CG_EVENT_MOUSE_MOVED: u32 = 5;
const K_CG_EVENT_SOURCE_COMBINED_SESSION: i32 = 0;
const K_CG_ANY_INPUT_EVENT_TYPE: u32 = u32::MAX;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
const K_IOPM_ASSERTION_LEVEL_ON: u32 = 255;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreate(source: *const c_void) -> CGEventRef;
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    fn CGEventCreateMouseEvent(source: *const c_void, kind: u32, pos: CGPoint, button: u32) -> CGEventRef;
    fn CGEventSourceSecondsSinceLastEventType(state: i32, kind: u32) -> f64;
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn CGEventPost(tap: u32, event: CGEventRef);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
    fn CFStringCreateWithCString(alloc: *const c_void, s: *const c_char, enc: u32) -> *const c_void;
    fn CFDictionaryCreate(
        alloc: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        count: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> *const c_void;
    static kCFBooleanTrue: *const c_void;
    // Only their addresses are used (they are tables of callbacks owned by CoreFoundation).
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
    static kAXTrustedCheckOptionPrompt: *const c_void;
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOPMAssertionCreateWithName(kind: *const c_void, level: u32, name: *const c_void, id: *mut u32) -> i32;
    fn IOPMAssertionRelease(id: u32) -> i32;
}

pub fn backend() -> Backend {
    Backend { nudger: Box::new(MacNudger { leg: 0, asked: false }), awake: Box::new(MacAwake { id: None }) }
}

pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

fn trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Ask macOS to show its own "wiggl3 would like to control this computer" prompt. That prompt also adds the
/// app to the Accessibility list, so the person only has to switch it on.
fn ask_for_access() {
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks as *const u8 as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const u8 as *const c_void,
        );
        if !options.is_null() {
            AXIsProcessTrustedWithOptions(options);
            CFRelease(options);
        }
    }
}

fn post(event: CGEventRef) -> Result<(), String> {
    if event.is_null() {
        return Err("macOS refused to create the input event.".into());
    }
    unsafe {
        CGEventPost(K_CG_HID_EVENT_TAP, event);
        CFRelease(event as *const c_void);
    }
    Ok(())
}

fn move_to(x: f64, y: f64) -> Result<(), String> {
    post(unsafe { CGEventCreateMouseEvent(std::ptr::null(), K_CG_EVENT_MOUSE_MOVED, CGPoint { x, y }, 0) })
}

/// Bounding box of all connected displays, in global coordinates.
fn all_displays() -> Option<Bounds> {
    let mut ids = [0u32; 16];
    let mut count = 0u32;
    if unsafe { CGGetActiveDisplayList(16, ids.as_mut_ptr(), &mut count) } != 0 || count == 0 {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for id in &ids[..count as usize] {
        let r = unsafe { CGDisplayBounds(*id) };
        x0 = x0.min(r.origin.x);
        y0 = y0.min(r.origin.y);
        x1 = x1.max(r.origin.x + r.size.width);
        y1 = y1.max(r.origin.y + r.size.height);
    }
    Some(Bounds { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

struct MacNudger {
    leg: u32,
    asked: bool,
}

impl Nudger for MacNudger {
    fn idle_for(&mut self) -> Option<Duration> {
        let secs = unsafe {
            CGEventSourceSecondsSinceLastEventType(K_CG_EVENT_SOURCE_COMBINED_SESSION, K_CG_ANY_INPUT_EVENT_TYPE)
        };
        (secs.is_finite() && secs >= 0.0).then(|| Duration::from_secs_f64(secs))
    }

    fn sweep(&mut self) -> Result<(), String> {
        if !trusted() {
            return Err("Accessibility permission is needed.".into());
        }
        let bounds = all_displays().ok_or("Could not read the display layout.")?;
        let here = unsafe {
            let e = CGEventCreate(std::ptr::null());
            let p = CGEventGetLocation(e);
            CFRelease(e as *const c_void);
            p
        };
        let target = next_target(&mut self.leg, bounds);
        glide((here.x, here.y), target, move_to)
    }

    fn on_start(&mut self) {
        // Once per launch, and only when permission is missing: show the system prompt.
        if !self.asked && !trusted() {
            self.asked = true;
            ask_for_access();
        }
    }

    fn notice(&mut self) -> Option<Notice> {
        if trusted() {
            None
        } else {
            Some(Notice {
                text: "wiggl3 needs Accessibility access to move the cursor.".into(),
                action: Some((NoticeAction::OpenAccessibilitySettings, "Open Settings")),
            })
        }
    }
}

struct MacAwake {
    id: Option<u32>,
}

impl KeepAwake for MacAwake {
    fn set(&mut self, on: bool) -> Result<(), String> {
        if on && self.id.is_none() {
            let kind = CString::new("PreventUserIdleDisplaySleep").unwrap();
            let name = CString::new("wiggl3 is keeping the display awake").unwrap();
            let mut id = 0u32;
            let status = unsafe {
                let k = CFStringCreateWithCString(std::ptr::null(), kind.as_ptr(), K_CF_STRING_ENCODING_UTF8);
                let n = CFStringCreateWithCString(std::ptr::null(), name.as_ptr(), K_CF_STRING_ENCODING_UTF8);
                let status = IOPMAssertionCreateWithName(k, K_IOPM_ASSERTION_LEVEL_ON, n, &mut id);
                CFRelease(k);
                CFRelease(n);
                status
            };
            if status != 0 {
                return Err("Could not keep the display awake.".into());
            }
            self.id = Some(id);
        } else if !on {
            if let Some(id) = self.id.take() {
                unsafe { IOPMAssertionRelease(id) };
            }
        }
        Ok(())
    }
}
