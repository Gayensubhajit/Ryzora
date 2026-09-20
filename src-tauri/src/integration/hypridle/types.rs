use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Structural parse safety status of the discovered configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypridleParseStatus {
    /// Clean configuration with zero malformed syntax or ambiguities.
    Valid,
    /// Parse succeeded with unknown blocks/properties or harmless notices, safe for read-only inspection.
    ValidWithWarnings,
    /// Malformed syntax, unclosed blocks, invalid values, conflicting duplicate general blocks, or duplicate keys within a block that make composition unsafe.
    Invalid,
}

/// A parsed general block with full provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HypridleGeneralBlock {
    pub start_line: usize,
    pub lock_cmd: Option<String>,
    pub before_sleep_cmd: Option<String>,
    pub after_sleep_cmd: Option<String>,
    pub extra_properties: HashMap<String, String>,
    pub raw_entries: Vec<(String, String)>,
    pub duplicate_keys: Vec<String>,
}

/// Supported properties in the hypridle general block (primary/first block).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct HypridleGeneralConfig {
    pub lock_cmd: Option<String>,
    pub before_sleep_cmd: Option<String>,
    pub after_sleep_cmd: Option<String>,
    pub extra_properties: HashMap<String, String>,
    pub raw_entries: Vec<(String, String)>,
    pub duplicate_keys: Vec<String>,
}

/// A parsed hypridle listener block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HypridleListener {
    pub start_line: usize,
    pub timeout: u64,
    pub on_timeout: String,
    pub on_resume: Option<String>,
    pub extra_properties: HashMap<String, String>,
    pub raw_entries: Vec<(String, String)>,
    pub duplicate_keys: Vec<String>,
}

/// Structured report of native Hypridle configuration discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HypridleDiscoveryReport {
    pub config_path: PathBuf,
    pub exists: bool,
    pub readable: bool,
    pub sha256: Option<String>,
    pub parse_status: HypridleParseStatus,
    /// Primary (first) general block.
    pub general: Option<HypridleGeneralConfig>,
    /// Any subsequent duplicate general blocks preserved with full contents and line numbers.
    pub duplicate_general_blocks: Vec<HypridleGeneralBlock>,
    pub listeners: Vec<HypridleListener>,
    pub duplicate_listener_timeouts: Vec<u64>,
    pub duplicate_general_properties: Vec<String>,
    pub duplicate_listener_properties: Vec<String>,
    pub warnings: Vec<String>,
    /// Systemd service status, populated only when systemd is explicitly queried.
    pub service_active: Option<bool>,
}
