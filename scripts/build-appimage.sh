#!/usr/bin/env bash
set -euo pipefail

# Ryzora AppImage Packaging Script
# Pure user-space packaging: Zero sudo, zero root escalation

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT_DIR/target/appimage/AppDir"
OUT_DIR="$ROOT_DIR/target/appimage"

echo "=== Ryzora AppImage Builder ==="
echo "Working directory: $ROOT_DIR"

# Read version from Cargo.toml
VERSION=$(grep '^version' "$ROOT_DIR/src-tauri/Cargo.toml" | head -n1 | cut -d '"' -f 2)
echo "Target version: $VERSION"

# 1. Clean AppDir
echo "[1/5] Cleaning and preparing AppDir..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/usr/bin"
mkdir -p "$APP_DIR/usr/share/applications"
mkdir -p "$APP_DIR/usr/share/metainfo"
mkdir -p "$APP_DIR/usr/share/icons"

# 2. Build or verify release binaries
echo "[2/5] Verifying / compiling release binaries..."
if [ ! -f "$ROOT_DIR/src-tauri/target/release/ryzora" ]; then
    echo "Release binary not found. Building..."
    cd "$ROOT_DIR"
    npm run build
    cargo build --manifest-path src-tauri/Cargo.toml --release --locked
fi

# Copy binaries
cp "$ROOT_DIR/src-tauri/target/release/ryzora" "$APP_DIR/usr/bin/ryzora"
chmod 755 "$APP_DIR/usr/bin/ryzora"

if [ -f "$ROOT_DIR/src-tauri/target/release/ryzora-ci" ]; then
    cp "$ROOT_DIR/src-tauri/target/release/ryzora-ci" "$APP_DIR/usr/bin/ryzora-ci"
    chmod 755 "$APP_DIR/usr/bin/ryzora-ci"
fi

# 3. Copy Desktop entry, Metainfo, and Icons
echo "[3/5] Installing desktop integration and icons..."
cp "$ROOT_DIR/dist-assets/io.ryzora.Ryzora.desktop" "$APP_DIR/io.ryzora.Ryzora.desktop"
cp "$ROOT_DIR/dist-assets/io.ryzora.Ryzora.desktop" "$APP_DIR/usr/share/applications/io.ryzora.Ryzora.desktop"
ln -sf "io.ryzora.Ryzora.desktop" "$APP_DIR/ryzora.desktop"
ln -sf "io.ryzora.Ryzora.desktop" "$APP_DIR/usr/share/applications/ryzora.desktop"

cp "$ROOT_DIR/dist-assets/io.ryzora.Ryzora.metainfo.xml" "$APP_DIR/usr/share/metainfo/io.ryzora.Ryzora.metainfo.xml"

# Root icon & .DirIcon for AppImage standard
cp "$ROOT_DIR/dist-assets/icons/hicolor/512x512/apps/ryzora.png" "$APP_DIR/ryzora.png"
cp "$ROOT_DIR/dist-assets/icons/hicolor/512x512/apps/ryzora.png" "$APP_DIR/.DirIcon"
cp -r "$ROOT_DIR/dist-assets/icons/hicolor" "$APP_DIR/usr/share/icons/"

# 4. Create standard AppRun launcher
echo "[4/5] Writing AppRun launcher..."
cat << 'APPRUN' > "$APP_DIR/AppRun"
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "${HERE}/usr/bin/ryzora" "$@"
APPRUN
chmod 755 "$APP_DIR/AppRun"

# Verify launcher permissions
if [ ! -x "$APP_DIR/AppRun" ]; then
    echo "Error: AppRun is not executable!"
    exit 1
fi

echo "AppDir assembled successfully at: $APP_DIR"

# 5. Packaging into AppImage
echo "[5/5] Packaging AppImage binary..."
APPIMAGETOOL="${APPIMAGETOOL:-$(which appimagetool 2>/dev/null || true)}"
if [ -n "$APPIMAGETOOL" ] && [ -x "$APPIMAGETOOL" ]; then
    mkdir -p "$OUT_DIR"
    ARCH=x86_64 "$APPIMAGETOOL" "$APP_DIR" "$OUT_DIR/Ryzora-${VERSION}-x86_64.AppImage"
    ln -sf "Ryzora-${VERSION}-x86_64.AppImage" "$OUT_DIR/Ryzora-x86_64.AppImage"
    echo "AppImage created successfully at: $OUT_DIR/Ryzora-${VERSION}-x86_64.AppImage"
else
    echo "Notice: appimagetool not found on local PATH."
    echo "The AppDir is complete and validated; ready to run directly via:"
    echo "  $APP_DIR/AppRun"
fi
