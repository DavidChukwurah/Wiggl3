//! The wiggl3 window, built to section 5 of the brief (377 px wide, Fibonacci spacing,
//! 34 px controls, Jersey 10 status text, no divider above the footer).

mod theme;

use crate::controller::{Controller, Handle, Snapshot};
use crate::engine::{validate_range, Command, Phase, Range};
use crate::nudge::{self, Notice};
use crate::settings::{DurationChoice, INTERVALS};
use crate::single::Instance;
use crate::shell::Shell;
use crate::tray;
use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Id,
    Margin, Popup, PopupCloseBehavior, Rect, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
};
use std::sync::Arc;
use std::time::Duration;
use theme::*;

const JERSEY: &str = "jersey10";

pub fn run(instance: Instance) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("wiggl3")
            .with_app_id("wiggl3")
            .with_icon(app_icon())
            .with_inner_size([WINDOW_W, 360.0])
            // Width is fixed; height follows the content. (`with_resizable(false)` would also pin
            // the height on X11, so the width is pinned through min/max instead.)
            .with_min_inner_size([WINDOW_W, 100.0])
            .with_max_inner_size([WINDOW_W, 1200.0])
            .with_decorations(false)
            .with_transparent(true),
        event_loop_builder: event_loop_hook(),
        ..Default::default()
    };
    eframe::run_native("wiggl3", options, Box::new(move |cc| Ok(Box::new(App::new(cc, instance)))))
}

/// The app icon (rodent glyph on a dark tile): 128 x 128 RGBA, drawn by `tools/make_icon.py` (see assets/).
fn app_icon() -> egui::IconData {
    egui::IconData { rgba: include_bytes!("../../assets/icon_128.rgba").to_vec(), width: 128, height: 128 }
}

/// Linux on Wayland: a native Wayland window cannot be hidden or restored by the app, so when an X11
/// connection (XWayland) is available the window is made through that instead. Everything else about the
/// session (cursor movement through the portal) is unaffected.
#[cfg(target_os = "linux")]
fn prefer_x11_window() -> bool {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
    wayland && x11rb::rust_connection::RustConnection::connect(None).is_ok()
}

#[cfg(target_os = "linux")]
fn event_loop_hook() -> Option<eframe::EventLoopBuilderHook> {
    prefer_x11_window().then(|| {
        Box::new(|builder: &mut eframe::EventLoopBuilder<eframe::UserEvent>| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            builder.with_x11();
        }) as eframe::EventLoopBuilderHook
    })
}

#[cfg(not(target_os = "linux"))]
fn event_loop_hook() -> Option<eframe::EventLoopBuilderHook> {
    None
}

/// Can this window be hidden by the app? Not on a native Wayland window (see `prefer_x11_window`).
fn window_can_hide(_cc: &eframe::CreationContext<'_>) -> bool {
    #[cfg(target_os = "linux")]
    {
        let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
        !wayland || prefer_x11_window()
    }
    #[cfg(not(target_os = "linux"))]
    true
}

/// The Win32 window handle (0 on other systems).
fn native_handle(_cc: &eframe::CreationContext<'_>) -> isize {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(h) = _cc.window_handle() {
            if let RawWindowHandle::Win32(w) = h.as_raw() {
                return w.hwnd.get();
            }
        }
    }
    0
}

#[derive(Default)]
struct RangeForm {
    start_date: String,
    start_time: String,
    end_date: String,
    end_time: String,
    expanded: bool,
    start_fault: bool,
    end_fault: bool,
}

struct App {
    _controller: Controller, // stopped when the app is dropped
    handle: Handle,
    shell: Shell,
    form: RangeForm,
    last_height: f32,
    seen_custom: bool,
    /// Whether closing the window hides it (a tray exists and the system allows hiding) instead of quitting.
    hides_on_close: bool,
    _instance: Instance,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, instance: Instance) -> Self {
        install_fonts(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        let controller = Controller::spawn(crate::settings::Settings::load(), move || {
            tray::refresh(); // Linux updates from here; Windows/macOS ignore other threads
            ctx.request_repaint();
        });
        let handle = controller.handle();
        let shell = Shell::new(cc.egui_ctx.clone(), native_handle(cc), window_can_hide(cc));
        let has_tray = tray::init(shell.clone(), handle.clone());
        let hides_on_close = has_tray && shell.can_hide();
        #[cfg(windows)]
        win_timer::start();
        let watcher_shell = shell.clone();
        instance.watch(move || watcher_shell.show());
        let mut app = Self {
            _controller: controller,
            handle,
            shell,
            form: RangeForm::default(),
            last_height: 0.0,
            seen_custom: false,
            hides_on_close,
            _instance: instance,
        };
        app.form.expanded = true;
        app.prefill_start(Local::now());
        app
    }

    /// The close dot and the window's own close request: hide to the tray if there is one, else quit.
    fn close_or_hide(&mut self, ctx: &egui::Context) {
        if self.hides_on_close {
            self.shell.hide();
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn prefill_start(&mut self, now: DateTime<Local>) {
        self.form.start_date = now.format("%Y-%m-%d").to_string();
        self.form.start_time = now.format("%H:%M").to_string();
    }

    fn parse_dt(date: &str, time: &str) -> Option<DateTime<Local>> {
        let date = date.trim();
        let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok().or_else(|| {
            let year = Local::now().format("%Y");
            NaiveDate::parse_from_str(&format!("{date} {year}"), "%b %d %Y").ok()
        })?;
        let t = NaiveTime::parse_from_str(time.trim(), "%H:%M").ok()?;
        Local.from_local_datetime(&d.and_time(t)).single()
    }

    /// Validate the form; on success start or update the schedule.
    fn submit_range(&mut self) {
        let now = Local::now();
        let start = Self::parse_dt(&self.form.start_date, &self.form.start_time);
        let end = Self::parse_dt(&self.form.end_date, &self.form.end_time);
        match validate_range(start, end, now) {
            Ok(range) => {
                self.form.start_fault = false;
                self.form.end_fault = false;
                self.form.expanded = false;
                self.handle.send(Command::SetRange(range));
            }
            Err(crate::engine::RangeFault::Start) => self.form.start_fault = true,
            Err(crate::engine::RangeFault::End) => self.form.end_fault = true, // red outline only, no text
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let pal = Palette::new(ctx.style().visuals.dark_mode);
        let now = Local::now();
        let snap = self.handle.snapshot();
        let view = snap.engine.view(now);
        let settings = snap.engine.settings().clone();

        // Selecting Custom range pre-fills Start from the system clock (unless a range is active).
        let is_custom = settings.duration == DurationChoice::Custom;
        if is_custom && !self.seen_custom && !view.range_active {
            self.prefill_start(now);
            self.form.expanded = true;
        }
        self.seen_custom = is_custom;

        let frame = Frame::NONE
            .fill(pal.window)
            .corner_radius(CornerRadius::same(R_WINDOW))
            .stroke(Stroke::new(1.0_f32, pal.border));

        let mut commands: Vec<Command> = Vec::new();
        let mut toggled: Option<bool> = None;
        let mut close_clicked = false;
        let tray_present = self.hides_on_close;
        let mut set_range_clicked = false;

        let out = egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let full_w = ui.available_width();
            if title_bar(ui, ctx, &pal) {
                close_clicked = true;
            }

            ui.horizontal(|ui| {
                ui.add_space(GAP_L);
                ui.vertical(|ui| {
                    ui.set_width(full_w - 2.0 * GAP_L);
                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                    // Header row
                    let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), CTRL_H), Sense::hover());
                    ui.painter().text(
                        egui::pos2(row.left(), row.center().y),
                        Align2::LEFT_CENTER,
                        if view.online { "Online" } else { "Offline" },
                        FontId::new(TEXT_STATUS, FontFamily::Name(JERSEY.into())),
                        pal.text,
                    );
                    let toggle_rect = Rect::from_min_size(
                        egui::pos2(row.right() - TOGGLE_W, row.center().y - TOGGLE_H / 2.0),
                        Vec2::new(TOGGLE_W, TOGGLE_H),
                    );
                    if toggle(ui, toggle_rect, view.toggle_on, &pal) {
                        toggled = Some(!view.toggle_on);
                    }

                    ui.add_space(GAP_L);

                    // Interval
                    small_label(ui, "Interval", &pal);
                    ui.add_space(5.0);
                    let current = INTERVALS.iter().position(|(s, _)| *s == settings.interval_secs).unwrap_or(2);
                    let labels: Vec<&str> = INTERVALS.iter().map(|(_, l)| *l).collect();
                    if let Some(i) = dropdown(ui, "interval", &labels, current, &pal) {
                        commands.push(Command::SetInterval(INTERVALS[i].0));
                    }

                    ui.add_space(GAP_M);

                    // Duration
                    small_label(ui, "Duration", &pal);
                    ui.add_space(5.0);
                    let labels: Vec<&str> = DurationChoice::ALL.iter().map(|d| d.label()).collect();
                    let current = DurationChoice::ALL.iter().position(|d| *d == settings.duration).unwrap_or(0);
                    if let Some(i) = dropdown(ui, "duration", &labels, current, &pal) {
                        commands.push(Command::SetDuration(DurationChoice::ALL[i]));
                    }

                    // Custom range panel
                    if is_custom {
                        ui.add_space(GAP_M);
                        set_range_clicked = range_panel(ui, &mut self.form, view.range_active, snap.engine.range(), &pal);
                    }

                    // Notice (permission, Wayland limits, errors): one truncated line
                    if let Some(notice) = &snap.notice {
                        ui.add_space(GAP_M);
                        notice_row(ui, notice, &pal);
                    }

                    // Footer: 21 px above, no divider
                    ui.add_space(GAP_L);
                    let (foot, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 13.0), Sense::hover());
                    let left = if tray_present { "Keeps running when closed" } else { "Closing the window stops it" };
                    let font = FontId::proportional(TEXT_SMALL);
                    ui.painter().text(foot.left_center(), Align2::LEFT_CENTER, left, font.clone(), pal.muted);
                    ui.painter().text(foot.right_center(), Align2::RIGHT_CENTER, &view.footer_right, font, pal.muted);
                    ui.add_space(GAP_L);
                    ui.cursor().top() // bottom of the content: where the window should end
                })
                .inner
            })
            .inner
        });

        // Make the window exactly as tall as its content.
        let wanted = out.inner + 1.0;
        if (wanted - self.last_height).abs() > 0.5 {
            self.last_height = wanted;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(WINDOW_W, wanted)));
        }

        if set_range_clicked {
            self.submit_range();
        }

        // Turning the toggle on with a Custom range means "Set range" (validates, may outline red).
        match toggled {
            Some(true) if is_custom => {
                if !view.range_active {
                    self.submit_range();
                }
            }
            Some(true) => commands.push(Command::Start),
            Some(false) => {
                commands.push(Command::Stop);
                self.form.expanded = true;
            }
            None => {}
        }
        for c in commands {
            self.handle.send(c);
        }

        if close_clicked {
            self.close_or_hide(ctx);
        }
        // The window's own close request (Alt+F4, Cmd+W) hides to the tray instead of quitting.
        if self.hides_on_close && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.shell.hide();
        }
        tray::refresh();

        // macOS keeps drawing a hidden window, so a slow heartbeat keeps the menu bar item current there.
        if cfg!(target_os = "macos") && self.hides_on_close {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
        schedule_repaint(ctx, &snap, now);
    }
}

/// Windows does not redraw a hidden window, so a timer on the main thread keeps the tray icon current.
#[cfg(windows)]
mod win_timer {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetTimer;

    unsafe extern "system" fn tick(_: HWND, _: u32, _: usize, _: u32) {
        crate::tray::refresh();
    }

    pub fn start() {
        unsafe { SetTimer(std::ptr::null_mut(), 0, 500, Some(tick)) };
    }
}

/// Countdown text changes on minute boundaries; otherwise the window stays fully idle.
fn schedule_repaint(ctx: &egui::Context, snap: &Snapshot, now: DateTime<Local>) {
    if let Phase::Running { ends: Some(end) } = snap.engine.phase() {
        let rem = (end - now).num_milliseconds().max(0) as f64 / 1000.0;
        let mins = (rem / 60.0).ceil().max(1.0);
        let until_change = rem - (mins - 1.0) * 60.0;
        let delay = if rem <= 120.0 { 1.0 } else { until_change.max(1.0) };
        ctx.request_repaint_after(Duration::from_secs_f64(delay));
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        JERSEY.into(),
        Arc::new(FontData::from_static(include_bytes!("../../assets/Jersey10-Regular.ttf"))),
    );
    fonts.families.insert(FontFamily::Name(JERSEY.into()), vec![JERSEY.into()]);
    ctx.set_fonts(fonts);
}

fn small_label(ui: &mut Ui, text: &str, pal: &Palette) {
    ui.label(egui::RichText::new(text).size(TEXT_SMALL).color(pal.muted));
}

/// Three neutral 12 px dots, 8 px gap, 13 px padding. They double as window controls for now
/// (first = close or hide to the tray, second = minimise) because there is no native title bar.
/// Returns true when the close dot was clicked.
fn title_bar(ui: &mut Ui, ctx: &egui::Context, pal: &Palette) -> bool {
    let mut close = false;
    let h = 12.0 + 2.0 * GAP_M;
    let (bar, bar_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click_and_drag());
    if bar_resp.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    for i in 0..3 {
        let center = egui::pos2(bar.left() + GAP_M + 6.0 + i as f32 * (12.0 + GAP_S), bar.center().y);
        let dot = Rect::from_center_size(center, Vec2::splat(12.0));
        let resp = ui.interact(dot, Id::new(("dot", i)), Sense::click());
        let resp = match i {
            0 => resp.on_hover_text("Close"),
            1 => resp.on_hover_text("Minimise"),
            _ => resp,
        };
        let color = if resp.hovered() && i < 2 { pal.muted } else { pal.off_track };
        ui.painter().circle_filled(center, 6.0, color);
        if resp.clicked() {
            match i {
                0 => close = true,
                1 => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                _ => {}
            }
        }
    }
    close
}

/// 34 x 21 toggle, 17 px knob, pale (dark mode) or near-black (light mode) when on. Returns true when clicked.
fn toggle(ui: &mut Ui, rect: Rect, on: bool, pal: &Palette) -> bool {
    let resp = ui.interact(rect, Id::new("power_toggle"), Sense::click());
    let t = ui.ctx().animate_bool_with_time(resp.id, on, 0.12);
    let track = pal.off_track.lerp_to_gamma(pal.track_on, t);
    ui.painter().rect_filled(rect, CornerRadius::same((TOGGLE_H / 2.0) as u8), track);
    let inset = (TOGGLE_H - KNOB) / 2.0;
    let x0 = rect.left() + inset + KNOB / 2.0;
    let x1 = rect.right() - inset - KNOB / 2.0;
    ui.painter().circle_filled(egui::pos2(x0 + (x1 - x0) * t, rect.center().y), KNOB / 2.0, Color32::WHITE.lerp_to_gamma(pal.knob_on, t));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn chevron(ui: &Ui, center: egui::Pos2, up: bool, color: Color32) {
    let s = 3.5_f32;
    let (a, b) = if up { (s * 0.5, -s * 0.5) } else { (-s * 0.5, s * 0.5) };
    let stroke = Stroke::new(1.3_f32, color);
    ui.painter().line_segment([egui::pos2(center.x - s, center.y + a), egui::pos2(center.x, center.y + b)], stroke);
    ui.painter().line_segment([egui::pos2(center.x, center.y + b), egui::pos2(center.x + s, center.y + a)], stroke);
}

/// Native-style select without the default arrow: 13 px chevron icon 13 px from the right edge,
/// 13 px left padding, 34 px right padding so text never touches the arrow.
fn dropdown(ui: &mut Ui, id: &str, options: &[&str], selected: usize, pal: &Palette) -> Option<usize> {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), CTRL_H), Sense::click());
    ui.painter().rect_filled(rect, CornerRadius::same(R_CTRL as u8), pal.surface);
    ui.painter().rect_stroke(rect, CornerRadius::same(R_CTRL as u8), Stroke::new(1.0_f32, pal.border), StrokeKind::Inside);

    let text_rect = Rect::from_min_max(
        egui::pos2(rect.left() + GAP_M, rect.top()),
        egui::pos2(rect.right() - GAP_XL, rect.bottom()),
    );
    let galley = ui.painter().layout(
        options.get(selected).copied().unwrap_or("").to_string(),
        FontId::proportional(TEXT_SMALL),
        pal.text,
        text_rect.width(),
    );
    ui.painter()
        .with_clip_rect(text_rect)
        .galley(egui::pos2(text_rect.left(), rect.center().y - galley.size().y / 2.0), galley, pal.text);
    chevron(ui, egui::pos2(rect.right() - GAP_M - 6.5, rect.center().y), false, pal.muted);

    let mut chosen = None;
    let width = rect.width();
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    Popup::menu(&resp)
        .id(Id::new(("dropdown", id)))
        .close_behavior(PopupCloseBehavior::CloseOnClick)
        .width(width)
        .frame(
            Frame::NONE
                .fill(pal.surface)
                .stroke(Stroke::new(1.0_f32, pal.border))
                .corner_radius(CornerRadius::same(R_CTRL as u8))
                .inner_margin(Margin::same(4)),
        )
        .show(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.set_min_width(width - 8.0);
            for (i, label) in options.iter().enumerate() {
                let (r, rr) = ui.allocate_exact_size(Vec2::new(ui.available_width(), CTRL_H), Sense::click());
                if rr.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(6), pal.panel);
                }
                let color = if i == selected { pal.text } else { pal.muted };
                ui.painter().text(
                    egui::pos2(r.left() + GAP_M - 4.0, r.center().y),
                    Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(TEXT_SMALL),
                    color,
                );
                if rr.clicked() {
                    chosen = Some(i);
                }
            }
        });
    chosen
}

/// A 34 px input with its own outline (red when `fault`). Returns true if the text changed.
fn field(ui: &mut Ui, rect: Rect, text: &mut String, hint: &str, fault: bool, pal: &Palette) -> bool {
    let stroke = Stroke::new(1.0_f32, if fault { pal.red } else { pal.border });
    ui.painter().rect_filled(rect, CornerRadius::same(R_CTRL as u8), pal.surface);
    ui.painter().rect_stroke(rect, CornerRadius::same(R_CTRL as u8), stroke, StrokeKind::Inside);
    let edit = TextEdit::singleline(text)
        .font(FontId::proportional(TEXT_SMALL))
        .text_color(pal.text)
        .hint_text(egui::RichText::new(hint).color(pal.muted))
        .frame(false)
        .margin(Margin::symmetric(10, 0))
        .vertical_align(Align::Center)
        .desired_width(rect.width());
    ui.put(rect, edit).changed()
}

/// Returns true when the Set range / Update button was clicked.
fn range_panel(ui: &mut Ui, form: &mut RangeForm, active: bool, range: Option<Range>, pal: &Palette) -> bool {
    let inner_w = ui.available_width() - 2.0 * GAP_M;
    let panel_frame = Frame::NONE
        .fill(pal.panel)
        .corner_radius(CornerRadius::same(R_CTRL as u8))
        .inner_margin(Margin::same(GAP_M as i8));

    let mut set_clicked = false;
    let mut now_clicked = false;
    panel_frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.set_width(inner_w);

        // Collapsed: one line with the range and a chevron.
        if active && !form.expanded {
            if let Some(r) = range {
                let text = format!("{} to {}", r.start.format("%b %-d %H:%M"), r.end.format("%b %-d %H:%M"));
                let (row, resp) = ui.allocate_exact_size(Vec2::new(inner_w, 21.0), Sense::click());
                ui.painter().text(
                    egui::pos2(row.left(), row.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(TEXT_SMALL),
                    pal.text,
                );
                chevron(ui, egui::pos2(row.right() - 6.5, row.center().y), false, pal.muted);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    form.expanded = true;
                }
                return;
            }
        }

        // Expanded: label column 34, date flexible, time 89, 8 px gaps.
        let date_w = inner_w - LABEL_COL_W - TIME_W - 2.0 * GAP_S;
        let mut edited = false;
        for (row_i, label) in ["Starts", "Ends"].iter().enumerate() {
            if row_i == 1 {
                ui.add_space(GAP_S);
            }
            let (row, _) = ui.allocate_exact_size(Vec2::new(inner_w, CTRL_H), Sense::hover());
            ui.painter().text(
                egui::pos2(row.left(), row.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(TEXT_SMALL),
                pal.muted,
            );
            let date_rect = Rect::from_min_size(egui::pos2(row.left() + LABEL_COL_W + GAP_S, row.top()), Vec2::new(date_w, CTRL_H));
            let time_rect = Rect::from_min_size(egui::pos2(date_rect.right() + GAP_S, row.top()), Vec2::new(TIME_W, CTRL_H));
            let (fault, date, time) = if row_i == 0 {
                (form.start_fault, &mut form.start_date, &mut form.start_time)
            } else {
                (form.end_fault, &mut form.end_date, &mut form.end_time)
            };
            edited |= field(ui, date_rect, date, "YYYY-MM-DD", fault, pal);
            edited |= field(ui, time_rect, time, "HH:MM", fault, pal);
        }
        if edited {
            // Editing a field clears the red outline.
            form.start_fault = false;
            form.end_fault = false;
        }

        ui.add_space(GAP_M);
        let (row, _) = ui.allocate_exact_size(Vec2::new(inner_w, CTRL_H), Sense::hover());

        // Button: 34 high, 21 px horizontal padding, label centred both ways.
        let label = if active { "Update" } else { "Set range" };
        let font = FontId::proportional(TEXT_SMALL);
        let text_w = ui.painter().layout_no_wrap(label.into(), font.clone(), pal.on_accent).size().x;
        let btn = Rect::from_min_size(
            egui::pos2(row.right() - text_w - 2.0 * GAP_L, row.top()),
            Vec2::new(text_w + 2.0 * GAP_L, CTRL_H),
        );
        let resp = ui.interact(btn, Id::new("set_range"), Sense::click());
        let fill = if resp.hovered() { pal.muted } else { pal.text };
        ui.painter().rect_filled(btn, CornerRadius::same(R_CTRL as u8), fill);
        ui.painter().text(btn.center(), Align2::CENTER_CENTER, label, font.clone(), pal.on_accent);
        set_clicked = resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();

        // Link: Use current time
        let link = "Use current time";
        let galley = ui.painter().layout_no_wrap(link.into(), font.clone(), pal.muted);
        let link_rect = Rect::from_min_size(
            egui::pos2(row.left(), row.center().y - galley.size().y / 2.0),
            galley.size(),
        );
        let lresp = ui.interact(link_rect.expand2(Vec2::new(0.0, 8.0)), Id::new("use_now"), Sense::click());
        let lcolor = if lresp.hovered() { pal.text } else { pal.muted };
        ui.painter().galley(link_rect.min, galley, lcolor);
        ui.painter().line_segment(
            [link_rect.left_bottom() + Vec2::new(0.0, 1.0), link_rect.right_bottom() + Vec2::new(0.0, 1.0)],
            Stroke::new(1.0_f32, lcolor),
        );
        now_clicked = lresp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    });

    if now_clicked {
        let now = Local::now();
        form.start_date = now.format("%Y-%m-%d").to_string();
        form.start_time = now.format("%H:%M").to_string();
        form.start_fault = false;
    }
    set_clicked
}

fn notice_row(ui: &mut Ui, notice: &Notice, pal: &Palette) {
    ui.horizontal(|ui| {
        if let Some((action, label)) = notice.action {
            let resp = ui.add(
                egui::Label::new(egui::RichText::new(label).size(TEXT_SMALL).color(pal.text).underline())
                    .sense(Sense::click()),
            );
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                nudge::perform(action);
            }
            ui.add_space(8.0);
        }
        ui.add(egui::Label::new(egui::RichText::new(&notice.text).size(TEXT_SMALL).color(pal.muted)).truncate());
    });
}
