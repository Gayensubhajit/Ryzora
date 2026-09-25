//! SilentSDDM Layout & Visual Configuration Model
//!
//! Provides typed configuration representation matching upstream SilentSDDM options:
//! https://github.com/uiriansan/SilentSDDM/wiki/Options
//!
//! Exposes:
//! - Background settings (fill mode: fill, fit, stretch)
//! - Lock screen settings (background, blur, brightness, saturation, padding, clock, date, message)
//! - Login screen settings (background, blur, brightness, saturation, login area, avatar, menu buttons)
//! - Serialization to `ryzora-active.conf` INI syntax
//! - Persistence to `~/.local/share/ryzora/lockscreens/silentsddm/configuration.json`

use super::discovery::silentsddm_data_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const SUPPORTED_FILL_MODES: &[&str] = &["fill", "fit", "stretch"];
pub const SUPPORTED_CLOCK_POSITIONS: &[&str] = &[
    "top-left",
    "top-center",
    "top-right",
    "center-left",
    "center",
    "center-right",
    "bottom-left",
    "bottom-center",
    "bottom-right",
];
pub const SUPPORTED_LOGIN_AREA_POSITIONS: &[&str] = &["left", "center", "right"];
pub const SUPPORTED_AVATAR_SHAPES: &[&str] = &["circle", "square"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SilentSddmBackgroundSettings {
    #[serde(default = "default_fill_mode")]
    pub fill_mode: String,
}

fn default_fill_mode() -> String {
    "fill".to_string()
}

impl Default for SilentSddmBackgroundSettings {
    fn default() -> Self {
        Self {
            fill_mode: default_fill_mode(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SilentSddmClockSettings {
    #[serde(default = "default_true")]
    pub display: bool,
    #[serde(default = "default_clock_pos")]
    pub position: String,
    #[serde(default = "default_center")]
    pub align: String,
    #[serde(default = "default_clock_format")]
    pub format: String,
    #[serde(default = "default_clock_font_size")]
    pub font_size: u32,
    #[serde(default = "default_clock_font_weight")]
    pub font_weight: u32,
    #[serde(default = "default_white")]
    pub color: String,
}

fn default_true() -> bool {
    true
}

fn default_clock_pos() -> String {
    "top-center".to_string()
}

fn default_center() -> String {
    "center".to_string()
}

fn default_clock_format() -> String {
    "hh:mm".to_string()
}

fn default_clock_font_size() -> u32 {
    70
}

fn default_clock_font_weight() -> u32 {
    900
}

fn default_white() -> String {
    "#FFFFFF".to_string()
}

impl Default for SilentSddmClockSettings {
    fn default() -> Self {
        Self {
            display: true,
            position: default_clock_pos(),
            align: default_center(),
            format: default_clock_format(),
            font_size: default_clock_font_size(),
            font_weight: default_clock_font_weight(),
            color: default_white(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SilentSddmDateSettings {
    #[serde(default = "default_true")]
    pub display: bool,
    #[serde(default = "default_date_format")]
    pub format: String,
    #[serde(default = "default_date_font_size")]
    pub font_size: u32,
    #[serde(default = "default_white")]
    pub color: String,
    #[serde(default = "default_date_margin_top")]
    pub margin_top: i32,
}

fn default_date_format() -> String {
    "dddd, MMMM dd, yyyy".to_string()
}

fn default_date_font_size() -> u32 {
    14
}

fn default_date_margin_top() -> i32 {
    -15
}

impl Default for SilentSddmDateSettings {
    fn default() -> Self {
        Self {
            display: true,
            format: default_date_format(),
            font_size: default_date_font_size(),
            color: default_white(),
            margin_top: default_date_margin_top(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SilentSddmMessageSettings {
    #[serde(default = "default_true")]
    pub display: bool,
    #[serde(default = "default_message_pos")]
    pub position: String,
    #[serde(default = "default_center")]
    pub align: String,
    #[serde(default = "default_message_text")]
    pub text: String,
    #[serde(default = "default_message_font_size")]
    pub font_size: u32,
    #[serde(default = "default_white")]
    pub color: String,
}

fn default_message_pos() -> String {
    "bottom-center".to_string()
}

fn default_message_text() -> String {
    "Press any key".to_string()
}

fn default_message_font_size() -> u32 {
    12
}

impl Default for SilentSddmMessageSettings {
    fn default() -> Self {
        Self {
            display: true,
            position: default_message_pos(),
            align: default_center(),
            text: default_message_text(),
            font_size: default_message_font_size(),
            color: default_white(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilentSddmLockScreenSettings {
    #[serde(default = "default_true")]
    pub display: bool,
    pub background: String,
    #[serde(default)]
    pub use_background_color: bool,
    #[serde(default = "default_black")]
    pub background_color: String,
    #[serde(default = "default_blur")]
    pub blur: u32,
    #[serde(default)]
    pub brightness: f64,
    #[serde(default)]
    pub saturation: f64,
    #[serde(default)]
    pub padding_top: i32,
    #[serde(default)]
    pub padding_right: i32,
    #[serde(default)]
    pub padding_bottom: i32,
    #[serde(default)]
    pub padding_left: i32,
    #[serde(default)]
    pub clock: SilentSddmClockSettings,
    #[serde(default)]
    pub date: SilentSddmDateSettings,
    #[serde(default)]
    pub message: SilentSddmMessageSettings,
}

fn default_black() -> String {
    "#000000".to_string()
}

fn default_blur() -> u32 {
    32
}

impl Default for SilentSddmLockScreenSettings {
    fn default() -> Self {
        Self {
            display: true,
            background: "silentsddm-silvia".to_string(),
            use_background_color: false,
            background_color: default_black(),
            blur: default_blur(),
            brightness: 0.0,
            saturation: 0.0,
            padding_top: 0,
            padding_right: 0,
            padding_bottom: 0,
            padding_left: 0,
            clock: SilentSddmClockSettings::default(),
            date: SilentSddmDateSettings::default(),
            message: SilentSddmMessageSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SilentSddmLoginAreaSettings {
    #[serde(default = "default_center")]
    pub position: String,
    #[serde(default = "default_neg_one")]
    pub margin: i32,
}

fn default_neg_one() -> i32 {
    -1
}

impl Default for SilentSddmLoginAreaSettings {
    fn default() -> Self {
        Self {
            position: default_center(),
            margin: default_neg_one(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilentSddmAvatarSettings {
    #[serde(default = "default_avatar_shape")]
    pub shape: String,
    #[serde(default = "default_avatar_active_size")]
    pub active_size: u32,
    #[serde(default = "default_avatar_inactive_size")]
    pub inactive_size: u32,
    #[serde(default = "default_avatar_inactive_opacity")]
    pub inactive_opacity: f64,
}

fn default_avatar_shape() -> String {
    "circle".to_string()
}

fn default_avatar_active_size() -> u32 {
    120
}

fn default_avatar_inactive_size() -> u32 {
    80
}

fn default_avatar_inactive_opacity() -> f64 {
    0.35
}

impl Default for SilentSddmAvatarSettings {
    fn default() -> Self {
        Self {
            shape: default_avatar_shape(),
            active_size: default_avatar_active_size(),
            inactive_size: default_avatar_inactive_size(),
            inactive_opacity: default_avatar_inactive_opacity(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilentSddmLoginScreenSettings {
    pub background: String,
    #[serde(default)]
    pub use_background_color: bool,
    #[serde(default = "default_black")]
    pub background_color: String,
    #[serde(default)]
    pub blur: u32,
    #[serde(default)]
    pub brightness: f64,
    #[serde(default)]
    pub saturation: f64,
    #[serde(default)]
    pub login_area: SilentSddmLoginAreaSettings,
    #[serde(default)]
    pub avatar: SilentSddmAvatarSettings,
    #[serde(default)]
    pub session_position: Option<String>,
    #[serde(default)]
    pub layout_position: Option<String>,
    #[serde(default)]
    pub keyboard_position: Option<String>,
    #[serde(default)]
    pub power_position: Option<String>,
}

impl Default for SilentSddmLoginScreenSettings {
    fn default() -> Self {
        Self {
            background: "silentsddm-silvia".to_string(),
            use_background_color: false,
            background_color: default_black(),
            blur: 0,
            brightness: 0.0,
            saturation: 0.0,
            login_area: SilentSddmLoginAreaSettings::default(),
            avatar: SilentSddmAvatarSettings::default(),
            session_position: None,
            layout_position: None,
            keyboard_position: None,
            power_position: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SilentSddmConfiguration {
    #[serde(default)]
    pub background: SilentSddmBackgroundSettings,
    #[serde(default)]
    pub lock_screen: SilentSddmLockScreenSettings,
    #[serde(default)]
    pub login_screen: SilentSddmLoginScreenSettings,
}

impl Default for SilentSddmConfiguration {
    fn default() -> Self {
        Self {
            background: SilentSddmBackgroundSettings::default(),
            lock_screen: SilentSddmLockScreenSettings::default(),
            login_screen: SilentSddmLoginScreenSettings::default(),
        }
    }
}

impl SilentSddmConfiguration {
    pub fn validate(&self) -> Result<(), String> {
        if !SUPPORTED_FILL_MODES.contains(&self.background.fill_mode.as_str()) {
            return Err(format!(
                "Unsupported background-fill-mode '{}'. Supported: {}",
                self.background.fill_mode,
                SUPPORTED_FILL_MODES.join(", ")
            ));
        }

        if !SUPPORTED_CLOCK_POSITIONS.contains(&self.lock_screen.clock.position.as_str()) {
            return Err(format!(
                "Unsupported lock clock position '{}'. Supported: {}",
                self.lock_screen.clock.position,
                SUPPORTED_CLOCK_POSITIONS.join(", ")
            ));
        }

        if !SUPPORTED_CLOCK_POSITIONS.contains(&self.lock_screen.message.position.as_str()) {
            return Err(format!(
                "Unsupported lock message position '{}'. Supported: {}",
                self.lock_screen.message.position,
                SUPPORTED_CLOCK_POSITIONS.join(", ")
            ));
        }

        if !SUPPORTED_LOGIN_AREA_POSITIONS.contains(&self.login_screen.login_area.position.as_str()) {
            return Err(format!(
                "Unsupported login area position '{}'. Supported: {}",
                self.login_screen.login_area.position,
                SUPPORTED_LOGIN_AREA_POSITIONS.join(", ")
            ));
        }

        if !SUPPORTED_AVATAR_SHAPES.contains(&self.login_screen.avatar.shape.as_str()) {
            return Err(format!(
                "Unsupported avatar shape '{}'. Supported: {}",
                self.login_screen.avatar.shape,
                SUPPORTED_AVATAR_SHAPES.join(", ")
            ));
        }

        if self.lock_screen.blur > 100 || self.login_screen.blur > 100 {
            return Err("Blur value must be between 0 and 100".to_string());
        }

        if self.lock_screen.brightness < -1.0 || self.lock_screen.brightness > 1.0
            || self.login_screen.brightness < -1.0 || self.login_screen.brightness > 1.0 {
            return Err("Brightness value must be between -1.0 and 1.0".to_string());
        }

        if self.lock_screen.saturation < -1.0 || self.lock_screen.saturation > 1.0
            || self.login_screen.saturation < -1.0 || self.login_screen.saturation > 1.0 {
            return Err("Saturation value must be between -1.0 and 1.0".to_string());
        }

        Ok(())
    }

    pub fn to_ini_string(&self, lock_bg_filename: &str, login_bg_filename: &str) -> String {
        format!(
            r##"; Ryzora Active SilentSDDM Configuration
[General]
scale = 1.0
enable-animations = true
animated-background-placeholder = ""
background-fill-mode = "{fill_mode}"

[LockScreen]
display = {lock_display}
padding-top = {lock_pt}
padding-right = {lock_pr}
padding-bottom = {lock_pb}
padding-left = {lock_pl}
background = "{lock_bg}"
use-background-color = {lock_use_color}
background-color = "{lock_color}"
blur = {lock_blur}
brightness = {lock_brightness:.1}
saturation = {lock_saturation:.1}
input-keystroke = false
ignore-shift-key = false

[LockScreen.Clock]
display = {clock_display}
position = "{clock_pos}"
align = "{clock_align}"
format = "{clock_format}"
font-family = "RedHatDisplay"
font-size = {clock_size}
font-weight = {clock_weight}
color = "{clock_color}"

[LockScreen.Date]
display = {date_display}
format = "{date_format}"
locale = "en_US"
font-family = "RedHatDisplay"
font-size = {date_size}
font-weight = 600
color = "{date_color}"
margin-top = {date_margin}

[LockScreen.Message]
display = {msg_display}
position = "{msg_pos}"
align = "{msg_align}"
text = "{msg_text}"
font-family = "RedHatDisplay"
font-size = {msg_size}
font-weight = 400
display-icon = true
icon = "enter.svg"
icon-size = 16
color = "{msg_color}"
paint-icon = true
spacing = 0

[LoginScreen]
background = "{login_bg}"
use-background-color = {login_use_color}
background-color = "{login_color}"
blur = {login_blur}
brightness = {login_brightness:.1}
saturation = {login_saturation:.1}

[LoginScreen.LoginArea]
position = "{login_area_pos}"
margin = {login_area_margin}

[LoginScreen.LoginArea.Avatar]
shape = "{avatar_shape}"
border-radius = 35
active-size = {avatar_active}
inactive-size = {avatar_inactive}
inactive-opacity = {avatar_opacity:.2}
active-border-size = 0
inactive-border-size = 0
active-border-color = "#FFFFFF"
inactive-border-color = "#FFFFFF"
always-active = false

[LoginScreen.LoginArea.Username]
font-family = "RedHatDisplay"
font-size = 16
font-weight = 700
color = "#FFFFFF"
margin = 10

[LoginScreen.LoginArea.PasswordInput]
width = 200
height = 30
display-icon = true
font-family = "RedHatDisplay"
font-size = 12
icon = "password.svg"
icon-size = 16
content-color = "#FFFFFF"
background-color = "#FFFFFF"
background-opacity = 0.15
border-size = 0
border-color = "#FFFFFF"
border-radius-left = 10
border-radius-right = 10
margin-top = 10
masked-character = "●"

[LoginScreen.LoginArea.LoginButton]
background-color = "#FFFFFF"
background-opacity = 0.15
active-background-color = "#FFFFFF"
active-background-opacity = 0.30
icon = "arrow-right.svg"
icon-size = 18
content-color = "#FFFFFF"
active-content-color = "#FFFFFF"
border-size = 0
border-color = "#FFFFFF"
border-radius-left = 10
border-radius-right = 10
"##,
            fill_mode = self.background.fill_mode,
            lock_display = self.lock_screen.display,
            lock_pt = self.lock_screen.padding_top,
            lock_pr = self.lock_screen.padding_right,
            lock_pb = self.lock_screen.padding_bottom,
            lock_pl = self.lock_screen.padding_left,
            lock_bg = lock_bg_filename,
            lock_use_color = self.lock_screen.use_background_color,
            lock_color = self.lock_screen.background_color,
            lock_blur = self.lock_screen.blur,
            lock_brightness = self.lock_screen.brightness,
            lock_saturation = self.lock_screen.saturation,
            clock_display = self.lock_screen.clock.display,
            clock_pos = self.lock_screen.clock.position,
            clock_align = self.lock_screen.clock.align,
            clock_format = self.lock_screen.clock.format,
            clock_size = self.lock_screen.clock.font_size,
            clock_weight = self.lock_screen.clock.font_weight,
            clock_color = self.lock_screen.clock.color,
            date_display = self.lock_screen.date.display,
            date_format = self.lock_screen.date.format,
            date_size = self.lock_screen.date.font_size,
            date_color = self.lock_screen.date.color,
            date_margin = self.lock_screen.date.margin_top,
            msg_display = self.lock_screen.message.display,
            msg_pos = self.lock_screen.message.position,
            msg_align = self.lock_screen.message.align,
            msg_text = self.lock_screen.message.text,
            msg_size = self.lock_screen.message.font_size,
            msg_color = self.lock_screen.message.color,
            login_bg = login_bg_filename,
            login_use_color = self.login_screen.use_background_color,
            login_color = self.login_screen.background_color,
            login_blur = self.login_screen.blur,
            login_brightness = self.login_screen.brightness,
            login_saturation = self.login_screen.saturation,
            login_area_pos = self.login_screen.login_area.position,
            login_area_margin = self.login_screen.login_area.margin,
            avatar_shape = self.login_screen.avatar.shape,
            avatar_active = self.login_screen.avatar.active_size,
            avatar_inactive = self.login_screen.avatar.inactive_size,
            avatar_opacity = self.login_screen.avatar.inactive_opacity,
        )
    }
}

pub fn get_configuration_path(home: &Path) -> std::path::PathBuf {
    silentsddm_data_dir(home).join("configuration.json")
}

pub fn load_configuration(home: &Path) -> SilentSddmConfiguration {
    let path = get_configuration_path(home);
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(config) = serde_json::from_str::<SilentSddmConfiguration>(&content) {
            return config;
        }
    }
    SilentSddmConfiguration::default()
}

pub fn save_configuration(
    home: &Path,
    config: &SilentSddmConfiguration,
) -> Result<(), String> {
    config.validate()?;
    let path = get_configuration_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create configuration parent directory: {}", e))?;
    }
    let tmp = path.with_extension("tmp");
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize SilentSDDM configuration: {}", e))?;
    fs::write(&tmp, content)
        .map_err(|e| format!("Failed to write tmp configuration: {}", e))?;
    fs::rename(&tmp, &path)
        .map_err(|e| format!("Failed to finalize configuration: {}", e))?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation_passes() {
        let config = SilentSddmConfiguration::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_fill_fit_stretch_modes() {
        for mode in ["fill", "fit", "stretch"] {
            let mut config = SilentSddmConfiguration::default();
            config.background.fill_mode = mode.to_string();
            assert!(config.validate().is_ok());
            let ini = config.to_ini_string("lock.mp4", "login.mp4");
            assert!(ini.contains(&format!("background-fill-mode = \"{}\"", mode)));
        }

        let mut invalid = SilentSddmConfiguration::default();
        invalid.background.fill_mode = "zoom".to_string();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_nine_clock_positions() {
        let positions = [
            "top-left", "top-center", "top-right",
            "center-left", "center", "center-right",
            "bottom-left", "bottom-center", "bottom-right",
        ];
        for pos in positions {
            let mut config = SilentSddmConfiguration::default();
            config.lock_screen.clock.position = pos.to_string();
            assert!(config.validate().is_ok());
            let ini = config.to_ini_string("lock.jpg", "login.jpg");
            assert!(ini.contains(&format!("position = \"{}\"", pos)));
        }

        let mut invalid = SilentSddmConfiguration::default();
        invalid.lock_screen.clock.position = "diagonal".to_string();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_login_area_positions() {
        for pos in ["left", "center", "right"] {
            let mut config = SilentSddmConfiguration::default();
            config.login_screen.login_area.position = pos.to_string();
            assert!(config.validate().is_ok());
            let ini = config.to_ini_string("lock.mp4", "login.mp4");
            assert!(ini.contains(&format!("position = \"{}\"", pos)));
        }

        let mut invalid = SilentSddmConfiguration::default();
        invalid.login_screen.login_area.position = "top".to_string();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_separate_lock_and_login_backgrounds() {
        let mut config = SilentSddmConfiguration::default();
        config.lock_screen.background = "lock_custom.mp4".to_string();
        config.login_screen.background = "login_custom.jpg".to_string();
        let ini = config.to_ini_string("lock_custom.mp4", "login_custom.jpg");
        assert!(ini.contains("[LockScreen]"));
        assert!(ini.contains("background = \"lock_custom.mp4\""));
        assert!(ini.contains("[LoginScreen]"));
        assert!(ini.contains("background = \"login_custom.jpg\""));
    }

    #[test]
    fn test_visual_filter_ranges() {
        let mut config = SilentSddmConfiguration::default();
        config.lock_screen.blur = 101;
        assert!(config.validate().is_err());

        config.lock_screen.blur = 50;
        config.lock_screen.brightness = 1.5;
        assert!(config.validate().is_err());

        config.lock_screen.brightness = -0.5;
        config.lock_screen.saturation = -1.2;
        assert!(config.validate().is_err());

        config.lock_screen.saturation = 0.8;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = SilentSddmConfiguration::default();
        config.background.fill_mode = "fit".to_string();
        config.lock_screen.clock.position = "bottom-right".to_string();
        config.login_screen.login_area.position = "left".to_string();

        save_configuration(tmp.path(), &config).unwrap();
        let loaded = load_configuration(tmp.path());
        assert_eq!(config, loaded);
    }
}
