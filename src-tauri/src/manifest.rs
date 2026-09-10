use crate::dependency::{DependencyKind, DependencySpec};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

// ─────────────────────────────────────────────────────────────────────────────
// Ryzora Package Manifest — spec version "1"
// ─────────────────────────────────────────────────────────────────────────────

/// All valid package types for Ryzora packages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    Rice,
    Theme,
    Waybar,
    Fastfetch,
    Lockscreen,
    Wallpaper,
    Terminal,
    Icon,
    Cursor,
    Font,
    Widget,
    Bundle,
}
impl std::fmt::Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PackageType::Rice => "rice",
            PackageType::Theme => "theme",
            PackageType::Waybar => "waybar",
            PackageType::Fastfetch => "fastfetch",
            PackageType::Lockscreen => "lockscreen",
            PackageType::Wallpaper => "wallpaper",
            PackageType::Terminal => "terminal",
            PackageType::Icon => "icon",
            PackageType::Cursor => "cursor",
            PackageType::Font => "font",
            PackageType::Widget => "widget",
            PackageType::Bundle => "bundle",
        };
        write!(f, "{}", s)
    }
}

/// Compatibility requirements embedded in a package manifest.
/// Uses package-author-facing field names (desktops/sessions/distros/required/optional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestCompatibility {
    /// Supported desktop environments / window managers. Empty = "universal".
    #[serde(default)]
    pub desktops: Vec<String>,

    /// Required display protocol ("wayland", "x11"). Empty = any.
    #[serde(default)]
    pub sessions: Vec<String>,

    /// Supported distros / families. Empty or ["all"] = universal.
    #[serde(default)]
    pub distros: Vec<String>,

    /// Binaries that MUST be present in PATH.
    #[serde(default)]
    pub required: Vec<String>,

    /// Binaries that improve the experience but are not mandatory.
    #[serde(default)]
    pub optional: Vec<String>,
}

/// A single file mapping inside a Ryzora package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestFile {
    /// Path relative to the package root (e.g. "files/hypr/hyprland.conf").
    pub source: String,

    /// Destination path on the user system. MUST start with "~/".
    /// Path traversal ("..") is forbidden.
    pub target: String,

    /// Human-readable description of what this file does.
    #[serde(default)]
    pub description: String,
}


/// Target-specific execution and configuration definition (e.g. Quickshell, SDDM).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetDefinition {
    #[serde(default = "default_true")]
    pub supported: bool,

    #[serde(default)]
    pub dependencies: Vec<String>,

    #[serde(default)]
    pub files: Vec<ManifestFile>,

    #[serde(default)]
    pub scope: Option<String>,

    #[serde(default)]
    pub entrypoint: Option<String>,
}

pub fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TargetEntry {
    Boolean(bool),
    Definition(TargetDefinition),
}

impl TargetEntry {
    pub fn to_definition(&self, target_name: &str) -> TargetDefinition {
        match self {
            TargetEntry::Boolean(b) => {
                let default_scope = if target_name == "sddm" { "system" } else { "user" };
                let default_deps = if *b {
                    vec![target_name.to_string()]
                } else {
                    vec![]
                };
                TargetDefinition {
                    supported: *b,
                    dependencies: default_deps,
                    files: vec![],
                    scope: Some(default_scope.to_string()),
                    entrypoint: None,
                }
            }
            TargetEntry::Definition(d) => d.clone(),
        }
    }
}

/// The authoritative Ryzora package manifest (spec v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RyzoraManifest {
    /// Unique package identifier (slug, e.g. "cyberpunk-neon-2077").
    pub id: String,

    /// Human-readable package name.
    pub name: String,

    /// Package version (semver, e.g. "1.0.0").
    pub version: String,

    /// Ryzora manifest specification version. Must be "1".
    pub ryzora_spec: String,

    /// Package author name or handle.
    pub author: String,

    /// Canonical package type.
    pub package_type: PackageType,

    /// Long description of the package.
    #[serde(default)]
    pub description: String,

    /// Searchable tags.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Hex color palette for UI previews.
    #[serde(default)]
    pub color_palette: Vec<String>,

    /// Compatibility requirements.
    pub compatibility: ManifestCompatibility,

    /// Files this package installs.
    #[serde(default)]
    pub files: Vec<ManifestFile>,

    /// Typed dependencies (Phase 9).
    #[serde(default)]
    pub dependencies: Vec<DependencySpec>,

    /// Target definitions for dual/multi-target packages (e.g. quickshell, sddm).
    #[serde(default)]
    pub targets: HashMap<String, TargetEntry>,
}

impl RyzoraManifest {

    pub fn get_target_definition(&self, target_name: &str) -> Option<TargetDefinition> {
        self.targets.get(target_name).map(|entry| entry.to_definition(target_name))
    }

    pub fn supports_target(&self, target_name: &str) -> bool {
        if let Some(def) = self.get_target_definition(target_name) {
            def.supported
        } else {
            false
        }
    }

    /// Resolve dependencies specific to a target selection ("quickshell", "sddm", "both", or None).
    pub fn target_dependencies(&self, target: Option<&str>) -> Result<Vec<DependencySpec>, String> {
        match target {
            None => {
                if self.targets.is_empty() {
                    Ok(self.all_dependencies())
                } else {
                    let mut result = Vec::new();
                    let mut seen = HashSet::new();
                    for (tname, entry) in &self.targets {
                        let def = entry.to_definition(tname);
                        if def.supported {
                            for dep_name in def.dependencies {
                                if !seen.contains(&dep_name) {
                                    seen.insert(dep_name.clone());
                                    result.push(DependencySpec {
                                        id: dep_name.clone(),
                                        kind: DependencyKind::SystemBinary,
                                        version_req: None,
                                        required: true,
                                        description: Some(format!("Required for target '{}': {}", tname, dep_name)),
                                    });
                                }
                            }
                        }
                    }
                    Ok(result)
                }
            }
            Some("quickshell") => {
                if !self.supports_target("quickshell") {
                    return Err(format!("Package '{}' does not support target 'quickshell'", self.id));
                }
                let def = self.get_target_definition("quickshell").unwrap();
                let deps = def.dependencies.into_iter().map(|dep_name| {
                    DependencySpec {
                        id: dep_name.clone(),
                        kind: DependencyKind::SystemBinary,
                        version_req: None,
                        required: true,
                        description: Some(format!("Required for Quickshell session lock: {}", dep_name)),
                    }
                }).collect();
                Ok(deps)
            }
            Some("sddm") => {
                if !self.supports_target("sddm") {
                    return Err(format!("Package '{}' does not support target 'sddm'", self.id));
                }
                let def = self.get_target_definition("sddm").unwrap();
                let deps = def.dependencies.into_iter().map(|dep_name| {
                    DependencySpec {
                        id: dep_name.clone(),
                        kind: DependencyKind::SystemBinary,
                        version_req: None,
                        required: true,
                        description: Some(format!("Required for SDDM login screen: {}", dep_name)),
                    }
                }).collect();
                Ok(deps)
            }
            Some("both") => {
                if !self.supports_target("quickshell") && !self.supports_target("sddm") {
                    return Err(format!("Package '{}' does not support target 'both'", self.id));
                }
                let mut result = Vec::new();
                let mut seen = HashSet::new();
                for t in &["quickshell", "sddm"] {
                    if let Some(def) = self.get_target_definition(t) {
                        if def.supported {
                            for dep_name in def.dependencies {
                                if !seen.contains(&dep_name) {
                                    seen.insert(dep_name.clone());
                                    result.push(DependencySpec {
                                        id: dep_name.clone(),
                                        kind: DependencyKind::SystemBinary,
                                        version_req: None,
                                        required: true,
                                        description: Some(format!("Required for target '{}': {}", t, dep_name)),
                                    });
                                }
                            }
                        }
                    }
                }
                Ok(result)
            }
            Some(other) => {
                if let Some(def) = self.get_target_definition(other) {
                    if !def.supported {
                        return Err(format!("Package '{}' target '{}' is marked unsupported", self.id, other));
                    }
                    let deps = def.dependencies.into_iter().map(|dep_name| {
                        DependencySpec {
                            id: dep_name.clone(),
                            kind: DependencyKind::SystemBinary,
                            version_req: None,
                            required: true,
                            description: Some(format!("Required for target '{}': {}", other, dep_name)),
                        }
                    }).collect();
                    Ok(deps)
                } else {
                    Err(format!("Package '{}' does not support target '{}'", self.id, other))
                }
            }
        }
    }

    /// Resolve files specific to target selection.
    pub fn target_files(&self, target: Option<&str>) -> Result<Vec<ManifestFile>, String> {
        match target {
            None => Ok(self.files.clone()),
            Some("quickshell") => {
                if !self.supports_target("quickshell") {
                    return Err(format!("Package '{}' does not support target 'quickshell'", self.id));
                }
                let def = self.get_target_definition("quickshell").unwrap();
                if !def.files.is_empty() {
                    Ok(def.files)
                } else {
                    let qfiles: Vec<ManifestFile> = self.files.iter()
                        .filter(|f| f.target.starts_with("~/"))
                        .cloned()
                        .collect();
                    if qfiles.is_empty() {
                        Ok(self.files.clone())
                    } else {
                        Ok(qfiles)
                    }
                }
            }
            Some("sddm") => {
                if !self.supports_target("sddm") {
                    return Err(format!("Package '{}' does not support target 'sddm'", self.id));
                }
                let def = self.get_target_definition("sddm").unwrap();
                if !def.files.is_empty() {
                    Ok(def.files)
                } else {
                    let sddm_files: Vec<ManifestFile> = self.files.iter()
                        .filter(|f| f.target.starts_with("/usr/share/sddm/themes/"))
                        .cloned()
                        .collect();
                    if !sddm_files.is_empty() {
                        Ok(sddm_files)
                    } else {
                        let theme_id = self.id.trim_start_matches("lockscreen-qylock-").trim_start_matches("lockscreen-");
                        Ok(vec![
                            ManifestFile {
                                source: "files/config".to_string(),
                                target: format!("/usr/share/sddm/themes/ryzora-{}/Main.qml", theme_id),
                                description: format!("SDDM greeter QML for {}", self.name),
                            },
                            ManifestFile {
                                source: "files/config".to_string(),
                                target: format!("/usr/share/sddm/themes/ryzora-{}/metadata.desktop", theme_id),
                                description: format!("SDDM theme metadata for {}", self.name),
                            }
                        ])
                    }
                }
            }
            Some("both") => {
                let mut all_files = Vec::new();
                if self.supports_target("quickshell") {
                    let mut qf = self.target_files(Some("quickshell"))?;
                    all_files.append(&mut qf);
                }
                if self.supports_target("sddm") {
                    let mut sf = self.target_files(Some("sddm"))?;
                    all_files.append(&mut sf);
                }
                if all_files.is_empty() {
                    return Err(format!("Package '{}' has no files for target 'both'", self.id));
                }
                Ok(all_files)
            }
            Some(other) => {
                if let Some(def) = self.get_target_definition(other) {
                    if !def.supported {
                        return Err(format!("Target '{}' is unsupported for package '{}'", other, self.id));
                    }
                    if !def.files.is_empty() {
                        Ok(def.files)
                    } else {
                        Ok(self.files.clone())
                    }
                } else {
                    Err(format!("Package '{}' does not support target '{}'", self.id, other))
                }
            }
        }
    }

    /// Return all dependencies, merging explicit  with legacy
    /// , , ,
    /// and .
    pub fn all_dependencies(&self) -> Vec<DependencySpec> {
        let mut result = Vec::new();
        let mut existing: HashSet<(String, DependencyKind)> = HashSet::new();

        // 0. Explicit dependencies deduplicated
        for dep in &self.dependencies {
            let key = (dep.id.clone(), dep.kind.clone());
            if !existing.contains(&key) {
                existing.insert(key);
                result.push(dep.clone());
            }
        }

        // 1. Required system binaries
        for req in &self.compatibility.required {
            let key = (req.clone(), DependencyKind::SystemBinary);
            if !existing.contains(&key) {
                existing.insert(key);
                result.push(DependencySpec {
                    id: req.clone(),
                    kind: DependencyKind::SystemBinary,
                    version_req: None,
                    required: true,
                    description: Some(format!("System binary required in PATH: {}", req)),
                });
            }
        }

        // 2. Optional system binaries
        for opt in &self.compatibility.optional {
            let key = (opt.clone(), DependencyKind::SystemBinary);
            if !existing.contains(&key) {
                existing.insert(key);
                result.push(DependencySpec {
                    id: opt.clone(),
                    kind: DependencyKind::SystemBinary,
                    version_req: None,
                    required: false,
                    description: Some(format!("Optional system binary: {}", opt)),
                });
            }
        }

        // 3. Desktop / WM capabilities
        // If a single desktop is specified, treat as mandatory.
        // If multiple desktops are listed (e.g. ["hyprland", "sway", "cosmic"]), they are alternative targets,
        // so they are evaluated as compatibility alternatives rather than co-requisite dependencies.
        if self.compatibility.desktops.len() == 1 {
            for dt in &self.compatibility.desktops {
                let dt_lower = dt.to_lowercase();
                if dt_lower != "universal" && dt_lower != "all" {
                    let key = (dt.clone(), DependencyKind::DesktopCapability);
                    if !existing.contains(&key) {
                        existing.insert(key);
                        result.push(DependencySpec {
                            id: dt.clone(),
                            kind: DependencyKind::DesktopCapability,
                            version_req: None,
                            required: true,
                            description: Some(format!("Desktop/WM capability: {}", dt)),
                        });
                    }
                }
            }
        }

        // 4. Session protocol capabilities
        for sess in &self.compatibility.sessions {
            let sess_lower = sess.to_lowercase();
            if sess_lower != "any" && sess_lower != "all" {
                let key = (sess.clone(), DependencyKind::DesktopCapability);
                if !existing.contains(&key) {
                    existing.insert(key);
                    result.push(DependencySpec {
                        id: sess.clone(),
                        kind: DependencyKind::DesktopCapability,
                        version_req: None,
                        required: true,
                        description: Some(format!("Display session capability: {}", sess)),
                    });
                }
            }
        }

        result
    }

    /// Format this manifest as canonical, pretty-printed JSON.
    pub fn to_canonical_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Structured result from manifest validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Validate a semver string of the form "X.Y.Z".
fn is_valid_semver(v: &str) -> bool {
    semver::Version::parse(v).is_ok()
}

/// Validate that a file target path is safe.
/// - Must start with "~/"
/// - Must not contain ".." segments
/// - Must not contain null bytes
fn validate_target_path(path: &str) -> Result<(), String> {
    if !path.starts_with("~/") {
        return Err(format!(
            "Target '{}' must start with '~/' (home-relative paths only)",
            path
        ));
    }
    if path.contains('\0') {
        return Err(format!("Target '{}' contains null byte", path));
    }
    for segment in path.split('/') {
        if segment == ".." {
            return Err(format!(
                "Target '{}' contains path traversal ('..') — rejected for safety",
                path
            ));
        }
    }
    Ok(())
}

/// Validate that a file source path does not escape the package root.
fn validate_source_path(path: &str) -> Result<(), String> {
    if path.starts_with('/') {
        return Err(format!(
            "Source '{}' must be a relative path within the package",
            path
        ));
    }
    if path.contains('\0') {
        return Err(format!("Source '{}' contains null byte", path));
    }
    for segment in path.split('/') {
        if segment == ".." {
            return Err(format!(
                "Source '{}' escapes the package root with '..' — rejected for safety",
                path
            ));
        }
    }
    Ok(())
}

/// Forbidden manifest fields that indicate shell-execution attempts.
const FORBIDDEN_FIELDS: &[&str] = &[
    "scripts",
    "hooks",
    "install",
    "pre_install",
    "post_install",
    "uninstall",
    "run",
    "exec",
    "shell",
    "cmd",
];

/// Validate a Ryzora manifest JSON string. Returns a structured result.
/// This is a read-only operation — nothing is written to disk.
pub fn validate_manifest_internal(manifest_json: &str) -> ManifestValidationResult {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Step 1: Parse as generic JSON to check for forbidden fields
    let raw: Value = match serde_json::from_str(manifest_json) {
        Ok(v) => v,
        Err(e) => {
            return ManifestValidationResult {
                valid: false,
                errors: vec![format!("Invalid JSON: {}", e)],
                warnings: vec![],
            };
        }
    };

    if let Value::Object(ref map) = raw {
        for field in FORBIDDEN_FIELDS {
            if map.contains_key(*field) {
                errors.push(format!(
                    "Forbidden field '{}' detected — Ryzora manifests must not contain shell execution fields",
                    field
                ));
            }
        }
    }

    if !errors.is_empty() {
        return ManifestValidationResult {
            valid: false,
            errors,
            warnings,
        };
    }

    // Step 2: Deserialize into typed struct
    let manifest: RyzoraManifest = match serde_json::from_str(manifest_json) {
        Ok(m) => m,
        Err(e) => {
            return ManifestValidationResult {
                valid: false,
                errors: vec![format!("Manifest schema error: {}", e)],
                warnings: vec![],
            };
        }
    };

    // Step 3: Field-level validation
    if manifest.id.trim().is_empty() {
        errors.push("Field 'id' must not be empty".to_string());
    } else if manifest.id.contains('/') || manifest.id.contains('\\') {
        errors.push(format!(
            "Field 'id' ('{}') must not contain path separators",
            manifest.id
        ));
    }

    if manifest.name.trim().is_empty() {
        errors.push("Field 'name' must not be empty".to_string());
    }

    if manifest.author.trim().is_empty() {
        errors.push("Field 'author' must not be empty".to_string());
    }

    if manifest.ryzora_spec != "1" {
        errors.push(format!(
            "Field 'ryzora_spec' is '{}' — only spec version '1' is supported",
            manifest.ryzora_spec
        ));
    }

    if !is_valid_semver(&manifest.version) {
        errors.push(format!(
            "Field 'version' ('{}') must be semver (X.Y.Z)",
            manifest.version
        ));
    }

    // Step 4: File entry safety validation
    if manifest.files.len() > 200 {
        warnings.push(format!(
            "Package declares {} files — this is unusually large",
            manifest.files.len()
        ));
    }

    for (i, file) in manifest.files.iter().enumerate() {
        if let Err(e) = validate_target_path(&file.target) {
            errors.push(format!("files[{}]: {}", i, e));
        }
        if let Err(e) = validate_source_path(&file.source) {
            errors.push(format!("files[{}]: {}", i, e));
        }
        if file.source.trim().is_empty() {
            errors.push(format!("files[{}]: 'source' must not be empty", i));
        }
    }

    // Step 5: Compatibility sanity warnings
    for session in &manifest.compatibility.sessions {
        let s = session.to_lowercase();
        if s != "wayland" && s != "x11" && s != "any" {
            warnings.push(format!(
                "compatibility.sessions: '{}' is not a recognised session type (wayland, x11, any)",
                session
            ));
        }
    }

    // Step 6: Dependency validation (Phase 9.1 hardened)
    if manifest.dependencies.len() > 64 {
        errors.push(format!(
            "Package declares {} dependencies — exceeds limit of 64",
            manifest.dependencies.len()
        ));
    }

    let mut seen_dep_keys = HashSet::new();
    for (i, dep) in manifest.dependencies.iter().enumerate() {
        let trimmed_id = dep.id.trim();
        if trimmed_id.is_empty() {
            errors.push(format!("dependencies[{}]: 'id' must not be empty", i));
        } else if trimmed_id.len() > 64 {
            errors.push(format!(
                "dependencies[{}]: 'id' exceeds maximum length of 64 characters",
                i
            ));
        } else if trimmed_id.contains("..")
            || !trimmed_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
        {
            errors.push(format!(
                "dependencies[{}]: 'id' ('{}') contains invalid characters or path traversal — only alphanumeric, dashes, underscores, dots, and colons are allowed",
                i, dep.id
            ));
        }

        let key = (trimmed_id.to_string(), dep.kind.clone());
        if !seen_dep_keys.insert(key) {
            warnings.push(format!(
                "dependencies[{}]: duplicate dependency specification for '{:?}:{}' — will be deduplicated",
                i, dep.kind, dep.id
            ));
        }

        if let Some(ref req_str) = dep.version_req {
            if req_str.len() > 64 {
                errors.push(format!(
                    "dependencies[{}]: 'version_req' exceeds maximum length of 64 characters",
                    i
                ));
            } else if semver::VersionReq::parse(req_str).is_err() {
                errors.push(format!(
                    "dependencies[{}]: 'version_req' ('{}') is not a valid SemVer requirement",
                    i, req_str
                ));
            }
        }
    }

    ManifestValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

/// Parse and validate a Ryzora manifest JSON string. Read-only.
#[tauri::command]
pub fn validate_manifest(manifest_json: String) -> ManifestValidationResult {
    validate_manifest_internal(&manifest_json)
}

/// Parse a Ryzora manifest JSON string into a typed struct.
#[tauri::command]
pub fn parse_manifest(manifest_json: String) -> Result<RyzoraManifest, String> {
    let result = validate_manifest_internal(&manifest_json);
    if !result.valid {
        return Err(result.errors.join("; "));
    }
    serde_json::from_str(&manifest_json).map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> String {
        let s = concat!(
            r#"{"id":"cyberpunk-neon-2077","name":"Cyberpunk Neon 2077","#,
            r#""version":"1.0.0","ryzora_spec":"1","author":"Ryzora Community","#,
            r#""package_type":"rice","description":"A rice.","tags":[],"color_palette":[],"#,
            r#""compatibility":{"desktops":["hyprland"],"sessions":["wayland"],"#,
            r#""distros":[],"required":["hyprland","waybar"],"optional":["rofi"]},"#,
            r#""files":[{"source":"files/hypr/hyprland.conf","target":"~/.config/hypr/hyprland.conf","description":"Main config"}]}"#
        );
        s.to_string()
    }

    #[test]
    fn test_valid_manifest_parses_correctly() {
        let result = validate_manifest_internal(&valid_json());
        assert!(result.valid, "Expected valid; errors: {:?}", result.errors);
        let m: RyzoraManifest = serde_json::from_str(&valid_json()).unwrap();
        assert_eq!(m.id, "cyberpunk-neon-2077");
        assert_eq!(m.package_type, PackageType::Rice);
        assert_eq!(m.files.len(), 1);
        assert_eq!(m.files[0].target, "~/.config/hypr/hyprland.conf");
    }

    #[test]
    fn test_missing_ryzora_spec_rejected() {
        let json = concat!(
            r#"{"id":"p","name":"P","version":"1.0.0","author":"A","package_type":"theme","#,
            r#""compatibility":{"desktops":[],"sessions":[],"distros":[],"required":[],"optional":[]},"files":[]}"#
        );
        let result = validate_manifest_internal(json);
        assert!(!result.valid, "Should fail without ryzora_spec");
    }

    #[test]
    fn test_target_path_traversal_rejected() {
        let json = concat!(
            r#"{"id":"evil","name":"Evil","version":"1.0.0","ryzora_spec":"1","author":"A","package_type":"theme","#,
            r#""compatibility":{"desktops":[],"sessions":[],"distros":[],"required":[],"optional":[]},"#,
            r#""files":[{"source":"files/evil.conf","target":"~/.config/../../etc/passwd","description":"bad"}]}"#
        );
        let result = validate_manifest_internal(json);
        assert!(!result.valid, "Path traversal in target should be rejected");
        assert!(
            result.errors.iter().any(|e| e.contains("..")),
            "Got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_absolute_target_rejected() {
        let json = concat!(
            r#"{"id":"abs","name":"Abs","version":"1.0.0","ryzora_spec":"1","author":"A","package_type":"theme","#,
            r#""compatibility":{"desktops":[],"sessions":[],"distros":[],"required":[],"optional":[]},"#,
            r#""files":[{"source":"files/c","target":"/etc/shadow","description":"abs"}]}"#
        );
        let result = validate_manifest_internal(json);
        assert!(!result.valid, "Absolute target should be rejected");
        assert!(
            result.errors.iter().any(|e| e.contains("~/")),
            "Got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_unknown_package_type_rejected() {
        let json = concat!(
            r#"{"id":"bad","name":"Bad","version":"1.0.0","ryzora_spec":"1","author":"A","package_type":"malware","#,
            r#""compatibility":{"desktops":[],"sessions":[],"distros":[],"required":[],"optional":[]},"files":[]}"#
        );
        let result = validate_manifest_internal(json);
        assert!(
            !result.valid,
            "Unknown package_type should fail; errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_shell_hook_field_rejected() {
        let json = concat!(
            r#"{"id":"hook","name":"Hook","version":"1.0.0","ryzora_spec":"1","author":"A","package_type":"rice","#,
            r#""scripts":{"install":"rm -rf ~/"},"#,
            r#""compatibility":{"desktops":[],"sessions":[],"distros":[],"required":[],"optional":[]},"files":[]}"#
        );
        let result = validate_manifest_internal(json);
        assert!(!result.valid, "Shell hook fields should be rejected");
        assert!(
            result.errors.iter().any(|e| e.contains("scripts")),
            "Got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_source_path_traversal_rejected() {
        let json = concat!(
            r#"{"id":"src","name":"Src","version":"1.0.0","ryzora_spec":"1","author":"A","package_type":"theme","#,
            r#""compatibility":{"desktops":[],"sessions":[],"distros":[],"required":[],"optional":[]},"#,
            r#""files":[{"source":"../../../etc/passwd","target":"~/.config/stolen.conf","description":"bad"}]}"#
        );
        let result = validate_manifest_internal(json);
        assert!(!result.valid, "Source path traversal should be rejected");
        assert!(
            result.errors.iter().any(|e| e.contains("..")),
            "Got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_invalid_semver_rejected() {
        let json = concat!(
            r#"{"id":"bv","name":"BV","version":"not-semver","ryzora_spec":"1","author":"A","package_type":"theme","#,
            r#""compatibility":{"desktops":[],"sessions":[],"distros":[],"required":[],"optional":[]},"files":[]}"#
        );
        let result = validate_manifest_internal(json);
        assert!(!result.valid, "Invalid semver should be rejected");
        assert!(
            result.errors.iter().any(|e| e.contains("semver")),
            "Got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_phase9_dependencies_parsing_and_mapping() {
        let json = r#"{
            "id": "rice-with-deps",
            "name": "Rice with Deps",
            "version": "1.0.0",
            "ryzora_spec": "1",
            "author": "Tester",
            "package_type": "rice",
            "compatibility": {
                "desktops": ["hyprland"],
                "sessions": ["wayland"],
                "distros": [],
                "required": ["hyprland", "waybar"],
                "optional": ["rofi"]
            },
            "dependencies": [
                {
                    "id": "catppuccin-gtk",
                    "kind": "package",
                    "version_req": "^1.0.0",
                    "required": true,
                    "description": "GTK theme package"
                }
            ],
            "files": []
        }"#;

        let result = validate_manifest_internal(json);
        assert!(result.valid, "Errors: {:?}", result.errors);

        let manifest: RyzoraManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dependencies[0].id, "catppuccin-gtk");

        // Test all_dependencies mapping
        let all = manifest.all_dependencies();
        assert!(all
            .iter()
            .any(|d| d.id == "catppuccin-gtk" && d.kind == DependencyKind::Package));
        assert!(all
            .iter()
            .any(|d| d.id == "waybar" && d.kind == DependencyKind::SystemBinary && d.required));
        assert!(all
            .iter()
            .any(|d| d.id == "rofi" && d.kind == DependencyKind::SystemBinary && !d.required));
        assert!(all
            .iter()
            .any(|d| d.id == "hyprland" && d.kind == DependencyKind::DesktopCapability));
        assert!(all
            .iter()
            .any(|d| d.id == "wayland" && d.kind == DependencyKind::DesktopCapability));
    }

    #[test]
    fn test_phase9_dependencies_validation_failures() {
        // Invalid SemVer version_req
        let bad_req_json = r#"{
            "id": "bad-req",
            "name": "Bad Req",
            "version": "1.0.0",
            "ryzora_spec": "1",
            "author": "Tester",
            "package_type": "rice",
            "compatibility": {"desktops": [], "sessions": [], "distros": [], "required": [], "optional": []},
            "dependencies": [
                {
                    "id": "some-dep",
                    "kind": "package",
                    "version_req": "invalid-version-req-!!!",
                    "required": true
                }
            ],
            "files": []
        }"#;
        let res = validate_manifest_internal(bad_req_json);
        assert!(!res.valid);
        assert!(res
            .errors
            .iter()
            .any(|e| e.contains("not a valid SemVer requirement")));

        // Path traversal in dependency id
        let traversal_dep_json = r#"{
            "id": "bad-dep-id",
            "name": "Bad Dep ID",
            "version": "1.0.0",
            "ryzora_spec": "1",
            "author": "Tester",
            "package_type": "rice",
            "compatibility": {"desktops": [], "sessions": [], "distros": [], "required": [], "optional": []},
            "dependencies": [
                {
                    "id": "../../etc/shadow",
                    "kind": "package",
                    "required": true
                }
            ],
            "files": []
        }"#;
        let res2 = validate_manifest_internal(traversal_dep_json);
        assert!(!res2.valid);
        assert!(res2
            .errors
            .iter()
            .any(|e| e.contains("invalid characters or path traversal")));
    }

    #[test]
    fn test_phase9_1_manifest_hardening_tests() {
        // 1. Prerelease semver is valid
        let pre_json = r#"{
            "id": "prerelease-pkg",
            "name": "Prerelease Package",
            "version": "1.0.0-beta.2",
            "ryzora_spec": "1",
            "author": "Tester",
            "package_type": "rice",
            "compatibility": {"desktops": [], "sessions": [], "distros": [], "required": [], "optional": []},
            "dependencies": [],
            "files": []
        }"#;
        let res1 = validate_manifest_internal(pre_json);
        assert!(
            res1.valid,
            "Expected valid prerelease semver: {:?}",
            res1.errors
        );

        // 2. Excessively long dependency ID (> 64 chars)
        let long_id = "a".repeat(65);
        let long_id_json = format!(
            r#"{{
            "id": "long-id-pkg",
            "name": "Long ID Package",
            "version": "1.0.0",
            "ryzora_spec": "1",
            "author": "Tester",
            "package_type": "rice",
            "compatibility": {{"desktops": [], "sessions": [], "distros": [], "required": [], "optional": []}},
            "dependencies": [{{"id": "{}", "kind": "package", "required": true}}],
            "files": []
        }}"#,
            long_id
        );
        let res2 = validate_manifest_internal(&long_id_json);
        assert!(!res2.valid);
        assert!(res2
            .errors
            .iter()
            .any(|e| e.contains("exceeds maximum length of 64 characters")));

        // 3. Duplicate dependency specs generate warning and are deduplicated
        let dup_json = r#"{
            "id": "dup-dep-pkg",
            "name": "Dup Dep Package",
            "version": "1.0.0",
            "ryzora_spec": "1",
            "author": "Tester",
            "package_type": "rice",
            "compatibility": {"desktops": [], "sessions": [], "distros": [], "required": [], "optional": []},
            "dependencies": [
                {"id": "theme-a", "kind": "package", "version_req": "^1.0.0", "required": true},
                {"id": "theme-a", "kind": "package", "version_req": "^1.0.0", "required": true}
            ],
            "files": []
        }"#;
        let res3 = validate_manifest_internal(dup_json);
        assert!(res3.valid);
        assert!(res3
            .warnings
            .iter()
            .any(|w| w.contains("duplicate dependency specification")));

        let manifest: RyzoraManifest = serde_json::from_str(dup_json).unwrap();
        let all = manifest.all_dependencies();
        assert_eq!(all.len(), 1, "Expected deduplication to 1 item");
    }
}
