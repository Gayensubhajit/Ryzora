# Ryzora Security Architecture & Invariants

Ryzora is engineered from the ground up around a **zero-trust, fail-closed, and declarative-only** execution model. Unlike traditional Linux theming scripts that execute arbitrary shell code as your user, Ryzora fundamentally forbids runtime script execution.

---

## 1. The Core Security Guarantee: Zero Subprocess Execution

**Ryzora never executes shell scripts, hook scripts, binaries, or subprocesses during package installation, updates, or removal.**

- No `Command::new` in installer runtime paths.
- No `sh`, `bash`, `python`, or `perl` interpreters invoked on package payloads.
- No `sudo`, `pkexec`, or privilege escalation routines.
- Packages are strictly declarative data files (configs, themes, wallpapers, assets). Ryzora parses their metadata and applies them via pure filesystem copies and symlinks.

---

## 2. Declarative Packages Only

Every Ryzora package contains a `ryzora.json` manifest. Payloads are restricted to safe declarative asset types:
- Plain text configurations (`.conf`, `.css`, `.json`, `.toml`, `.yaml`, `.ini`)
- Raster and declarative images (`.png`, `.jpg`, `.jpeg`, `.webp`, `.svg`)
- Desktop entries (`.desktop`) with strictly sanitized keys

Any package containing executable ELF binaries, shared objects (`.so`), Python scripts, shell scripts, or binary payloads is **rejected unconditionally** by both `ManifestSynthesizer` and the runtime `Installer`.

---

## 3. Sandboxed Target Confinement

Every component target path must resolve strictly within an authorized component directory under `$HOME`.

### Allowed Targets by Component:
- **Hyprland**: `~/.config/hypr/**`
- **Waybar**: `~/.config/waybar/**`
- **Kitty**: `~/.config/kitty/**`
- **Alacritty**: `~/.config/alacritty/**`
- **Fastfetch**: `~/.config/fastfetch/**`
- **Hyprlock**: `~/.config/hypr/hyprlock.conf`
- **Wallpapers**: `~/Pictures/Wallpapers/**`
- **GTK Themes**: `~/.themes/<id>/**` or `~/.local/share/themes/<id>/**`
- **Icons**: `~/.icons/<id>/**` or `~/.local/share/icons/<id>/**`
- **KDE Plasma**: `~/.local/share/plasma/**`, `~/.local/share/color-schemes/**`

### Forbidden Targets:
- Any path containing path traversal sequences (`..`).
- Any path containing null bytes (`\0`).
- System paths (`/etc`, `/usr`, `/var`, `/bin`, `/boot`).
- Sensitive user directories (`~/.ssh`, `~/.gnupg`, `~/.bashrc`, `~/.profile`, `~/.local/bin`).
- Shell startup files or login hooks.

---

## 4. Symlink Containment

Symlinks within package archives or target structures are subject to rigorous checks:
- Symlinks must point **exclusively** to files within the same package installation boundary.
- Symlinks pointing to `/etc/shadow`, `~/.ssh/id_rsa`, or any location outside the component target are rejected.
- When an existing symlink at the destination points outside the package sandbox, Ryzora will never write through the symlink; the link is safely broken or replaced.

---

## 5. Active-Content Rejection (SVG Hardening)

SVG images can harbor XML-based active content attacks (XSS, script injection). Ryzora inspects all SVG files before staging:
- Rejects `<script>` tags.
- Rejects inline event handlers (`onload=`, `onclick=`, `onerror=`, etc.).
- Rejects `javascript:` and `data:text/html` URI schemes.
- Rejects `<foreignObject>` elements.
- Rejects invalid or truncated UTF-8 streams.

---

## 6. Archive Decompression Bounds

To prevent decompression bombs and resource exhaustion attacks:
- Maximum unpacked payload size: 100 MB.
- Maximum entry count: 1,000 files.
- Zip-slip prevention: Every entry's relative path is checked against canonical boundaries before writing.
- Duplicate archive entries are rejected.

---

## 7. Cryptographic Trust Architecture

Ryzora uses Ed25519 asymmetric cryptography over canonical tree hashes.

```text
Trust Tier Hierarchy:
┌──────────────────────────────────────────────┐
│  Tier 1: Official Root (Certified by Ryzora) │
├──────────────────────────────────────────────┤
│  Tier 2: Trusted Author (Verified Key ID)    │
├──────────────────────────────────────────────┤
│  Tier 3: Community / Unvetted (Unsigned)     │
└──────────────────────────────────────────────┘
```

- **Canonical Tree Hash**: Computed recursively over sorted normalized paths and SHA-256 file digests.
- **Fail-Closed Official Trust**: Official packages must verify against the embedded official root key. If the production root key is unconfigured or fails validation, Ryzora fails closed.
- **External Provider Isolation**: External items from GitHub, Rice, Wallpaper, KDE, GNOME, or Fastfetch providers default to Tier 3 (Community / Unvetted). Provider origin alone **never** confers trusted status.

---

## 8. Atomic Pre-Mutation Snapshots & Complete Rollback

Before any file is written, modified, or symlinked:
1. Ryzora inspects all destination paths that will be touched.
2. A cryptographic snapshot of existing files and symlink targets is archived to `~/.local/share/ryzora/snapshots/<id>/`.
3. If any failure occurs during staging or application, Ryzora triggers an atomic rollback:
   - Restores modified files from backup.
   - Deletes newly created files.
   - Re-creates removed symlinks.
   - Restores the exact prior state without leaving orphan files.

---

## 9. Deep-Link Protocol Safety (`ryzora://`)

Deep links are governed by a strict navigation grammar:
- `ryzora://package/<id>`
- `ryzora://repository/<id>`
- `ryzora://creator/<id>`
- `ryzora://category/<id>`

**Invariant**: Deep links only navigate within the application UI or display a package review modal. They **never** initiate automated installation, downloads, or state mutations.
