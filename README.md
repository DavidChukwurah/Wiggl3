# wiggl3

**wiggl3 is a tiny desktop app that keeps your computer looking active.** When you stop touching the mouse, it
glides the cursor across your screens every so often, so chat apps such as Teams, Slack and Zoom don't flip you
to "Away", and the screen doesn't lock or sleep while you are reading, waiting or on a call.

- One small window: an on/off switch, how often to nudge (15 seconds to 4 minutes), and how long to keep going
  (until you turn it off, 1, 2, 4 or 8 hours, or a custom start and end time).
- **Smart:** it only moves the cursor when you have actually been idle. If you are using the computer, it stays out
  of the way.
- **Lives in the tray / menu bar.** Close the window and it keeps running; the icon's menu has Open, Pause/Resume
  and Quit. Desktop notifications tell you when it starts and stops.
- **One copy only.** Starting it a second time just brings the first window forward.
- **Private.** No account, no internet connection, no data collected. It saves two settings on your computer
  (interval and duration) and nothing else.
- **Small and free.** One file of roughly 4 to 7 MB. MIT licence.

## Supported computers

| System | Versions | Download |
|---|---|---|
| **Windows** | Windows 10 and 11, 64-bit (Intel/AMD; also runs on Windows 11 on ARM through Windows' built-in emulation) | `wiggl3-windows-x64.exe` |
| **macOS** | macOS 11 (Big Sur) or newer, Apple silicon and Intel | `wiggl3-mac-universal.dmg` |
| **Linux, Intel/AMD** | Desktop Linux with glibc 2.28 or newer: Ubuntu 18.10+, Debian 10+, Fedora, RHEL / Rocky / Alma 8+, openSUSE, Arch, Mint, Pop!_OS, Manjaro and similar. X11 **and** Wayland sessions. | `wiggl3-linux-x86_64.tar.gz` |
| **Linux, ARM** | The same, on 64-bit ARM (Raspberry Pi 4/5 with a 64-bit OS, ARM laptops) | `wiggl3-linux-aarch64.tar.gz` |

Not supported: 32-bit systems, Windows 7/8, macOS 10.x, Alpine Linux and other musl-based distributions,
the BSDs, phones and tablets. [SUPPORTED-PLATFORMS.md](SUPPORTED-PLATFORMS.md) has the details, including what
has been tested and what has not.

## Download

Go to the **[Releases page](../../releases/latest)** and download the one file for your system (names in the table
above). You do **not** need Git, any GitHub knowledge, a compiler or an installer.

The files are not signed with a paid certificate, so Windows and macOS show a one-time warning the first time.
That is expected for free, unsigned software; the steps below get past it.

### Windows

1. Double-click `wiggl3-windows-x64.exe`. There is nothing to install: put it anywhere you like.
2. If Windows shows **"Windows protected your PC"**, click **More info**, then **Run anyway**.
3. If your computer is managed by an employer that blocks unsigned programs, ask your IT team. wiggl3 does not try to get around that.

The icon appears next to the clock (click the `^` arrow if it is hidden). Left-click opens the window, right-click shows the menu.

### macOS

1. Open `wiggl3-mac-universal.dmg` and drag **wiggl3** onto **Applications**.
2. The first time, **right-click (or Control-click) wiggl3 in Applications, choose Open, then Open** again.
   If macOS says it "can't be opened", go to **System Settings > Privacy & Security** and click **Open Anyway**.
3. Turn wiggl3 on. macOS asks for permission to control the computer: click **Open System Settings** and switch
   **wiggl3** on under **Privacy & Security > Accessibility**. This is what lets it move the cursor. Until then, wiggl3
   shows a notice with an "Open Settings" link.
4. If the notice stays after you switched it on, quit wiggl3 (menu bar icon > Quit) and open it again.

The icon appears in the menu bar. Click it for Open, Pause/Resume and Quit. (The Dock icon does not reopen a
closed window; use the menu bar icon.)

### Linux

1. Download and unpack: `tar xzf wiggl3-linux-x86_64.tar.gz` (use `aarch64` on ARM), then `cd wiggl3`.
2. Run it right away with `./wiggl3`, **or** run `./install.sh` to add it to your applications menu. The installer
   works for your user only: no `sudo`, nothing outside your home folder. `./uninstall.sh` removes it again.

Almost every desktop already has what it needs (OpenGL/Mesa, `libxkbcommon`, X11 or Wayland libraries). If wiggl3
does not start, run `./wiggl3` in a terminal; the message names the missing library.

**Good to know on Linux**

- **Tray icon:** KDE Plasma, XFCE, Cinnamon, MATE, Budgie, LXQt and others show it. **GNOME does not show tray icons
  by default**: install the "AppIndicator and KStatusNotifierItem Support" extension to get one. Without a tray,
  wiggl3 still works, but closing the window quits it (the window says so: "Closing the window stops it").
  Use the minimise dot instead.
- **Wayland:** the cursor is moved through the desktop's standard remote-desktop permission. The first time you turn
  wiggl3 on, your desktop asks you to allow it; say yes. GNOME and KDE provide this. Some minimal compositors (Sway,
  Hyprland and similar) may not; wiggl3 then shows "Can't nudge input here" instead of pretending to work.
- **The window on Wayland** is drawn through XWayland when it is available, because a Wayland application is not
  allowed to hide its own window. Without XWayland it still works, but closing the window quits it.

## Using it

- **On/off switch:** Online means it is nudging when you are idle (or waiting for a scheduled start). Offline means it does nothing.
- **Interval:** how long you must be idle before it moves the cursor, and how often it repeats.
- **Duration:** until you turn it off, 1 / 2 / 4 / 8 hours, or **Custom range** with a start and an end entered as
  `YYYY-MM-DD` and `HH:MM` (24-hour); "Use current time" fills in now. A start in the future schedules it.
- **Tray / menu bar:** Open, Pause (turns it off) / Resume (turns it back on), Quit.
- **Settings** live in `%APPDATA%\wiggl3` (Windows), `~/Library/Application Support/wiggl3` (macOS) or
  `~/.config/wiggl3` (Linux). Delete that folder to reset.

## What wiggl3 is not

wiggl3 is open about what it is. It does not disguise its process name, hide its tray icon, or try to evade
monitoring or software blocks. If your employer's rules say your status must reflect real activity, using any tool
like this may breach them; that is your decision to make.

## Questions

**Does it work on a locked or sleeping computer?** No. It keeps an *unlocked, awake* computer looking active.

**Does it stop the screen sleeping?** Yes: while it is on it asks the operating system to stay awake, and the cursor
movement counts as activity too.

**Why is the Windows notification headed "Windows PowerShell"?** Windows only shows notifications from registered
apps, and wiggl3 borrows that one. The text itself starts with "wiggl3".

**Is it safe?** The source code is in this repository, each release lists SHA-256 checksums (`SHA256SUMS.txt`) so you
can verify your download, and the app makes no network connections.

## For the person publishing it

See [PUBLISHING.md](PUBLISHING.md) (no Git commands needed). Developers: [source/README.md](source/README.md).
Third-party licences: [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
