//! Ryzora manifest data models, validation, and Tauri IPC commands.
//!
//! Pure domain models and validation logic are extracted to `ryzora-core`.
//! This module re-exports core types for complete backwards compatibility with
//! the Tauri application and exposes Tauri IPC commands.

pub use ryzora_core::manifest::*;

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
