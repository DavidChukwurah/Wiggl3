#!/bin/bash
# Runs INSIDE the manylinux_2_28 container (AlmaLinux 8, glibc 2.28) so the finished program also runs on
# older distributions. Called from the GitHub workflow; can be run by hand with docker:
#   docker run --rm -v "$PWD":/src quay.io/pypa/manylinux_2_28_x86_64 bash /src/source/tools/ci_linux.sh x86_64
set -euo pipefail
ARCH="$1"
RUST_VERSION="${RUST_VERSION:-1.98.1}"
curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "$RUST_VERSION"
export PATH="$HOME/.cargo/bin:$PATH"
cd /src/source
cargo test --locked
cargo build --release --locked
bash tools/package_linux.sh target/release/wiggl3 "$ARCH" /src/dist
echo "highest glibc symbol needed:"
objdump -T target/release/wiggl3 | grep -o 'GLIBC_[0-9.]*' | sort -V | uniq | tail -1
