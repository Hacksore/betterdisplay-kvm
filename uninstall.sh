#!/bin/bash

set -euo pipefail

BIN_NAME="betterdisplay-kvm"
INSTALL_DIR="${HOME}/Library/Application Support/${BIN_NAME}"
PLIST_DEST="${HOME}/Library/LaunchAgents/com.github.hacksore.betterdisplay-kvm.plist"
LABEL="com.github.hacksore.betterdisplay-kvm"

echo "==> Stopping ${BIN_NAME} service if running"
launchctl bootout "gui/$(id -u)/${LABEL}" 2>/dev/null || true

echo "==> Uninstalling ${BIN_NAME}"
rm -rf "${INSTALL_DIR}" "${PLIST_DEST}"
