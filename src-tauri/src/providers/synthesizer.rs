use super::ProviderItem;
use crate::manifest::{
    validate_manifest_internal, ManifestCompatibility, ManifestFile, PackageType, RyzoraManifest,
};
use std::path::{Component, Path};

/// Synthesizes an authoritative, declarative `ryzora_spec: "1"` manifest from a ProviderItem.
/// This is a critical security boundary: all source and target paths are strictly inspected,
/// script hooks and execution binaries are stripped, and dangerous target destinations
/// (e.g. ~/.ssh, ~/.bashrc, /etc) are unconditionally rejected.
pub struct ManifestSynthesizer;

impl ManifestSynthesizer {
    /// Validates and converts a ProviderItem into an authoritative RyzoraManifest.
    pub fn synthesize(item: &ProviderItem) -> Result<RyzoraManifest, String> {
        // 1. Validate ID
        let clean_id = item.id.trim().to_lowercase();
        if clean_id.is_empty() {
            return Err("Package ID cannot be empty".to_string());
        }
        if clean_id.contains("..")
            || clean_id.contains('/')
            || clean_id.contains('\\')
            || clean_id.contains('\0')
        {
            return Err(format!(
                "Package ID '{}' contains forbidden characters",
                clean_id
            ));
        }
        if !clean_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(format!(
                "Package ID '{}' contains invalid characters",
                clean_id
            ));
        }

        // 2. Validate and clean file mappings
        let mut sanitized_files = Vec::new();
        let mut stripped_scripts = Vec::new();

        for file in &item.files {
            let src = file.source.trim();
            let tgt = file.target.trim();

            // 1. Validate target destination (fatal rejection for sensitive targets like ~/.ssh, ~/.bashrc, /etc)
            validate_synthesizer_target(tgt, &clean_id, &item.package_type)?;

            // 2. Validate source path within package boundary
            validate_synthesizer_source(src, &clean_id)?;

            // 3. Check for prohibited script/hook extensions and strip them
            if is_prohibited_script(src) || is_prohibited_script(tgt) {
                stripped_scripts.push(format!("{} -> {}", src, tgt));
                continue; // Strip script from declarative manifest
            }

            sanitized_files.push(ManifestFile {
                source: src.to_string(),
                target: tgt.to_string(),
                description: format!("File from provider {}", item.provenance.provider_id),
            });
        }

        // 3. Construct compatibility spec
        let desktops = if item.package_type == PackageType::Fastfetch
            || item.package_type == PackageType::Wallpaper
            || item
                .supported_desktops
                .iter()
                .any(|d| d.to_lowercase() == "universal" || d.to_lowercase() == "all")
            || item.supported_desktops.is_empty()
            || item.supported_desktops.len() > 1
        {
            vec!["universal".to_string()]
        } else {
            item.supported_desktops.clone()
        };

        let compatibility = ManifestCompatibility {
            desktops,
            sessions: Vec::new(),
            distros: Vec::new(),
            required: Vec::new(),
            optional: Vec::new(),
        };

        // Construct description with provenance attribution
        let mut full_description = item.description.clone();
        if let Some(ref spdx) = item.provenance.license_spdx {
            full_description.push_str(&format!("\n\nLicense: {}", spdx));
        }
        full_description.push_str(&format!("\nSource: {}", item.provenance.source_url));

        // Clean and normalize version into valid SemVer (X.Y.Z)
        let clean_version = {
            let v_trimmed = item.version.trim().trim_start_matches('v');
            if semver::Version::parse(v_trimmed).is_ok() {
                v_trimmed.to_string()
            } else {
                "1.0.0".to_string()
            }
        };

        // 4. Construct canonical RyzoraManifest
        let manifest = RyzoraManifest {
            ryzora_spec: "1".to_string(),
            id: clean_id,
            name: item.title.clone(),
            version: clean_version,
            author: item.author.name.clone(),
            package_type: item.package_type.clone(),
            description: full_description,
            tags: item.tags.clone(),
            color_palette: Vec::new(),
            compatibility,
            files: sanitized_files,
            dependencies: Vec::new(),
        };

        // 5. Run manifest internal schema validation
        let json_repr = serde_json::to_string(&manifest)
            .map_err(|e| format!("Failed to serialize synthesized manifest: {}", e))?;
        let val_res = validate_manifest_internal(&json_repr);
        if !val_res.valid {
            return Err(format!(
                "Synthesized manifest failed validation: {:?}",
                val_res.errors
            ));
        }

        Ok(manifest)
    }
}

/// Detects whether a filename represents an executable script or installer hook.
fn is_prohibited_script(path_str: &str) -> bool {
    let lower = path_str.to_lowercase();
    lower.ends_with(".sh")
        || lower.ends_with(".bash")
        || lower.ends_with(".zsh")
        || lower.ends_with(".fish")
        || lower.ends_with(".exe")
        || lower.ends_with(".bat")
        || lower.ends_with(".cmd")
        || lower.ends_with(".py")
        || lower.contains("install.sh")
        || lower.contains("setup.sh")
        || lower.contains("post_install")
        || lower.contains("pre_install")
}

/// Validates that a source path stays within the package root boundary.
fn validate_synthesizer_source(src: &str, pkg_id: &str) -> Result<(), String> {
    if src.is_empty() {
        return Err(format!("Package '{}' has empty source path", pkg_id));
    }
    if src.starts_with('/') || src.starts_with('\\') {
        return Err(format!(
            "Package '{}' source path must be relative: '{}'",
            pkg_id, src
        ));
    }
    if src.contains('\0') {
        return Err(format!(
            "Package '{}' source path contains null bytes",
            pkg_id
        ));
    }
    for comp in Path::new(src).components() {
        if let Component::ParentDir = comp {
            return Err(format!(
                "Package '{}' source path contains parent traversal ('..'): '{}'",
                pkg_id, src
            ));
        }
    }
    Ok(())
}

/// Validates that a target path conforms to strict desktop user-space placement rules.
fn validate_synthesizer_target(
    tgt: &str,
    pkg_id: &str,
    _pkg_type: &PackageType,
) -> Result<(), String> {
    if !tgt.starts_with("~/") {
        return Err(format!(
            "Package '{}' target path '{}' must start with '~/'",
            pkg_id, tgt
        ));
    }
    if tgt.contains('\0') {
        return Err(format!(
            "Package '{}' target path '{}' contains null bytes",
            pkg_id, tgt
        ));
    }
    for comp in Path::new(tgt).components() {
        if let Component::ParentDir = comp {
            return Err(format!(
                "Package '{}' target path '{}' contains parent traversal ('..')",
                pkg_id, tgt
            ));
        }
    }

    let rel = &tgt[2..]; // Strip "~/"

    // Explicit rejection of sensitive shell and auth targets
    let forbidden_sensitive = [
        ".ssh",
        ".gnupg",
        ".bashrc",
        ".bash_profile",
        ".bash_logout",
        ".profile",
        ".zshrc",
        ".zshenv",
        ".zprofile",
        ".config/fish/config.fish",
        ".xinitrc",
        ".xprofile",
        ".pam_environment",
    ];

    for forbidden in &forbidden_sensitive {
        if rel == *forbidden || rel.starts_with(&format!("{}/", forbidden)) {
            return Err(format!(
                "Security violation: Package '{}' targets forbidden sensitive destination '{}'",
                pkg_id, tgt
            ));
        }
    }

    // Allowed destination prefixes
    let allowed_prefixes = [
        ".config/",
        ".local/share/",
        ".local/state/",
        "Pictures/Wallpapers/",
        ".themes/",
        ".icons/",
    ];

    let is_allowed = allowed_prefixes
        .iter()
        .any(|prefix| rel.starts_with(prefix));
    if !is_allowed {
        return Err(format!(
            "Package '{}' target '{}' is outside allowed customization directories (~/.config/, ~/.local/, ~/Pictures/Wallpapers/, ~/.themes/, ~/.icons/)",
            pkg_id, tgt
        ));
    }

    Ok(())
}
