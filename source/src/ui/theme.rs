//! Palette and metrics from section 5 of the brief. Neutral greys only: no accent colour. (Red is used
//! solely for the outline of a wrong date or time.)

use eframe::egui::Color32;

// Spacing scale (Fibonacci) and sizes
pub const GAP_S: f32 = 8.0;
pub const GAP_M: f32 = 13.0;
pub const GAP_L: f32 = 21.0;
pub const GAP_XL: f32 = 34.0;
pub const WINDOW_W: f32 = 377.0;
pub const CTRL_H: f32 = 34.0;
pub const R_CTRL: f32 = 8.0;
pub const R_WINDOW: u8 = 13;
pub const TEXT_SMALL: f32 = 11.0;
pub const TEXT_STATUS: f32 = 34.0;
pub const TOGGLE_W: f32 = 34.0;
pub const TOGGLE_H: f32 = 21.0;
pub const KNOB: f32 = 17.0;
pub const LABEL_COL_W: f32 = 34.0;
pub const TIME_W: f32 = 89.0;

pub struct Palette {
    pub window: Color32,
    pub surface: Color32,
    pub panel: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub off_track: Color32,
    pub red: Color32,
    pub on_accent: Color32,
    /// Toggle track when on: the text colour (near-white in dark mode, near-black in light mode).
    pub track_on: Color32,
    /// Toggle knob when on (contrasts with `track_on`).
    pub knob_on: Color32,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        let red = Color32::from_rgb(0xFF, 0x3B, 0x30);
        if dark {
            Self {
                window: Color32::from_rgb(0x1E, 0x1E, 0x20),
                surface: Color32::from_rgb(0x2C, 0x2C, 0x2E),
                panel: Color32::from_rgb(0x17, 0x17, 0x19),
                border: Color32::from_rgb(0x3A, 0x3A, 0x3C),
                text: Color32::from_rgb(0xF5, 0xF5, 0xF7),
                muted: Color32::from_rgb(0x98, 0x98, 0x9D),
                off_track: Color32::from_rgb(0x48, 0x48, 0x4A),
                red,
                on_accent: Color32::from_rgb(0x1D, 0x1D, 0x1F),
                track_on: Color32::from_rgb(0xF5, 0xF5, 0xF7),
                knob_on: Color32::from_rgb(0x1D, 0x1D, 0x1F),
            }
        } else {
            Self {
                window: Color32::from_rgb(0xF2, 0xF2, 0xF4),
                surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                panel: Color32::from_rgb(0xE6, 0xE6, 0xE9),
                border: Color32::from_rgb(0xD2, 0xD2, 0xD7),
                text: Color32::from_rgb(0x1D, 0x1D, 0x1F),
                muted: Color32::from_rgb(0x86, 0x86, 0x8B),
                off_track: Color32::from_rgb(0xC7, 0xC7, 0xCC),
                red,
                on_accent: Color32::from_rgb(0xF5, 0xF5, 0xF7),
                track_on: Color32::from_rgb(0x1D, 0x1D, 0x1F),
                knob_on: Color32::WHITE,
            }
        }
    }
}
