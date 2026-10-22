#!/bin/sh
# Checks that the build prerequisites are present and says what is missing.
# It installs nothing, never uses sudo and never touches the network.
# Exit code 1 = something required is missing.
# (Building needs only Rust and a C compiler/linker. No X11, Wayland, GTK or GL development packages:
#  the window libraries are loaded when the program runs, not when it is built.)

fail=0
say()  { printf '%s\n' "$*"; }
miss() { say "  MISSING: $*"; fail=1; }

OS=$(uname -s)

if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
  ver=$(rustc --version | sed -n 's/^rustc \([0-9][0-9]*\.[0-9][0-9]*\).*/\1/p')
  major=${ver%%.*}; minor=${ver#*.}
  if [ -z "$ver" ]; then
    say "  WARNING: could not read the Rust version from: $(rustc --version)"
  elif [ "$major" -lt 1 ] || { [ "$major" -eq 1 ] && [ "$minor" -lt 89 ]; }; then
    miss "Rust 1.89 or newer (found $ver). Update it with: rustup update stable"
  else
    say "  ok: Rust $ver"
  fi
else
  miss "Rust (cargo and rustc), version 1.89 or newer. Get it from https://rustup.rs or your IT team's software portal."
fi

if [ "$OS" = "Darwin" ]; then
  if xcode-select -p >/dev/null 2>&1; then say "  ok: Xcode Command Line Tools"; else miss "Xcode Command Line Tools. Install them with: xcode-select --install"; fi
else
  if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1 || command -v clang >/dev/null 2>&1; then
    say "  ok: C compiler"
  else
    miss "a C compiler and linker (Debian/Ubuntu: build-essential, Fedora: gcc, Arch: base-devel)"
  fi
fi

if [ "$fail" -ne 0 ]; then say ""; say "Fix the MISSING items above, then run this again."; exit 1; fi
say "Ready to build."
