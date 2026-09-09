#!/usr/bin/env bash
set -euo pipefail

# Ryzora Independent Release Artifacts Verifier
# Operates independently of the build pipeline to audit release packages,
# verify SHA256 checksums, and check Ed25519 cryptographic signatures.

usage() {
    echo "Usage: $0 <artifacts-dir> <release-pubkey-hex-or-file>"
    echo "Example: $0 dist/release-v0.1.0 release-pubkey.pub"
    exit 1
}

if [ "$#" -lt 2 ]; then
    usage
fi

ARTIFACTS_DIR="$1"
PUBKEY="$2"

if [ ! -d "$ARTIFACTS_DIR" ]; then
    echo "[-] Error: Artifacts directory not found: $ARTIFACTS_DIR"
    exit 1
fi

echo "========================================================"
echo "    Ryzora Independent Release Artifact Verification   "
echo "========================================================"
echo "Target directory : $ARTIFACTS_DIR"
echo "Release Pubkey   : $PUBKEY"
echo ""

# 1. Check for SHA256SUMS.txt and detached signature
SHA_FILE="$ARTIFACTS_DIR/SHA256SUMS.txt"
SIG_FILE="$ARTIFACTS_DIR/SHA256SUMS.txt.sig"

if [ ! -f "$SHA_FILE" ]; then
    echo "[-] Error: $SHA_FILE missing!"
    exit 1
fi
if [ ! -f "$SIG_FILE" ]; then
    echo "[-] Error: $SIG_FILE missing!"
    exit 1
fi
echo "[+] Step 1: Checksum and signature files located."

# 2. Cryptographic signature verification
echo "[+] Step 2: Verifying detached Ed25519 signature over SHA256SUMS.txt..."
# Resolve ryzora-ci
RYZORA_CI=""
if command -v ryzora-ci >/dev/null 2>&1; then
    RYZORA_CI="ryzora-ci"
elif [ -f "src-tauri/target/release/ryzora-ci" ]; then
    RYZORA_CI="src-tauri/target/release/ryzora-ci"
elif [ -f "src-tauri/target/debug/ryzora-ci" ]; then
    RYZORA_CI="src-tauri/target/debug/ryzora-ci"
fi

if [ -n "$RYZORA_CI" ]; then
    "$RYZORA_CI" verify-file "$SHA_FILE" "$SIG_FILE" "$PUBKEY"
    echo "    -> Cryptographic signature verified successfully!"
else
    echo "[-] Error: ryzora-ci binary not found for signature verification."
    exit 1
fi

# 3. Independent Checksum Verification
echo "[+] Step 3: Independently validating SHA256 hashes..."
FAILURES=0
COUNT=0
while IFS= read -r line; do
    # Skip empty lines or comments
    [ -z "$line" ] && continue
    [[ "$line" =~ ^# ]] && continue

    EXPECTED_HASH=$(echo "$line" | awk "{print \$1}")
    FILENAME=$(echo "$line" | awk "{print \$2}")
    # Handle optional leading asterisks or paths
    FILENAME="${FILENAME#\*}"
    FILENAME="${FILENAME#./}"
    
    # Skip checking the checksum file itself or signature file if listed
    if [ "$FILENAME" = "SHA256SUMS.txt" ] || [ "$FILENAME" = "SHA256SUMS.txt.sig" ]; then
        continue
    fi

    FILEPATH="$ARTIFACTS_DIR/$FILENAME"
    if [ ! -f "$FILEPATH" ]; then
        echo "    [-] Missing artifact: $FILENAME"
        FAILURES=$((FAILURES + 1))
        continue
    fi

    ACTUAL_HASH=$(sha256sum "$FILEPATH" | awk "{print \$1}")
    if [ "$EXPECTED_HASH" = "$ACTUAL_HASH" ]; then
        echo "    [OK] $FILENAME: $ACTUAL_HASH"
        COUNT=$((COUNT + 1))
    else
        echo "    [FAIL] $FILENAME hash mismatch!"
        echo "           Expected: $EXPECTED_HASH"
        echo "           Actual:   $ACTUAL_HASH"
        FAILURES=$((FAILURES + 1))
    fi
done < "$SHA_FILE"

if [ "$FAILURES" -ne 0 ]; then
    echo "[-] Verification failed: $FAILURES checksum error(s) detected."
    exit 1
fi
echo "    -> All $COUNT artifact checksums match exactly."

# 4. Package Structure and Format Integrity
echo "[+] Step 4: Inspecting package format integrity..."
for file in "$ARTIFACTS_DIR"/*; do
    basename="${file##*/}"
    case "$basename" in
        *.tar.gz)
            echo "    -> Inspecting tarball: $basename"
            tar -tzf "$file" >/dev/null || { echo "[-] Corrupt tarball: $basename"; exit 1; }
            ;;
        *.deb)
            echo "    -> Inspecting Debian package: $basename"
            ar t "$file" | grep -q "debian-binary" || { echo "[-] Invalid .deb archive: missing debian-binary"; exit 1; }
            ar t "$file" | grep -q "control.tar" || { echo "[-] Invalid .deb archive: missing control.tar"; exit 1; }
            ar t "$file" | grep -q "data.tar" || { echo "[-] Invalid .deb archive: missing data.tar"; exit 1; }
            ;;
        *.AppImage)
            echo "    -> Inspecting AppImage: $basename"
                        # Verify ELF header (0x7f 0x45 0x4c 0x46)
            MAGIC=$(od -An -N4 -tx1 "$file" | tr -d " 	
")
            if [ "$MAGIC" != "7f454c46" ]; then
                echo "[-] AppImage does not have valid ELF header (got $MAGIC)!"
                exit 1
            fi
            ;;
        *.pkg.tar.zst)
            echo "    -> Inspecting Arch package: $basename"
            tar -tf "$file" >/dev/null || { echo "[-] Corrupt Arch package: $basename"; exit 1; }
            ;;
    esac
done

# 5. Security & Hygiene Check
echo "[+] Step 5: Checking artifact directory hygiene..."
SENSITIVE_FILES=$(find "$ARTIFACTS_DIR" -type f \( -name "*.key" -o -name "*.priv" -o -name "*secret*" -o -name ".env*" \))
if [ -n "$SENSITIVE_FILES" ]; then
    echo "[-] CRITICAL SECURITY VIOLATION: Private key or secret file detected in artifacts:"
    echo "$SENSITIVE_FILES"
    exit 1
fi
echo "    -> Hygiene clean: Zero private keys or sensitive files found."

echo ""
echo "========================================================"
echo "    ALL INDEPENDENT VERIFICATION CHECKS PASSED (OK)     "
echo "========================================================"
exit 0
