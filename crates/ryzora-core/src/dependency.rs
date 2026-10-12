use serde::{Deserialize, Serialize};

pub fn default_true() -> bool {
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// Dependency Data Model (Phase 9 & 9.1)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Package,
    SystemBinary,
    DesktopCapability,
    RuntimeCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySpec {
    pub id: String,
    pub kind: DependencyKind,
    #[serde(default)]
    pub version_req: Option<String>,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
}
