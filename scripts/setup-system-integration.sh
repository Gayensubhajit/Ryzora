#!/usr/bin/env bash
set -euo pipefail

# Ryzora System Integration Installer
# Installs privileged helpers and Polkit security policies into system directories

if [ "$(id -u)" -ne 0 ]; then
    echo "This script must be run as root (or with sudo):"
    echo "  sudo $0"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
RES_DIR="$ROOT_DIR/src-tauri/resources"

echo "=== Ryzora System Integration Setup ==="
mkdir -p /usr/lib/ryzora /usr/share/polkit-1/actions /usr/share/polkit-1/rules.d

echo "Installing privileged helpers to /usr/lib/ryzora/..."
cp "$RES_DIR/ryzora-sddm-helper" /usr/lib/ryzora/ryzora-sddm-helper
chmod 755 /usr/lib/ryzora/ryzora-sddm-helper
chown root:root /usr/lib/ryzora/ryzora-sddm-helper

if [ -f "$RES_DIR/ryzora-package-helper" ]; then
    cp "$RES_DIR/ryzora-package-helper" /usr/lib/ryzora/ryzora-package-helper
    chmod 755 /usr/lib/ryzora/ryzora-package-helper
    chown root:root /usr/lib/ryzora/ryzora-package-helper
fi

echo "Installing Polkit policies to /usr/share/polkit-1/actions/..."
cp "$RES_DIR/io.ryzora.sddm.policy" /usr/share/polkit-1/actions/io.ryzora.sddm.policy
chmod 644 /usr/share/polkit-1/actions/io.ryzora.sddm.policy
chown root:root /usr/share/polkit-1/actions/io.ryzora.sddm.policy

if [ -f "$RES_DIR/io.ryzora.package.policy" ]; then
    cp "$RES_DIR/io.ryzora.package.policy" /usr/share/polkit-1/actions/io.ryzora.package.policy
    chmod 644 /usr/share/polkit-1/actions/io.ryzora.package.policy
    chown root:root /usr/share/polkit-1/actions/io.ryzora.package.policy
fi

if [ -f "$RES_DIR/io.ryzora.sddm.rules" ]; then
    cp "$RES_DIR/io.ryzora.sddm.rules" /usr/share/polkit-1/rules.d/io.ryzora.sddm.rules
    chmod 644 /usr/share/polkit-1/rules.d/io.ryzora.sddm.rules
    chown root:root /usr/share/polkit-1/rules.d/io.ryzora.sddm.rules
fi

if [ -f "$RES_DIR/io.ryzora.package.rules" ]; then
    cp "$RES_DIR/io.ryzora.package.rules" /usr/share/polkit-1/rules.d/io.ryzora.package.rules
    chmod 644 /usr/share/polkit-1/rules.d/io.ryzora.package.rules
    chown root:root /usr/share/polkit-1/rules.d/io.ryzora.package.rules
fi

echo "✓ Ryzora system integration successfully installed and verified."
