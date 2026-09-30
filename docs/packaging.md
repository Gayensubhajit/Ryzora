# Ryzora Distribution & Packaging Architecture

This document specifies the authoritative distribution strategy, packaging formats, multi-ecosystem repository pipeline, and security invariants for Ryzora.

---

## 1. Overview & Strategy

Ryzora is a universal Linux desktop customization platform and software center. Because Linux has fragmented packaging systems, Ryzora uses a unified, reproducible source release pipeline that fans out to distinct distribution channels without duplicating core build logic.

### Phased Distribution Roadmap

```
Phase A: Initial Public Release (Self-Contained & User-Centric)
├── GitHub Releases (Universal Binaries + Checksums + Ed25519 Signatures)
├── AppImage (Universal standalone portable binary)
├── AUR (Arch User Repository — PKGBUILD recipe)
└── Flatpak (Flathub — Universal sandboxed desktop app)

Phase B: Multi-Ecosystem Repository Builds (via Open Build Service - OBS)
├── Arch Linux Repository
├── Fedora / RHEL (RPM via COPR & OBS)
├── openSUSE (RPM via OBS)
└── Debian / Ubuntu (Signed APT Repository via OBS)

Phase C: Upstream Distro Inclusion
├── Arch [extra] repository
├── Fedora Official Packages
└── Debian / Ubuntu Official Repositories
```

---

## 2. Packaging Layout

Packaging configurations live strictly under `packaging/` in the repository root:

```
packaging/
├── arch/
│   ├── PKGBUILD              # Clean Arch recipe building from release tarball
│   ├── PKGBUILD-bin          # Fast binary repackaging recipe
│   └── .SRCINFO              # AUR metadata cache
├── fedora/
│   └── ryzora.spec           # RPM specification for Fedora / openSUSE / COPR
├── debian/
│   ├── control               # Package metadata and dependencies
│   ├── rules                 # Debhelper build & install targets
│   └── changelog             # Debian package changelog
├── flatpak/
│   └── io.ryzora.Ryzora.yml  # Flathub / Flatpak manifest
└── appimage/
    ├── AppRun                # Runtime environment launcher
    └── build.sh              # Local build wrapper
```

---

## 3. Platform Packaging Details

### 3.1 Arch Linux / AUR
- **Format**: PKGBUILD with pacman compatibility
- **Commands**: `yay -S ryzora` or `paru -S ryzora`
- **Location**: `packaging/arch/PKGBUILD`
- **Build Mode**: Pure unprivileged `makepkg` building from tagged release tarball.
- **Dependencies**: GTK 3, WebKit2GTK 4.1, OpenSSL, Hicolor icon theme.

### 3.2 Fedora & openSUSE (RPM)
- **Format**: RPM spec (`packaging/fedora/ryzora.spec`)
- **Commands**:
  - Fedora: `sudo dnf install ryzora` (via COPR or OBS)
  - openSUSE: `sudo zypper install ryzora` (via OBS)
- **Tooling**: Built via Open Build Service (OBS) or Fedora COPR using mock chroot environments.

### 3.3 Debian & Ubuntu (DEB)
- **Format**: Debian package (`packaging/debian/`)
- **Commands**: `sudo apt install ryzora`
- **Repository Model**: Signed APT repository managed through OBS (`repo.ryzora.dev` or OBS debian pool).

### 3.4 Flatpak (Flathub)
- **App ID**: `io.ryzora.Ryzora`
- **Install**: `flatpak install flathub io.ryzora.Ryzora`
- **Sandbox Configuration & Security Contract**:
  - Wayland / X11 display socket
  - Network access for community catalog browsing and asset retrieval
  - User dotfile configuration namespaces (`~/.config/hypr`, `~/.config/waybar`, `~/.config/ryzora`)
  - **No Weakened Sandbox Escalation**: Flatpak runs purely in user space. Privileged operations (`ryzora-sddm-helper`) require host installation and are runtime capability-detected; the manifest does not request `--filesystem=host` or arbitrary host command execution.

### 3.5 AppImage
- **Format**: Standalone portable ELF binary (`Ryzora-x86_64.AppImage`)
- **Install / Run**:
  ```bash
  chmod +x Ryzora-x86_64.AppImage
  ./Ryzora-x86_64.AppImage
  ```
- **Builder**: `scripts/build-appimage.sh` constructs a valid `AppDir` with desktop integration, MIME types, and icons.

---

## 4. Privileged Operations & Security Architecture

Ryzora is not a simple sandboxed text editor; it performs configuration and theme activation for:
1. **User-level desktop environments**: Hyprland configs, Waybar styles, terminal color schemes, wallpaper services (Hyprpaper/Swaybg).
2. **Session lock engines**: Qylock, Hyprlock, Quickshell, Swaylock (user-space execution).
3. **Display managers**: SDDM login screen and lock screen themes (system-level privileged integration).

### Security Invariants:
- **No Direct Sudo Escalation**: Ryzora's desktop UI runs strictly as an unprivileged user process.
- **Narrow Polkit Helper**: Privileged SDDM operations are delegated to `ryzora-sddm-helper` guarded by `io.ryzora.sddm.policy`.
- **Fail-Closed & Rollback Guarantees**:
  - Pre-flight validation and SHA-256 asset checksumming before modifying any system config.
  - Staging to temporary locations before atomic replacement.
  - Automatic rollback on validation failure.
- **Flatpak Sandbox Isolation**: The Flatpak manifest only exposes user config paths and Polkit/Portal interfaces, preserving system security boundaries.

---

## 5. Automated CI/CD Release Pipeline

The single source of truth is a signed Git tag (`v*.*.*`).

```
git tag v1.0.0
       │
       ▼
.github/workflows/release.yml
       ├── 1. Build frontend & Tauri release binaries
       ├── 2. Package AppImage & Debian .deb artifacts
       ├── 3. Generate SHA256SUMS.txt
       ├── 4. Sign SHA256SUMS.txt using Ed25519 (ryzora-ci sign-file)
       ├── 5. Publish GitHub Release
       └── 6. Trigger OBS webhook / package sync
```
