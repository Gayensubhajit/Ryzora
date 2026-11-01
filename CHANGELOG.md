# Changelog

All notable changes to Ryzora are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-rc1] - 2026-11-01

### Added
- **Native Qt 6 / QML + Rust Architecture**: Fully replaced legacy web/Tauri runtimes with a responsive Qt 6.8+ QML desktop interface powered by a memory-safe Rust core via C-ABI (`ryzora-qt-backend`).
- **Store Experience 3.0 & Discover**: Curated top hero carousel, categorical filtering (Themes, Lock Screens, Status Bars, Fastfetch), and responsive catalog grid.
- **Library 2.0 (Installed Application Manager)**: Unified management of officially installed software, AUR packages, and Flatpak runtimes/apps across user and system scopes. Includes Grid/List views, search, quick launch, and provider-aware uninstallation with optional data purging.
- **Updates Center**: Concurrent update discovery and transaction management for Pacman, AUR (yay/paru), and Flatpak. Supports individual and batch updates with live phase tracking.
- **First-Run Onboarding Wizard**: Automated capability probing, package provider discovery, initial theme selection, and non-repeating completion persistence.
- **Community Content Decoupling**: Lightweight core distribution package (~30 MiB compressed) decoupling heavy videos and wallpapers; on-demand asset downloader with cryptographic SHA-256 verification and local cache management.
- **Studio & Customization Engine**: Real-time previewing, Fastfetch Studio configuration generator, and Hyprlock / SDDM integration.
- **Atomic Backup & Restore Engine**: Byte-accurate filesystem snapshots before destructive transactions, automatic retention management, and fail-closed rollback guarantees.
- **Native Arch Linux Packaging**: Automated PKGBUILD script (`scripts/build-arch-package.sh`) generating `.pkg.tar.zst` packages with hardened Polkit policies and restricted helper utilities.

### Fixed
- **Settings Persistence Reliability (2.2)**: Resolved save failure by eliminating strict unknown-field rejections, migrating to authoritative `~/.config/ryzora/settings.json` with atomic rename and disk sync (`sync_all()`), and implementing automated backup recovery (`settings.json.bak`).
- **Switch Contrast & Visibility**: Redesigned OFF-state switch tracks and thumbs with high-contrast borders and elevated backgrounds across both Dark and Light themes.
- **Accent Color Engine**: Synchronized 7 accent swatches (Ocean Blue, Deep Silver, Amethyst, Emerald, Amber Fire, Rose, Cyan) with real-time UI token propagation and disk persistence.
- **C-ABI Safety**: Protected FFI entry points with `std::panic::catch_unwind` to prevent panics across ABI boundaries, returning structured JSON errors.
- **Storage Accounting**: Reconnected Community Content Manager button in Settings, displaying exact disk usage and cache cleanup actions.

