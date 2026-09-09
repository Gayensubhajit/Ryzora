# Ryzora Package Authoring & Contribution Handbook

## 1. Core Philosophy: Pure Declarative Packaging

Unlike traditional script-driven dotfile installers (`curl ... | sh`), Ryzora uses pure declarative manifests.
- **No Scripts**: Never use `install.sh`, `post_install.sh`, or shell hooks.
- **No Sudo**: Packages only place files into user directories (`~/.config/*`, `~/.local/share/*`).
- **No Root Execution**: Attempts to write to `/etc`, `/usr`, or `/bin` are rejected with fatal security errors.
- **Snapshot Guarantee**: Ryzora automatically snapshots target directories before touching them.

---

## 2. Directory Structure

A standard Ryzora package directory:

```text
my-cool-rice/
├── ryzora.json          # Package manifest (or manifest.json)
├── release.sig          # Optional detached Ed25519 signature
├── README.md            # User-facing documentation
├── assets/
│   ├── preview.jpg      # Screenshot/hero image
│   └── icon.png         # Optional icon
└── files/
    ├── hyprland.conf    # Target config files
    └── waybar/
        ├── config.jsonc
        └── style.css
```

---

## 3. Manifest Reference (`ryzora.json`)

```json
{
  "ryzora_spec": "1",
  "id": "my-cool-rice",
  "name": "My Cool Rice",
  "version": "1.0.0",
  "author": "RiceCrafter",
  "package_type": "rice",
  "category": "rices",
  "description": "A clean futuristic Hyprland desktop with Waybar and Catppuccin accents",
  "tags": ["hyprland", "waybar", "catppuccin", "dark"],
  "homepage": "https://github.com/example/my-cool-rice",
  "license": "MIT",
  "compatibility": {
    "desktops": ["hyprland"],
    "sessions": ["wayland"],
    "distros": ["arch", "fedora", "ubuntu"],
    "required": ["hyprland", "waybar"],
    "optional": ["kitty", "fastfetch"]
  },
  "files": [
    {
      "source": "files/hyprland.conf",
      "target": "~/.config/hypr/hyprland.conf"
    },
    {
      "source": "files/waybar/config.jsonc",
      "target": "~/.config/waybar/config.jsonc"
    },
    {
      "source": "files/waybar/style.css",
      "target": "~/.config/waybar/style.css"
    }
  ]
}
```

### Safety Rules:
1. `source`: Must be a relative path inside the package directory. Path traversal (`../`) is strictly rejected.
2. `target`: Must begin with `~/` or be relative to the user's home directory.
3. Hidden dangerous targets like `~/.ssh`, `~/.bashrc`, `~/.profile` are audited and restricted.

---

## 4. Local Pre-flight Verification

Before submitting your package to the community repository, validate it with the `ryzora-ci` engine:

```bash
# Audit package structure, manifest schema, and security invariants
ryzora-ci audit ./my-cool-rice

# Optionally sign with your author key
ryzora-ci sign-package ./my-cool-rice ~/.config/ryzora/keys/author.key

# Verify cryptographic evaluation
ryzora-ci verify-package ./my-cool-rice
```

---

## 5. Submitting to Community Repository

1. Fork the Ryzora repository.
2. Place your package directory under `community/<package-id>/`.
3. Open a Pull Request. The GitHub Actions security audit gate will run `ryzora-ci audit` and evaluate trust tiers automatically.
