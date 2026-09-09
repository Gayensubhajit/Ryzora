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

# 7. Build .deb if dpkg-deb is available
DPKG_DEB="$(which dpkg-deb 2>/dev/null || true)"
if [ -n "$DPKG_DEB" ] && [ -x "$DPKG_DEB" ]; then
    mkdir -p "$OUT_DIR"
    dpkg-deb --build --root-owner-group "$PKG_DIR" "$OUT_DIR/${PKG_NAME}_${VERSION}_${ARCH}.deb"
    echo "Debian package created at: $OUT_DIR/${PKG_NAME}_${VERSION}_${ARCH}.deb"
else
    echo "dpkg-deb not found on local PATH. Package tree is fully ready in:"
    echo "  $PKG_DIR"
fi
