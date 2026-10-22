//! Desktop notifications (start, stop, scheduled end). Fire-and-forget on a short-lived thread so a
//! slow notification service can never stall the controller. Local only: no network.

use crate::engine::Event;

pub fn for_event(event: Event) {
    let (title, body) = match event {
        Event::Started => ("wiggl3 is on", "Your cursor will glide across the screens when you go idle."),
        Event::Stopped => ("wiggl3 is off", "It has stopped moving your cursor."),
        Event::Finished => ("wiggl3 is off", "Your set time is up, so it turned itself off."),
    };
    show(title, body);
}

pub fn show(title: &str, body: &str) {
    let (title, body) = (title.to_string(), body.to_string());
    let _ = std::thread::Builder::new().name("wiggl3-notify".into()).spawn(move || {
        let mut n = notify_rust::Notification::new();
        n.summary(&title).body(&body);
        #[cfg(not(target_os = "macos"))]
        n.appname("wiggl3");
        #[cfg(target_os = "macos")]
        let _ = notify_rust::set_application("com.wiggl3.app");
        if let Err(e) = n.show() {
            eprintln!("wiggl3: could not show a notification: {e}");
        }
    });
}
