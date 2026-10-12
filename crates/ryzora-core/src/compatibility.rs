//! Ryzora desktop compatibility specifications.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityRequirements {
    pub supported_distros: Vec<String>,
    pub supported_desktops: Vec<String>,
    pub supported_sessions: Vec<String>,
    pub required_binaries: Vec<String>,
    pub optional_binaries: Vec<String>,
}

impl Default for CompatibilityRequirements {
    fn default() -> Self {
        Self {
            supported_distros: Vec::new(),
            supported_desktops: Vec::new(),
            supported_sessions: vec!["wayland".to_string()],
            required_binaries: Vec::new(),
            optional_binaries: Vec::new(),
        }
    }
}
