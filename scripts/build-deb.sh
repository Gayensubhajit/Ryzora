#!/usr/bin/env bash
set -euo pipefail

# Ryzora Debian (.deb) Packaging Script
# Pure user-space packaging: Zero sudo, zero root escalation

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION=$(grep '^version' "$ROOT_DIR/src-tauri/Cargo.toml" | head -n1 | cut -d '"' -f 2)
ARCH="amd64"
PKG_NAME="ryzora"
PKG_DIR="$ROOT_DIR/target/deb/${PKG_NAME}_${VERSION}_${ARCH}"
OUT_DIR="$ROOT_DIR/target/deb"

echo "=== Ryzora Debian Package Builder ==="
echo "Version: $VERSION, Arch: $ARCH"

# 1. Prepare clean package directory
rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/DEBIAN"
mkdir -p "$PKG_DIR/usr/bin"
mkdir -p "$PKG_DIR/usr/share/applications"
mkdir -p "$PKG_DIR/usr/share/metainfo"
mkdir -p "$PKG_DIR/usr/share/icons"
mkdir -p "$PKG_DIR/usr/share/doc/$PKG_NAME"

# 2. Verify / build binaries
if [ ! -f "$ROOT_DIR/src-tauri/target/release/ryzora" ]; then
    echo "Release binary not found. Building..."
    cd "$ROOT_DIR"
    npm run build
    cargo build --manifest-path src-tauri/Cargo.toml --release --locked
fi

cp "$ROOT_DIR/src-tauri/target/release/ryzora" "$PKG_DIR/usr/bin/ryzora"
chmod 755 "$PKG_DIR/usr/bin/ryzora"

if [ -f "$ROOT_DIR/src-tauri/target/release/ryzora-ci" ]; then
    cp "$ROOT_DIR/src-tauri/target/release/ryzora-ci" "$PKG_DIR/usr/bin/ryzora-ci"
    chmod 755 "$PKG_DIR/usr/bin/ryzora-ci"
fi

# 3. Desktop files, Metainfo, Icons
cp "$ROOT_DIR/dist-assets/io.ryzora.Ryzora.desktop" "$PKG_DIR/usr/share/applications/io.ryzora.Ryzora.desktop"
ln -sf "io.ryzora.Ryzora.desktop" "$PKG_DIR/usr/share/applications/ryzora.desktop"
cp "$ROOT_DIR/dist-assets/io.ryzora.Ryzora.metainfo.xml" "$PKG_DIR/usr/share/metainfo/io.ryzora.Ryzora.metainfo.xml"
cp -r "$ROOT_DIR/dist-assets/icons/hicolor" "$PKG_DIR/usr/share/icons/"

# 3b. Privileged SDDM helper & Polkit policy
mkdir -p "$PKG_DIR/usr/lib/ryzora"
cp "$ROOT_DIR/src-tauri/resources/ryzora-sddm-helper" "$PKG_DIR/usr/lib/ryzora/ryzora-sddm-helper"
chmod 755 "$PKG_DIR/usr/lib/ryzora/ryzora-sddm-helper"

mkdir -p "$PKG_DIR/usr/share/polkit-1/actions"
cp "$ROOT_DIR/src-tauri/resources/io.ryzora.sddm.policy" "$PKG_DIR/usr/share/polkit-1/actions/io.ryzora.sddm.policy"
chmod 644 "$PKG_DIR/usr/share/polkit-1/actions/io.ryzora.sddm.policy"

# Generate DEBIAN/postrm cleanup
cat << 'POSTRM' > "$PKG_DIR/DEBIAN/postrm"
#!/bin/sh
set -e
if [ "$1" = "remove" ] || [ "$1" = "purge" ]; then
    rm -f /usr/lib/ryzora/ryzora-sddm-helper
    rmdir /usr/lib/ryzora 2>/dev/null || true
    rm -f /usr/share/polkit-1/actions/io.ryzora.sddm.policy
    rm -f /etc/sddm.conf.d/zz-ryzora-theme.conf /etc/sddm.conf.d/ryzora-theme.conf
fi
exit 0
POSTRM
chmod 755 "$PKG_DIR/DEBIAN/postrm" 

# 4. Doc & License
cp "$ROOT_DIR/LICENSE" "$PKG_DIR/usr/share/doc/$PKG_NAME/copyright"
cp "$ROOT_DIR/README.md" "$PKG_DIR/usr/share/doc/$PKG_NAME/README.md"

# 5. Generate DEBIAN/control
INSTALLED_SIZE=$(du -sk "$PKG_DIR/usr" | cut -f1)
cat << CONTROL > "$PKG_DIR/DEBIAN/control"
Package: ryzora
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Installed-Size: ${INSTALLED_SIZE}
Maintainer: Ryzora Contributors <contributors@ryzora.org>
Depends: libwebkit2gtk-4.1-0, libgtk-3-0, libayatana-appindicator3-1, hicolor-icon-theme
Homepage: https://github.com/Gayensubhajit/Ryzora
Description: Universal Linux Desktop Customization Platform
 Ryzora is an aesthetic, secure, and declarative Linux desktop customization
 suite. It supports fastfetch, rices, wallpapers, shell themes, and KDE/GNOME
 look-and-feel assets with cryptographically verified, sandbox-contained packages.
CONTROL

# 6. Generate md5sums
(
    cd "$PKG_DIR"
    find usr -type f -exec md5sum {} + > DEBIAN/md5sums
    chmod 644 DEBIAN/md5sums
)

echo "Package structure assembled at: $PKG_DIR"

# 7. Build .deb package (using dpkg-deb or native user-space ar fallback)
mkdir -p "$OUT_DIR"
DEB_FILE="$OUT_DIR/${PKG_NAME}_${VERSION}_${ARCH}.deb"
DPKG_DEB="$(which dpkg-deb 2>/dev/null || true)"

if [ -n "$DPKG_DEB" ] && [ -x "$DPKG_DEB" ]; then
    echo "Using dpkg-deb to assemble package..."
    dpkg-deb --build --root-owner-group "$PKG_DIR" "$DEB_FILE"
    echo "Debian package created at: $DEB_FILE"
else
    echo "dpkg-deb not found. Using pure user-space ar/tar packaging fallback (zero sudo)..."
    STAGING_TMP="$(mktemp -d)"
    echo "2.0" > "$STAGING_TMP/debian-binary"
    (
        cd "$PKG_DIR/DEBIAN"
        tar --owner=0 --group=0 --numeric-owner -czf "$STAGING_TMP/control.tar.gz" ./control ./md5sums
    )
    (
        cd "$PKG_DIR"
        tar --owner=0 --group=0 --numeric-owner -cJf "$STAGING_TMP/data.tar.xz" ./usr
    )
    (
        cd "$STAGING_TMP"
        ar rcs "$DEB_FILE" debian-binary control.tar.gz data.tar.xz
    )
    rm -rf "$STAGING_TMP"
    echo "Debian package created successfully via native user-space ar fallback at: $DEB_FILE"
fi
