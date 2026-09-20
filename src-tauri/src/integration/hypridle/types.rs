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

/// Validated high-level idle action.
///
/// Encodes feature intent rather than arbitrary shell commands.
/// Device specifiers (e.g. keyboard backlight or display panel) are optional
/// and parameterized so that host-specific capabilities are discovered by the
/// feature layer rather than hardcoded in the shared composer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleAction {
    /// Lock session via loginctl lock-session
    LockSession,
    /// Turn off monitor display power (DPMS off)
    DpmsOff,
    /// Suspend the system via systemctl suspend
    Suspend,
    /// Dim screen brightness to an integer percentage (1..100)
    DimScreen {
        percent: u8,
        device: Option<String>,
    },
    /// Turn off keyboard backlight
    TurnOffKeyboardBacklight {
        device: Option<String>,
    },
}

impl IdleAction {
    /// Helper constructor for generic screen dimming.
    pub fn dim_screen(percent: u8) -> Self {
        IdleAction::DimScreen {
            percent,
            device: None,
        }
    }

    /// Helper constructor for host-specific screen dimming.
    pub fn dim_screen_device(percent: u8, device: impl Into<String>) -> Self {
        IdleAction::DimScreen {
            percent,
            device: Some(device.into()),
        }
    }

    /// Helper constructor for generic keyboard backlight shutoff.
    pub fn turn_off_keyboard_backlight() -> Self {
        IdleAction::TurnOffKeyboardBacklight { device: None }
    }

    /// Helper constructor for host-specific keyboard backlight shutoff.
    pub fn turn_off_keyboard_backlight_device(device: impl Into<String>) -> Self {
        IdleAction::TurnOffKeyboardBacklight {
            device: Some(device.into()),
        }
    }

    /// Returns the optional device identifier associated with this action.
    pub fn device(&self) -> Option<&str> {
        match self {
            IdleAction::DimScreen { device, .. } => device.as_deref(),
            IdleAction::TurnOffKeyboardBacklight { device } => device.as_deref(),
            _ => None,
        }
    }

    /// Renders the validated enum action into a strictly known-safe command string.
    pub fn to_command_string(&self) -> String {
        match self {
            IdleAction::LockSession => "loginctl lock-session".to_string(),
            IdleAction::DpmsOff => "hyprctl dispatch dpms off".to_string(),
            IdleAction::Suspend => "systemctl suspend".to_string(),
            IdleAction::DimScreen { percent, device } => {
                let clamped = (*percent).clamp(1, 100);
                match device {
                    Some(dev) => format!("brightnessctl -sd {} set {}%", dev, clamped),
                    None => format!("brightnessctl -s set {}%", clamped),
                }
            }
            IdleAction::TurnOffKeyboardBacklight { device } => {
                match device {
                    Some(dev) => format!("brightnessctl -sd {} set 0", dev),
                    None => "brightnessctl -sd *::kbd_backlight set 0".to_string(),
                }
            }
        }
    }
}

/// Validated high-level idle resume action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleResumeAction {
    /// Turn monitor display power back on (DPMS on)
    DpmsOn,
    /// Restore screen brightness
    RestoreScreen {
        device: Option<String>,
    },
    /// Restore keyboard backlight
    RestoreKeyboardBacklight {
        device: Option<String>,
    },
}

impl IdleResumeAction {
    /// Helper constructor for generic screen brightness restoration.
    pub fn restore_screen() -> Self {
        IdleResumeAction::RestoreScreen { device: None }
    }

    /// Helper constructor for host-specific screen brightness restoration.
    pub fn restore_screen_device(device: impl Into<String>) -> Self {
        IdleResumeAction::RestoreScreen {
            device: Some(device.into()),
        }
    }

    /// Helper constructor for generic keyboard backlight restoration.
    pub fn restore_keyboard_backlight() -> Self {
        IdleResumeAction::RestoreKeyboardBacklight { device: None }
    }

    /// Helper constructor for host-specific keyboard backlight restoration.
    pub fn restore_keyboard_backlight_device(device: impl Into<String>) -> Self {
        IdleResumeAction::RestoreKeyboardBacklight {
            device: Some(device.into()),
        }
    }

    /// Returns the optional device identifier associated with this resume action.
    pub fn device(&self) -> Option<&str> {
        match self {
            IdleResumeAction::RestoreScreen { device } => device.as_deref(),
            IdleResumeAction::RestoreKeyboardBacklight { device } => device.as_deref(),
            _ => None,
        }
    }

    /// Renders the validated enum resume action into a strictly known-safe command string.
    pub fn to_command_string(&self) -> String {
        match self {
            IdleResumeAction::DpmsOn => "hyprctl dispatch dpms on".to_string(),
            IdleResumeAction::RestoreScreen { device } => {
                match device {
                    Some(dev) => format!("brightnessctl -rd {}", dev),
                    None => "brightnessctl -r".to_string(),
                }
            }
            IdleResumeAction::RestoreKeyboardBacklight { device } => {
                match device {
                    Some(dev) => format!("brightnessctl -rd {}", dev),
                    None => "brightnessctl -rd *::kbd_backlight".to_string(),
                }
            }
        }
    }
}

/// High-level validated intent for a single idle listener.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdleListenerIntent {
    pub timeout: u64,
    pub action: IdleAction,
    pub resume_action: Option<IdleResumeAction>,
}

/// Request to compose a unified Hypridle configuration from active feature intents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypridleCompositionRequest {
    pub native_discovery: HypridleDiscoveryReport,
    pub session_lock_active: bool,
    pub idle_management_active: bool,
    pub idle_intents: Vec<IdleListenerIntent>,
}

/// Generated artifacts resulting from a valid Hypridle composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposedHypridlePlan {
    pub overlay_path: PathBuf,
    pub overlay_content: String,
    pub dropin_path: PathBuf,
    pub dropin_content: String,
    pub legacy_dropin_to_migrate: Option<PathBuf>,
}
