//! Pure state machine for wiggl3. No I/O, no threads, no clock reads: every method takes `now`,
//! so the behaviour in the brief's state table (section 6) is unit-testable.

use crate::settings::{DurationChoice, Settings};
use chrono::{DateTime, Duration, Local};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

/// Which field of a custom range is wrong (drives the red outline; no error text is shown).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeFault {
    Start,
    End,
}

/// Validate a custom range per section 5: end must be present, after start, and not in the past.
pub fn validate_range(
    start: Option<DateTime<Local>>,
    end: Option<DateTime<Local>>,
    now: DateTime<Local>,
) -> Result<Range, RangeFault> {
    let start = start.ok_or(RangeFault::Start)?;
    let end = end.ok_or(RangeFault::End)?;
    if end <= start || end <= now {
        return Err(RangeFault::End);
    }
    Ok(Range { start, end })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Running { ends: Option<DateTime<Local>> },
    Scheduled { starts: DateTime<Local>, ends: DateTime<Local> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Start with the current Duration choice (ignored for Custom: use `SetRange`).
    Start,
    Stop,
    SetInterval(u64),
    SetDuration(DurationChoice),
    SetRange(Range),
    /// Tray: turn off, remembering what was running so Resume can pick it up again.
    Pause,
    Resume,
}

/// Things worth telling the user about (shown as desktop notifications).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Started,
    Stopped,
    /// A timed run or custom range reached its end by itself.
    Finished,
}

/// What the window should show, straight from the table in section 6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub online: bool,
    pub toggle_on: bool,
    pub footer_right: String,
    /// A custom range is currently scheduled or running (button reads "Update").
    pub range_active: bool,
}

#[derive(Clone, Debug)]
pub struct Engine {
    settings: Settings,
    phase: Phase,
    range: Option<Range>,
    /// The most recent custom range, so Resume can restart it after a Pause.
    last_range: Option<Range>,
}

impl Engine {
    pub fn new(settings: Settings) -> Self {
        Self { settings, phase: Phase::Idle, range: None, last_range: None }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn range(&self) -> Option<Range> {
        self.range
    }

    pub fn is_running(&self) -> bool {
        matches!(self.phase, Phase::Running { .. })
    }

    pub fn apply(&mut self, cmd: Command, now: DateTime<Local>) -> Vec<Event> {
        let was_running = self.is_running();
        match cmd {
            Command::Start => self.start(now),
            Command::Stop | Command::Pause => {
                self.phase = Phase::Idle;
                self.range = None;
            }
            Command::Resume => self.resume(now),
            Command::SetInterval(secs) => self.settings.interval_secs = secs,
            Command::SetDuration(d) => self.set_duration(d, now),
            Command::SetRange(r) => self.set_range(r, now),
        }
        self.transition_events(was_running, false)
    }

    /// Advance time-driven transitions (scheduled start, timed end).
    pub fn tick(&mut self, now: DateTime<Local>) -> Vec<Event> {
        let was_running = self.is_running();
        let mut finished = false;
        match self.phase {
            Phase::Scheduled { starts, ends } => {
                if now >= ends {
                    // e.g. the computer slept through the whole range
                    self.phase = Phase::Idle;
                    self.range = None;
                    finished = true;
                } else if now >= starts {
                    self.phase = Phase::Running { ends: Some(ends) };
                }
            }
            Phase::Running { ends: Some(ends) } if now >= ends => {
                self.phase = Phase::Idle;
                self.range = None;
                finished = true;
            }
            _ => {}
        }
        self.transition_events(was_running, finished)
    }

    /// The next moment `tick` has something to do.
    pub fn next_deadline(&self) -> Option<DateTime<Local>> {
        match self.phase {
            Phase::Scheduled { starts, .. } => Some(starts),
            Phase::Running { ends } => ends,
            Phase::Idle => None,
        }
    }

    pub fn view(&self, now: DateTime<Local>) -> View {
        let (online, toggle_on, footer_right) = match self.phase {
            Phase::Idle => (false, false, String::new()),
            Phase::Running { ends: None } => (true, true, "No end time".to_string()),
            Phase::Running { ends: Some(end) } => (true, true, countdown(end - now)),
            Phase::Scheduled { starts, .. } => {
                let fmt = if starts.date_naive() == now.date_naive() { "%H:%M" } else { "%b %-d %H:%M" };
                (false, true, format!("Starts {}", starts.format(fmt)))
            }
        };
        View { online, toggle_on, footer_right, range_active: self.range.is_some() }
    }

    fn start(&mut self, now: DateTime<Local>) {
        if self.phase != Phase::Idle {
            return;
        }
        self.phase = match self.settings.duration {
            DurationChoice::Forever => Phase::Running { ends: None },
            DurationChoice::Custom => return, // needs a validated range
            d => Phase::Running { ends: Some(now + Duration::hours(d.hours().unwrap_or(1))) },
        };
    }

    fn resume(&mut self, now: DateTime<Local>) {
        if self.phase != Phase::Idle {
            return;
        }
        match (self.settings.duration, self.last_range) {
            (DurationChoice::Custom, Some(r)) if r.end > now => self.set_range(r, now),
            _ => self.start(now),
        }
    }

    fn set_duration(&mut self, d: DurationChoice, now: DateTime<Local>) {
        self.settings.duration = d;
        match (self.phase, d) {
            (Phase::Idle, _) | (_, DurationChoice::Custom) => {}
            // Spec is silent: a fixed duration chosen mid-run restarts the countdown from now.
            (Phase::Running { .. }, DurationChoice::Forever) => {
                self.phase = Phase::Running { ends: None };
                self.range = None;
            }
            (Phase::Running { .. }, fixed) => {
                let h = fixed.hours().unwrap_or(1);
                self.phase = Phase::Running { ends: Some(now + Duration::hours(h)) };
                self.range = None;
            }
            // Leaving Custom while only scheduled cancels the schedule.
            (Phase::Scheduled { .. }, _) => {
                self.phase = Phase::Idle;
                self.range = None;
            }
        }
    }

    fn set_range(&mut self, r: Range, now: DateTime<Local>) {
        if r.end <= r.start || r.end <= now {
            return;
        }
        self.settings.duration = DurationChoice::Custom;
        self.range = Some(r);
        self.last_range = Some(r);
        self.phase = if r.start <= now {
            Phase::Running { ends: Some(r.end) }
        } else {
            Phase::Scheduled { starts: r.start, ends: r.end }
        };
    }

    fn transition_events(&self, was_running: bool, finished: bool) -> Vec<Event> {
        match (was_running, self.is_running()) {
            (false, true) => vec![Event::Started],
            (true, false) if finished => vec![Event::Finished],
            (true, false) => vec![Event::Stopped],
            (false, false) if finished => vec![Event::Finished],
            _ => vec![],
        }
    }
}

/// "3h 12m left" / "12m left". Minutes round up so the last minute never shows "0m".
pub fn countdown(remaining: Duration) -> String {
    let secs = remaining.num_seconds().max(0);
    let mins = (secs + 59) / 60;
    if mins >= 60 {
        format!("{}h {:02}m left", mins / 60, mins % 60)
    } else {
        format!("{}m left", mins.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(h: u32, m: u32, s: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 10, 16, h, m, s).unwrap()
    }

    fn engine(duration: DurationChoice) -> Engine {
        Engine::new(Settings { duration, ..Settings::default() })
    }

    #[test]
    fn idle_view_matches_table() {
        let v = engine(DurationChoice::Forever).view(t(9, 0, 0));
        assert_eq!(v, View { online: false, toggle_on: false, footer_right: String::new(), range_active: false });
    }

    #[test]
    fn running_no_end() {
        let mut e = engine(DurationChoice::Forever);
        assert_eq!(e.apply(Command::Start, t(9, 0, 0)), vec![Event::Started]);
        let v = e.view(t(9, 0, 1));
        assert!(v.online && v.toggle_on);
        assert_eq!(v.footer_right, "No end time");
        assert_eq!(e.next_deadline(), None);
    }

    #[test]
    fn running_timed_counts_down_and_finishes() {
        let mut e = engine(DurationChoice::Hours4);
        e.apply(Command::Start, t(9, 0, 0));
        assert_eq!(e.view(t(9, 48, 0)).footer_right, "3h 12m left");
        assert_eq!(e.tick(t(12, 59, 59)), vec![]);
        assert_eq!(e.tick(t(13, 0, 0)), vec![Event::Finished]);
        assert_eq!(e.view(t(13, 0, 0)).online, false);
        assert!(!e.is_running());
    }

    #[test]
    fn manual_stop_is_stopped_not_finished() {
        let mut e = engine(DurationChoice::Hours1);
        e.apply(Command::Start, t(9, 0, 0));
        assert_eq!(e.apply(Command::Stop, t(9, 5, 0)), vec![Event::Stopped]);
    }

    #[test]
    fn custom_without_range_does_not_start() {
        let mut e = engine(DurationChoice::Custom);
        assert_eq!(e.apply(Command::Start, t(9, 0, 0)), vec![]);
        assert_eq!(e.phase(), Phase::Idle);
    }

    #[test]
    fn future_range_is_scheduled_then_starts_then_finishes() {
        let mut e = engine(DurationChoice::Custom);
        let r = Range { start: t(14, 30, 0), end: t(18, 0, 0) };
        assert_eq!(e.apply(Command::SetRange(r), t(9, 0, 0)), vec![]);
        let v = e.view(t(9, 0, 0));
        assert!(!v.online && v.toggle_on && v.range_active);
        assert_eq!(v.footer_right, "Starts 14:30");
        assert_eq!(e.next_deadline(), Some(t(14, 30, 0)));
        assert_eq!(e.tick(t(14, 30, 0)), vec![Event::Started]);
        assert_eq!(e.view(t(15, 0, 0)).footer_right, "3h 00m left");
        assert_eq!(e.tick(t(18, 0, 0)), vec![Event::Finished]);
        assert!(!e.view(t(18, 0, 0)).range_active);
    }

    #[test]
    fn range_starting_now_runs_immediately() {
        let mut e = engine(DurationChoice::Custom);
        let r = Range { start: t(9, 0, 0), end: t(10, 0, 0) };
        assert_eq!(e.apply(Command::SetRange(r), t(9, 0, 0)), vec![Event::Started]);
    }

    #[test]
    fn slept_through_whole_range_finishes_without_running() {
        let mut e = engine(DurationChoice::Custom);
        e.apply(Command::SetRange(Range { start: t(14, 0, 0), end: t(15, 0, 0) }), t(9, 0, 0));
        assert_eq!(e.tick(t(16, 0, 0)), vec![Event::Finished]);
        assert_eq!(e.phase(), Phase::Idle);
    }

    #[test]
    fn invalid_ranges_are_rejected_by_validation_and_engine() {
        let now = t(9, 0, 0);
        assert_eq!(validate_range(Some(t(9, 0, 0)), None, now), Err(RangeFault::End));
        assert_eq!(validate_range(Some(t(10, 0, 0)), Some(t(10, 0, 0)), now), Err(RangeFault::End));
        assert_eq!(validate_range(Some(t(10, 0, 0)), Some(t(9, 30, 0)), now), Err(RangeFault::End));
        assert_eq!(validate_range(Some(t(7, 0, 0)), Some(t(8, 0, 0)), now), Err(RangeFault::End));
        assert_eq!(validate_range(None, Some(t(11, 0, 0)), now), Err(RangeFault::Start));
        assert!(validate_range(Some(t(9, 0, 0)), Some(t(11, 0, 0)), now).is_ok());

        let mut e = engine(DurationChoice::Custom);
        e.apply(Command::SetRange(Range { start: t(7, 0, 0), end: t(8, 0, 0) }), now);
        assert_eq!(e.phase(), Phase::Idle);
    }

    #[test]
    fn interval_change_is_just_a_setting() {
        let mut e = engine(DurationChoice::Forever);
        e.apply(Command::Start, t(9, 0, 0));
        assert_eq!(e.apply(Command::SetInterval(15), t(9, 0, 1)), vec![]);
        assert_eq!(e.settings().interval_secs, 15);
        assert!(e.is_running());
    }

    #[test]
    fn changing_duration_mid_run_rebases_countdown() {
        let mut e = engine(DurationChoice::Forever);
        e.apply(Command::Start, t(9, 0, 0));
        e.apply(Command::SetDuration(DurationChoice::Hours2), t(9, 30, 0));
        assert_eq!(e.view(t(9, 30, 0)).footer_right, "2h 00m left");
        e.apply(Command::SetDuration(DurationChoice::Forever), t(9, 31, 0));
        assert_eq!(e.view(t(9, 31, 0)).footer_right, "No end time");
    }

    #[test]
    fn leaving_custom_cancels_a_pending_schedule() {
        let mut e = engine(DurationChoice::Custom);
        e.apply(Command::SetRange(Range { start: t(14, 0, 0), end: t(15, 0, 0) }), t(9, 0, 0));
        e.apply(Command::SetDuration(DurationChoice::Forever), t(9, 1, 0));
        assert_eq!(e.phase(), Phase::Idle);
        assert!(e.range().is_none());
    }

    #[test]
    fn pause_and_resume_a_timed_run_restarts_the_countdown() {
        let mut e = engine(DurationChoice::Hours2);
        e.apply(Command::Start, t(9, 0, 0));
        assert_eq!(e.apply(Command::Pause, t(9, 30, 0)), vec![Event::Stopped]);
        assert!(!e.is_running());
        assert_eq!(e.apply(Command::Resume, t(10, 0, 0)), vec![Event::Started]);
        assert_eq!(e.view(t(10, 0, 0)).footer_right, "2h 00m left");
    }

    #[test]
    fn resume_restores_a_custom_range_that_is_still_in_the_future() {
        let mut e = engine(DurationChoice::Custom);
        let r = Range { start: t(9, 0, 0), end: t(12, 0, 0) };
        e.apply(Command::SetRange(r), t(9, 0, 0));
        e.apply(Command::Pause, t(10, 0, 0));
        assert_eq!(e.apply(Command::Resume, t(10, 30, 0)), vec![Event::Started]);
        assert_eq!(e.view(t(10, 30, 0)).footer_right, "1h 30m left");
        // once the range is over, Resume has nothing to restore and does not start
        e.apply(Command::Pause, t(11, 0, 0));
        assert_eq!(e.apply(Command::Resume, t(13, 0, 0)), vec![]);
        assert!(!e.is_running());
    }

    #[test]
    fn countdown_formatting() {
        assert_eq!(countdown(Duration::seconds(3 * 3600 + 12 * 60)), "3h 12m left");
        assert_eq!(countdown(Duration::seconds(3600)), "1h 00m left");
        assert_eq!(countdown(Duration::seconds(59 * 60)), "59m left");
        assert_eq!(countdown(Duration::seconds(1)), "1m left");
        assert_eq!(countdown(Duration::seconds(-5)), "1m left");
    }
}
