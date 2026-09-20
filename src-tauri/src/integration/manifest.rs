//! Ryzora Reversible Integration Core — Manifest Persistence Engine
//!
//! Stores lightweight integration manifests in ~/.local/state/ryzora/integrations/{id}.json.
//! All persistence operations are atomic, durable, and corruption-resilient.

use super::types::{IntegrationError, IntegrationManifest};
use std::fs;
use std::path::{Path, PathBuf};

/// Sanitizes an integration identifier into a safe filename.
pub fn sanitize_integration_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Returns the persistent directory where integration manifests are stored.
pub fn get_manifest_storage_dir(home_dir: &Path) -> PathBuf {
    home_dir.join(".local/state/ryzora/integrations")
}

/// Atomically saves an integration manifest to disk.
pub fn save_manifest(
    manifest: &IntegrationManifest,
    home_dir: &Path,
) -> Result<PathBuf, IntegrationError> {
    let storage_dir = get_manifest_storage_dir(home_dir);
    fs::create_dir_all(&storage_dir).map_err(|e| {
        IntegrationError::IoError(format!(
            "Failed to create integration manifest directory {}: {}",
            storage_dir.display(),
            e
        ))
    })?;

    let safe_id = sanitize_integration_id(&manifest.integration_id);
    let target_path = storage_dir.join(format!("{}.json", safe_id));
    let temp_path = storage_dir.join(format!(".{}.tmp_{}", safe_id, std::process::id()));

    let json_bytes = serde_json::to_vec_pretty(manifest).map_err(|e| {
        IntegrationError::ManifestCorrupted(format!("Failed to serialize manifest: {}", e))
    })?;

    fs::write(&temp_path, &json_bytes).map_err(|e| {
        IntegrationError::IoError(format!(
            "Failed to write temp manifest {}: {}",
            temp_path.display(),
            e
        ))
    })?;

    fs::rename(&temp_path, &target_path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        IntegrationError::IoError(format!(
            "Failed to atomically rename manifest {} to {}: {}",
            temp_path.display(),
            target_path.display(),
            e
        ))
    })?;

    Ok(target_path)
}

/// Loads a manifest by integration ID from persistent storage.
pub fn load_manifest(
    integration_id: &str,
    home_dir: &Path,
) -> Result<IntegrationManifest, IntegrationError> {
    let storage_dir = get_manifest_storage_dir(home_dir);
    let safe_id = sanitize_integration_id(integration_id);
    let path = storage_dir.join(format!("{}.json", safe_id));

    if !path.exists() {
        return Err(IntegrationError::ManifestNotFound(integration_id.to_string()));
    }

    let bytes = fs::read(&path).map_err(|e| {
        IntegrationError::IoError(format!("Failed to read manifest {}: {}", path.display(), e))
    })?;

    let manifest: IntegrationManifest = serde_json::from_slice(&bytes).map_err(|e| {
        IntegrationError::ManifestCorrupted(format!(
            "Manifest {} is corrupted or invalid JSON: {}",
            path.display(),
            e
        ))
    })?;

    Ok(manifest)
}

/// Deletes a manifest from persistent storage after successful rollback.
pub fn delete_manifest(integration_id: &str, home_dir: &Path) -> Result<(), IntegrationError> {
    let storage_dir = get_manifest_storage_dir(home_dir);
    let safe_id = sanitize_integration_id(integration_id);
    let path = storage_dir.join(format!("{}.json", safe_id));

    if path.exists() {
        fs::remove_file(&path).map_err(|e| {
            IntegrationError::IoError(format!(
                "Failed to delete manifest {}: {}",
                path.display(),
                e
            ))
        })?;
    }

    Ok(())
}

/// Lists all active integration manifests stored on the system.
pub fn list_manifests(home_dir: &Path) -> Result<Vec<IntegrationManifest>, IntegrationError> {
    let storage_dir = get_manifest_storage_dir(home_dir);
    if !storage_dir.exists() {
        return Ok(Vec::new());
    }

    let mut list = Vec::new();
    let entries = fs::read_dir(&storage_dir).map_err(|e| {
        IntegrationError::IoError(format!(
            "Failed to read manifest directory {}: {}",
            storage_dir.display(),
            e
        ))
    })?;

    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(bytes) = fs::read(&p) {
                if let Ok(manifest) = serde_json::from_slice::<IntegrationManifest>(&bytes) {
                    list.push(manifest);
                }
            }
        }
    }

    list.sort_by(|a, b| a.integration_id.cmp(&b.integration_id));
    Ok(list)
}
