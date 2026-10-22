//! Linux tray through the StatusNotifierItem protocol (`ksni`, pure D-Bus: no GTK). The service runs on its
//! own thread, so menu clicks and icon updates work from any thread and while the window is hidden.

use super::{icon_bytes, Handle, Shell, ICON_PX};
use ksni::blocking::{Handle as KsniHandle, TrayMethods};
use ksni::menu::{MenuItem, StandardItem};
use std::sync::{Mutex, OnceLock};

struct Item {
    online: bool,
    toggle_on: bool,
    shell: Shell,
    controller: Handle,
}

fn argb(rgba: &[u8]) -> ksni::Icon {
    // StatusNotifierItem wants ARGB32 in network byte order.
    let data = rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
    ksni::Icon { width: ICON_PX as i32, height: ICON_PX as i32, data }
}

impl ksni::Tray for Item {
    fn id(&self) -> String {
        "wiggl3".into()
    }
    fn title(&self) -> String {
        "wiggl3".into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        // The panel colour is unknown here, so use the outlined icon that reads on light and dark panels.
        vec![argb(icon_bytes("mid", self.online))]
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        self.shell.show();
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Open".into(),
                activate: Box::new(|t: &mut Self| t.shell.show()),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if self.toggle_on { "Pause" } else { "Resume" }.into(),
                activate: Box::new(|t: &mut Self| t.controller.toggle_pause()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_: &mut Self| crate::shell::quit_now()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

static HANDLE: OnceLock<Mutex<KsniHandle<Item>>> = OnceLock::new();
static SHOWN: Mutex<Option<(bool, bool)>> = Mutex::new(None);

/// False when no tray host is running (for example GNOME without the AppIndicator extension).
pub fn init(shell: Shell, controller: Handle) -> bool {
    let item = Item { online: false, toggle_on: false, shell, controller };
    match item.spawn() {
        Ok(handle) => HANDLE.set(Mutex::new(handle)).is_ok(),
        Err(e) => {
            eprintln!("wiggl3: no tray available: {e}");
            false
        }
    }
}

pub fn apply(online: bool, toggle_on: bool) {
    let Ok(mut shown) = SHOWN.lock() else { return };
    if *shown == Some((online, toggle_on)) {
        return;
    }
    if let Some(handle) = HANDLE.get().and_then(|h| h.lock().ok()) {
        let _ = handle.update(|t| {
            t.online = online;
            t.toggle_on = toggle_on;
        });
        *shown = Some((online, toggle_on));
    }
}

pub fn remove() {}
