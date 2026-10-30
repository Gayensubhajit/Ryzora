#!/usr/bin/env bash
set -euo pipefail

# Ryzora Native Arch Linux (.pkg.tar.zst) Packaging Script
# Pure user-space packaging: Zero sudo, zero root escalation

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCH_DIR="$ROOT_DIR/packaging/arch"
OUT_DIR="$ROOT_DIR/target/arch"

echo "=== Ryzora Arch Linux Package Builder ==="
echo "Working directory: $ROOT_DIR"

# 1. Read version from Cargo.toml
VERSION=$(grep '^version' "$ROOT_DIR/crates/ryzora-core/Cargo.toml" | head -n1 | cut -d '"' -f 2)
echo "Target version: $VERSION"

mkdir -p "$OUT_DIR"

# 2. Generate clean reproducible source tarball from current git tree
echo "[1/4] Generating reproducible source archive from working tree..."
TARBALL="$ARCH_DIR/ryzora-${VERSION}.tar.gz"
git -C "$ROOT_DIR" add -A
TREE_ID=$(git -C "$ROOT_DIR" write-tree)
git -C "$ROOT_DIR" archive --format=tar.gz --prefix="ryzora-${VERSION}/" "$TREE_ID" -o "$TARBALL"

# 3. Update .SRCINFO metadata
echo "[2/4] Updating .SRCINFO..."
(cd "$ARCH_DIR" && makepkg --printsrcinfo > .SRCINFO)

# 4. Build package with makepkg
echo "[3/4] Compiling and packaging with makepkg..."
(cd "$ARCH_DIR" && makepkg -f --nodeps)

# 5. Locate and copy output package to target/arch/
echo "[4/4] Collecting built package artifacts..."
PKG_LIST=$(cd "$ARCH_DIR" && makepkg --packagelist)
for pkg in $PKG_LIST; do
    if [ -f "$pkg" ]; then
        cp -v "$pkg" "$OUT_DIR/"
        echo "Package built successfully: $OUT_DIR/$(basename "$pkg")"
    fi
done

echo "=== Build Complete ==="

