//! Background controller: owns the engine, the OS backend and all timing, so scheduled starts and timed
//! ends keep working even when the window is hidden or minimised. The window and the tray only send
//! commands and read snapshots.

use crate::alerts;
use crate::engine::{Command, Engine};
use crate::nudge::{self, Notice};
use crate::settings::Settings;
use chrono::Local;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Snapshot {
    pub engine: Engine,
    pub notice: Option<Notice>,
}

enum Msg {
    Cmd(Command),
    Shutdown,
}

/// A cheap, cloneable way to talk to the controller from the window, the tray or another thread.
#[derive(Clone)]
pub struct Handle {
    tx: Sender<Msg>,
    shared: Arc<Mutex<Snapshot>>,
}

impl Handle {
    pub fn send(&self, cmd: Command) {
        let _ = self.tx.send(Msg::Cmd(cmd));
    }

    pub fn snapshot(&self) -> Snapshot {
        match self.shared.lock() {
            Ok(s) => s.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Tray menu: pause when it is on (running or scheduled), resume when it is off.
    pub fn toggle_pause(&self) {
        let on = self.snapshot().engine.view(Local::now()).toggle_on;
        self.send(if on { Command::Pause } else { Command::Resume });
    }
}

/// Owns the controller thread; dropping it stops the thread and releases the keep-awake request.
pub struct Controller {
    handle: Handle,
    join: Option<JoinHandle<()>>,
}

impl Controller {
    /// `on_change` runs on the controller thread after every state change, so the window can repaint and
    /// the tray can update even while the window is hidden.
    pub fn spawn(settings: Settings, on_change: impl Fn() + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Snapshot { engine: Engine::new(settings.clone()), notice: None }));
        let shared_thread = shared.clone();

        let join = std::thread::Builder::new()
            .name("wiggl3-controller".into())
            .spawn(move || {
                // The backend is created on this thread: on Windows the keep-awake state belongs to the thread.
                let mut backend = nudge::platform_backend();
                let mut engine = Engine::new(settings);
                let mut saved = engine.settings().clone();
                // When to next look at the idle time and, if the user is idle, move the cursor.
                let mut next_check: Option<Instant> = None;
                let mut last_error: Option<String> = None;

                loop {
                    let notice = backend.nudger.notice().or_else(|| last_error.clone().map(Notice::text));

                    let mut wait = Duration::from_secs(3600);
                    if let Some(deadline) = engine.next_deadline() {
                        wait = wait.min((deadline - Local::now()).to_std().unwrap_or(Duration::ZERO));
                    }
                    if let Some(at) = next_check {
                        wait = wait.min(at.saturating_duration_since(Instant::now()));
                    }
                    if notice.is_some() {
                        wait = wait.min(Duration::from_secs(3)); // re-check permissions
                    }

                    let was_running = engine.is_running();
                    match rx.recv_timeout(wait) {
                        Ok(Msg::Cmd(cmd)) => {
                            for event in engine.apply(cmd, Local::now()) {
                                alerts::for_event(event);
                            }
                        }
                        Ok(Msg::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                            let _ = backend.awake.set(false);
                            break;
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                    for event in engine.tick(Local::now()) {
                        alerts::for_event(event);
                    }

                    match (was_running, engine.is_running()) {
                        (false, true) => {
                            backend.nudger.on_start();
                            keep_awake(&mut *backend.awake, true);
                            last_error = None;
                            next_check = Some(Instant::now()); // look straight away
                        }
                        (true, false) => {
                            keep_awake(&mut *backend.awake, false);
                            last_error = None;
                            next_check = None;
                        }
                        _ => {}
                    }

                    if engine.is_running() && next_check.is_some_and(|at| Instant::now() >= at) {
                        // Read the interval now, so a change made mid-cycle applies from this point on.
                        let interval = Duration::from_secs(engine.settings().interval_secs);
                        match backend.nudger.idle_for() {
                            // The user is active: their own input already counts. Look again at the
                            // moment they would have been idle for a full interval.
                            Some(idle) if idle + Duration::from_millis(500) < interval => {
                                let wait = (interval - idle).max(Duration::from_secs(1));
                                next_check = Some(Instant::now() + wait);
                            }
                            // Idle (or the system can't tell): move the cursor across the screens.
                            _ => {
                                last_error = backend.nudger.sweep().err();
                                next_check = Some(Instant::now() + interval);
                            }
                        }
                    }

                    if engine.settings() != &saved {
                        saved = engine.settings().clone();
                        saved.save();
                    }

                    let notice = backend.nudger.notice().or_else(|| last_error.clone().map(Notice::text));
                    if let Ok(mut snap) = shared_thread.lock() {
                        *snap = Snapshot { engine: engine.clone(), notice };
                    }
                    on_change();
                }
            })
            .expect("could not start the controller thread");

        Self { handle: Handle { tx, shared }, join: Some(join) }
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }
}

/// Keeping the machine awake is a bonus: the cursor movement itself already counts as activity. A failure
/// is logged but never shown as an error to the user.
fn keep_awake(awake: &mut dyn nudge::KeepAwake, on: bool) {
    if let Err(e) = awake.set(on) {
        eprintln!("wiggl3: {e}");
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        let _ = self.handle.tx.send(Msg::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
