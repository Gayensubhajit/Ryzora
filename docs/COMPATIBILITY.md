# Ryzora Compatibility Matrix

Ryzora is designed to support the modern Linux desktop ecosystem, focusing on declarative desktop environments and Wayland compositors.

---

## 1. Desktop Environments & Compositors

| Desktop Environment / Compositor | Session Type | Support Level | Supported Features |
| :--- | :--- | :--- | :--- |
| **Hyprland** | Wayland | **Tier 1 (Full)** | Complete rices, window rules, keybindings, animations, decoration |
| **Sway** | Wayland | **Tier 1 (Full)** | Sway config, i3-compatible structures, bar configs |
| **KDE Plasma (6.x & 5.x)** | Wayland / X11 | **Tier 2 (Rich)** | Color schemes, look-and-feel packages, desktop themes, Aurorae |
| **GNOME (45+)** | Wayland / X11 | **Tier 2 (Rich)** | GTK 3/4 themes, icon themes, wallpapers |
| **XFCE** | X11 | **Tier 3 (Standard)**| GTK themes, icon packs, terminal palettes |
| **Universal** | Any | **Tier 1 (Full)** | Wallpapers, Fastfetch presets, Starship prompts |

---

## 2. Desktop Components & Utilities

| Component | Category | Supported Target | Detection Method |
| :--- | :--- | :--- | :--- |
| **Waybar** | Status Bar | `~/.config/waybar/` | Binary check `/usr/bin/waybar` |
| **Kitty** | Terminal | `~/.config/kitty/` | Binary check `/usr/bin/kitty` |
| **Alacritty** | Terminal | `~/.config/alacritty/` | Binary check `/usr/bin/alacritty` |
| **Fastfetch** | System Info | `~/.config/fastfetch/` | Binary check `/usr/bin/fastfetch` |
| **Hyprlock** | Lockscreen | `~/.config/hypr/` | Binary check `/usr/bin/hyprlock` |
| **Rofi / Wayland-Rofi** | App Launcher | `~/.config/rofi/` | Binary check `/usr/bin/rofi` |
| **Starship** | Shell Prompt | `~/.config/starship.toml` | Binary check `/usr/bin/starship` |

---

## 3. Distribution Support

| Distribution Family | Package Formats | Tested Distributions | Notes |
| :--- | :--- | :--- | :--- |
| **Arch Linux** | AUR (`ryzora`, `ryzora-bin`), PKGBUILD | Arch Linux, Garuda, Manjaro, EndeavourOS | Rolling release fully tested |
| **Debian / Ubuntu** | `.deb`, Tarball | Ubuntu 22.04 LTS, 24.04 LTS, Debian 12, Pop!_OS | Full WebKitGTK 4.1 compatibility |
| **Fedora / Red Hat** | AppImage, Tarball, Source | Fedora 39, 40 | Wayland native |
| **openSUSE** | AppImage, Tarball | openSUSE Tumbleweed, Leap | Full support |
| **Generic Linux** | AppImage, Source | NixOS, Void Linux, Alpine (glibc) | Self-contained AppImage |
