//! Linux. Two input paths, chosen by session type (decision D2: portal first, because GNOME is
//! dropping X11 sessions):
//!   * Wayland: `org.freedesktop.portal.RemoteDesktop` (user approves once; a restore token is
//!     kept so later runs don't ask again). Idle time is asked from GNOME (Mutter) or KDE over D-Bus;
//!     where neither answers, it moves every interval.
//!   * X11: XScreenSaver for idle time, XTest fake input for the cursor glide.
//!
//! Sleep/screensaver is inhibited through `org.freedesktop.portal.Inhibit`.
//! If no input path works, the backend says so plainly instead of pretending.

use super::{glide, next_target, Backend, Bounds, KeepAwake, Notice, Nudger};
use crate::settings::config_dir;
use ashpd::desktop::{
    inhibit::{InhibitFlags, InhibitOptions, InhibitProxy},
    remote_desktop::{DeviceType, RemoteDesktop, SelectDevicesOptions},
    PersistMode, Request,
};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::screensaver::ConnectionExt as _;
use x11rb::protocol::xproto::{ConnectionExt as _, Window, MOTION_NOTIFY_EVENT};
use x11rb::protocol::xtest;
use x11rb::rust_connection::RustConnection;

const NO_INPUT_NOTICE: &str =
    "Can't nudge input here; chat apps may show Away.";
/// Wayland can't tell us the screen size, so each sweep is a fixed relative move, right then left.
const PORTAL_SWEEP_PX: i32 = 800;

pub fn backend() -> Backend {
    let wayland = std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v == "wayland")
        || std::env::var_os("WAYLAND_DISPLAY").is_some();
    let nudger: Box<dyn Nudger> = if wayland {
        Box::new(PortalNudger::new())
    } else if std::env::var_os("DISPLAY").is_some() {
        Box::new(X11Nudger::default())
    } else {
        Box::new(NoInput)
    };
    Backend { nudger, awake: Box::new(PortalAwake::new()) }
}

struct NoInput;

impl Nudger for NoInput {
    fn sweep(&mut self) -> Result<(), String> {
        Err(NO_INPUT_NOTICE.into())
    }
    fn notice(&mut self) -> Option<Notice> {
        Some(Notice::text(NO_INPUT_NOTICE))
    }
}

// ---------------------------------------------------------------- X11 (XTest)

#[derive(Default)]
struct X11Nudger {
    conn: Option<X11Conn>,
    leg: u32,
}

struct X11Conn {
    conn: RustConnection,
    root: Window,
}

impl X11Conn {
    fn open() -> Result<Self, String> {
        let (conn, screen) = RustConnection::connect(None).map_err(|e| e.to_string())?;
        let root = conn.setup().roots[screen].root;
        Ok(Self { conn, root })
    }

    fn idle_ms(&self) -> Option<u32> {
        self.conn.screensaver_query_info(self.root).ok()?.reply().ok().map(|r| r.ms_since_user_input)
    }

    /// The root window spans all monitors, so this is the whole desktop.
    fn bounds(&self) -> Result<Bounds, String> {
        let g = self.conn.get_geometry(self.root).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?;
        Ok(Bounds { x: 0.0, y: 0.0, w: g.width as f64, h: g.height as f64 })
    }

    fn pointer(&self) -> Result<(f64, f64), String> {
        let p = self.conn.query_pointer(self.root).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?;
        Ok((p.root_x as f64, p.root_y as f64))
    }

    fn move_to(&self, x: f64, y: f64) -> Result<(), String> {
        // detail = 0 means the coordinates are absolute.
        xtest::fake_input(&self.conn, MOTION_NOTIFY_EVENT, 0, 0, self.root, x as i16, y as i16, 0)
            .map_err(|e| e.to_string())?;
        self.conn.flush().map_err(|e| e.to_string())
    }
}

impl X11Nudger {
    fn conn(&mut self) -> Result<&X11Conn, String> {
        if self.conn.is_none() {
            self.conn = Some(X11Conn::open().map_err(|e| format!("Could not reach the X server: {e}"))?);
        }
        Ok(self.conn.as_ref().expect("connection just opened"))
    }
}

impl Nudger for X11Nudger {
    fn idle_for(&mut self) -> Option<Duration> {
        let ms = self.conn().ok()?.idle_ms()?;
        Some(Duration::from_millis(ms as u64))
    }

    fn sweep(&mut self) -> Result<(), String> {
        let mut leg = self.leg;
        let result = (|| {
            let c = self.conn()?;
            let (bounds, here) = (c.bounds()?, c.pointer()?);
            let target = next_target(&mut leg, bounds);
            glide(here, target, |x, y| c.move_to(x, y))
        })();
        match result {
            Ok(()) => {
                self.leg = leg;
                Ok(())
            }
            Err(e) => {
                self.conn = None; // reconnect next time
                Err(e)
            }
        }
    }
}

// ---------------------------------------------------------------- Wayland (RemoteDesktop portal)

#[derive(Clone, Debug)]
enum PortalState {
    Idle,
    Connecting,
    Ready,
    Failed,
}

struct PortalNudger {
    tx: Option<Sender<()>>,
    state: Arc<Mutex<PortalState>>,
    idle: DbusIdle,
}

impl PortalNudger {
    fn new() -> Self {
        Self { tx: None, state: Arc::new(Mutex::new(PortalState::Idle)), idle: DbusIdle::default() }
    }

    fn set(state: &Mutex<PortalState>, value: PortalState) {
        if let Ok(mut s) = state.lock() {
            *s = value;
        }
    }
}

impl Nudger for PortalNudger {
    fn idle_for(&mut self) -> Option<Duration> {
        self.idle.idle_ms().map(Duration::from_millis)
    }

    fn sweep(&mut self) -> Result<(), String> {
        if matches!(self.state.lock().map(|s| s.clone()), Ok(PortalState::Failed)) {
            return Err(NO_INPUT_NOTICE.into());
        }
        if self.tx.is_none() {
            let (tx, rx) = mpsc::channel();
            let state = self.state.clone();
            std::thread::Builder::new()
                .name("wiggl3-portal".into())
                .spawn(move || portal_worker(rx, state))
                .map_err(|e| e.to_string())?;
            self.tx = Some(tx);
        }
        if let Some(tx) = &self.tx {
            if tx.send(()).is_err() {
                self.tx = None;
                return Err(NO_INPUT_NOTICE.into());
            }
        }
        Ok(())
    }

    fn notice(&mut self) -> Option<Notice> {
        match self.state.lock().map(|s| s.clone()).unwrap_or(PortalState::Idle) {
            PortalState::Connecting => {
                Some(Notice::text("Approve the input-control request so wiggl3 can nudge."))
            }
            PortalState::Failed => Some(Notice::text(NO_INPUT_NOTICE)),
            PortalState::Idle | PortalState::Ready => None,
        }
    }

    fn on_start(&mut self) {
        // Allow another attempt (and another approval prompt) after an earlier failure.
        if matches!(self.state.lock().map(|s| s.clone()), Ok(PortalState::Failed)) {
            self.tx = None;
            Self::set(&self.state, PortalState::Idle);
        }
    }
}

// ---------------------------------------------------------------- Wayland idle time (D-Bus)

/// Wayland has no XScreenSaver. GNOME and KDE both answer "how long since the user last did anything" over
/// D-Bus; ask whichever one is there. Remembers which one works, and gives up for good if neither does
/// (the controller then simply moves the cursor every interval).
#[derive(Default)]
struct DbusIdle {
    conn: Option<zbus::blocking::Connection>,
    source: Option<IdleSource>,
    gave_up: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum IdleSource {
    Mutter,
    Kde,
}

impl DbusIdle {
    fn idle_ms(&mut self) -> Option<u64> {
        if self.gave_up {
            return None;
        }
        if self.conn.is_none() {
            match zbus::blocking::Connection::session() {
                Ok(c) => self.conn = Some(c),
                Err(_) => {
                    self.gave_up = true;
                    return None;
                }
            }
        }
        let conn = self.conn.as_ref()?;
        let order = match self.source {
            Some(known) => vec![known],
            None => vec![IdleSource::Mutter, IdleSource::Kde],
        };
        for source in order {
            if let Some(ms) = Self::ask(conn, source) {
                self.source = Some(source);
                return Some(ms);
            }
        }
        if self.source.is_some() {
            // It worked before and failed now: forget it and probe again next time.
            self.source = None;
        } else {
            self.gave_up = true;
        }
        None
    }

    fn ask(conn: &zbus::blocking::Connection, source: IdleSource) -> Option<u64> {
        match source {
            IdleSource::Mutter => {
                let reply = conn
                    .call_method(
                        Some("org.gnome.Mutter.IdleMonitor"),
                        "/org/gnome/Mutter/IdleMonitor/Core",
                        Some("org.gnome.Mutter.IdleMonitor"),
                        "GetIdletime",
                        &(),
                    )
                    .ok()?;
                reply.body().deserialize::<u64>().ok()
            }
            IdleSource::Kde => {
                let reply = conn
                    .call_method(
                        Some("org.freedesktop.ScreenSaver"),
                        "/org/freedesktop/ScreenSaver",
                        Some("org.freedesktop.ScreenSaver"),
                        "GetSessionIdleTime",
                        &(),
                    )
                    .ok()?;
                reply.body().deserialize::<u32>().ok().map(u64::from)
            }
        }
    }
}

fn token_path() -> Option<std::path::PathBuf> {
    config_dir().map(|d| d.join("portal-token"))
}

fn save_token(token: &str) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let Some(path) = token_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path) {
        let _ = f.write_all(token.as_bytes());
    }
}

async fn open_session() -> Result<(RemoteDesktop, ashpd::desktop::Session<RemoteDesktop>), String> {
    let rd = RemoteDesktop::new().await.map_err(|e| e.to_string())?;
    let session = rd.create_session(Default::default()).await.map_err(|e| e.to_string())?;
    let saved = token_path().and_then(|p| std::fs::read_to_string(p).ok());
    let options = SelectDevicesOptions::default()
        .set_devices(DeviceType::Pointer | DeviceType::Keyboard)
        .set_persist_mode(PersistMode::ExplicitlyRevoked)
        .set_restore_token(saved.as_deref().map(str::trim));
    rd.select_devices(&session, options).await.map_err(|e| e.to_string())?;
    let started = rd
        .start(&session, None, Default::default())
        .await
        .map_err(|e| e.to_string())?
        .response()
        .map_err(|e| e.to_string())?;
    if let Some(token) = started.restore_token() {
        save_token(token);
    }
    Ok((rd, session))
}

fn portal_worker(rx: Receiver<()>, state: Arc<Mutex<PortalState>>) {
    PortalNudger::set(&state, PortalState::Connecting);
    let (rd, session) = match pollster::block_on(open_session()) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("wiggl3: input portal unavailable: {e}");
            PortalNudger::set(&state, PortalState::Failed);
            return;
        }
    };
    PortalNudger::set(&state, PortalState::Ready);
    let mut direction = 1.0_f64;
    for () in rx {
        // Glide right, then on the next sweep left, and so on; the cursor stays where it ends.
        let result = glide((0.0, 0.0), (PORTAL_SWEEP_PX as f64 * direction, 0.0), {
            let mut last_x = 0.0_f64;
            let (rd, session) = (&rd, &session);
            move |x, _| {
                let dx = x - last_x;
                last_x = x;
                if dx.abs() < 0.5 {
                    return Ok(());
                }
                pollster::block_on(rd.notify_pointer_motion(session, dx, 0.0, Default::default()))
                    .map_err(|e| e.to_string())
            }
        });
        direction = -direction;
        if let Err(e) = result {
            eprintln!("wiggl3: portal input failed: {e}");
            PortalNudger::set(&state, PortalState::Failed);
            return;
        }
    }
    let _ = pollster::block_on(session.close());
}

// ---------------------------------------------------------------- keep awake (Inhibit portal)

/// Runs the D-Bus calls on their own thread so a missing or slow portal can't stall the controller.
struct PortalAwake {
    tx: Sender<bool>,
    error: Arc<Mutex<Option<String>>>,
}

impl PortalAwake {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel::<bool>();
        let error = Arc::new(Mutex::new(None));
        let err = error.clone();
        let _ = std::thread::Builder::new().name("wiggl3-inhibit".into()).spawn(move || {
            let mut held: Option<Request<()>> = None;
            for on in rx {
                let outcome: Result<(), String> = pollster::block_on(async {
                    if on && held.is_none() {
                        let proxy = InhibitProxy::new().await.map_err(|e| e.to_string())?;
                        let options = InhibitOptions::default().set_reason("wiggl3 is keeping the computer awake");
                        held = Some(
                            proxy
                                .inhibit(None, InhibitFlags::Idle | InhibitFlags::Suspend, options)
                                .await
                                .map_err(|e| e.to_string())?,
                        );
                    } else if !on {
                        if let Some(req) = held.take() {
                            req.close().await.map_err(|e| e.to_string())?;
                        }
                    }
                    Ok(())
                });
                if let Ok(mut slot) = err.lock() {
                    *slot = outcome.err();
                }
            }
        });
        Self { tx, error }
    }
}

impl KeepAwake for PortalAwake {
    fn set(&mut self, on: bool) -> Result<(), String> {
        let _ = self.tx.send(on);
        // Report the outcome of the previous request, if it failed.
        match self.error.lock().ok().and_then(|mut e| e.take()) {
            Some(e) => Err(format!("Could not keep the computer awake: {e}")),
            None => Ok(()),
        }
    }
}
