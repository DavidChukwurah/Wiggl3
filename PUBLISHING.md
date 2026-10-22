# Publishing wiggl3 (no Git needed)

**wiggl3** keeps a computer looking active (it moves the cursor when you are idle). It runs on **Windows 10/11**,
**macOS 11+** (Apple silicon and Intel) and **desktop Linux** (Intel/AMD and ARM, X11 and Wayland); see
[SUPPORTED-PLATFORMS.md](SUPPORTED-PLATFORMS.md).

Your users only ever click **Download** on a normal web page. They never see Git, GitHub's code, or a compiler.

**Why GitHub is involved at all:** a program has to be *built* once on each kind of computer. Apple only allows Mac
apps to be built on a Mac, and Windows programs are best built on Windows. GitHub lends you a free Windows machine,
a free Mac and a free Linux machine for a few minutes, builds the files, and keeps them on a Releases page with
permanent links. That is all it is used for. You do the whole job in your web browser:

## One-time steps (about 15 minutes, mostly waiting)

1. **Unzip** the download on your computer.
2. **Clean the repository** at https://github.com/DavidChukwurah/wiggl3 (only needed if it holds older files).
   Delete these if they exist: `Run-Windows.bat`, `Run-Mac.command`, `Run-Linux.sh`, `README.txt`,
   `GITHUB-SETUP.md`, `source/assets/trail_length_short.svg`. (Open a file, click the `...` menu, **Delete file**, **Commit changes**.)
   Or start fresh: make a new empty public repository and use that instead.
3. **Upload the files.** On the repository page click **Add file > Upload files**, then drag in everything
   that is *inside* the unzipped folder (`source`, `docs`, `README.md`, `LICENSE` and the others), not the folder itself.
   Click **Commit changes**. If the page limits you to 100 files at a time, upload `source` first and the rest second.
4. **Add the build recipe.** The `.github` folder is hidden on some computers and may not upload by drag-and-drop.
   So do this part by hand: click **Add file > Create new file**. In the name box type exactly
   `.github/workflows/build.yml` (typing each `/` creates the folders). Open the file **`build.yml`** from the unzipped folder in
   Notepad / TextEdit, copy everything, paste it into GitHub, click **Commit changes**.
   (If `.github/workflows/build.yml` already appeared after step 3, you can skip this; just check it is the new one.)
5. **Allow it to publish.** Repository **Settings > Actions > General > Workflow permissions**: choose
   **Read and write permissions**, then **Save**.
6. **Watch it build.** Click the **Actions** tab. A run called *build* starts by itself and takes about 10 minutes.
   Four green ticks (Windows, macOS, Linux Intel, Linux ARM) and a *release* job mean it worked.
   You can also start it any time with **Actions > build > Run workflow**.
7. **Get your links.** Click **Releases** (right-hand side of the repository page): there is a release called
   *wiggl3 (latest build)* with the files. Send people this one link, which always points at the newest files:

   `https://github.com/DavidChukwurah/wiggl3/releases/latest`

   Direct links to each file never change either, for example
   `https://github.com/DavidChukwurah/wiggl3/releases/latest/download/wiggl3-windows-x64.exe`.

## Optional: a proper download page

The file `docs/index.html` is a one-page website that detects the visitor's system and shows one big Download
button. To turn it on: **Settings > Pages > Build and deployment > Source: Deploy from a branch > Branch: main, folder: /docs > Save.**
After a minute it is live at `https://DavidChukwurah.github.io/wiggl3/`. If your repository is not named
`DavidChukwurah/wiggl3`, change the one line `const REPO = ...` near the top of `docs/index.html`.

## Updating later

Upload the changed files the same way (**Add file > Upload files**, same names replace the old ones). Every change
to the main branch rebuilds and refreshes the *latest* release by itself. For a numbered release (v0.3.1, ...) create a release
with that tag from the **Releases** page; it builds and attaches the files.

## If something goes red

Open the failed run in the **Actions** tab, click the red step, copy the last 30 lines of the error and send them
to whoever maintains the code. The first run of any new build setup can need a small fix; the Linux ARM job is allowed to fail without stopping the others.

## Good to know

- The repository should be **public**. Public repositories get the free ARM Linux machine and unlimited build
  minutes; private ones use paid minutes (Mac minutes cost ten times as much).
- The files are **unsigned**. Windows SmartScreen and macOS Gatekeeper show a one-time warning on first open
  (the README tells users what to click). Removing the warnings needs paid certificates (an Apple Developer
  account, about 99 USD a year, and a Windows code-signing certificate); that is not set up.
- Nothing here installs software on anyone's computer or uses `sudo`.
