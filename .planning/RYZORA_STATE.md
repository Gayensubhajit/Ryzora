# Ryzora — Project State Handoff (2026-09-21)

> **Read this first** before touching any Ryzora code in a new chat.

---

## Where we are

| Item | Value |
|---|---|
| HEAD commit | `8bccf87` |
| Branch | `main` |
| Frontend tests | 174 / 174 passed |
| Backend tests | 535 / 535 passed |
| Build | clean |
| TypeScript | 0 errors |

---

## What is Ryzora?

A Tauri v2 (Rust + React/TypeScript) desktop **lock-screen store** for Arch/Hyprland.  
Users browse, install, apply, and deactivate lock-screen themes.

Two integration targets exist independently:

| Target key | What it controls | System |
|---|---|---|
| `quickshell` | Session lock (idle/lid-close) | Hyprland + Dusky hyprlock |
| `sddm` | Login screen (display manager) | SDDM |

---

## Host system state (DO NOT DISTURB)

```
~/.config/hypr/hypridle.conf   SHA256: 9d117906fb409c9f707a7d1408dbf8c6006e0c804b83194e9ce9b041a1b8c402
hypridle.service               ACTIVE (lid-close locking works)
/usr/share/polkit-1/rules.d/io.ryzora.sddm.rules   PRESENT
~/user_scripts/hyprlock/lock.sh   Dusky lock wrapper (DO NOT DELETE)
~/.local/share/ryzora/state/active_lockscreen.json  authoritative state
```

---

## Completed phases

| Phase | Commit | What it did |
|---|---|---|
| 3A | b9ae238 | Hypridle read-only discovery |
| 3B | e1aa4ec | Shared hypridle composer |
| 3C | 06224d0 | Transactional hypridle lifecycle (enable/disable with rollback) |
| 3D.1 | 8bccf87 | Target-aware ⋯ overflow menu + safe active uninstall |

---

## Key files

### Backend
- `src-tauri/src/installer.rs` — apply/deactivate/uninstall logic
- `src-tauri/src/integration/session_lock.rs` — Phase 3C transactional enable/disable
- `src-tauri/src/sddm_helper.rs` — SDDM polkit helper

### Frontend
- `src/views/LockScreenDetailView.tsx` — detail page controller
- `src/components/detail/ProductHero.tsx` — hero + Apply dropdown + ⋯ menu + uninstall modal
- `src/components/StoreCard.tsx` — store card ⋯ menu
- `src/components/PackageCard.tsx` — installed card ⋯ menu
- `src/context/AppContext.tsx` — IPC wrappers

### Tests
- `src/tests/target_aware_apply_and_card_navigation.test.ts` — 10 Phase 3D.1 scenarios

---

## IPC / Context API

```ts
applyLockscreen(id: string, target: "quickshell" | "sddm" | "both")
deactivateLockscreen(target: "quickshell" | "sddm" | "both")
testLockscreen(id: string, target: "quickshell" | "sddm")
deactivateAndUninstallLockscreen(id: string, activeTargets: ("quickshell"|"sddm")[])
loadInstalledPackages()
refreshActiveLockscreen()
```

---

## CSS token system

All colors use `--rz-*` CSS variables:
`--rz-surface`, `--rz-surface-elevated`, `--rz-surface-hover`,
`--rz-border-subtle`, `--rz-border-strong`,
`--rz-text`, `--rz-text-secondary`, `--rz-text-muted`,
`--rz-accent`, `--rz-accent-muted`

**Never hardcode colors. Always use theme tokens.**

---

## Hard constraints (NEVER break these)

1. **Never modify** `~/.config/hypr/hypridle.conf` — verify hash before and after any hypridle work
2. **Never call** `deactivateLockscreen("both")` when only one target is active — always dispatch to the specific active target
3. **Always deactivate before uninstalling** an active package — fail closed if deactivation fails
4. **SDDM writes** go through the polkit helper — never write SDDM config directly from user space
5. **Never delete** `~/user_scripts/hyprlock/lock.sh`
6. **Never touch** `10-ryoku-lid.conf` (lid-close config)
7. **Phase 3C session_lock.rs** — frozen, do not rewrite

---

## Run commands

```bash
npx tsc --noEmit                                              # type check
npm test -- --run                                             # frontend tests
npm run build                                                 # production bundle
cargo test --manifest-path src-tauri/Cargo.toml --lib        # backend tests
```

---

## Next logical work

Phase 3D or 3E: **Idle Management UI**
- Expose hypridle timeout settings in the lock-screen detail page
- Backend already has discovery (Phase 3A) and composer (Phase 3B)
- UI work: add a "Idle Settings" tab or section in LockScreenDetailView with timeout sliders
- Do NOT start by rewriting session_lock.rs or hypridle.rs
