# Third-party notices

wiggl3 itself is MIT licensed (see `LICENSE`).

- **Logo glyph:** "pest_control_rodent" from **Material Symbols** by Google, Apache License 2.0
  (`source/assets/MATERIAL-SYMBOLS-LICENSE.txt`). The app icons are drawn from it by `source/tools/make_icon.py`.
- **Font:** **Jersey 10** by Sarah Cadigan-Fried, SIL Open Font License 1.1 (`source/assets/OFL.txt`). Used for the
  "Online / Offline" heading.
- **Rust libraries:** egui / eframe (MIT or Apache-2.0), tray-icon (MIT or Apache-2.0), ksni (Unlicense),
  ashpd (MIT), zbus (MIT), x11rb (MIT or Apache-2.0), windows-sys (MIT or Apache-2.0), notify-rust (MIT or Apache-2.0),
  chrono, serde, serde_json, dirs, winit, pollster and their dependencies, each under its own permissive licence.
  The exact versions are pinned in `source/Cargo.lock`; `cargo tree` lists them all.
