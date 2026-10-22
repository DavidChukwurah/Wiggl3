#!/bin/bash
# Wraps a built wiggl3 binary into a proper Mac app (own icon, own Accessibility permission).
# Usage: bash make_mac_app.sh <path-to-binary> <output wiggl3.app>
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="$1"; APP="$2"
[ -f "$BIN" ] && [ -n "$APP" ] || { echo "usage: make_mac_app.sh <binary> <wiggl3.app>"; exit 1; }
ICON="$HERE/../assets/icon_512.png"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$HERE/../Cargo.toml" | head -1)"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/wiggl3"
chmod +x "$APP/Contents/MacOS/wiggl3"

ICONSET="$(mktemp -d)/wiggl3.iconset"
mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do sips -z $s $s "$ICON" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null 2>&1 || true; done
for s in 16 32 128 256; do sips -z $((s*2)) $((s*2)) "$ICON" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null 2>&1 || true; done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/wiggl3.icns" >/dev/null 2>&1 || echo "(icon could not be created; continuing)"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>wiggl3</string>
  <key>CFBundleDisplayName</key><string>wiggl3</string>
  <key>CFBundleIdentifier</key><string>com.wiggl3.app</string>
  <key>CFBundleExecutable</key><string>wiggl3</string>
  <key>CFBundleIconFile</key><string>wiggl3</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

xattr -cr "$APP" >/dev/null 2>&1 || true
# Ad-hoc signature (no paid certificate). Plain ad-hoc makes macOS identify the app by a fingerprint of the exact
# file, so every new build would look like a different app and lose its Accessibility permission. Pinning the
# identity to the bundle id keeps the permission across updates. Fall back to plain ad-hoc if that is rejected.
REQ='=designated => identifier "com.wiggl3.app"'
if codesign --force --deep --sign - --identifier com.wiggl3.app --requirements "$REQ" "$APP"; then
  echo "signed ad hoc with a fixed identity (com.wiggl3.app)"
elif codesign --force --deep --sign - "$APP"; then
  echo "WARNING: signed ad hoc WITHOUT a fixed identity; Accessibility may need re-granting after each update"
else
  echo "WARNING: could not sign the app; macOS may refuse to keep its permissions" >&2
fi
codesign --verify --deep --strict "$APP" && echo "signature verifies" || echo "WARNING: signature does not verify"
codesign -dr - "$APP" 2>&1 | sed 's/^/  requirement: /' || true
echo "created $APP"
