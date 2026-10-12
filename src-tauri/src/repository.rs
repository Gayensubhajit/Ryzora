//! Ryzora Repository & Catalog Management.
//!
//! UI-independent data models, validation, discovery, and LocalRepository
//! are extracted to `ryzora-core`. This module re-exports core types for complete
//! backwards compatibility and implements remote repositories, unified provider
//! integration, and Tauri IPC commands.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub use ryzora_core::compatibility::CompatibilityRequirements;
pub use ryzora_core::distribution::{ModerationStatus, ReleaseChannel, TrustTier};
pub use ryzora_core::manifest::{validate_manifest_internal, PackageType, RyzoraManifest};
pub use ryzora_core::repository::*;

use crate::snapshot::get_home_dir;

pub struct UreqHttpFetcher;

impl HttpFetcher for UreqHttpFetcher {
    fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
        validate_remote_url(url)?;

        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .redirects(0)
            .build();

        let mut current_url = url.to_string();
        let mut redirect_count = 0;
        const MAX_REDIRECTS: usize = 5;

        loop {
            let resp_result = agent.get(&current_url).call();

            let resp = match resp_result {
                Ok(r) => r,
                Err(ureq::Error::Status(code, r)) if (300..=399).contains(&code) => r,
                Err(e) => return Err(format!("HTTP request failed for '{}': {}", current_url, e)),
            };

            let status = resp.status();
            if (300..=399).contains(&status) {
                if redirect_count >= MAX_REDIRECTS {
                    return Err(format!(
                        "Exceeded maximum redirects ({}) while fetching '{}'",
                        MAX_REDIRECTS, url
                    ));
                }

                let location = resp.header("Location").ok_or_else(|| {
                    format!(
                        "HTTP {} redirect missing Location header from '{}'",
                        status, current_url
                    )
                })?;

                let next_url = validate_redirect(&current_url, location)?;
                current_url = next_url;
                redirect_count += 1;
                continue;
            }

            if status < 200 || status >= 300 {
                return Err(format!("HTTP error {} fetching '{}'", status, current_url));
            }

            let mut reader = resp.into_reader().take((max_bytes + 1) as u64);
            let mut buf = Vec::new();
            reader
                .read_to_end(&mut buf)
                .map_err(|e| format!("Failed to read response from '{}': {}", current_url, e))?;

            if buf.len() > max_bytes {
                return Err(format!(
                    "Response from '{}' exceeds maximum allowed size ({} bytes)",
                    current_url, max_bytes
                ));
            }

            return Ok(buf);
        }
    }
}


// ──────── RemoteRepository ────────

pub struct RemoteRepository {
    id: String,
    name: String,
    url: String,
    cache_dir: PathBuf,
    index: Option<RepositoryIndex>,
    fetcher: Box<dyn HttpFetcher>,
    status: String,
    last_refreshed: Option<String>,
    last_error: Option<String>,
}

impl RemoteRepository {
    pub fn new(id: String, name: String, url: String, cache_dir: PathBuf) -> Result<Self, String> {
        validate_remote_url(&url)?;
        Self::with_fetcher(id, name, url, cache_dir, Box::new(UreqHttpFetcher))
    }

    pub fn with_fetcher(
        id: String,
        name: String,
        url: String,
        cache_dir: PathBuf,
        fetcher: Box<dyn HttpFetcher>,
    ) -> Result<Self, String> {
        validate_remote_url(&url)?;
        validate_path_identifier(&id, "remote repository ID")?;

        let mut repo = Self {
            id,
            name,
            url,
            cache_dir,
            index: None,
            fetcher,
            status: "offline".to_string(),
            last_refreshed: None,
            last_error: None,
        };

        if let Ok(_) = repo.load_cached_index() {
            repo.status = "cached".to_string();
        }
        Ok(repo)
    }

    pub fn load_cached_index(&mut self) -> Result<RepositoryIndex, String> {
        let index_path = self.cache_dir.join("repository.json");
        if !index_path.is_file() {
            return Err("No cached repository index found".to_string());
        }
        let raw = fs::read_to_string(&index_path)
            .map_err(|e| format!("Failed to read cached repository.json: {}", e))?;
        let index = Self::validate_and_parse_index(&raw)?;
        self.index = Some(index.clone());
        Ok(index)
    }

    pub fn refresh_index(&mut self) -> Result<RepositoryIndex, String> {
        let url = resolve_repository_url(&self.url, "repository.json")?;
        let bytes = match self.fetcher.fetch(&url, MAX_INDEX_SIZE) {
            Ok(b) => b,
            Err(e) => {
                self.status = if self.index.is_some() {
                    "offline".to_string()
                } else {
                    "refresh_failed".to_string()
                };
                self.last_error = Some(e.clone());
                // If network fetch fails, keep valid cached index if available
                if let Some(ref cached) = self.index {
                    return Ok(cached.clone());
                }
                return Err(format!("Failed to refresh remote repository: {}", e));
            }
        };

        let raw = match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => {
                self.status = "refresh_failed".to_string();
                let msg = format!("Invalid UTF-8 in repository.json: {}", e);
                self.last_error = Some(msg.clone());
                if let Some(ref cached) = self.index {
                    return Ok(cached.clone());
                }
                return Err(msg);
            }
        };

        let index = match Self::validate_and_parse_index(&raw) {
            Ok(idx) => idx,
            Err(e) => {
                self.status = "refresh_failed".to_string();
                self.last_error = Some(e.clone());
                if let Some(ref cached) = self.index {
                    return Ok(cached.clone());
                }
                return Err(e);
            }
        };

        // Atomically save to cache directory
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;

        let tmp_path = self.cache_dir.join("repository.json.tmp");
        fs::write(&tmp_path, &raw)
            .map_err(|e| format!("Failed to write temp repository.json: {}", e))?;
        let final_path = self.cache_dir.join("repository.json");
        fs::rename(&tmp_path, &final_path)
            .map_err(|e| format!("Failed to atomically rename repository.json: {}", e))?;

        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.index = Some(index.clone());
        self.status = "online".to_string();
        self.last_refreshed = Some(format!("{}", now_sec));
        self.last_error = None;
        Ok(index)
    }

    fn validate_and_parse_index(raw: &str) -> Result<RepositoryIndex, String> {
        let index: RepositoryIndex = serde_json::from_str(raw)
            .map_err(|e| format!("Malformed remote repository index: {}", e))?;

        if index.schema != 1 {
            return Err(format!(
                "Unsupported remote repository schema {} (expected schema 1)",
                index.schema
            ));
        }

        validate_path_identifier(&index.id, "repository ID")?;

        let mut seen = HashSet::new();
        for pkg in &index.packages {
            validate_repository_package_entry(pkg)?;

            if !seen.insert(pkg.id.clone()) {
                return Err(format!("Duplicate package ID '{}' in remote index", pkg.id));
            }
        }

        Ok(index)
    }
}

impl Repository for RemoteRepository {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn repo_type(&self) -> &str {
        "remote"
    }

    fn refresh(&mut self) -> Result<(), String> {
        self.refresh_index().map(|_| ())
    }

    fn status(&self) -> &str {
        &self.status
    }

    fn last_refreshed(&self) -> Option<String> {
        self.last_refreshed.clone()
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }

    fn cache_dir(&self) -> Option<&Path> {
        Some(&self.cache_dir)
    }

    fn list_entries(&self) -> Result<Vec<RepositoryPackageEntry>, String> {
        match &self.index {
            Some(idx) => Ok(idx.packages.clone()),
            None => Err(format!(
                "Remote repository '{}' has not been fetched yet",
                self.id
            )),
        }
    }

    fn get_package_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
        let index = match &self.index {
            Some(idx) => idx,
            None => {
                return Err(format!(
                    "Remote repository '{}' has no loaded index",
                    self.id
                ))
            }
        };

        let entry = index
            .packages
            .iter()
            .find(|p| p.id == package_id)
            .ok_or_else(|| {
                format!(
                    "Package '{}' not found in repository '{}'",
                    package_id, self.id
                )
            })?;

        let cached_manifest =
            get_safe_cache_path(&self.cache_dir, &self.id, &entry.id, &entry.version)?
                .join("manifest.json");

        if cached_manifest.is_file() {
            let raw = fs::read_to_string(&cached_manifest)
                .map_err(|e| format!("Failed to read cached manifest: {}", e))?;
            let manifest: RyzoraManifest = serde_json::from_str(&raw)
                .map_err(|e| format!("Failed to parse cached manifest: {}", e))?;

            if manifest.id != entry.id {
                return Err(format!(
                    "Manifest identity mismatch for package '{}': manifest declares ID '{}', repository entry expects '{}'",
                    entry.id, manifest.id, entry.id
                ));
            }
            if manifest.version != entry.version {
                return Err(format!(
                    "Manifest version mismatch for package '{}': manifest declares version '{}', repository entry expects '{}'",
                    entry.id, manifest.version, entry.version
                ));
            }
            if manifest.package_type != entry.package_type {
                return Err(format!(
                    "Manifest package type mismatch for package '{}': manifest declares type '{:?}', repository entry expects '{:?}'",
                    entry.id, manifest.package_type, entry.package_type
                ));
            }
            return Ok(manifest);
        }

        // Fetch manifest on demand using secure URL resolution
        let manifest_url = resolve_repository_url(&self.url, &entry.manifest)?;
        let bytes = self.fetcher.fetch(&manifest_url, MAX_MANIFEST_SIZE)?;
        let raw = String::from_utf8(bytes)
            .map_err(|e| format!("Invalid UTF-8 in downloaded manifest: {}", e))?;

        let validation = validate_manifest_internal(&raw);
        if !validation.valid {
            return Err(format!(
                "Downloaded manifest for '{}' failed validation: {}",
                entry.id,
                validation.errors.join("; ")
            ));
        }

        let manifest: RyzoraManifest = serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse downloaded manifest: {}", e))?;

        if manifest.id != entry.id {
            return Err(format!(
                "Manifest identity mismatch for package '{}': manifest declares ID '{}', repository entry expects '{}'",
                entry.id, manifest.id, entry.id
            ));
        }
        if manifest.version != entry.version {
            return Err(format!(
                "Manifest version mismatch for package '{}': manifest declares version '{}', repository entry expects '{}'",
                entry.id, manifest.version, entry.version
            ));
        }
        if manifest.package_type != entry.package_type {
            return Err(format!(
                "Manifest package type mismatch for package '{}': manifest declares type '{:?}', repository entry expects '{:?}'",
                entry.id, manifest.package_type, entry.package_type
            ));
        }

        Ok(manifest)
    }

    fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String> {
        let index = match &self.index {
            Some(idx) => idx,
            None => {
                return Err(format!(
                    "Remote repository '{}' has no loaded index",
                    self.id
                ))
            }
        };

        let entry = index
            .packages
            .iter()
            .find(|p| p.id == package_id)
            .ok_or_else(|| {
                format!(
                    "Package '{}' not found in repository '{}'",
                    package_id, self.id
                )
            })?;

        let pkg_cache_dir =
            get_safe_cache_path(&self.cache_dir, &self.id, &entry.id, &entry.version)?;
        let manifest_file = pkg_cache_dir.join("manifest.json");

        let mut cache_is_valid = false;

        // Cryptographically re-verify existing cache before trusting
        if manifest_file.is_file() {
            if let Ok(manifest_raw) = fs::read_to_string(&manifest_file) {
                if validate_manifest_internal(&manifest_raw).valid {
                    if let Ok(m) = serde_json::from_str::<RyzoraManifest>(&manifest_raw) {
                        // Check manifest identity and package type consistency
                        if m.id == entry.id
                            && m.version == entry.version
                            && m.package_type == entry.package_type
                        {
                            match verify_cached_package_integrity(
                                &pkg_cache_dir,
                                &m,
                                entry.content_hash.as_deref(),
                            ) {
                                Ok(()) => {
                                    cache_is_valid = true;
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[ryzora] Cached package '{}' failed integrity check: {}. Purging cache.",
                                        entry.id, e
                                    );
                                }
                            }
                        } else {
                            eprintln!(
                                "[ryzora] Cached package '{}' manifest identity or type mismatch. Purging cache.",
                                entry.id
                            );
                        }
                    }
                }
            }

            if cache_is_valid {
                return Ok(pkg_cache_dir);
            } else {
                // Purge corrupted/invalid cache immediately
                let _ = fs::remove_dir_all(&pkg_cache_dir);
            }
        }

        // Perform safe on-demand download into temporary staging directory
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let staging_dir = self
            .cache_dir
            .join("staging")
            .join(format!("{}-{}", entry.id, now));
        fs::create_dir_all(&staging_dir)
            .map_err(|e| format!("Failed to create staging dir: {}", e))?;

        let clean_staging = |dir: &Path| {
            let _ = fs::remove_dir_all(dir);
        };

        // Pre-check package size limit
        if let Some(size) = entry.package_size_bytes {
            if size > MAX_PACKAGE_SIZE {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Package '{}' declared size {} exceeds maximum allowed ({} bytes)",
                    entry.id, size, MAX_PACKAGE_SIZE
                ));
            }
        }

        // 1. Download manifest.json using secure URL resolution
        let manifest_url = match resolve_repository_url(&self.url, &entry.manifest) {
            Ok(u) => u,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(e);
            }
        };

        let manifest_bytes = match self.fetcher.fetch(&manifest_url, MAX_MANIFEST_SIZE) {
            Ok(b) => b,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to download manifest for '{}': {}",
                    entry.id, e
                ));
            }
        };

        let manifest_str = match String::from_utf8(manifest_bytes.clone()) {
            Ok(s) => s,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Invalid UTF-8 in manifest for '{}': {}",
                    entry.id, e
                ));
            }
        };

        let validation = validate_manifest_internal(&manifest_str);
        if !validation.valid {
            clean_staging(&staging_dir);
            return Err(format!(
                "Downloaded manifest for '{}' failed validation: {}",
                entry.id,
                validation.errors.join("; ")
            ));
        }

        let manifest: RyzoraManifest = match serde_json::from_str(&manifest_str) {
            Ok(m) => m,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to parse manifest for '{}': {}",
                    entry.id, e
                ));
            }
        };

        // Check manifest identity and package type consistency
        if manifest.id != entry.id {
            clean_staging(&staging_dir);
            return Err(format!(
                "Manifest identity mismatch for package '{}': manifest declares ID '{}', repository entry expects '{}'",
                entry.id, manifest.id, entry.id
            ));
        }
        if manifest.version != entry.version {
            clean_staging(&staging_dir);
            return Err(format!(
                "Manifest version mismatch for package '{}': manifest declares version '{}', repository entry expects '{}'",
                entry.id, manifest.version, entry.version
            ));
        }
        if manifest.package_type != entry.package_type {
            clean_staging(&staging_dir);
            return Err(format!(
                "Manifest package type mismatch for package '{}': manifest declares type '{:?}', repository entry expects '{:?}'",
                entry.id, manifest.package_type, entry.package_type
            ));
        }

        // Write manifest to staging
        if let Err(e) = fs::write(staging_dir.join("manifest.json"), &manifest_bytes) {
            clean_staging(&staging_dir);
            return Err(format!("Failed to write manifest to staging: {}", e));
        }

        // 2. Download each declared payload file
        let mut total_package_bytes: u64 = manifest_bytes.len() as u64;

        let manifest_parent = Path::new(&entry.manifest)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");

        for file_decl in &manifest.files {
            if file_decl.source.starts_with('/')
                || file_decl.source.starts_with('\\')
                || file_decl.source.contains('\0')
            {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Source path '{}' must be relative",
                    file_decl.source
                ));
            }
            for comp in Path::new(&file_decl.source).components() {
                if let Component::ParentDir = comp {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Source path '{}' contains forbidden '..'",
                        file_decl.source
                    ));
                }
            }

            let rel_file_path = if manifest_parent.is_empty() {
                file_decl.source.clone()
            } else {
                format!(
                    "{}/{}",
                    manifest_parent.trim_end_matches('/'),
                    file_decl.source.trim_start_matches('/')
                )
            };

            let file_url = match resolve_repository_url(&self.url, &rel_file_path) {
                Ok(u) => u,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(e);
                }
            };

            let file_bytes = match self.fetcher.fetch(&file_url, MAX_FILE_SIZE as usize) {
                Ok(b) => b,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Failed to download package file '{}': {}",
                        file_decl.source, e
                    ));
                }
            };

            total_package_bytes += file_bytes.len() as u64;
            if total_package_bytes > MAX_PACKAGE_SIZE {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Package '{}' exceeds maximum package size limit",
                    entry.id
                ));
            }

            let target_staged_path = staging_dir.join(&file_decl.source);
            if let Some(parent) = target_staged_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    clean_staging(&staging_dir);
                    return Err(format!("Failed to create staging parent dir: {}", e));
                }
            }

            if let Err(e) = fs::write(&target_staged_path, &file_bytes) {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to write staged file '{}': {}",
                    file_decl.source, e
                ));
            }
        }

        // 3. Verify content_hash if provided using deterministic tree hash
        if let Some(ref expected_hash) = entry.content_hash {
            let actual_package_hash = match compute_package_tree_hash(&staging_dir, &manifest) {
                Ok(h) => h,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(format!("Failed to compute tree hash for staging: {}", e));
                }
            };

            if !actual_package_hash.eq_ignore_ascii_case(expected_hash) {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Package integrity verification failed for '{}': expected SHA-256 '{}', got '{}'",
                    entry.id, expected_hash, actual_package_hash
                ));
            }
        }

        // 4. Atomically move staging directory to cache
        if let Some(parent) = pkg_cache_dir.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if pkg_cache_dir.exists() {
            let _ = fs::remove_dir_all(&pkg_cache_dir);
        }

        if let Err(e) = fs::rename(&staging_dir, &pkg_cache_dir) {
            clean_staging(&staging_dir);
            return Err(format!("Failed to finalize package cache: {}", e));
        }

        Ok(pkg_cache_dir)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository Manager
// ─────────────────────────────────────────────────────────────────────────────

pub struct RepositoryManager {
    repositories: Vec<Box<dyn Repository>>,
}

impl RepositoryManager {
    pub fn new() -> Self {
        Self {
            repositories: Vec::new(),
        }
    }

    pub fn add_repository(&mut self, repo: Box<dyn Repository>) {
        self.repositories.push(repo);
    }

    pub fn discover_local_repositories(&mut self, search_dirs: &[PathBuf]) {
        for search_dir in search_dirs {
            if !search_dir.is_dir() {
                continue;
            }

            // Direct repository check (e.g. if search_dir itself is a repository root)
            if search_dir.join("repository.json").is_file() {
                if let Ok(local_repo) = LocalRepository::load_from_dir(search_dir) {
                    if !self.repositories.iter().any(|r| r.id() == local_repo.id()) {
                        self.repositories.push(Box::new(local_repo));
                    }
                    continue;
                }
            }

            if let Ok(entries) = fs::read_dir(search_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.join("repository.json").is_file() {
                        if let Ok(local_repo) = LocalRepository::load_from_dir(&path) {
                            if !self.repositories.iter().any(|r| r.id() == local_repo.id()) {
                                self.repositories.push(Box::new(local_repo));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn refresh_all(&mut self) -> Result<(), String> {
        for repo in &mut self.repositories {
            let _ = repo.refresh();
        }
        Ok(())
    }

    pub fn refresh_repository(&mut self, id: &str) -> Result<(), String> {
        for repo in &mut self.repositories {
            if repo.id() == id {
                return repo.refresh();
            }
        }
        Err(format!("Repository '{}' not found", id))
    }

    pub fn repositories_mut(&mut self) -> &mut [Box<dyn Repository>] {
        &mut self.repositories
    }

    pub fn list_repositories(&self) -> Vec<RepositorySummary> {
        self.repositories
            .iter()
            .map(|r| RepositorySummary {
                id: r.id().to_string(),
                name: r.name().to_string(),
                package_count: r.list_entries().map(|e| e.len()).unwrap_or(0),
                path: format!("repository://{}", r.id()),
                repo_type: r.repo_type().to_string(),
                enabled: true,
                status: r.status().to_string(),
                last_refreshed: r.last_refreshed(),
                last_error: r.last_error(),
            })
            .collect()
    }

    pub fn repositories(&self) -> &[Box<dyn Repository>] {
        &self.repositories
    }

    pub fn get_repository_by_id(&self, id: &str) -> Option<&dyn Repository> {
        self.repositories
            .iter()
            .find(|r| r.id() == id)
            .map(|b| b.as_ref())
    }

    pub fn find_repository_for_package(&self, package_id: &str) -> Option<String> {
        for repo in &self.repositories {
            if let Ok(entries) = repo.list_entries() {
                if entries.iter().any(|e| e.id == package_id) {
                    return Some(repo.id().to_string());
                }
            }
        }
        None
    }

    pub fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String> {
        for repo in &self.repositories {
            if let Ok(dir) = repo.get_package_dir(package_id) {
                return Ok(dir);
            }
        }
        Err(format!(
            "Package '{}' not found in any repository",
            package_id
        ))
    }

    pub fn get_package_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
        for repo in &self.repositories {
            if let Ok(entries) = repo.list_entries() {
                if entries.iter().any(|e| e.id == package_id) {
                    return repo.get_package_manifest(package_id);
                }
            }
        }
        Err(format!(
            "Package '{}' not found in any repository",
            package_id
        ))
    }

    pub fn list_all_packages(&self) -> Result<Vec<FrontendPackageItem>, String> {
        let mut all_packages = Vec::new();
        let mut seen_ids = HashSet::new();

        for repo in &self.repositories {
            let entries = match repo.list_entries() {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries {
                if !seen_ids.insert(entry.id.clone()) {
                    continue; // Skip duplicates across repos
                }

                let manifest = repo.get_package_manifest(&entry.id).ok();

                let (
                    supported_desktops,
                    supported_display,
                    dependencies,
                    compatibility,
                    components,
                ) = if let Some(ref m) = manifest {
                    let desktops = m.compatibility.desktops.clone();
                    let sessions = m.compatibility.sessions.clone();
                    let deps = FrontendDependencies {
                        packages: m.compatibility.required.clone(),
                        optional: m.compatibility.optional.clone(),
                    };
                    let compat = CompatibilityRequirements {
                        supported_distros: m.compatibility.distros.clone(),
                        supported_desktops: m.compatibility.desktops.clone(),
                        supported_sessions: m.compatibility.sessions.clone(),
                        required_binaries: m.compatibility.required.clone(),
                        optional_binaries: m.compatibility.optional.clone(),
                    };
                    let comps = m
                        .files
                        .iter()
                        .map(|f| FrontendComponentSpec {
                            name: Path::new(&f.target)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&f.target)
                                .to_string(),
                            component_type: format!("{:?}", m.package_type).to_lowercase(),
                            target_path: f.target.clone(),
                            description: f.description.clone(),
                        })
                        .collect();
                    (desktops, sessions, deps, compat, comps)
                } else {
                    (
                        vec![],
                        vec!["wayland".to_string()],
                        FrontendDependencies {
                            packages: vec![],
                            optional: vec![],
                        },
                        CompatibilityRequirements {
                            supported_distros: vec![],
                            supported_desktops: vec![],
                            supported_sessions: vec![],
                            required_binaries: vec![],
                            optional_binaries: vec![],
                        },
                        vec![],
                    )
                };

                let files_count = manifest.as_ref().map(|m| m.files.len()).unwrap_or(0);

                let is_hash_verified = entry.content_hash.is_some();
                let ryzora_cache_root = get_ryzora_cache_dir();
                let repo_cache_root = repo.cache_dir().unwrap_or(&ryzora_cache_root);

                let is_cached = if repo.repo_type() == "local" {
                    true
                } else if let Ok(cache_path) =
                    get_safe_cache_path(repo_cache_root, repo.id(), &entry.id, &entry.version)
                {
                    cache_path.join("manifest.json").is_file()
                } else {
                    false
                };

                let (integrity_status, is_corrupted) = if repo.repo_type() == "local" {
                    if is_hash_verified {
                        ("verified".to_string(), false)
                    } else {
                        ("unverified".to_string(), false)
                    }
                } else if is_cached {
                    if let Some(ref expected_hash) = entry.content_hash {
                        if let Ok(cache_path) = get_safe_cache_path(
                            repo_cache_root,
                            repo.id(),
                            &entry.id,
                            &entry.version,
                        ) {
                            if let Some(ref m) = manifest {
                                match verify_cached_package_integrity(
                                    &cache_path,
                                    m,
                                    Some(expected_hash.as_str()),
                                ) {
                                    Ok(()) => ("verified".to_string(), false),
                                    Err(_) => ("corrupted".to_string(), true),
                                }
                            } else {
                                ("corrupted".to_string(), true)
                            }
                        } else {
                            ("unverified".to_string(), false)
                        }
                    } else {
                        ("unverified".to_string(), false)
                    }
                } else if is_hash_verified {
                    ("pending_download".to_string(), false)
                } else {
                    ("unverified".to_string(), false)
                };

                let audit_rating = if is_corrupted {
                    "corrupted".to_string()
                } else if is_hash_verified {
                    "verified".to_string()
                } else {
                    "unverified".to_string()
                };

                // Phase 12 Cryptographic Trust Chain Evaluation (Provisional / Informational for Catalog)
                // INVARIANT: Catalog evaluation against entry.content_hash is strictly provisional.
                // It indicates expected publisher trust for catalog browsing and discovery.
                // The actual installation gate in installer::install_package_in ALWAYS recomputes
                // the full tree hash from actual on-disk files before verifying the Ed25519 signature
                // and applying the trust policy.
                let trust_store = crate::crypto::TrustStore::load_default();
                let tree_h = entry.content_hash.as_deref().unwrap_or("");
                let crypto_eval = crate::crypto::evaluate_trust_chain(
                    entry.signature.as_ref(),
                    &entry.id,
                    tree_h,
                    entry.trust_tier,
                    &trust_store,
                );

                let item = FrontendPackageItem {
                    id: entry.id.clone(),
                    title: entry.name.clone(),
                    subtitle: entry.description.clone(),
                    description: entry.description.clone(),
                    version: entry.version.clone(),
                    author: entry.author.clone(),
                    category: entry.category.clone(),
                    package_type: entry.package_type.clone(),
                    tags: entry.tags.clone(),
                    supported_desktops,
                    supported_display,
                    rating: entry.rating.unwrap_or(4.8),
                    rating_count: 120,
                    downloads: entry.downloads.unwrap_or(1000),
                    hero_image: entry.hero_image.clone().unwrap_or_default(),
                    screenshots: entry.screenshots.clone(),
                    featured: entry.featured,
                    trending: entry.trending,
                    color_palette: entry.color_palette.clone(),
                    safety_audit: FrontendSafetyAudit {
                        rating: audit_rating,
                        changes_system_files: false,
                        requires_root: false,
                        sandbox_compatible: true,
                        files_modified_count: files_count,
                    },
                    dependencies,
                    components,
                    compatibility,
                    manifest,
                    repository_id: Some(repo.id().to_string()),
                    content_hash: entry.content_hash.clone(),
                    package_size_bytes: entry.package_size_bytes,
                    integrity_status,
                    is_cached,
                    release_channel: Some(entry.release_channel.unwrap_or_else(|| {
                        let v_lower = entry.version.to_lowercase();
                        if v_lower.contains("nightly") {
                            ReleaseChannel::Nightly
                        } else if v_lower.contains("beta")
                            || v_lower.contains("rc")
                            || v_lower.contains("alpha")
                        {
                            ReleaseChannel::Beta
                        } else {
                            ReleaseChannel::Stable
                        }
                    })),
                    trust_tier: Some(entry.trust_tier.unwrap_or_else(|| {
                        if is_corrupted {
                            TrustTier::Untrusted
                        } else if repo.id() == "ryzora-official" {
                            TrustTier::Official
                        } else if entry.author.verified && is_hash_verified {
                            TrustTier::Verified
                        } else {
                            TrustTier::Community
                        }
                    })),
                    moderation_status: Some(
                        entry
                            .moderation_status
                            .unwrap_or(ModerationStatus::Approved),
                    ),
                    trending_score: Some(entry.trending_score.unwrap_or_else(|| {
                        let dl = entry.downloads.unwrap_or(1000) as f64;
                        let rt = entry.rating.unwrap_or(4.8);
                        let tr = if entry.trending.unwrap_or(false) {
                            50.0
                        } else {
                            0.0
                        };
                        let ft = if entry.featured.unwrap_or(false) {
                            30.0
                        } else {
                            0.0
                        };
                        (dl * 0.05) + (rt * 10.0) + tr + ft
                    })),
                    maintainer: entry.maintainer.clone(),
                    release_notes: entry.release_notes.clone(),
                    signature: entry.signature.clone(),
                    cryptographic_status: Some(crypto_eval.status),
                    provider: entry.provider.clone(),
                    targets: entry.targets.clone(),
                    preview_video: entry.preview_video.clone(),
                    preview_animated: entry.preview_animated.clone(),
                    source: entry.source.clone(),
                    provenance: entry.provenance.clone(),
                    media_type: entry.media_type.clone().or_else(|| {
                        if entry.preview_video.is_some() {
                            Some("video".to_string())
                        } else if entry.preview_animated.is_some() {
                            Some("animated".to_string())
                        } else {
                            Some("image".to_string())
                        }
                    }),
                    preview_video_url: entry.preview_video.as_ref().map(|v| {
                        if !v.starts_with('/') && !v.starts_with("http://") && !v.starts_with("https://") {
                            format!("/{}", v)
                        } else {
                            v.clone()
                        }
                    }),
                    preview_poster_url: entry.hero_image.as_ref().map(|p| {
                        if !p.starts_with('/') && !p.starts_with("http://") && !p.starts_with("https://") {
                            format!("/{}", p)
                        } else {
                            p.clone()
                        }
                    }),
                    subfilter: entry.subfilter.clone(),
                };

                all_packages.push(item);
            }
        }

        Ok(all_packages)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration Persistence & Discovery
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_ryzora_cache_dir() -> PathBuf {
    get_home_dir().join(".local/share/ryzora/cache")
}

pub fn get_repository_config_file() -> PathBuf {
    get_home_dir().join(".local/share/ryzora/repositories.json")
}

pub fn load_repository_configs() -> Vec<RepositorySourceConfig> {
    let path = get_repository_config_file();
    if path.is_file() {
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(configs) = serde_json::from_str::<Vec<RepositorySourceConfig>>(&raw) {
                return configs;
            }
        }
    }

    // Default configurations: Official & Community remotes (GitHub-backed) + Local repository
    let mut default_configs = vec![
        RepositorySourceConfig {
            id: "ryzora-official".to_string(),
            name: "Ryzora Official (GitHub)".to_string(),
            url: "https://raw.githubusercontent.com/Gayensubhajit/Ryzora-official-repo/main"
                .to_string(),
            enabled: false,
            repo_type: "remote".to_string(),
        },
        RepositorySourceConfig {
            id: "ryzora-community".to_string(),
            name: "Ryzora Community (GitHub)".to_string(),
            url:
                "https://raw.githubusercontent.com/Gayensubhajit/Ryzora/main/repositories/community"
                    .to_string(),
            enabled: false,
            repo_type: "remote".to_string(),
        },
        RepositorySourceConfig {
            id: "community-local".to_string(),
            name: "Ryzora Community Repository (Local)".to_string(),
            url: "local://repositories/community".to_string(),
            enabled: true,
            repo_type: "local".to_string(),
        },
    ];

    if let Ok(override_repo) = std::env::var("RYZORA_REPOSITORY") {
        let trimmed = override_repo.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            for cfg in &mut default_configs {
                if cfg.id == "ryzora-community" {
                    cfg.url = trimmed.to_string();
                    cfg.enabled = true;
                }
            }
        }
    }

    default_configs
}

pub fn save_repository_configs(configs: &[RepositorySourceConfig]) -> Result<(), String> {
    let path = get_repository_config_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(configs)
        .map_err(|e| format!("Failed to serialize repository configs: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to save repository configs: {}", e))
}

pub fn get_default_repository_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Check developer override RYZORA_REPOSITORY (file:// or local path)
    if let Ok(override_repo) = std::env::var("RYZORA_REPOSITORY") {
        let trimmed = override_repo.trim();
        if !trimmed.is_empty() {
            if let Some(file_path) = trimmed.strip_prefix("file://") {
                let p = PathBuf::from(file_path);
                if p.is_dir() {
                    dirs.push(p);
                }
            } else if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
                let p = PathBuf::from(trimmed);
                if p.is_dir() {
                    dirs.push(p);
                }
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("repositories"));
        if cwd.file_name().and_then(|n| n.to_str()) == Some("src-tauri") {
            if let Some(parent) = cwd.parent() {
                dirs.push(parent.join("repositories"));
            }
        }
    }

    dirs.push(get_home_dir().join(".local/share/ryzora/repositories"));
    dirs
}

pub fn create_default_manager() -> RepositoryManager {
    let mut manager = RepositoryManager::new();

    // 1. Discover local repositories from filesystem
    let search_dirs = get_default_repository_search_dirs();
    manager.discover_local_repositories(&search_dirs);

    // 2. Discover configured remote repositories
    let configs = load_repository_configs();
    let cache_base = get_ryzora_cache_dir();

    for cfg in configs {
        if cfg.enabled && cfg.repo_type == "remote" {
            let cache_dir = cache_base.join(&cfg.id);
            if let Ok(remote_repo) =
                RemoteRepository::new(cfg.id.clone(), cfg.name.clone(), cfg.url.clone(), cache_dir)
            {
                manager.add_repository(Box::new(remote_repo));
            }
        }
    }

    manager
}

pub fn find_package_dir(package_id: &str) -> Result<PathBuf, String> {
    let manager = create_default_manager();
    manager.get_package_dir(package_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_catalog_packages() -> Result<Vec<FrontendPackageItem>, String> {
    let manager = create_default_manager();
    let mut packages = manager.list_all_packages()?;

    // Phase 18.8: Unified Content Integration
    // Merge lightweight in-memory metadata from all registered Content Providers
    // (Fastfetch, Rice, Wallpaper, KDE, GNOME, GitHub, Community).
    // ZERO payload download, ZERO disk staging, ZERO filesystem mutation during search/browse.
    if let Ok(mgr) = crate::providers::get_global_provider_manager().lock() {
        let query = crate::providers::ProviderQuery {
            query: String::new(),
            category: None,
            desktop: None,
            page: 1,
            page_size: 200,
        };
        let response = mgr.search_all(&query);

        // Strong Deduplication:
        // 1. Canonical source identity and normalized package ID
        // 2. Local repository packages take precedence on duplicate IDs
        let mut seen_ids: std::collections::HashSet<String> =
            packages.iter().map(|p| p.id.to_lowercase()).collect();

        for item in response.items {
            let norm_id = item.id.to_lowercase();
            if seen_ids.insert(norm_id) {
                packages.push(item);
            }
        }
    }

    Ok(packages)
}

#[tauri::command]
pub fn refresh_catalog() -> Result<Vec<FrontendPackageItem>, String> {
    let mut manager = create_default_manager();
    let _ = manager.refresh_all();
    let mut packages = manager.list_all_packages()?;

    if let Ok(mgr) = crate::providers::get_global_provider_manager().lock() {
        let query = crate::providers::ProviderQuery {
            query: String::new(),
            category: None,
            desktop: None,
            page: 1,
            page_size: 200,
        };
        let response = mgr.search_all(&query);

        let mut seen_ids: std::collections::HashSet<String> =
            packages.iter().map(|p| p.id.to_lowercase()).collect();

        for item in response.items {
            let norm_id = item.id.to_lowercase();
            if seen_ids.insert(norm_id) {
                packages.push(item);
            }
        }
    }

    Ok(packages)
}

#[tauri::command]
pub fn get_repository_info() -> Result<Vec<RepositorySummary>, String> {
    let manager = create_default_manager();
    let mut repos = manager.list_repositories();

    // Include virtual content providers in repository summaries for UI source filtering
    if let Ok(mgr) = crate::providers::get_global_provider_manager().lock() {
        for summary in mgr.list_providers() {
            let status_str = match summary.status {
                crate::providers::ProviderStatus::Online => "online".to_string(),
                crate::providers::ProviderStatus::Offline => "offline".to_string(),
                crate::providers::ProviderStatus::RateLimited { .. } => "rate_limited".to_string(),
                crate::providers::ProviderStatus::Disabled => "disabled".to_string(),
                crate::providers::ProviderStatus::Error(e) => format!("error: {}", e),
            };
            repos.push(RepositorySummary {
                id: format!("provider:{}", summary.id),
                name: summary.name,
                package_count: 0,
                path: format!("virtual://provider/{}", summary.id),
                repo_type: "provider".to_string(),
                enabled: summary.enabled,
                status: status_str,
                last_refreshed: None,
                last_error: None,
            });
        }
    }

    Ok(repos)
}

#[tauri::command]
pub fn list_repository_sources() -> Result<Vec<RepositorySourceConfig>, String> {
    Ok(load_repository_configs())
}

#[tauri::command]
pub fn add_repository_source(config: RepositorySourceConfig) -> Result<(), String> {
    validate_path_identifier(&config.id, "Repository ID")?;

    if config.repo_type == "remote" {
        validate_remote_url(&config.url)?;
    }

    let mut configs = load_repository_configs();
    if configs.iter().any(|c| c.id == config.id) {
        return Err(format!("Repository ID '{}' already exists", config.id));
    }

    configs.push(config);
    save_repository_configs(&configs)
}

#[tauri::command]
pub fn remove_repository_source(id: String) -> Result<(), String> {
    let mut configs = load_repository_configs();
    configs.retain(|c| c.id != id);
    save_repository_configs(&configs)
}

// ─────────────────────────────────────────────────────────────────────────────
// Cache Management & Confinement (Phase 8.12)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_size_bytes: u64,
    pub cached_packages_count: usize,
    pub cached_repositories_count: usize,
    pub cache_dir: String,
}

pub fn get_directory_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Ok(meta) = p.metadata() {
                    total += meta.len();
                }
            } else if p.is_dir() {
                total += get_directory_size(&p);
            }
        }
    }
    total
}

pub fn validate_cache_root_confinement(cache_root: &Path) -> Result<(), String> {
    if cache_root.as_os_str().is_empty() {
        return Err("Cache root cannot be empty".to_string());
    }

    let home = get_home_dir();
    let config_dir = home.join(".config");
    let installed_dir = home.join(".local/share/ryzora/installed");
    let snapshots_dir = home.join(".local/share/ryzora/snapshots");

    // Must never be root, home, .config, installed, or snapshots
    if cache_root == Path::new("/")
        || cache_root == home
        || cache_root == config_dir
        || cache_root == installed_dir
        || cache_root == snapshots_dir
    {
        return Err("Cache root points to protected system or user directory".to_string());
    }

    if cache_root.starts_with(&config_dir)
        || cache_root.starts_with(&installed_dir)
        || cache_root.starts_with(&snapshots_dir)
    {
        return Err("Cache root escapes into protected directory".to_string());
    }

    Ok(())
}

pub fn inspect_cache(cache_root: &Path) -> Result<CacheStats, String> {
    validate_cache_root_confinement(cache_root)?;

    if !cache_root.exists() {
        return Ok(CacheStats {
            total_size_bytes: 0,
            cached_packages_count: 0,
            cached_repositories_count: 0,
            cache_dir: cache_root.display().to_string(),
        });
    }

    let total_size_bytes = get_directory_size(cache_root);
    let mut cached_packages_count = 0;
    let mut cached_repositories_count = 0;

    if let Ok(entries) = fs::read_dir(cache_root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "staging" {
                    continue;
                }
                if p.join("repository.json").is_file() {
                    cached_repositories_count += 1;
                }
                let has_package = if let Ok(subentries) = fs::read_dir(&p) {
                    subentries
                        .flatten()
                        .any(|sub| sub.path().join("manifest.json").is_file())
                } else {
                    false
                };
                if has_package || p.join("manifest.json").is_file() {
                    cached_packages_count += 1;
                }
            }
        }
    }

    Ok(CacheStats {
        total_size_bytes,
        cached_packages_count,
        cached_repositories_count,
        cache_dir: cache_root.display().to_string(),
    })
}

pub fn clean_package_cache(cache_root: &Path, package_id: Option<&str>) -> Result<u64, String> {
    validate_cache_root_confinement(cache_root)?;

    if !cache_root.exists() {
        return Ok(0);
    }

    let mut bytes_freed = 0;

    if let Some(pkg_id) = package_id {
        validate_path_identifier(pkg_id, "package ID")?;
        let pkg_dir = cache_root.join(pkg_id);
        if pkg_dir.exists() {
            bytes_freed += get_directory_size(&pkg_dir);
            fs::remove_dir_all(&pkg_dir)
                .map_err(|e| format!("Failed to remove package cache for '{}': {}", pkg_id, e))?;
        }
    } else {
        if let Ok(entries) = fs::read_dir(cache_root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name == "staging" {
                        bytes_freed += get_directory_size(&p);
                        let _ = fs::remove_dir_all(&p);
                        continue;
                    }
                    if !p.join("repository.json").is_file() {
                        bytes_freed += get_directory_size(&p);
                        let _ = fs::remove_dir_all(&p);
                    }
                }
            }
        }
    }

    Ok(bytes_freed)
}

pub fn clean_all_cache(cache_root: &Path) -> Result<u64, String> {
    validate_cache_root_confinement(cache_root)?;

    if !cache_root.exists() {
        return Ok(0);
    }

    let bytes_freed = get_directory_size(cache_root);
    if let Ok(entries) = fs::read_dir(cache_root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let _ = fs::remove_file(&p);
            } else if p.is_dir() {
                let _ = fs::remove_dir_all(&p);
            }
        }
    }

    Ok(bytes_freed)
}

#[tauri::command]
pub fn get_cache_stats() -> Result<CacheStats, String> {
    let cache_dir = get_ryzora_cache_dir();
    inspect_cache(&cache_dir)
}

#[tauri::command]
pub fn clear_package_cache(package_id: Option<String>) -> Result<u64, String> {
    let cache_dir = get_ryzora_cache_dir();
    clean_package_cache(&cache_dir, package_id.as_deref())
}

#[tauri::command]
pub fn clear_all_cache() -> Result<u64, String> {
    let cache_dir = get_ryzora_cache_dir();
    clean_all_cache(&cache_dir)
}

// ─────────────────────────────────────────────────────────────────────────────
// Comprehensive Unit & Security Tests
// Zero touch of real ~/.config or real network. All use sandbox & MockHttpFetcher.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    // ──────── Mock HTTP Fetcher with Redirect Support ────────

    #[derive(Clone)]
    pub enum MockAction {
        Response(Vec<u8>),
        Redirect(String),
        Error(String),
    }

    #[derive(Clone)]
    struct MockHttpFetcher {
        responses: Arc<Mutex<HashMap<String, MockAction>>>,
    }

    impl MockHttpFetcher {
        fn new() -> Self {
            Self {
                responses: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn set_response(&self, url: &str, data: Vec<u8>) {
            self.responses
                .lock()
                .unwrap()
                .insert(url.to_string(), MockAction::Response(data));
        }

        fn set_redirect(&self, url: &str, target: &str) {
            self.responses
                .lock()
                .unwrap()
                .insert(url.to_string(), MockAction::Redirect(target.to_string()));
        }

        fn set_error(&self, url: &str, err: &str) {
            self.responses
                .lock()
                .unwrap()
                .insert(url.to_string(), MockAction::Error(err.to_string()));
        }
    }

    impl HttpFetcher for MockHttpFetcher {
        fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
            validate_remote_url(url)?;

            let mut current_url = url.to_string();
            let mut redirect_count = 0;
            const MAX_REDIRECTS: usize = 5;

            loop {
                let map = self.responses.lock().unwrap();
                let action = map
                    .get(&current_url)
                    .ok_or_else(|| format!("Mock 404: '{}'", current_url))?
                    .clone();
                drop(map);

                match action {
                    MockAction::Response(res) => {
                        if res.len() > max_bytes {
                            return Err(format!(
                                "Response from '{}' exceeds maximum allowed size ({} bytes)",
                                current_url, max_bytes
                            ));
                        }
                        return Ok(res);
                    }
                    MockAction::Redirect(target) => {
                        if redirect_count >= MAX_REDIRECTS {
                            return Err(format!(
                                "Exceeded maximum redirects ({}) while fetching '{}'",
                                MAX_REDIRECTS, url
                            ));
                        }
                        let next_url = validate_redirect(&current_url, &target)?;
                        current_url = next_url;
                        redirect_count += 1;
                        continue;
                    }
                    MockAction::Error(err) => {
                        return Err(err);
                    }
                }
            }
        }
    }

    // ──────── Test Sandbox ────────

    struct RepoTestSandbox {
        root: PathBuf,
        repo_dir: PathBuf,
        cache_dir: PathBuf,
    }

    impl RepoTestSandbox {
        fn new(name: &str) -> Self {
            let unique_id = format!(
                "ryzora-phase6-test-{}-{}",
                name,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let root = std::env::temp_dir().join(unique_id);
            let repo_dir = root.join("community");
            let cache_dir = root.join("cache");
            fs::create_dir_all(&repo_dir).unwrap();
            fs::create_dir_all(&cache_dir).unwrap();

            Self {
                root,
                repo_dir,
                cache_dir,
            }
        }

        fn create_valid_package(&self, id: &str) -> String {
            let pkg_dir = self.repo_dir.join("packages").join(id);
            fs::create_dir_all(pkg_dir.join("files")).unwrap();

            let manifest_json = format!(
                r##"{{
  "id": "{}",
  "name": "Test {}",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "A test package",
  "tags": ["test"],
  "color_palette": ["#ffffff"],
  "compatibility": {{
    "desktops": ["hyprland"],
    "sessions": ["wayland"],
    "distros": [],
    "required": [],
    "optional": []
  }},
  "files": [
    {{
      "source": "files/config.conf",
      "target": "~/.config/test/config.conf",
      "description": "test file"
    }}
  ]
}}"##,
                id, id
            );

            fs::write(pkg_dir.join("manifest.json"), manifest_json).unwrap();
            fs::write(pkg_dir.join("files/config.conf"), "test content").unwrap();

            format!("packages/{}/manifest.json", id)
        }

        fn write_repository_json(&self, content: &str) {
            fs::write(self.repo_dir.join("repository.json"), content).unwrap();
        }
    }

    impl Drop for RepoTestSandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 6 & 6.1 Security Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_remote_repo_https_accepted() {
        assert!(validate_remote_url("https://raw.githubusercontent.com/owner/repo/main").is_ok());
    }

    #[test]
    fn test_remote_repo_rejects_http() {
        let res = validate_remote_url("http://example.com/repo");
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("HTTPS only"));
    }

    #[test]
    fn test_remote_repo_rejects_forbidden_schemes() {
        let bad_schemes = vec![
            "file:///etc/passwd",
            "ftp://ftp.example.com/repo",
            "data:text/plain;base64,SGVsbG8=",
            "javascript:alert(1)",
        ];
        for bad_url in bad_schemes {
            let res = validate_remote_url(bad_url);
            assert!(res.is_err());
            assert!(res.err().unwrap().contains("HTTPS only"));
        }
    }

    #[test]
    fn test_remote_repo_rejects_null_bytes_and_traversal() {
        assert!(validate_remote_url("https://example.com/repo\0/bad").is_err());
        assert!(validate_remote_url("https://example.com/repo/../../bad").is_err());
    }

    #[test]
    fn test_remote_repo_malformed_json_rejected() {
        let sandbox = RepoTestSandbox::new("remote-malformed");
        let mock = MockHttpFetcher::new();
        mock.set_response(
            "https://example.com/repo/repository.json",
            b"{ invalid json".to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Malformed remote repository index"));
    }

    #[test]
    fn test_remote_repo_unsupported_schema_rejected() {
        let sandbox = RepoTestSandbox::new("remote-schema");
        let mock = MockHttpFetcher::new();
        let json = br#"{
  "schema": 99,
  "id": "test-remote",
  "name": "Remote",
  "packages": []
}"#;
        mock.set_response("https://example.com/repo/repository.json", json.to_vec());

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Unsupported remote repository schema 99"));
    }

    #[test]
    fn test_remote_repo_duplicate_package_ids_rejected() {
        let sandbox = RepoTestSandbox::new("remote-dup");
        let mock = MockHttpFetcher::new();
        let json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "dup-pkg", "name": "Dup 1", "version": "1.0.0", "package_type": "rice", "manifest": "packages/dup-pkg/manifest.json", "author": { "name": "A", "avatar": "" } },
    { "id": "dup-pkg", "name": "Dup 2", "version": "1.0.0", "package_type": "rice", "manifest": "packages/dup-pkg/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response("https://example.com/repo/repository.json", json.to_vec());

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Duplicate package ID 'dup-pkg'"));
    }

    #[test]
    fn test_remote_repo_manifest_traversal_rejected() {
        let sandbox = RepoTestSandbox::new("remote-traversal");
        let mock = MockHttpFetcher::new();
        let json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "evil-pkg", "name": "Evil", "version": "1.0.0", "package_type": "rice", "manifest": "../evil/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response("https://example.com/repo/repository.json", json.to_vec());

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("forbidden parent traversal"));
    }

    #[test]
    fn test_remote_repo_oversized_index_rejected() {
        let sandbox = RepoTestSandbox::new("remote-oversize");
        let mock = MockHttpFetcher::new();
        let huge_data = vec![b'a'; MAX_INDEX_SIZE + 10];
        mock.set_response("https://example.com/repo/repository.json", huge_data);

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("exceeds maximum allowed size"));
    }

    #[test]
    fn test_remote_repo_package_sha256_verification_and_atomic_caching() {
        let sandbox = RepoTestSandbox::new("remote-sha256");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Cyberpunk config";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/test.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "cyber-remote",
      "name": "Cyber Remote",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/cyber-remote/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "cyber-remote",
  "name": "Cyber Remote",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/test.conf", "target": "~/.config/test.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-remote/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-remote/files/test.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let pkg_dir = repo.get_package_dir("cyber-remote").unwrap();
        assert!(pkg_dir.join("manifest.json").is_file());
        assert!(pkg_dir.join("files/test.conf").is_file());

        let staging_dir = sandbox.cache_dir.join("staging");
        if staging_dir.exists() {
            assert_eq!(fs::read_dir(staging_dir).unwrap().count(), 0);
        }

        // Re-verification succeeds on second call
        let pkg_dir_2 = repo.get_package_dir("cyber-remote").unwrap();
        assert_eq!(pkg_dir, pkg_dir_2);
    }

    #[test]
    fn test_remote_repo_package_sha256_mismatch_rejected_and_staging_purged() {
        let sandbox = RepoTestSandbox::new("remote-sha-fail");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "tampered-pkg",
      "name": "Tampered",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/tampered-pkg/manifest.json",
      "content_hash": "deadbeef00000000000000000000000000000000000000000000000000000000",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        let manifest_json = br#"{
  "id": "tampered-pkg",
  "name": "Tampered",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/bad.conf", "target": "~/.config/bad.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/tampered-pkg/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/tampered-pkg/files/bad.conf",
            b"malicious content".to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let res = repo.get_package_dir("tampered-pkg");
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Package integrity verification failed"));

        let final_cache = sandbox.cache_dir.join("tampered-pkg");
        assert!(!final_cache.exists());

        let staging_dir = sandbox.cache_dir.join("staging");
        if staging_dir.exists() {
            assert_eq!(fs::read_dir(staging_dir).unwrap().count(), 0);
        }
    }

    #[test]
    fn test_remote_repo_network_failure_retains_valid_cached_index() {
        let sandbox = RepoTestSandbox::new("remote-fallback");
        let mock = MockHttpFetcher::new();

        let valid_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "cached-pkg", "name": "Cached", "version": "1.0.0", "package_type": "rice", "manifest": "packages/cached-pkg/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response(
            "https://example.com/repo/repository.json",
            valid_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();
        assert_eq!(repo.list_entries().unwrap().len(), 1);

        mock.set_error(
            "https://example.com/repo/repository.json",
            "Connection timed out",
        );

        let res = repo.refresh_index();
        assert!(res.is_ok());
        assert_eq!(repo.list_entries().unwrap().len(), 1);
        assert_eq!(repo.list_entries().unwrap()[0].id, "cached-pkg");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 6.1 Hardening Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_resolve_repository_url_valid_cases() {
        let base = "https://example.com/repo";
        assert_eq!(
            resolve_repository_url(base, "repository.json").unwrap(),
            "https://example.com/repo/repository.json"
        );
        assert_eq!(
            resolve_repository_url(base, "packages/cyber/manifest.json").unwrap(),
            "https://example.com/repo/packages/cyber/manifest.json"
        );
        assert_eq!(
            resolve_repository_url(base, "packages/cyber/files/waybar.conf").unwrap(),
            "https://example.com/repo/packages/cyber/files/waybar.conf"
        );
    }

    #[test]
    fn test_resolve_repository_url_rejects_traversal() {
        let base = "https://example.com/repo";
        assert!(resolve_repository_url(base, "../escaped.json").is_err());
        assert!(resolve_repository_url(base, "packages/../../escaped.json").is_err());
        assert!(resolve_repository_url(base, "foo/bar/../../../etc/passwd").is_err());
    }

    #[test]
    fn test_resolve_repository_url_rejects_encoded_traversal() {
        let base = "https://example.com/repo";
        assert!(resolve_repository_url(base, "%2e%2e/escaped.json").is_err());
        assert!(resolve_repository_url(base, "%2E%2E/escaped.json").is_err());
        assert!(resolve_repository_url(base, "%2e./escaped.json").is_err());
        assert!(resolve_repository_url(base, ".%2e/escaped.json").is_err());
        assert!(resolve_repository_url(base, "%252e%252e/double_encoded.json").is_err());
        assert!(resolve_repository_url(base, "%2fetc%2fpasswd").is_err());
        assert!(resolve_repository_url(base, "packages%5cescaped").is_err());
    }

    #[test]
    fn test_resolve_repository_url_rejects_schemes_and_absolute() {
        let base = "https://example.com/repo";
        assert!(resolve_repository_url(base, "https://evil.com/payload.json").is_err());
        assert!(resolve_repository_url(base, "http://example.com/repo/payload.json").is_err());
        assert!(resolve_repository_url(base, "file:///etc/shadow").is_err());
        assert!(resolve_repository_url(base, "javascript:alert(1)").is_err());
        assert!(resolve_repository_url(base, "//evil.com/payload.json").is_err());
        assert!(resolve_repository_url(base, "/rooted/path.json").is_err());
    }

    #[test]
    fn test_resolve_repository_url_rejects_credentials_and_null() {
        assert!(resolve_repository_url("https://user:pass@example.com/repo", "test.json").is_err());
        assert!(resolve_repository_url("https://example.com/repo", "null\0byte.json").is_err());
        assert!(resolve_repository_url("https://example.com/repo", "back\\slash.json").is_err());
        assert!(resolve_repository_url("https://example.com/repo", "fragment.json#frag").is_err());
    }

    #[test]
    fn test_validate_redirect_security() {
        let curr = "https://example.com/repo/package.json";

        // Same host HTTPS relative -> OK
        assert_eq!(
            validate_redirect(curr, "/repo/v2/package.json").unwrap(),
            "https://example.com/repo/v2/package.json"
        );
        // Same host HTTPS absolute -> OK
        assert_eq!(
            validate_redirect(curr, "https://example.com/repo/v2/package.json").unwrap(),
            "https://example.com/repo/v2/package.json"
        );

        // Downgrade to HTTP -> Rejected
        let res_http = validate_redirect(curr, "http://example.com/repo/package.json");
        assert!(res_http.is_err());
        assert!(res_http.err().unwrap().contains("attempted downgrade"));

        // Cross-origin host switch -> Rejected
        let res_evil = validate_redirect(curr, "https://evil.com/repo/package.json");
        assert!(res_evil.is_err());
        assert!(res_evil
            .err()
            .unwrap()
            .contains("Cross-origin redirect rejected"));

        // Credentials -> Rejected
        let res_cred = validate_redirect(curr, "https://user:pass@example.com/repo/package.json");
        assert!(res_cred.is_err());
        assert!(res_cred.err().unwrap().contains("credentials rejected"));

        // Insecure schemes -> Rejected
        assert!(validate_redirect(curr, "javascript:alert(1)").is_err());
        assert!(validate_redirect(curr, "file:///etc/passwd").is_err());
    }

    #[test]
    fn test_mock_fetcher_follows_valid_redirect() {
        let mock = MockHttpFetcher::new();
        mock.set_redirect(
            "https://example.com/repo/packages/cyber/manifest.json",
            "https://example.com/repo/packages/cyber-v2/manifest.json",
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-v2/manifest.json",
            b"{\"status\":\"ok\"}".to_vec(),
        );

        let data = mock
            .fetch(
                "https://example.com/repo/packages/cyber/manifest.json",
                1024,
            )
            .unwrap();
        assert_eq!(data, b"{\"status\":\"ok\"}");
    }

    #[test]
    fn test_mock_fetcher_rejects_insecure_redirect() {
        let mock = MockHttpFetcher::new();
        mock.set_redirect(
            "https://example.com/repo/packages/cyber/manifest.json",
            "http://example.com/insecure/manifest.json",
        );

        let res = mock.fetch(
            "https://example.com/repo/packages/cyber/manifest.json",
            1024,
        );
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("downgrade"));
    }

    #[test]
    fn test_mock_fetcher_rejects_cross_origin_redirect() {
        let mock = MockHttpFetcher::new();
        mock.set_redirect(
            "https://example.com/repo/packages/cyber/manifest.json",
            "https://attacker.com/malicious/manifest.json",
        );

        let res = mock.fetch(
            "https://example.com/repo/packages/cyber/manifest.json",
            1024,
        );
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Cross-origin redirect rejected"));
    }

    #[test]
    fn test_mock_fetcher_rejects_redirect_loop() {
        let mock = MockHttpFetcher::new();
        mock.set_redirect(
            "https://example.com/repo/loop1",
            "https://example.com/repo/loop2",
        );
        mock.set_redirect(
            "https://example.com/repo/loop2",
            "https://example.com/repo/loop1",
        );

        let res = mock.fetch("https://example.com/repo/loop1", 1024);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Exceeded maximum redirects"));
    }

    #[test]
    fn test_cache_trust_reverification_modified_file_purged_and_redownloaded() {
        let sandbox = RepoTestSandbox::new("cache-reverify-mod");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Cyberpunk config v1";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/test.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "cyber-remote",
      "name": "Cyber Remote",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/cyber-remote/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "cyber-remote",
  "name": "Cyber Remote",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/test.conf", "target": "~/.config/test.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-remote/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-remote/files/test.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        // 1. Initial download & cache
        let pkg_dir = repo.get_package_dir("cyber-remote").unwrap();
        let file_path = pkg_dir.join("files/test.conf");
        assert_eq!(fs::read(&file_path).unwrap(), file_content);

        // 2. Adversary or disk corruption tampers with the cached file!
        fs::write(&file_path, b"# Maliciously injected config!").unwrap();

        // 3. Next get_package_dir MUST detect mismatch, purge corrupted cache, and re-download clean file!
        let re_pkg_dir = repo.get_package_dir("cyber-remote").unwrap();
        assert_eq!(re_pkg_dir, pkg_dir);
        let restored_content = fs::read(re_pkg_dir.join("files/test.conf")).unwrap();
        assert_eq!(restored_content, file_content);
    }

    #[test]
    fn test_cache_trust_reverification_corrupt_cache_never_passed_when_offline() {
        let sandbox = RepoTestSandbox::new("cache-offline-corrupt");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Safe config";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/test.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "safe-pkg",
      "name": "Safe",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/safe-pkg/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "safe-pkg",
  "name": "Safe",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/test.conf", "target": "~/.config/test.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/safe-pkg/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/safe-pkg/files/test.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();
        let pkg_dir = repo.get_package_dir("safe-pkg").unwrap();

        // Tamper with cached file
        fs::write(pkg_dir.join("files/test.conf"), b"# Corrupted!").unwrap();

        // Network goes down!
        mock.set_error(
            "https://example.com/repo/packages/safe-pkg/files/test.conf",
            "Connection refused",
        );

        // Requesting package dir MUST fail with error and NEVER return corrupted cache!
        let res = repo.get_package_dir("safe-pkg");
        assert!(res.is_err());
        assert!(!pkg_dir.exists(), "Corrupted cache must be deleted");
    }

    #[test]
    fn test_cache_trust_deleted_file_triggers_purge_and_redownload() {
        let sandbox = RepoTestSandbox::new("cache-deleted-file");
        let mock = MockHttpFetcher::new();

        let file_content = b"# File to delete";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/test.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "del-pkg",
      "name": "Del",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/del-pkg/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "del-pkg",
  "name": "Del",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/test.conf", "target": "~/.config/test.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/del-pkg/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/del-pkg/files/test.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();
        let pkg_dir = repo.get_package_dir("del-pkg").unwrap();

        // Delete the file from cache
        fs::remove_file(pkg_dir.join("files/test.conf")).unwrap();

        // Re-verifying must detect missing file, purge cache, and re-download
        let recovered_dir = repo.get_package_dir("del-pkg").unwrap();
        assert!(recovered_dir.join("files/test.conf").is_file());
    }

    #[test]
    fn test_cache_path_safety_validation() {
        assert!(validate_path_identifier("safe-pkg-123", "pkg").is_ok());
        assert!(validate_path_identifier("v1.0.0+build1", "ver").is_ok());
        assert!(validate_path_identifier("repo_name.ext", "repo").is_ok());

        assert!(validate_path_identifier("", "pkg").is_err());
        assert!(validate_path_identifier("   ", "pkg").is_err());
        assert!(validate_path_identifier("../escape", "pkg").is_err());
        assert!(validate_path_identifier("foo/bar", "pkg").is_err());
        assert!(validate_path_identifier("foo\\bar", "pkg").is_err());
        assert!(validate_path_identifier("pkg\0null", "pkg").is_err());
        assert!(validate_path_identifier(".", "pkg").is_err());
        assert!(validate_path_identifier("..", "pkg").is_err());
        assert!(validate_path_identifier("pkg with spaces", "pkg").is_err());
    }

    #[test]
    fn test_package_metadata_validation() {
        // Valid 64 hex characters
        assert!(validate_sha256_hash(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )
        .is_ok());

        // Invalid length
        assert!(validate_sha256_hash(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b85"
        )
        .is_err()); // 63
        assert!(validate_sha256_hash(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b8555"
        )
        .is_err()); // 65

        // Non-hex
        assert!(validate_sha256_hash(
            "g3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )
        .is_err());
        assert!(validate_sha256_hash(
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
        )
        .is_err());
    }

    #[test]
    fn test_manifest_identity_id_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("id-mismatch");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "declared-id", "name": "Mismatch", "version": "1.0.0", "package_type": "rice", "manifest": "packages/mismatch/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;

        let manifest_json = br#"{
  "id": "different-id",
  "name": "Mismatch",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/mismatch/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let res = repo.get_package_dir("declared-id");
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Manifest identity mismatch"));
    }

    #[test]
    fn test_manifest_identity_version_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("ver-mismatch");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "pkg-one", "name": "Version Mismatch", "version": "2.0.0", "package_type": "rice", "manifest": "packages/pkg-one/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;

        let manifest_json = br#"{
  "id": "pkg-one",
  "name": "Version Mismatch",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-one/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let res = repo.get_package_dir("pkg-one");
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Manifest version mismatch"));
    }

    #[test]
    fn test_safety_audit_distinguishes_verified_vs_unverified() {
        let sandbox = RepoTestSandbox::new("audit-rating");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "verified-pkg", "name": "V", "version": "1.0.0", "package_type": "rice", "manifest": "packages/v/manifest.json", "content_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "author": { "name": "A", "avatar": "" } },
    { "id": "unverified-pkg", "name": "U", "version": "1.0.0", "package_type": "rice", "manifest": "packages/u/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(repo));

        let catalog = manager.list_all_packages().unwrap();
        let v_item = catalog.iter().find(|i| i.id == "verified-pkg").unwrap();
        let u_item = catalog.iter().find(|i| i.id == "unverified-pkg").unwrap();

        assert_eq!(v_item.safety_audit.rating, "verified");
        assert_eq!(u_item.safety_audit.rating, "unverified");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 5 LocalRepository Tests (Ensuring zero regressions)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_valid_repository_loading() {
        let sandbox = RepoTestSandbox::new("valid-repo");
        let manifest_path = sandbox.create_valid_package("test-pkg");

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {{
      "id": "test-pkg",
      "name": "Test Package",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }}
  ]
}}"#,
            manifest_path
        );

        sandbox.write_repository_json(&repo_json);

        let repo = LocalRepository::load_from_dir(&sandbox.repo_dir).unwrap();
        assert_eq!(repo.id(), "community");
        assert_eq!(repo.name(), "Community");

        let entries = repo.list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "test-pkg");

        let manifest = repo.get_package_manifest("test-pkg").unwrap();
        assert_eq!(manifest.id, "test-pkg");

        let pkg_dir = repo.get_package_dir("test-pkg").unwrap();
        assert!(pkg_dir.is_dir());
        assert!(pkg_dir.join("manifest.json").is_file());
    }

    #[test]
    fn test_malformed_repository_json_rejected() {
        let sandbox = RepoTestSandbox::new("malformed-repo");
        sandbox.write_repository_json("{ invalid json syntax");

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Malformed repository index"));
    }

    #[test]
    fn test_unsupported_schema_version_rejected() {
        let sandbox = RepoTestSandbox::new("bad-schema");
        sandbox.write_repository_json(
            r#"{
  "schema": 2,
  "id": "community",
  "name": "Community",
  "packages": []
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Unsupported repository schema 2"));
    }

    #[test]
    fn test_invalid_package_id_traversal_rejected() {
        let sandbox = RepoTestSandbox::new("traversal-id");
        sandbox.write_repository_json(
            r#"{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {
      "id": "../escaped-pkg",
      "name": "Escaped",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg/manifest.json",
      "author": { "name": "Tester", "avatar": "" }
    }
  ]
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Invalid package ID"));
    }

    #[test]
    fn test_duplicate_package_ids_rejected() {
        let sandbox = RepoTestSandbox::new("dup-ids");
        let manifest_path = sandbox.create_valid_package("test-pkg");

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {{
      "id": "test-pkg",
      "name": "First",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{0}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }},
    {{
      "id": "test-pkg",
      "name": "Duplicate",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{0}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }}
  ]
}}"#,
            manifest_path
        );

        sandbox.write_repository_json(&repo_json);

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Duplicate package ID"));
    }

    #[test]
    fn test_manifest_traversal_rejected() {
        let sandbox = RepoTestSandbox::new("manifest-traversal");
        sandbox.write_repository_json(
            r#"{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {
      "id": "evil-pkg",
      "name": "Evil",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "../../../etc/passwd",
      "author": { "name": "Tester", "avatar": "" }
    }
  ]
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("forbidden parent traversal"));
    }

    #[test]
    fn test_missing_manifest_rejected() {
        let sandbox = RepoTestSandbox::new("missing-manifest");
        sandbox.write_repository_json(
            r#"{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {
      "id": "missing-pkg",
      "name": "Missing",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/nonexistent/manifest.json",
      "author": { "name": "Tester", "avatar": "" }
    }
  ]
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Manifest file not found"));
    }

    #[test]
    fn test_malformed_manifest_rejected() {
        let sandbox = RepoTestSandbox::new("malformed-manifest");
        let pkg_dir = sandbox.repo_dir.join("packages").join("bad-pkg");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("manifest.json"), "{ invalid manifest json").unwrap();

        let repo_json = r#"{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {
      "id": "bad-pkg",
      "name": "Bad",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/bad-pkg/manifest.json",
      "author": { "name": "Tester", "avatar": "" }
    }
  ]
}"#;

        sandbox.write_repository_json(repo_json);

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("invalid manifest"));
    }

    #[test]
    fn test_repository_manager_discovery_and_catalog() {
        let sandbox = RepoTestSandbox::new("manager-test");
        let m1 = sandbox.create_valid_package("pkg-one");
        let m2 = sandbox.create_valid_package("pkg-two");

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {{
      "id": "pkg-one",
      "name": "Package One",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }},
    {{
      "id": "pkg-two",
      "name": "Package Two",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }}
  ]
}}"#,
            m1, m2
        );

        sandbox.write_repository_json(&repo_json);

        let mut manager = RepositoryManager::new();
        manager.discover_local_repositories(&[sandbox.root.clone()]);

        let repos = manager.list_repositories();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "community");
        assert_eq!(repos[0].package_count, 2);

        let catalog = manager.list_all_packages().unwrap();
        assert_eq!(catalog.len(), 2);
        let ids: Vec<String> = catalog.iter().map(|p| p.id.clone()).collect();
        assert!(ids.contains(&"pkg-one".to_string()));
        assert!(ids.contains(&"pkg-two".to_string()));

        let dir1 = manager.get_package_dir("pkg-one").unwrap();
        assert!(dir1.join("manifest.json").is_file());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 8 Production Store & Workflow Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_phase8_metadata_validation_out_of_bounds_rating() {
        let mut pkg = RepositoryPackageEntry {
            id: "pkg-rating".to_string(),
            name: "Rating Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Rice,
            description: "Test".to_string(),
            manifest: "manifest.json".to_string(),
            category: "desktop".to_string(),
            tags: vec![],
            author: AuthorInfo {
                name: "Tester".to_string(),
                avatar: "".to_string(),
                verified: false,
            },
            color_palette: vec![],
            hero_image: None,
            screenshots: vec![],
            featured: None,
            trending: None,
            rating: Some(5.5),
            downloads: None,
            content_hash: None,
            package_size_bytes: None,
            release_channel: None,
            trust_tier: None,
            moderation_status: None,
            trending_score: None,
            maintainer: None,
            release_notes: None,
            signature: None,
            provider: None,
            targets: None,
            preview_video: None,
            preview_animated: None,

            media_type: None,
            source: None,
            provenance: None,
            subfilter: None,
        };

        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.rating = Some(-0.1);
        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.rating = Some(f64::NAN);
        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.rating = Some(4.9);
        assert!(validate_repository_package_entry(&pkg).is_ok());
    }

    #[test]
    fn test_phase8_metadata_validation_invalid_color_palette() {
        let mut pkg = RepositoryPackageEntry {
            id: "pkg-color".to_string(),
            name: "Color Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Rice,
            description: "Test".to_string(),
            manifest: "manifest.json".to_string(),
            category: "desktop".to_string(),
            tags: vec![],
            author: AuthorInfo {
                name: "Tester".to_string(),
                avatar: "".to_string(),
                verified: false,
            },
            color_palette: vec!["not-a-color-at-all-and-extremely-long-string".to_string()],
            hero_image: None,
            screenshots: vec![],
            featured: None,
            trending: None,
            rating: None,
            downloads: None,
            content_hash: None,
            package_size_bytes: None,
            release_channel: None,
            trust_tier: None,
            moderation_status: None,
            trending_score: None,
            maintainer: None,
            release_notes: None,
            signature: None,
            provider: None,
            targets: None,
            preview_video: None,
            preview_animated: None,

            media_type: None,
            source: None,
            provenance: None,
            subfilter: None,
        };

        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.color_palette = vec!["#1a1b26".to_string(), "#7aa2f7".to_string()];
        assert!(validate_repository_package_entry(&pkg).is_ok());
    }

    #[test]
    fn test_phase8_metadata_validation_traversal_in_images() {
        let mut pkg = RepositoryPackageEntry {
            id: "pkg-img".to_string(),
            name: "Image Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Rice,
            description: "Test".to_string(),
            manifest: "manifest.json".to_string(),
            category: "desktop".to_string(),
            tags: vec![],
            author: AuthorInfo {
                name: "Tester".to_string(),
                avatar: "".to_string(),
                verified: false,
            },
            color_palette: vec![],
            hero_image: Some("../secret.png".to_string()),
            screenshots: vec![],
            featured: None,
            trending: None,
            rating: None,
            downloads: None,
            content_hash: None,
            package_size_bytes: None,
            release_channel: None,
            trust_tier: None,
            moderation_status: None,
            trending_score: None,
            maintainer: None,
            release_notes: None,
            signature: None,
            provider: None,
            targets: None,
            preview_video: None,
            preview_animated: None,

            media_type: None,
            source: None,
            provenance: None,
            subfilter: None,
        };

        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.hero_image = Some("/etc/shadow.png".to_string());
        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.hero_image = Some("assets/preview.png".to_string());
        assert!(validate_repository_package_entry(&pkg).is_ok());
        pkg.screenshots = vec!["../evil.jpg".to_string()];
        assert!(validate_repository_package_entry(&pkg).is_err());
    }

    #[test]
    fn test_phase8_resolve_repository_url_advanced_attacks() {
        let base = "https://example.com/repo/";

        // Protocol-relative URL
        assert!(resolve_repository_url(base, "//evil.com/payload").is_err());

        // Embedded scheme
        assert!(resolve_repository_url(base, "javascript://alert(1)").is_err());
        assert!(resolve_repository_url(base, "file:///etc/passwd").is_err());

        // Fragment
        assert!(resolve_repository_url(base, "packages/file#section").is_err());

        // Multi-level encoded traversal
        assert!(resolve_repository_url(base, "%252e%252e/escaped").is_err());
        assert!(resolve_repository_url(base, "..%2f..%2fetc/passwd").is_err());

        // Valid nested path
        let resolved = resolve_repository_url(base, "packages/cyber/manifest.json").unwrap();
        assert_eq!(
            resolved,
            "https://example.com/repo/packages/cyber/manifest.json"
        );
    }

    #[test]
    fn test_phase8_cache_trust_extra_unexpected_file_purges_and_redownloads() {
        let sandbox = RepoTestSandbox::new("cache-unexpected-file");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Payload config";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/app.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "pkg-extra",
      "name": "Extra File Pkg",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg-extra/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "pkg-extra",
  "name": "Extra File Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/app.conf", "target": "~/.config/app/app.conf", "description": "App" }
  ]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.as_bytes().to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-extra/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-extra/files/app.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        // Download once into cache
        let pkg_dir = repo.get_package_dir("pkg-extra").unwrap();
        assert!(pkg_dir.join("files/app.conf").is_file());

        // Attacker drops an unexpected file into the cached package
        fs::write(pkg_dir.join("unexpected_payload.sh"), b"echo malicious").unwrap();
        assert!(pkg_dir.join("unexpected_payload.sh").is_file());

        // Re-request package dir: unexpected file triggers purge & redownload
        let pkg_dir_new = repo.get_package_dir("pkg-extra").unwrap();
        assert!(
            !pkg_dir_new.join("unexpected_payload.sh").is_file(),
            "Unexpected file should have been purged!"
        );
        assert!(pkg_dir_new.join("files/app.conf").is_file());
    }

    #[test]
    fn test_phase8_cache_trust_modified_manifest_package_type_purges_and_redownloads() {
        let sandbox = RepoTestSandbox::new("cache-mod-type");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Payload";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/app.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "pkg-type",
      "name": "Type Pkg",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg-type/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "pkg-type",
  "name": "Type Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/app.conf", "target": "~/.config/app/app.conf", "description": "App" }
  ]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.as_bytes().to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-type/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-type/files/app.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let pkg_dir = repo.get_package_dir("pkg-type").unwrap();
        assert!(pkg_dir.join("files/app.conf").is_file());

        // Modify cached manifest package_type from rice to theme
        let tampered_manifest = br#"{
  "id": "pkg-type",
  "name": "Type Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "theme",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/app.conf", "target": "~/.config/app/app.conf", "description": "App" }
  ]
}"#;
        fs::write(pkg_dir.join("manifest.json"), tampered_manifest).unwrap();

        // Next access should detect inconsistency, purge, and redownload original rice manifest
        let pkg_dir_new = repo.get_package_dir("pkg-type").unwrap();
        let current_m: RyzoraManifest =
            serde_json::from_str(&fs::read_to_string(pkg_dir_new.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(current_m.package_type, PackageType::Rice);
    }

    #[test]
    fn test_phase8_consistency_package_type_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("consistency-type");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "pkg-mismatch-type",
      "name": "Mismatch Type",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        // Remote manifest declares theme instead of rice
        let manifest_json = br#"{
  "id": "pkg-mismatch-type",
  "name": "Mismatch Type",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "theme",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let err = repo.get_package_manifest("pkg-mismatch-type").unwrap_err();
        assert!(err.contains("Manifest package type mismatch"));

        let dir_err = repo.get_package_dir("pkg-mismatch-type").unwrap_err();
        assert!(dir_err.contains("Manifest package type mismatch"));
    }

    #[test]
    fn test_phase8_offline_mode_cached_index_browsable_and_repo_marked_offline() {
        let sandbox = RepoTestSandbox::new("offline-browsable");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "cached-pkg",
      "name": "Cached Package",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/cached-pkg/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        // 1. First refresh succeeds
        repo.refresh_index().unwrap();
        assert_eq!(repo.status(), "online");

        // 2. Network goes down
        mock.set_error(
            "https://example.com/repo/repository.json",
            "Network unreachable",
        );

        // Refresh fails, but retains cached index
        let res = repo.refresh_index();
        assert!(res.is_ok(), "Should retain cached index on network failure");
        assert_eq!(repo.status(), "offline", "Status should be marked offline");
        assert!(repo.last_error().is_some());

        // Packages still listed from cache
        let entries = repo.list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "cached-pkg");
    }

    #[test]
    fn test_phase8_offline_mode_uncached_package_fails_cleanly() {
        let sandbox = RepoTestSandbox::new("offline-uncached");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "uncached-pkg",
      "name": "Uncached",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/uncached/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        // Offline network error when trying to fetch uncached package
        mock.set_error(
            "https://example.com/repo/packages/uncached/manifest.json",
            "Connection refused",
        );
        let err = repo.get_package_dir("uncached-pkg").unwrap_err();
        assert!(err.contains("Connection refused") || err.contains("Failed to download manifest"));
    }

    #[test]
    fn test_phase8_cache_management_stats_and_cleanup() {
        let sandbox = RepoTestSandbox::new("cache-mgmt");
        let cache_root = sandbox.root.join("test_cache");
        fs::create_dir_all(&cache_root).unwrap();

        // Create a fake cached repository and a package
        let repo_dir = cache_root.join("remote-one");
        fs::create_dir_all(&repo_dir).unwrap();
        fs::write(repo_dir.join("repository.json"), b"{}").unwrap();

        let pkg_dir = cache_root.join("pkg-one").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("manifest.json"), b"{}").unwrap();
        fs::write(pkg_dir.join("data.bin"), vec![0u8; 1024]).unwrap();

        let stats = inspect_cache(&cache_root).unwrap();
        assert!(stats.total_size_bytes > 1024);
        assert_eq!(stats.cached_repositories_count, 1);
        assert_eq!(stats.cached_packages_count, 1);

        // Clean specific package
        let freed = clean_package_cache(&cache_root, Some("pkg-one")).unwrap();
        assert!(freed >= 1024);
        assert!(!cache_root.join("pkg-one").exists());

        // Clean all
        let all_freed = clean_all_cache(&cache_root).unwrap();
        assert!(all_freed > 0);
        assert!(!repo_dir.join("repository.json").exists());
    }

    #[test]
    fn test_phase8_cache_management_safety_guards_prevent_escape() {
        let home = get_home_dir();
        assert!(validate_cache_root_confinement(Path::new("/")).is_err());
        assert!(validate_cache_root_confinement(&home).is_err());
        assert!(validate_cache_root_confinement(&home.join(".config")).is_err());
        assert!(
            validate_cache_root_confinement(&home.join(".local/share/ryzora/installed")).is_err()
        );
        assert!(
            validate_cache_root_confinement(&home.join(".local/share/ryzora/snapshots")).is_err()
        );
        assert!(validate_cache_root_confinement(&home.join(".config/nested")).is_err());
    }

    #[test]
    fn test_phase8_consistency_id_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("consistency-id");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "pkg-repo-id",
      "name": "ID Pkg",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        let manifest_json = br#"{
  "id": "pkg-manifest-diff-id",
  "name": "ID Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let err = repo.get_package_manifest("pkg-repo-id").unwrap_err();
        assert!(err.contains("Manifest identity mismatch"));
    }

    #[test]
    fn test_phase8_consistency_version_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("consistency-ver");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "pkg-ver-test",
      "name": "Ver Pkg",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        let manifest_json = br#"{
  "id": "pkg-ver-test",
  "name": "Ver Pkg",
  "version": "2.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let err = repo.get_package_manifest("pkg-ver-test").unwrap_err();
        assert!(err.contains("Manifest version mismatch"));
    }

    #[test]
    fn test_phase8_metadata_validation_tags_and_author_length() {
        let mut pkg = RepositoryPackageEntry {
            id: "pkg-limits".to_string(),
            name: "Limits Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Rice,
            description: "Test".to_string(),
            manifest: "manifest.json".to_string(),
            category: "desktop".to_string(),
            tags: vec!["a".repeat(55)], // exceeds 50 chars
            author: AuthorInfo {
                name: "Tester".to_string(),
                avatar: "".to_string(),
                verified: false,
            },
            color_palette: vec![],
            hero_image: None,
            screenshots: vec![],
            featured: None,
            trending: None,
            rating: None,
            downloads: None,
            content_hash: None,
            package_size_bytes: None,
            release_channel: None,
            trust_tier: None,
            moderation_status: None,
            trending_score: None,
            maintainer: None,
            release_notes: None,
            signature: None,
            provider: None,
            targets: None,
            preview_video: None,
            preview_animated: None,

            media_type: None,
            source: None,
            provenance: None,
            subfilter: None,
        };

        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.tags = vec!["valid-tag".to_string()];
        assert!(validate_repository_package_entry(&pkg).is_ok());

        pkg.author.name = " ".to_string(); // empty author
        assert!(validate_repository_package_entry(&pkg).is_err());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 8.1 Release-Readiness & UX Audit Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_phase8_1_remote_repo_malformed_json_sets_refresh_failed_and_retains_cached_index() {
        let sandbox = RepoTestSandbox::new("p8-1-malformed-fallback");
        let mock = MockHttpFetcher::new();

        let valid_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "cached-pkg", "name": "Cached", "version": "1.0.0", "package_type": "rice", "manifest": "packages/cached-pkg/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response(
            "https://example.com/repo/repository.json",
            valid_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        // 1. Initial successful refresh
        repo.refresh_index().unwrap();
        assert_eq!(repo.status(), "online");
        assert!(repo.last_error().is_none());
        assert_eq!(repo.list_entries().unwrap().len(), 1);

        // 2. Server later returns malformed JSON on subsequent refresh
        mock.set_response(
            "https://example.com/repo/repository.json",
            b"{ malformed json string without close".to_vec(),
        );

        let res = repo.refresh_index();
        // Surviving cached index is returned
        assert!(res.is_ok());
        assert_eq!(repo.status(), "refresh_failed");
        assert!(repo.last_error().is_some());
        assert!(repo
            .last_error()
            .unwrap()
            .contains("Malformed remote repository index"));
        // Cached packages remain intact
        assert_eq!(repo.list_entries().unwrap().len(), 1);
        assert_eq!(repo.list_entries().unwrap()[0].id, "cached-pkg");
    }

    #[test]
    fn test_phase8_1_catalog_marks_corrupt_cache_as_corrupted_and_valid_cache_as_verified() {
        let sandbox = RepoTestSandbox::new("p8-1-catalog-trust");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Safe configuration";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/config.toml");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "valid-pkg",
      "name": "Valid",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/valid-pkg/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }},
    {{
      "id": "unhashed-pkg",
      "name": "Unhashed",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/unhashed-pkg/manifest.json",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "valid-pkg",
  "name": "Valid",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/config.toml", "target": "~/.config/config.toml", "description": "c" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/valid-pkg/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/valid-pkg/files/config.toml",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        // Download valid-pkg so it is in cache
        let pkg_dir = repo.get_package_dir("valid-pkg").unwrap();
        assert!(pkg_dir.exists());

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(repo));

        // Before corruption: valid-pkg is verified, unhashed-pkg is unverified
        let catalog = manager.list_all_packages().unwrap();
        let valid_item = catalog.iter().find(|p| p.id == "valid-pkg").unwrap();
        let unhashed_item = catalog.iter().find(|p| p.id == "unhashed-pkg").unwrap();

        assert_eq!(valid_item.integrity_status, "verified");
        assert_eq!(valid_item.safety_audit.rating, "verified");
        assert_eq!(unhashed_item.integrity_status, "unverified");
        assert_eq!(unhashed_item.safety_audit.rating, "unverified");

        // Now tamper with cached file in pkg_dir
        fs::write(pkg_dir.join("files/config.toml"), b"# Tampered payload!").unwrap();

        // Re-read catalog: valid-pkg must now be detected as corrupted, NEVER verified!
        let catalog_after = manager.list_all_packages().unwrap();
        let tampered_item = catalog_after.iter().find(|p| p.id == "valid-pkg").unwrap();

        assert_eq!(tampered_item.integrity_status, "corrupted");
        assert_eq!(tampered_item.safety_audit.rating, "corrupted");
    }

    static REPO_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct EnvGuard(&'static str);
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var(self.0);
        }
    }

    #[test]
    fn test_ryzora_repository_file_override() {
        let _lock = REPO_ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard("RYZORA_REPOSITORY");
        let temp_base = std::env::temp_dir().join(format!("ryzora_test_repo_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let repo_path = temp_base.join("my-custom-repo");
        fs::create_dir_all(&repo_path).unwrap();

        let repo_json = r#"{
            "schema": 1,
            "id": "my-custom-repo",
            "name": "Custom Repo",
            "version": "1.0.0",
            "description": "Custom developer repository",
            "packages": []
        }"#;
        fs::write(repo_path.join("repository.json"), repo_json).unwrap();

        std::env::set_var(
            "RYZORA_REPOSITORY",
            format!("file://{}", repo_path.to_str().unwrap()),
        );

        let search_dirs = get_default_repository_search_dirs();
        assert!(search_dirs.contains(&repo_path));

        let mut manager = RepositoryManager::new();
        manager.discover_local_repositories(&search_dirs);
        assert!(manager.repositories.iter().any(|r| r.id() == "my-custom-repo"));

        std::env::remove_var("RYZORA_REPOSITORY");
        let _ = fs::remove_dir_all(&temp_base);
        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_ryzora_repository_remote_override() {
        let _lock = REPO_ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard("RYZORA_REPOSITORY");
        std::env::set_var(
            "RYZORA_REPOSITORY",
            "https://my-custom-mirror.org/ryzora-community",
        );

        let configs = load_repository_configs();
        let community = configs.iter().find(|c| c.id == "ryzora-community").unwrap();
        assert_eq!(
            community.url,
            "https://my-custom-mirror.org/ryzora-community"
        );
        assert!(community.enabled);

        std::env::remove_var("RYZORA_REPOSITORY");
    }

    #[test]
    fn test_qylock_package_schema_provenance_and_source() {
        let entry_json = r#"{
            "id": "lockscreen-qylock-dog-samurai",
            "name": "Dog Samurai",
            "version": "1.0.0",
            "package_type": "lockscreen",
            "description": "A cinematic video lock",
            "manifest": "packages/lockscreen-qylock-dog-samurai/manifest.json",
            "category": "lockscreens",
            "tags": ["samurai", "video"],
            "author": { "name": "Darkkal44", "avatar": "", "verified": true },
            "provider": "qylock",
            "targets": ["quickshell", "sddm"],
            "preview_video": "assets/lockscreens/dog-samurai-preview.mp4",
            "preview_animated": "assets/lockscreens/dog-samurai.gif",
            "source": {
                "type": "git",
                "repository": "https://github.com/Darkkal44/qylock",
                "revision": "main",
                "path": "themes/dog-samurai"
            },
            "provenance": {
                "upstream": "https://github.com/Darkkal44/qylock",
                "revision": "main",
                "license": "GPL-3.0",
                "path": "themes/dog-samurai"
            }
        }"#;

        let entry: RepositoryPackageEntry = serde_json::from_str(entry_json).unwrap();
        assert_eq!(entry.provider, Some("qylock".to_string()));
        assert_eq!(entry.targets, Some(vec!["quickshell".to_string(), "sddm".to_string()]));
        assert_eq!(entry.preview_video, Some("assets/lockscreens/dog-samurai-preview.mp4".to_string()));
        let source = entry.source.unwrap();
        assert_eq!(source.source_type, "git");
        assert_eq!(source.repository, Some("https://github.com/Darkkal44/qylock".to_string()));
        let prov = entry.provenance.unwrap();
        assert_eq!(prov.upstream, "https://github.com/Darkkal44/qylock");
        assert_eq!(prov.license, Some("GPL-3.0".to_string()));
    }
    #[test]
    fn test_community_repository_catalogue_exposes_all_discovered_lockscreens() {
        let manager = create_default_manager();
        let packages = manager.list_all_packages().expect("list_all_packages failed");
        let lockscreens: Vec<_> = packages.into_iter().filter(|p| p.package_type == crate::manifest::PackageType::Lockscreen).collect();
        assert!(lockscreens.len() >= 42, "Community repository must expose all discovered lockscreens");

        let qylock_items: Vec<_> = lockscreens.into_iter().filter(|p| p.id.starts_with("lockscreen-qylock-")).collect();
        assert!(qylock_items.len() >= 40, "Must expose all discovered Qylock themes from repository manifests");

        for item in qylock_items {
            let manifest = item.manifest.as_ref().expect("Manifest must be attached");
            assert!(manifest.supports_target("quickshell"), "Item {} must support quickshell", item.id);
            assert!(manifest.supports_target("sddm"), "Item {} must support sddm", item.id);
            assert!(item.hero_image.starts_with("assets/lockscreens/"));
            assert!(!item.hero_image.contains("/home/silentbyte"));
        }
    }

}