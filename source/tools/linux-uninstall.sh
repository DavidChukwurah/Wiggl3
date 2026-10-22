#!/bin/sh
# Removes what install.sh added. Your settings (~/.config/wiggl3) are left alone; delete that folder too if you want.
D="${XDG_DATA_HOME:-$HOME/.local/share}"
rm -rf "$D/wiggl3" "$D/applications/wiggl3.desktop" "$D/icons/hicolor/512x512/apps/wiggl3.png"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$D/applications" >/dev/null 2>&1 || true
echo "wiggl3 removed."
