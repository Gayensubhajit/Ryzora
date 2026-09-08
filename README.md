# Ryzora 🌌
> **The Universal Linux Desktop Customization Platform**

Ryzora is a standalone native Linux desktop customization marketplace and configuration manager. It allows Linux users to safely discover, preview, install, manage, and rollback desktop customization components across any Linux desktop environment.

---

## 🚀 Features

- **Universal Linux Support**: Runs seamlessly on Hyprland, Sway, KDE Plasma, GNOME, XFCE, Cinnamon, COSMIC, and other window managers and desktop environments.
- **Complete Desktop Rices**: Discover and install bundled configurations containing window manager settings, bars, application launchers, terminal palettes, fastfetch layouts, and wallpapers in one click.
- **Modular Categories**:
  - 🎨 **Themes**: GTK, Qt, Icon sets, Cursor themes
  - 🖥️ **Rices**: Complete full-desktop visual overhauls
  - 📊 **Status Bars**: Waybar, Polybar, AGS widgets
  - ⚡ **Fastfetch**: Custom ASCII logos and system fetch layouts
  - 🔒 **Lockscreens**: Hyprlock, Swaylock, SDDM configurations
  - 🖼️ **Wallpapers**: Curated high-res dynamic wallpapers
  - 💻 **Terminal**: Starship prompts, Kitty, Alacritty color schemes
- **Automatic System Detection**: Rust-powered environment probe that detects your Linux distribution, active desktop/window manager, display server (Wayland vs. X11), and installed desktop tools.
- **Safety First Architecture**:
  - **Zero Arbitrary Scripts**: Never executes opaque `install.sh` scripts.
  - **Manifest Validation**: All packages use a typed, declaratively verified manifest format specifying exact file destinations and required dependencies.
  - **Pre-Installation Snapshots**: Automatically takes timestamped snapshots of modified directories (`~/.config/*`).
  - **1-Click Rollback**: Easily restore previous configurations if an installation doesn't fit your workflow.

---

## 🛠️ Tech Stack

- **Framework**: [Tauri 2](https://v2.tauri.app/)
- **Frontend**: React, TypeScript, Tailwind CSS, Lucide Icons
- **Backend / Native**: Rust
- **Platform**: Linux (Wayland & X11)

---

## 📦 Getting Started

### Prerequisites

Ensure you have the following installed on your Linux system:
- **Node.js** (v20+) and **npm**
- **Rust** and **Cargo** (1.80+)
- **WebKit2GTK** (webkit2gtk-4.1) and system build tools:
  ```bash
  # Arch / Garuda Linux:
  sudo pacman -S --needed base-devel webkit2gtk-4.1 librsvg
  ```

### Development

```bash
# Install frontend dependencies
npm install

# Run application in development mode (Tauri 2 + Vite)
npm run tauri dev
```

### Production Build

```bash
npm run tauri build
```

---

## 📄 License

Distributed under the [MIT License](LICENSE).
