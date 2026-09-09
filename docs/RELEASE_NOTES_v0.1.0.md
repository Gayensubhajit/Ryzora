# Ryzora 0.1.0 — Initial Release Notes

**Release Date:** 2026-09-09  
**Version:** `0.1.0`  
**License:** MIT  
**Target Platform:** Linux (`x86_64`)  

---

## 🌟 Welcome to Ryzora

Ryzora is a high-performance, declarative, and cryptographically verified desktop customization platform for Linux. Designed to eliminate the hazards of arbitrary script execution (`curl | bash`), Ryzora replaces ad-hoc customization with verified manifests, atomic transactions, automatic rollback, and multi-tier cryptographic trust.

---

## 🛡️ Core Security Architecture & Invariants

1. **Zero Subprocess Execution (`Zero-Exec Invariant`):**
   Ryzora does not execute shell scripts, shell hooks, `sudo`, `pkexec`, or shell interpreters during customization installations. Customizations are strictly declarative file trees.

2. **Isolated Filesystem Sandboxing:**
   All applied configurations are confined strictly to authorized target directories (`~/.config`, `~/.local/share/themes`, `~/.local/share/icons`, `~/.local/share/wallpapers`, etc.). System paths (`/usr`, `/etc`, `/var`, `/bin`) and root modifications are unconditionally rejected.

3. **Atomic Transactions & Full Rollback:**
   Every customization installation creates a pre-apply filesystem snapshot. If validation, decompression, or placement fails, Ryzora rolls back changes atomically to preserve your desktop state.

4. **Multi-Tier Pure Ed25519 Cryptographic Trust:**
   - 🛡️ **Official Core:** Packages signed by the authoritative Ryzora Official Root Key.
   - 👤 **Verified Creator:** Packages signed by an author key registered in your local Trust Store.
   - ⚠️ **Community Unvetted:** Valid manifests without author verification, requiring explicit consent.
   - 🚫 **Revoked / Blocked:** Immediate quarantine for revoked keys or signature failures.

5. **Separation of Trust Anchors:**
   The Official Root Key is kept in an offline air-gapped environment and is **never** exposed for routine signing. Release artifacts are signed by an operational Release Signing Key that can be independently audited.

---

## 🧩 7 Native Content Providers

Ryzora aggregates Linux customization ecosystems into a single, cohesive, lazy-staged catalog:
- **GitHub Provider:** Direct declarative ingest of repositories with security boundary sandboxing.
- **Community Provider:** Curated community repositories and creator contributions.
- **Fastfetch Provider:** Beautiful, instant terminal system info presets and ASCII art configurations.
- **Complete Rice Provider:** Holistic multi-component desktop setups (window manager, bar, terminal, lockscreen).
- **Wallpaper Provider:** High-definition desktop backgrounds with colorway metadata.
- **KDE Plasma Provider:** Plasma desktop themes, color schemes, and icon assets.
- **GNOME / GTK Provider:** GTK themes, window controls, and shell styling.

---

## 📦 Distribution Packages

| Package Type | Filename | Description |
|---|---|---|
| **AppImage** | `Ryzora-0.1.0-x86_64.AppImage` | Standalone universal binary, runs on any modern Linux distribution without installation. |
| **Debian / Ubuntu** | `ryzora_0.1.0_amd64.deb` | Native Debian package for Ubuntu, Debian, Pop!_OS, Mint, etc. |
| **Arch Linux / AUR** | `ryzora-bin-0.1.0-1-x86_64.pkg.tar.zst` | Arch package compatible with Arch Linux, Garuda, Manjaro, EndeavourOS. |
| **Source Tarball** | `ryzora-0.1.0.tar.gz` | Clean source code bundle for custom distribution packaging. |
| **Binary Tarball** | `ryzora-v0.1.0-linux-x86_64.tar.gz` | Pre-compiled portable archive with desktop integration files and icons. |

---

## 🔐 Cryptographic Verification Guide

Every Ryzora release includes `SHA256SUMS.txt` and a detached Ed25519 signature `SHA256SUMS.txt.sig`.

### 1. Verify SHA-256 Checksums
```bash
sha256sum -c SHA256SUMS.txt
```

### 2. Verify Detached Signature with `ryzora-ci`
```bash
ryzora-ci verify-file SHA256SUMS.txt SHA256SUMS.txt.sig <RELEASE_PUBKEY_HEX>
```

### 3. Independent Verification Script
```bash
./scripts/verify-release-artifacts.sh dist/release-v0.1.0/ <RELEASE_PUBKEY_HEX>
```

---

## 🚀 Getting Started

Launch Ryzora:
```bash
ryzora
```
Or open via deep-link:
```bash
ryzora "ryzora://category/wallpapers"
```

For full installation and desktop integration instructions, see [docs/INSTALLATION.md](file:///home/silentbyte/Documents/Code%20Playground/Ryzora/docs/INSTALLATION.md).
