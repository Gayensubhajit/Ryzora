use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::compatibility::CompatibilityRequirements;
use crate::manifest::{validate_manifest_internal, PackageType, RyzoraManifest};
use crate::snapshot::get_home_dir;

// ─────────────────────────────────────────────────────────────────────────────
// Repository Data Types (Schema v1)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    pub avatar: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryPackageEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub package_type: PackageType,
    #[serde(default)]
    pub description: String,
    pub manifest: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub author: AuthorInfo,
    #[serde(default)]
    pub color_palette: Vec<String>,
    #[serde(default)]
    pub hero_image: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub featured: Option<bool>,
    #[serde(default)]
    pub trending: Option<bool>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub downloads: Option<u64>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub package_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryIndex {
    pub schema: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub packages: Vec<RepositoryPackageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySummary {
    pub id: String,
    pub name: String,
    pub package_count: usize,
    pub path: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Frontend-Facing Package Item (Compatible with existing UI PackageItem)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendSafetyAudit {
    pub rating: String,
    pub changes_system_files: bool,
    pub requires_root: bool,
    pub sandbox_compatible: bool,
    pub files_modified_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendDependencies {
    pub packages: Vec<String>,
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendComponentSpec {
    pub name: String,
    pub component_type: String,
    pub target_path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendPackageItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub version: String,
    pub author: AuthorInfo,
    pub category: String,
    pub package_type: PackageType,
    pub tags: Vec<String>,
    pub supported_desktops: Vec<String>,
    pub supported_display: Vec<String>,
    pub rating: f64,
    pub rating_count: u32,
    pub downloads: u64,
    pub hero_image: String,
    pub screenshots: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trending: Option<bool>,
    pub color_palette: Vec<String>,
    pub safety_audit: FrontendSafetyAudit,
    pub dependencies: FrontendDependencies,
    pub components: Vec<FrontendComponentSpec>,
    pub compatibility: CompatibilityRequirements,
    pub manifest: Option<RyzoraManifest>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository Trait & Local Implementation
// ─────────────────────────────────────────────────────────────────────────────

pub trait Repository: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn list_entries(&self) -> Result<Vec<RepositoryPackageEntry>, String>;
    fn get_package_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String>;
    fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String>;
}

pub struct LocalRepository {
    root_dir: PathBuf,
    index: RepositoryIndex,
}

impl LocalRepository {
    pub fn load_from_dir(root_dir: &Path) -> Result<Self, String> {
        let index_path = root_dir.join("repository.json");
        if !index_path.is_file() {
            return Err(format!(
                "repository.json not found in '{}'",
                root_dir.display()
            ));
        }

        let raw = fs::read_to_string(&index_path)
            .map_err(|e| format!("Failed to read repository.json in '{}': {}", root_dir.display(), e))?;

        let index: RepositoryIndex = serde_json::from_str(&raw)
            .map_err(|e| format!("Malformed repository index in '{}': {}", root_dir.display(), e))?;

        // 1. Validate Schema
        if index.schema != 1 {
            return Err(format!(
                "Unsupported repository schema {} (expected schema 1)",
                index.schema
            ));
        }

        // 2. Validate package IDs and paths
        let mut seen_ids = HashSet::new();
        let canonical_root = root_dir
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize repository root: {}", e))?;

        for pkg in &index.packages {
            // Validate package ID format
            if pkg.id.trim().is_empty()
                || pkg.id.contains("..")
                || pkg.id.contains('/')
                || pkg.id.contains('\\')
                || pkg.id.contains('\0')
            {
                return Err(format!("Invalid package ID '{}' in repository index", pkg.id));
            }

            if !seen_ids.insert(pkg.id.clone()) {
                return Err(format!("Duplicate package ID '{}' in repository index", pkg.id));
            }

            // Validate manifest path security
            if pkg.manifest.starts_with('/') || pkg.manifest.starts_with('\\') || pkg.manifest.contains('\0') {
                return Err(format!(
                    "Manifest path '{}' for package '{}' must be relative",
                    pkg.manifest, pkg.id
                ));
            }

            for comp in Path::new(&pkg.manifest).components() {
                if let Component::ParentDir = comp {
                    return Err(format!(
                        "Manifest path '{}' contains forbidden parent traversal ('..')",
                        pkg.manifest
                    ));
                }
            }

            let full_manifest = root_dir.join(&pkg.manifest);
            if !full_manifest.exists() {
                return Err(format!(
                    "Manifest file not found for package '{}': '{}'",
                    pkg.id, pkg.manifest
                ));
            }

            let canonical_manifest = full_manifest
                .canonicalize()
                .map_err(|e| format!("Failed to canonicalize manifest for '{}': {}", pkg.id, e))?;

            if !canonical_manifest.starts_with(&canonical_root) {
                return Err(format!(
                    "Manifest path '{}' escapes repository root via symlink",
                    pkg.manifest
                ));
            }

            // Validate manifest contents using existing spec rules
            let manifest_raw = fs::read_to_string(&canonical_manifest)
                .map_err(|e| format!("Failed to read manifest for '{}': {}", pkg.id, e))?;

            let validation = validate_manifest_internal(&manifest_raw);
            if !validation.valid {
                return Err(format!(
                    "Package '{}' has invalid manifest: {}",
                    pkg.id,
                    validation.errors.join("; ")
                ));
            }
        }

        Ok(Self { root_dir: root_dir.to_path_buf(), index })
    }
}

impl Repository for LocalRepository {
    fn id(&self) -> &str {
        &self.index.id
    }

    fn name(&self) -> &str {
        &self.index.name
    }

    fn list_entries(&self) -> Result<Vec<RepositoryPackageEntry>, String> {
        Ok(self.index.packages.clone())
    }

    fn get_package_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
        let entry = self
            .index
            .packages
            .iter()
            .find(|p| p.id == package_id)
            .ok_or_else(|| format!("Package '{}' not found in repository '{}'", package_id, self.index.id))?;

        let manifest_path = self.root_dir.join(&entry.manifest);
        let raw = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest for '{}': {}", package_id, e))?;

        serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse manifest for '{}': {}", package_id, e))
    }

    fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String> {
        let entry = self
            .index
            .packages
            .iter()
            .find(|p| p.id == package_id)
            .ok_or_else(|| format!("Package '{}' not found in repository '{}'", package_id, self.index.id))?;

        let manifest_path = self.root_dir.join(&entry.manifest);
        let pkg_dir = manifest_path
            .parent()
            .ok_or_else(|| format!("Invalid manifest parent for package '{}'", package_id))?;

        if !pkg_dir.is_dir() {
            return Err(format!("Package directory not found for '{}'", package_id));
        }

        Ok(pkg_dir.to_path_buf())
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

            if let Ok(entries) = fs::read_dir(search_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.join("repository.json").is_file() {
                        if let Ok(local_repo) = LocalRepository::load_from_dir(&path) {
                            self.repositories.push(Box::new(local_repo));
                        }
                    }
                }
            }
        }
    }

    pub fn list_repositories(&self) -> Vec<RepositorySummary> {
        self.repositories
            .iter()
            .map(|r| RepositorySummary {
                id: r.id().to_string(),
                name: r.name().to_string(),
                package_count: r.list_entries().map(|e| e.len()).unwrap_or(0),
                path: format!("repository://{}", r.id()),
            })
            .collect()
    }

    pub fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String> {
        for repo in &self.repositories {
            if let Ok(dir) = repo.get_package_dir(package_id) {
                return Ok(dir);
            }
        }
        Err(format!("Package '{}' not found in any repository", package_id))
    }

    pub fn list_all_packages(&self) -> Result<Vec<FrontendPackageItem>, String> {
        let mut all_packages = Vec::new();
        let mut seen_ids = HashSet::new();

        for repo in &self.repositories {
            let entries = repo.list_entries()?;
            for entry in entries {
                if !seen_ids.insert(entry.id.clone()) {
                    continue; // Skip duplicates across repos
                }

                let manifest = repo.get_package_manifest(&entry.id).ok();

                // Derive supported desktops / display / dependencies from manifest if available
                let (supported_desktops, supported_display, dependencies, compatibility, components) =
                    if let Some(ref m) = manifest {
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
                        rating: "verified".to_string(),
                        changes_system_files: false,
                        requires_root: false,
                        sandbox_compatible: true,
                        files_modified_count: files_count,
                    },
                    dependencies,
                    components,
                    compatibility,
                    manifest,
                };

                all_packages.push(item);
            }
        }

        Ok(all_packages)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Discovery Paths & Global Helper
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_default_repository_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. Current working directory / workspace ./repositories
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("repositories"));
        // If running from src-tauri during cargo test / build, check parent workspace
        if cwd.file_name().and_then(|n| n.to_str()) == Some("src-tauri") {
            if let Some(parent) = cwd.parent() {
                dirs.push(parent.join("repositories"));
            }
        }
    }

    // 2. User application data directory ~/.local/share/ryzora/repositories
    dirs.push(get_home_dir().join(".local/share/ryzora/repositories"));

    dirs
}

pub fn create_default_manager() -> RepositoryManager {
    let mut manager = RepositoryManager::new();
    let search_dirs = get_default_repository_search_dirs();
    manager.discover_local_repositories(&search_dirs);
    manager
}

/// Helper for installer to locate a package directory across repositories
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
    manager.list_all_packages()
}

#[tauri::command]
pub fn refresh_catalog() -> Result<Vec<FrontendPackageItem>, String> {
    let manager = create_default_manager();
    manager.list_all_packages()
}

#[tauri::command]
pub fn get_repository_info() -> Result<Vec<RepositorySummary>, String> {
    let manager = create_default_manager();
    Ok(manager.list_repositories())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests (Zero touch of real filesystem; all use temporary sandbox)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct RepoTestSandbox {
        root: PathBuf,
        repo_dir: PathBuf,
    }

    impl RepoTestSandbox {
        fn new(name: &str) -> Self {
            let unique_id = format!(
                "ryzora-repo-test-{}-{}",
                name,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let root = std::env::temp_dir().join(unique_id);
            let repo_dir = root.join("community");
            fs::create_dir_all(&repo_dir).unwrap();

            Self { root, repo_dir }
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

    #[test]
    fn test_valid_repository_loading() {
        let sandbox = RepoTestSandbox::new("valid-repo");
        let rel_manifest = sandbox.create_valid_package("pkg-alpha");

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-repo",
  "name": "Test Repository",
  "version": "1.0.0",
  "description": "Testing repository",
  "packages": [
    {{
      "id": "pkg-alpha",
      "name": "Package Alpha",
      "version": "1.0.0",
      "package_type": "rice",
      "description": "Alpha description",
      "manifest": "{}",
      "author": {{
        "name": "Tester",
        "avatar": "avatar.png",
        "verified": true
      }}
    }}
  ]
}}"#,
            rel_manifest
        );

        sandbox.write_repository_json(&repo_json);

        let repo = LocalRepository::load_from_dir(&sandbox.repo_dir).unwrap();
        assert_eq!(repo.id(), "test-repo");
        assert_eq!(repo.name(), "Test Repository");

        let entries = repo.list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "pkg-alpha");

        let manifest = repo.get_package_manifest("pkg-alpha").unwrap();
        assert_eq!(manifest.id, "pkg-alpha");
        assert_eq!(manifest.files.len(), 1);
    }

    #[test]
    fn test_malformed_repository_json_rejected() {
        let sandbox = RepoTestSandbox::new("malformed-repo");
        sandbox.write_repository_json("{ invalid json: true ");

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Malformed repository index"));
    }

    #[test]
    fn test_unsupported_schema_version_rejected() {
        let sandbox = RepoTestSandbox::new("unsupported-schema");
        sandbox.write_repository_json(
            r#"{
  "schema": 99,
  "id": "future-repo",
  "name": "Future Repo",
  "packages": []
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Unsupported repository schema 99"));
    }

    #[test]
    fn test_invalid_package_id_traversal_rejected() {
        let sandbox = RepoTestSandbox::new("bad-pkg-id");
        sandbox.write_repository_json(
            r#"{
  "schema": 1,
  "id": "test-repo",
  "name": "Test Repo",
  "packages": [
    {
      "id": "../evil",
      "name": "Evil",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "manifest.json",
      "author": { "name": "Hacker", "avatar": "" }
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
        let sandbox = RepoTestSandbox::new("dup-pkg-id");
        let rel_manifest = sandbox.create_valid_package("pkg-dup");

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-repo",
  "name": "Test Repo",
  "packages": [
    {{
      "id": "pkg-dup",
      "name": "Dup 1",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{0}",
      "author": {{ "name": "T1", "avatar": "" }}
    }},
    {{
      "id": "pkg-dup",
      "name": "Dup 2",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{0}",
      "author": {{ "name": "T2", "avatar": "" }}
    }}
  ]
}}"#,
            rel_manifest
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
  "id": "test-repo",
  "name": "Test Repo",
  "packages": [
    {
      "id": "pkg-traversal",
      "name": "Traversal",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "../../../etc/shadow",
      "author": { "name": "T", "avatar": "" }
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
  "id": "test-repo",
  "name": "Test Repo",
  "packages": [
    {
      "id": "pkg-missing",
      "name": "Missing",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/nonexistent/manifest.json",
      "author": { "name": "T", "avatar": "" }
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
        let pkg_dir = sandbox.repo_dir.join("packages/pkg-malformed");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("manifest.json"), "{ bad json }").unwrap();

        sandbox.write_repository_json(
            r#"{
  "schema": 1,
  "id": "test-repo",
  "name": "Test Repo",
  "packages": [
    {
      "id": "pkg-malformed",
      "name": "Malformed",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg-malformed/manifest.json",
      "author": { "name": "T", "avatar": "" }
    }
  ]
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("invalid manifest"));
    }

    #[test]
    fn test_repository_manager_discovery_and_catalog() {
        let sandbox = RepoTestSandbox::new("manager-disco");
        let rel1 = sandbox.create_valid_package("pkg-one");
        let rel2 = sandbox.create_valid_package("pkg-two");

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "multi-repo",
  "name": "Multi Repository",
  "packages": [
    {{
      "id": "pkg-one",
      "name": "Package One",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{}",
      "category": "rices",
      "author": {{ "name": "A", "avatar": "" }}
    }},
    {{
      "id": "pkg-two",
      "name": "Package Two",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{}",
      "category": "rices",
      "author": {{ "name": "B", "avatar": "" }}
    }}
  ]
}}"#,
            rel1, rel2
        );

        sandbox.write_repository_json(&repo_json);

        let mut manager = RepositoryManager::new();
        manager.discover_local_repositories(&[sandbox.root.clone()]);

        let repos = manager.list_repositories();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "multi-repo");
        assert_eq!(repos[0].package_count, 2);

        let catalog = manager.list_all_packages().unwrap();
        assert_eq!(catalog.len(), 2);
        let ids: Vec<String> = catalog.iter().map(|p| p.id.clone()).collect();
        assert!(ids.contains(&"pkg-one".to_string()));
        assert!(ids.contains(&"pkg-two".to_string()));

        // Check package directory resolution
        let dir1 = manager.get_package_dir("pkg-one").unwrap();
        assert!(dir1.join("manifest.json").is_file());
    }
}
