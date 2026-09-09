# Ryzora Installation Guide

Ryzora is the modern, declarative, and cryptographically verified Linux desktop customization platform. It is distributed in multiple Linux packaging formats to support any modern Linux distribution.

---

## 1. Arch Linux / Manjaro / Garuda (AUR)

Ryzora is available on the Arch User Repository (AUR) in both source-built and precompiled binary packages.

### Option A: AUR Source Package (`ryzora`)
Compiles the application locally with full release optimizations:
```bash
yay -S ryzora
# or with paru:
paru -S ryzora
```

### Option B: AUR Precompiled Binary (`ryzora-bin`)
Installs the precompiled, cryptographically certified binary release:
```bash
yay -S ryzora-bin
# or with paru:
paru -S ryzora-bin
```

### Manual PKGBUILD Build:
```bash
git clone https://github.com/Gayensubhajit/Ryzora.git
cd Ryzora/packaging/arch
makepkg -si
```

---

## 2. Debian / Ubuntu / Linux Mint / Pop!_OS (`.deb`)

Download the official release `.deb` package from the [Releases page](https://github.com/Gayensubhajit/Ryzora/releases).

```bash
# Download latest release
wget https://github.com/Gayensubhajit/Ryzora/releases/latest/download/ryzora_0.1.0_amd64.deb

# Install package and dependencies
sudo apt update
sudo apt install -y ./ryzora_0.1.0_amd64.deb
```

To uninstall:
```bash
sudo apt remove ryzora
```

---

## 3. Generic Linux (AppImage)

The AppImage is a standalone bundle containing all application dependencies. It runs on any modern Linux distribution without root privileges.

```bash
# 1. Download AppImage
wget https://github.com/Gayensubhajit/Ryzora/releases/latest/download/Ryzora-0.1.0-x86_64.AppImage

# 2. Make executable
chmod +x Ryzora-0.1.0-x86_64.AppImage

# 3. Launch
./Ryzora-0.1.0-x86_64.AppImage
```

### Desktop Integration for AppImage:
You can use `appimaged` or `gearlever` to automatically integrate the AppImage with your application launcher and register the `ryzora://` URI scheme.

---

## 4. Building from Source

### Prerequisites:
- Rust (stable toolchain >= 1.75): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Node.js (>= 20) and npm
- System libraries:
  - Debian/Ubuntu: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`
  - Arch Linux: `sudo pacman -S webkit2gtk-4.1 gtk3 libayatana-appindicator openssl`
  - Fedora: `sudo dnf install webkit2gtk4.1-devel gtk3-devel libayatana-appindicator3-devel`

### Build Steps:
```bash
git clone https://github.com/Gayensubhajit/Ryzora.git
cd Ryzora

# Install frontend dependencies and build UI
npm ci
npm run build

# Build Rust backend with release optimizations
cargo build --manifest-path src-tauri/Cargo.toml --release --locked

# Binary location:
./src-tauri/target/release/ryzora
```

---

## 5. Verifying Release Integrity & Signatures

Every official Ryzora release includes a `SHA256SUMS.txt` checksum file and an Ed25519 signature `SHA256SUMS.txt.sig`.

```bash
# 1. Verify SHA-256 Checksums
sha256sum -c SHA256SUMS.txt

# 2. Verify Ed25519 Signature using ryzora-ci
ryzora-ci verify-file SHA256SUMS.txt SHA256SUMS.txt.sig official-release-pubkey.hex
```
