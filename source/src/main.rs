// No console window on Windows release builds (GUI subsystem).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod alerts;
mod controller;
mod engine;
mod nudge;
mod settings;
mod shell;
mod single;
mod tray;
mod ui;

fn main() -> eframe::Result {
    match single::acquire() {
        single::Start::First(instance) => ui::run(instance),
        // Another copy is already running and has been asked to show its window.
        single::Start::Second => Ok(()),
    }
}
