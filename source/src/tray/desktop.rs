//! Windows and macOS tray icon through `tray-icon`. It must be created and touched on the main thread.
//! Clicks arrive through event handlers (called on the main thread by the system's own message loop), so
//! they work even while the window is hidden and not being redrawn.

use super::{icon_bytes, Handle, Shell, ICON_PX};
use std::cell::RefCell;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

struct State {
    icon: TrayIcon,
    pause: MenuItem,
    shown: Option<(bool, bool, &'static str)>,
    #[cfg(windows)]
    theme_checked: Option<std::time::Instant>,
    #[cfg(windows)]
    light_taskbar: bool,
}

thread_local! {
    static TRAY: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn icon(rgba: &[u8]) -> Option<Icon> {
    Icon::from_rgba(rgba.to_vec(), ICON_PX, ICON_PX).ok()
}

/// Which icon set to use: macOS recolours a "template" image itself; Windows has a light or a dark taskbar.
fn kind(_light_taskbar: bool) -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "template"
    }
    #[cfg(windows)]
    {
        if _light_taskbar {
            "dark"
        } else {
            "light"
        }
    }
}

pub fn init(shell: Shell, controller: Handle) -> bool {
    #[cfg(windows)]
    let light = windows_theme::taskbar_is_light();
    #[cfg(not(windows))]
    let light = false;

    let menu = Menu::new();
    let open = MenuItem::new("Open", true, None);
    let pause = MenuItem::new("Pause", true, None);
    let quit = MenuItem::new("Quit", true, None);
    if menu.append_items(&[&open, &pause, &PredefinedMenuItem::separator(), &quit]).is_err() {
        return false;
    }
    let Some(first) = icon(icon_bytes(kind(light), false)) else { return false };

    let builder = TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip("wiggl3");
    // macOS draws a "template" icon in the right colour for a light or a dark menu bar.
    #[cfg(target_os = "macos")]
    let builder = builder.with_icon_templated(first);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.with_icon(first);
    #[cfg(windows)]
    let builder = builder.with_menu_on_left_click(false); // left click opens the window, right click the menu
    let Ok(tray) = builder.build() else { return false };

    let (open_id, pause_id, quit_id) = (open.id().clone(), pause.id().clone(), quit.id().clone());
    let click_shell = shell.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == open_id {
            shell.show();
        } else if event.id == pause_id {
            controller.toggle_pause();
            // The controller thread refreshes the tray when it has changed state.
        } else if event.id == quit_id {
            crate::shell::quit_now();
        }
    }));
    #[cfg(windows)]
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event: tray_icon::TrayIconEvent| {
        if let tray_icon::TrayIconEvent::Click {
            button: tray_icon::MouseButton::Left,
            button_state: tray_icon::MouseButtonState::Up,
            ..
        } = event
        {
            click_shell.show();
        }
    }));
    #[cfg(not(windows))]
    drop(click_shell);

    TRAY.with(|t| {
        *t.borrow_mut() = Some(State {
            icon: tray,
            pause,
            shown: None,
            #[cfg(windows)]
            theme_checked: Some(std::time::Instant::now()),
            #[cfg(windows)]
            light_taskbar: light,
        });
    });
    true
}

/// `online`: actually moving the cursor. `toggle_on`: running or scheduled (so the menu offers Pause).
pub fn apply(online: bool, toggle_on: bool) {
    TRAY.with(|t| {
        let mut guard = t.borrow_mut();
        let Some(state) = guard.as_mut() else { return };

        #[cfg(windows)]
        {
            // Follow a change between a light and a dark taskbar, checking every few seconds.
            if state.theme_checked.is_none_or(|at| at.elapsed() > std::time::Duration::from_secs(5)) {
                state.theme_checked = Some(std::time::Instant::now());
                let light = windows_theme::taskbar_is_light();
                if light != state.light_taskbar {
                    state.light_taskbar = light;
                    state.shown = None;
                }
            }
        }
        #[cfg(windows)]
        let k = kind(state.light_taskbar);
        #[cfg(not(windows))]
        let k = kind(false);

        if state.shown == Some((online, toggle_on, k)) {
            return;
        }
        state.shown = Some((online, toggle_on, k));
        if let Some(i) = icon(icon_bytes(k, online)) {
            #[cfg(target_os = "macos")]
            let _ = state.icon.set_icon_templated(Some(i));
            #[cfg(not(target_os = "macos"))]
            let _ = state.icon.set_icon(Some(i));
        }
        state.pause.set_text(if toggle_on { "Pause" } else { "Resume" });
        let _ = state.icon.set_tooltip(Some(if online { "wiggl3 - on" } else { "wiggl3 - off" }));
    });
}

pub fn remove() {
    TRAY.with(|t| {
        t.borrow_mut().take(); // dropping the TrayIcon removes it from the tray
    });
}

#[cfg(windows)]
mod windows_theme {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// True when the taskbar (and so the tray) is light. Dark is the default when the setting can't be read.
    pub fn taskbar_is_light() -> bool {
        let sub = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        let name = wide("SystemUsesLightTheme");
        let mut value: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                sub.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut value as *mut u32).cast(),
                &mut size,
            )
        };
        status == 0 && value == 1
    }
}
