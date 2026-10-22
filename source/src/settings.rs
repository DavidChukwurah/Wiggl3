//! Persisted user settings: the interval and the duration choice, nothing else.
//! Stored as JSON in the OS config directory (`%APPDATA%\wiggl3`, `~/Library/Application Support/wiggl3`,
//! `~/.config/wiggl3`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const INTERVALS: [(u64, &str); 5] = [
    (15, "15 seconds"),
    (30, "30 seconds"),
    (60, "1 minute"),
    (120, "2 minutes"),
    (240, "4 minutes"),
];
pub const DEFAULT_INTERVAL_SECS: u64 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurationChoice {
    Forever,
    Hours1,
    Hours2,
    Hours4,
    Hours8,
    Custom,
}

impl DurationChoice {
    pub const ALL: [DurationChoice; 6] = [
        Self::Forever,
        Self::Hours1,
        Self::Hours2,
        Self::Hours4,
        Self::Hours8,
        Self::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Forever => "Until I turn it off",
            Self::Hours1 => "1 hour",
            Self::Hours2 => "2 hours",
            Self::Hours4 => "4 hours",
            Self::Hours8 => "8 hours",
            Self::Custom => "Custom range",
        }
    }

    /// Length in hours for the fixed-length choices.
    pub fn hours(self) -> Option<i64> {
        match self {
            Self::Hours1 => Some(1),
            Self::Hours2 => Some(2),
            Self::Hours4 => Some(4),
            Self::Hours8 => Some(8),
            Self::Forever | Self::Custom => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub interval_secs: u64,
    pub duration: DurationChoice,
}

impl Default for Settings {
    fn default() -> Self {
        Self { interval_secs: DEFAULT_INTERVAL_SECS, duration: DurationChoice::Forever }
    }
}

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("wiggl3"))
}

impl Settings {
    fn path() -> Option<PathBuf> {
        config_dir().map(|d| d.join("settings.json"))
    }

    /// Load settings; any problem (missing, unreadable, corrupt file) falls back to the defaults.
    /// An interval that is not one of the offered choices is repaired to the default.
    pub fn load() -> Self {
        let mut s: Settings = Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        if !INTERVALS.iter().any(|(secs, _)| *secs == s.interval_secs) {
            s.interval_secs = DEFAULT_INTERVAL_SECS;
        }
        s
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let Ok(json) = serde_json::to_string_pretty(self) else { return };
        // Write to a temporary file first so a crash mid-write can never leave a half-written settings file.
        let tmp = path.with_extension("json.tmp");
        let result = std::fs::write(&tmp, json).and_then(|()| std::fs::rename(&tmp, &path));
        if let Err(e) = result {
            eprintln!("wiggl3: could not save settings: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_one_minute_and_forever() {
        let s = Settings::default();
        assert_eq!(s.interval_secs, 60);
        assert_eq!(s.duration, DurationChoice::Forever);
    }

    #[test]
    fn partial_or_unknown_json_falls_back_per_field() {
        let s: Settings = serde_json::from_str(r#"{"interval_secs": 30}"#).unwrap();
        assert_eq!((s.interval_secs, s.duration), (30, DurationChoice::Forever));
        // a field from an older version is ignored rather than rejected
        let s: Settings = serde_json::from_str(r#"{"interval_secs": 15, "nudge": "Wiggle"}"#).unwrap();
        assert_eq!(s.interval_secs, 15);
    }

    #[test]
    fn every_duration_choice_has_a_label_and_fixed_ones_have_hours() {
        for d in DurationChoice::ALL {
            assert!(!d.label().is_empty());
        }
        assert_eq!(DurationChoice::Hours8.hours(), Some(8));
        assert_eq!(DurationChoice::Forever.hours(), None);
        assert_eq!(DurationChoice::Custom.hours(), None);
    }
}
