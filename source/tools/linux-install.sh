#!/bin/sh
# Installs wiggl3 for the current user only (no sudo, nothing outside your home folder) and adds it to the
# applications menu. Run it from the unpacked folder:  ./install.sh
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
APPDIR="${XDG_DATA_HOME:-$HOME/.local/share}/wiggl3"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/512x512/apps"
mkdir -p "$APPDIR" "$APPS" "$ICONS"
cp "$HERE/wiggl3" "$APPDIR/wiggl3"; chmod 755 "$APPDIR/wiggl3"
cp "$HERE/wiggl3.png" "$ICONS/wiggl3.png"
cat > "$APPS/wiggl3.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=wiggl3
Comment=Keeps chat apps from showing Away
Exec=$APPDIR/wiggl3
Icon=wiggl3
Terminal=false
Categories=Utility;
StartupWMClass=wiggl3
DESKTOP
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" >/dev/null 2>&1 || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor" >/dev/null 2>&1 || true
echo "Installed. Find wiggl3 in your applications menu, or run: $APPDIR/wiggl3"
