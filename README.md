# Ryzora 🌌
> **The Universal Linux Desktop Customization Platform**

Ryzora is a native Linux desktop customization marketplace and configuration manager. It allows Linux users to safely discover, preview, install, manage, and rollback desktop configurations (rices, bars, themes, terminals, lockscreens, and wallpapers) across any Linux desktop environment without compromising system security.

---

## 🛡️ Security Architecture & Invariants

Unlike traditional dotfile installers that pipe arbitrary scripts into a shell (`curl ... | sh`), Ryzora is built around uncompromising security invariants:

1. **Zero Privilege Escalation**: Absolutely NO `sudo`, `pkexec`, or setuid binaries. Ryzora operates exclusively within user-space.
2. **Zero Shell Execution**: Packages never execute shell scripts (`install.sh`, `post_install.sh`) or invoke system package managers (`pacman`, `apt`, `yay`).
3. **Pure Declarative Manifests**: All file placements are typed, validated, and restricted to the user's home directory (`~/.config/*`). Attempts to target `/etc`, `/usr`, or `/bin` are strictly blocked.
4. **Ed25519 Cryptographic Trust**: Packages are cryptographically signed and verified over canonical directory tree hashes (`release.sig`). Unsigned packages remain isolated in the Community tier.
5. **Atomic Snapshots & 1-Click Rollback**: Before any installation or update touches a file, a byte-accurate snapshot is taken. If anything fails, state is rolled back automatically.

---

## 🚀 Key Features

- **Universal Linux Support**: Native support for Hyprland, Sway, KDE Plasma, GNOME, XFCE, COSMIC, and other desktop environments on Wayland and X11.
- **Discover Marketplace**: Explore curated community rices, themes, status bars, and terminal themes with high-res screenshots, ratings, and trust badges.
- **Dependency Solver**: Pure-Rust probe detects missing desktop binaries (e.g. `waybar`, `fastfetch`) and warns before installation without installing system packages.
- **Collections (`.ryzlist`)**: Export, share, and install reproducible sets of customizations with atomic transaction boundaries.
- **Integrity Health Dashboard**: Authoritative local filesystem audits against installed SHA-256 hashes to detect file tampering or accidental deletions.
- **Ryzora Hub & Creator Profiles**: View trending creators, author statistics, and manage installed updates.
- **Offline & Cache Resilience**: Cached packages and local repositories remain browsable and installable even without an internet connection.

---

## 📦 Installation & Packaging

### Arch Linux / AUR
Ryzora is packaged for Arch Linux via `PKGBUILD`:
```bash
cd packaging/arch
makepkg -si
```

### AppImage (Universal Linux)
Download the standalone `Ryzora-x86_64.AppImage` from GitHub Releases, make it executable, and run:
```bash
chmod +x Ryzora-x86_64.AppImage
./Ryzora-x86_64.AppImage
```

To build an AppImage locally:
```bash
./scripts/build-appimage.sh
```

### Standalone Binary Tarball
Extract the release tarball and run directly:
```bash
tar -xzf ryzora-v0.1.0-linux-x86_64.tar.gz
./ryzora-v0.1.0-linux-x86_64/ryzora
```

---

## 🛠️ Development & Building from Source

### Prerequisites
- **Node.js** (v20+) and **npm**
- **Rust** and **Cargo** (1.80+)
- **WebKit2GTK** (webkit2gtk-4.1), GTK3, and OpenSSL:
  ```bash
  # Arch / Garuda Linux:
  sudo pacman -S --needed base-devel webkit2gtk-4.1 librsvg gtk3 openssl
  ```

### Development Mode
```bash
npm install
npm run tauri dev
```

### Run Tests & Security Verification
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
npm run build
```

---

## 📚 Documentation

- [Root Key Ceremony & Cryptographic Trust](docs/ROOT_KEY_CEREMONY.md)
- [Release Signing & Signature Schema](docs/RELEASE_SIGNING.md)
- [Package Authoring & Contribution Handbook](docs/PACKAGING_GUIDE.md)
- [Versioning & Compatibility Policy](docs/VERSIONING.md)

---

## 📄 License

Distributed under the [MIT License](LICENSE).
