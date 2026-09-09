#!/usr/bin/env bash
set -euo pipefail

# Ryzora AppImage Packaging Script
# Pure user-space packaging: Zero sudo, zero root escalation

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT_DIR/target/appimage/AppDir"
OUT_DIR="$ROOT_DIR/target/appimage"

echo "=== Ryzora AppImage Builder ==="
echo "Working directory: $ROOT_DIR"

# 1. Build frontend and backend
echo "[1/4] Compiling release binaries..."
cd "$ROOT_DIR"
npm run build
cargo build --manifest-path src-tauri/Cargo.toml --release

# 2. Assemble AppDir structure
echo "[2/4] Assembling AppDir..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/usr/bin"
mkdir -p "$APP_DIR/usr/share/applications"
mkdir -p "$APP_DIR/usr/share/icons"

# Copy binaries
cp "$ROOT_DIR/src-tauri/target/release/ryzora" "$APP_DIR/usr/bin/ryzora"
chmod 755 "$APP_DIR/usr/bin/ryzora"

if [ -f "$ROOT_DIR/src-tauri/target/release/ryzora-ci" ]; then
    cp "$ROOT_DIR/src-tauri/target/release/ryzora-ci" "$APP_DIR/usr/bin/ryzora-ci"
    chmod 755 "$APP_DIR/usr/bin/ryzora-ci"
fi

# Copy Desktop entry and Icons
cp "$ROOT_DIR/dist-assets/ryzora.desktop" "$APP_DIR/ryzora.desktop"
cp "$ROOT_DIR/dist-assets/ryzora.desktop" "$APP_DIR/usr/share/applications/ryzora.desktop"
cp "$ROOT_DIR/dist-assets/icons/hicolor/256x256/apps/ryzora.png" "$APP_DIR/ryzora.png"
cp -r "$ROOT_DIR/dist-assets/icons/hicolor" "$APP_DIR/usr/share/icons/"

# Create standard AppRun launcher
cat << 'APPRUN' > "$APP_DIR/AppRun"
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "${HERE}/usr/bin/ryzora" "$@"
APPRUN
chmod 755 "$APP_DIR/AppRun"

echo "[3/4] AppDir assembled successfully at: $APP_DIR"

# 3. Packaging into AppImage
echo "[4/4] Generating AppImage binary..."
APPIMAGETOOL="${APPIMAGETOOL:-$(which appimagetool 2>/dev/null || true)}"
if [ -n "$APPIMAGETOOL" ] && [ -x "$APPIMAGETOOL" ]; then
    ARCH=x86_64 "$APPIMAGETOOL" "$APP_DIR" "$OUT_DIR/Ryzora-x86_64.AppImage"
    echo "AppImage created at: $OUT_DIR/Ryzora-x86_64.AppImage"
else
    echo "Notice: appimagetool not found on local PATH. The AppDir is complete and can be run directly via:"
    echo "  $APP_DIR/AppRun"
    echo "Or packaged with appimagetool in CI / container."
fi
