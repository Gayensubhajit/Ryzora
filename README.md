# Ryzora 🌌
> **The Universal Linux Desktop Customization Platform**

[![Release](https://img.shields.io/badge/release-v0.1.0-blue.svg)](https://github.com/Gayensubhajit/Ryzora/releases/tag/v0.1.0)
[![Security Invariant](https://img.shields.io/badge/security-zero--exec-emerald.svg)](docs/SECURITY.md)
[![License](https://img.shields.io/badge/license-MIT-purple.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20x86__64-orange.svg)](docs/COMPATIBILITY.md)
[![Architecture](https://img.shields.io/badge/trust-pure%20Ed25519-cyan.svg)](docs/ROOT_KEY_CEREMONY.md)

Ryzora is a native Linux desktop customization marketplace and declarative configuration manager. It allows Linux users to safely discover, preview, install, manage, and rollback desktop configurations (rices, bars, themes, terminals, lockscreens, and wallpapers) across any Linux desktop environment without compromising system security.

---

## 🛡️ Security Architecture & Invariants

Unlike traditional dotfile installers that pipe arbitrary scripts into a shell (`curl ... | sh`), Ryzora is built around uncompromising security invariants:

1. **Zero Privilege Escalation**: Absolutely NO `sudo`, `pkexec`, or setuid binaries. Ryzora operates exclusively within user-space.
2. **Zero Shell Execution**: Packages never execute shell scripts (`install.sh`, `post_install.sh`) or invoke system package managers (`pacman`, `apt`, `yay`). Customizations are strictly declarative file trees.
3. **Pure Declarative Manifests**: All file placements are typed, validated, and restricted to the user's home directory (`~/.config/*`, `~/.local/share/*`). Attempts to target `/etc`, `/usr`, or `/bin` are strictly blocked.
4. **Multi-Tier Pure Ed25519 Trust**: Packages are cryptographically signed and verified over canonical directory tree hashes (`release.sig`). Root trust anchors are kept air-gapped and separated from release signing keys.
5. **Atomic Snapshots & 1-Click Rollback**: Before any installation or update touches a file, a byte-accurate snapshot is taken. If anything fails, state is rolled back automatically.

---

## 🚀 Key Features

- **Universal Linux Support**: Native support for Hyprland, Sway, KDE Plasma, GNOME, XFCE, COSMIC, and other desktop environments on Wayland and X11.
- **7 Native Content Providers**: Integrated, lazy-staged catalogs for GitHub repos, Community submissions, Fastfetch configurations, Complete Rices, Wallpapers, KDE Plasma look-and-feel assets, and GNOME/GTK themes.
- **Discover Marketplace**: Explore curated community rices, themes, status bars, and terminal themes with high-res screenshots, ratings, and trust badges.
- **Dependency Solver**: Pure-Rust probe detects missing desktop binaries (e.g. `waybar`, `fastfetch`) and warns before installation without installing system packages.
- **Collections (`.ryzlist`)**: Export, share, and install reproducible sets of customizations with atomic transaction boundaries.
- **Integrity Health Dashboard**: Authoritative local filesystem audits against installed SHA-256 hashes to detect file tampering or accidental deletions.
- **Strict `ryzora://` Deep Links**: Safely open and inspect packages or creators (`ryzora://package/id`) with guaranteed zero automatic execution or mutation.
- **Offline & Cache Resilience**: Cached packages and local repositories remain browsable and installable even without an internet connection.

---

## 📦 Installation & Packaging

### AppImage (Universal Linux)
Download the standalone `Ryzora-0.1.0-x86_64.AppImage` from GitHub Releases, make it executable, and run:
```bash
chmod +x Ryzora-0.1.0-x86_64.AppImage
./Ryzora-0.1.0-x86_64.AppImage
```

To build an AppImage locally:
```bash
./scripts/build-appimage.sh
```

### Debian / Ubuntu / Pop!_OS (.deb)
Download `ryzora_0.1.0_amd64.deb` and install via `apt` or `dpkg`:
```bash
sudo dpkg -i ryzora_0.1.0_amd64.deb
# or
sudo apt-get install ./ryzora_0.1.0_amd64.deb
```

To build a `.deb` locally (pure user-space, zero `sudo` required):
```bash
./scripts/build-deb.sh
```

### Arch Linux / AUR
Ryzora is packaged for Arch Linux via `PKGBUILD`:
```bash
cd packaging/arch
makepkg -si
```
Or for pre-compiled binary package:
```bash
cd packaging/arch
makepkg -p PKGBUILD-bin -si
```

### Standalone Binary Tarball
Extract the release tarball and run directly:
```bash
tar -xzf ryzora-v0.1.0-linux-x86_64.tar.gz
./ryzora-v0.1.0-linux-x86_64/ryzora
```

---

## 🔐 Cryptographic Verification Guide

Every Ryzora release includes `SHA256SUMS.txt`, a detached Ed25519 signature `SHA256SUMS.txt.sig`, and a complete `RELEASE_MANIFEST.txt`.

### 1. Verify SHA-256 Checksums
```bash
sha256sum -c SHA256SUMS.txt
```

### 2. Verify Detached Signature with `ryzora-ci`
```bash
ryzora-ci verify-file SHA256SUMS.txt SHA256SUMS.txt.sig ryzora-release-0.1.0.pub
```

### 3. Independent All-In-One Verification Script
```bash
./scripts/verify-release-artifacts.sh dist/release-v0.1.0/ dist/release-v0.1.0/ryzora-release-0.1.0.pub
```

---

## 🛠️ Development & Building from Source

### Prerequisites
- **Node.js** (v20+) and **npm**
- **Rust** and **Cargo** (1.80+)
- **WebKit2GTK** (webkit2gtk-4.1), GTK3, and OpenSSL

### Build & Run
```bash
npm install
npm run tauri dev
```

### Run Tests & Verification
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
npm run build
```

---

## 📚 Documentation

- [Release Notes v0.1.0](docs/RELEASE_NOTES_v0.1.0.md)
- [Root Key Ceremony & Cryptographic Trust](docs/ROOT_KEY_CEREMONY.md)
- [Release Signing & Signature Schema](docs/RELEASE_SIGNING.md)
- [Installation Guide](docs/INSTALLATION.md)
- [Security Model & Threat Matrix](docs/SECURITY.md)
- [Compatibility & Supported Desktop Environments](docs/COMPATIBILITY.md)
- [Creator & Package Authoring Handbook](docs/CREATOR_GUIDE.md)
- [Versioning Policy](docs/VERSIONING.md)

---

## 📄 License

Distributed under the [MIT License](LICENSE).
