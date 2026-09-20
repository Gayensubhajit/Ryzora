//! Ryzora SilentSDDM Integration — Phase S1+
//!
//! Manages the SilentSDDM SDDM theme as a Ryzora-owned provider.
//! Architecture:
//!   - Theme engine is installed once as shared infrastructure
//!   - Wallpapers are lazy-downloaded per user selection
//!   - Custom user videos live in Ryzora's own data dir, never inside the theme
//!   - SDDM activation is reversible via /etc/sddm.conf.d/zz-ryzora-theme.conf
//!   - NO coupling to integration/session_lock.rs — SDDM and Hyprland session lock are separate
//!
//! Submodules:
//!   - discovery: read-only host capability + state report (Phase S1)
//!   - engine:    theme engine install/uninstall (Phase S3)
//!   - assets:    CAS blob store, upstream wallpaper & custom video management (Phase S3)
//!   - lifecycle: SDDM activate/deactivate/test (Phase S4)

pub mod discovery;
pub mod engine;
pub mod assets;
