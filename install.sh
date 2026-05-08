#!/bin/bash

# TODO: do we need this or is it just for dev?

set -euo pipefail

# Simple development installer for betterdisplay-kvm on macOS.
# - Builds the release binary
# - Runs --install, which copies the binary to the user's Application Support
#   directory and refreshes the LaunchAgent

BIN_NAME="betterdisplay-kvm"
BUILD_BIN="target/release/${BIN_NAME}"

# building in release mode is recommended
cargo build --release

echo "==> Installing ${BIN_NAME}"

if [[ ! -f "${BUILD_BIN}" ]]; then
  echo "Error: ${BUILD_BIN} not found. Build the project first (e.g. cargo build --release)." >&2
  exit 1
fi

"${BUILD_BIN}" --install

echo "done"
