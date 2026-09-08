use crate::manifest::RyzoraManifest;
use crate::system::{check_binary, SystemInfo};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Dependency Data Model (Phase 9)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Package,
    SystemBinary,
    DesktopCapability,
    RuntimeCapability,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySpec {
    pub id: String,
    pub kind: DependencyKind,
    #[serde(default)]
    pub version_req: Option<String>,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyStatus {
    Satisfied,
    Missing,
    Incompatible,
    Conflict,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPackageNode {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub version_req: Option<String>,
    pub required: bool,
    pub repository_id: Option<String>,
    pub status: DependencyStatus,
    pub status_message: Option<String>,
    pub dependencies: Vec<DependencySpec>,
    pub transitive_packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSystemDependency {
    pub binary: String,
    pub required: bool,
    pub status: DependencyStatus,
    pub path: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedCapabilityDependency {
    pub capability: String,
    pub kind: DependencyKind,
    pub required: bool,
    pub status: DependencyStatus,
    pub current_value: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolutionReport {
    pub root_package_id: String,
    pub resolved: bool,
    pub root_package: Option<ResolvedPackageNode>,
    pub packages: Vec<ResolvedPackageNode>,
    pub system_dependencies: Vec<ResolvedSystemDependency>,
    pub capability_dependencies: Vec<ResolvedCapabilityDependency>,
    pub missing_required: Vec<String>,
    pub missing_optional: Vec<String>,
    pub conflicts: Vec<String>,
    pub cycles: Vec<Vec<String>>,
    pub install_order: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Package Provider Interface
// ─────────────────────────────────────────────────────────────────────────────

pub trait PackageProvider {
    fn get_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String>;
    fn get_repository_id(&self, package_id: &str) -> Option<String>;
}

pub struct RepositoryPackageProvider {
    manager: crate::repository::RepositoryManager,
}

impl RepositoryPackageProvider {
    pub fn new() -> Self {
        Self {
            manager: crate::repository::create_default_manager(),
        }
    }
}

impl Default for RepositoryPackageProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageProvider for RepositoryPackageProvider {
    fn get_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
        self.manager.get_package_manifest(package_id)
    }

    fn get_repository_id(&self, package_id: &str) -> Option<String> {
        for repo in self.manager.repositories() {
            if let Ok(entries) = repo.list_entries() {
                if entries.iter().any(|e| e.id == package_id) {
                    return Some(repo.id().to_string());
                }
            }
        }
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Dependency Resolver Engine
// ─────────────────────────────────────────────────────────────────────────────

pub struct DependencyResolver<'a, P: PackageProvider> {
    provider: &'a P,
    system_info: &'a SystemInfo,
}

impl<'a, P: PackageProvider> DependencyResolver<'a, P> {
    pub fn new(provider: &'a P, system_info: &'a SystemInfo) -> Self {
        Self {
            provider,
            system_info,
        }
    }

    /// Resolve all dependencies starting from a package ID in the catalog.
    pub fn resolve(&self, root_package_id: &str) -> Result<DependencyResolutionReport, String> {
        let manifest = self.provider.get_manifest(root_package_id).map_err(|e| {
            format!(
                "Failed to load manifest for root package '{}': {}",
                root_package_id, e
            )
        })?;
        Ok(self.resolve_manifest(&manifest))
    }

    /// Resolve dependencies starting from an existing manifest.
    pub fn resolve_manifest(&self, root_manifest: &RyzoraManifest) -> DependencyResolutionReport {
        let root_id = root_manifest.id.clone();
        let mut packages_map: HashMap<String, ResolvedPackageNode> = HashMap::new();
        let mut version_requirements: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();
        let mut cycles: Vec<Vec<String>> = Vec::new();
        let mut conflicts: Vec<String> = Vec::new();

        let mut system_deps_map: HashMap<String, ResolvedSystemDependency> = HashMap::new();
        let mut capability_deps_map: HashMap<String, ResolvedCapabilityDependency> = HashMap::new();

        // 1. Recursive resolution traversal
        let mut call_stack: Vec<String> = Vec::new();
        self.traverse_package(
            root_manifest,
            None,
            true,
            &mut call_stack,
            &mut packages_map,
            &mut version_requirements,
            &mut adj_list,
            &mut cycles,
            &mut conflicts,
            &mut system_deps_map,
            &mut capability_deps_map,
        );

        // 2. Validate multi-package version constraints
        for (pkg_id, reqs) in &version_requirements {
            if let Some(pkg_node) = packages_map.get(pkg_id) {
                if let Ok(pkg_ver) = Version::parse(&pkg_node.version) {
                    for (requester, req_str) in reqs {
                        if let Ok(req) = VersionReq::parse(req_str) {
                            if !req.matches(&pkg_ver) {
                                conflicts.push(format!(
                                    "Package '{}' requires '{} {}', but version {} is resolved",
                                    requester, pkg_id, req_str, pkg_node.version
                                ));
                            }
                        }
                    }
                }
            }
        }

        // 3. Topological sorting (leaves first, root last)
        let mut install_order = Vec::new();
        if cycles.is_empty() {
            let mut visited = HashSet::new();
            self.topological_sort(&root_id, &adj_list, &mut visited, &mut install_order);
        }

        // 4. Summarize missing dependencies
        let mut missing_required = Vec::new();
        let mut missing_optional = Vec::new();

        for sys_dep in system_deps_map.values() {
            if sys_dep.status == DependencyStatus::Missing {
                if sys_dep.required {
                    missing_required.push(sys_dep.binary.clone());
                } else {
                    missing_optional.push(sys_dep.binary.clone());
                }
            }
        }

        for cap_dep in capability_deps_map.values() {
            if cap_dep.status == DependencyStatus::Missing
                || cap_dep.status == DependencyStatus::Incompatible
            {
                if cap_dep.required {
                    missing_required.push(cap_dep.capability.clone());
                } else {
                    missing_optional.push(cap_dep.capability.clone());
                }
            }
        }

        for pkg in packages_map.values() {
            if pkg.status == DependencyStatus::Missing && pkg.required {
                missing_required.push(format!("package:{}", pkg.package_id));
            } else if pkg.status == DependencyStatus::Incompatible && pkg.required {
                conflicts.push(format!(
                    "Package '{}' is incompatible: {}",
                    pkg.package_id,
                    pkg.status_message.as_deref().unwrap_or("unknown reason")
                ));
            }
        }

        let is_resolved = missing_required.is_empty() && conflicts.is_empty() && cycles.is_empty();

        let root_node = packages_map.get(&root_id).cloned();
        let mut all_packages: Vec<ResolvedPackageNode> = packages_map.into_values().collect();
        all_packages.sort_by(|a, b| a.package_id.cmp(&b.package_id));

        let mut all_sys_deps: Vec<ResolvedSystemDependency> =
            system_deps_map.into_values().collect();
        all_sys_deps.sort_by(|a, b| a.binary.cmp(&b.binary));

        let mut all_cap_deps: Vec<ResolvedCapabilityDependency> =
            capability_deps_map.into_values().collect();
        all_cap_deps.sort_by(|a, b| a.capability.cmp(&b.capability));

        DependencyResolutionReport {
            root_package_id: root_id,
            resolved: is_resolved,
            root_package: root_node,
            packages: all_packages,
            system_dependencies: all_sys_deps,
            capability_dependencies: all_cap_deps,
            missing_required,
            missing_optional,
            conflicts,
            cycles,
            install_order,
        }
    }

    fn traverse_package(
        &self,
        manifest: &RyzoraManifest,
        version_req: Option<String>,
        required: bool,
        call_stack: &mut Vec<String>,
        packages: &mut HashMap<String, ResolvedPackageNode>,
        version_requirements: &mut HashMap<String, Vec<(String, String)>>,
        adj_list: &mut HashMap<String, Vec<String>>,
        cycles: &mut Vec<Vec<String>>,
        conflicts: &mut Vec<String>,
        system_deps: &mut HashMap<String, ResolvedSystemDependency>,
        capability_deps: &mut HashMap<String, ResolvedCapabilityDependency>,
    ) {
        let pkg_id = manifest.id.clone();

        // Check for cycle
        if let Some(pos) = call_stack.iter().position(|id| id == &pkg_id) {
            let mut cycle = call_stack[pos..].to_vec();
            cycle.push(pkg_id.clone());
            cycles.push(cycle);
            return;
        }

        call_stack.push(pkg_id.clone());

        // Check if package already processed
        if let Some(existing) = packages.get(&pkg_id) {
            if let Some(ref req_str) = version_req {
                if let Ok(req) = VersionReq::parse(req_str) {
                    if let Ok(ver) = Version::parse(&existing.version) {
                        if !req.matches(&ver) {
                            conflicts.push(format!(
                                "Conflicting version requirement for package '{}': requires '{}', but '{}' is already resolved",
                                pkg_id, req_str, existing.version
                            ));
                        }
                    }
                }
            }
            call_stack.pop();
            return;
        }

        // Validate SemVer of this package against requirement
        let mut status = DependencyStatus::Satisfied;
        let mut status_message = None;

        if let Some(ref req_str) = version_req {
            match VersionReq::parse(req_str) {
                Ok(req) => match Version::parse(&manifest.version) {
                    Ok(ver) => {
                        if !req.matches(&ver) {
                            status = DependencyStatus::Incompatible;
                            status_message = Some(format!(
                                "Version {} does not satisfy requirement '{}'",
                                manifest.version, req_str
                            ));
                        }
                    }
                    Err(e) => {
                        status = DependencyStatus::Incompatible;
                        status_message = Some(format!(
                            "Invalid semver version '{}': {}",
                            manifest.version, e
                        ));
                    }
                },
                Err(e) => {
                    status = DependencyStatus::Incompatible;
                    status_message =
                        Some(format!("Invalid semver requirement '{}': {}", req_str, e));
                }
            }
        }

        let repo_id = self.provider.get_repository_id(&pkg_id);
        let all_deps = manifest.all_dependencies();
        let mut child_package_ids = Vec::new();

        // Process all dependencies declared by this package
        for dep in &all_deps {
            match dep.kind {
                DependencyKind::Package => {
                    child_package_ids.push(dep.id.clone());
                    if let Some(ref req_str) = dep.version_req {
                        version_requirements
                            .entry(dep.id.clone())
                            .or_default()
                            .push((pkg_id.clone(), req_str.clone()));
                    }

                    // Attempt to resolve child package from provider
                    match self.provider.get_manifest(&dep.id) {
                        Ok(child_manifest) => {
                            self.traverse_package(
                                &child_manifest,
                                dep.version_req.clone(),
                                dep.required,
                                call_stack,
                                packages,
                                version_requirements,
                                adj_list,
                                cycles,
                                conflicts,
                                system_deps,
                                capability_deps,
                            );
                        }
                        Err(e) => {
                            let child_node = ResolvedPackageNode {
                                package_id: dep.id.clone(),
                                name: dep.id.clone(),
                                version: "unknown".to_string(),
                                version_req: dep.version_req.clone(),
                                required: dep.required,
                                repository_id: None,
                                status: DependencyStatus::Missing,
                                status_message: Some(format!(
                                    "Package not found in any repository: {}",
                                    e
                                )),
                                dependencies: Vec::new(),
                                transitive_packages: Vec::new(),
                            };
                            packages.insert(dep.id.clone(), child_node);
                        }
                    }
                }
                DependencyKind::SystemBinary => {
                    let (installed, path) = check_binary(&dep.id);
                    let dep_status = if installed {
                        DependencyStatus::Satisfied
                    } else {
                        DependencyStatus::Missing
                    };

                    let entry = system_deps.entry(dep.id.clone()).or_insert_with(|| {
                        ResolvedSystemDependency {
                            binary: dep.id.clone(),
                            required: dep.required,
                            status: dep_status,
                            path,
                            description: dep.description.clone(),
                        }
                    });

                    // If any dependent marks it required, it becomes required
                    if dep.required {
                        entry.required = true;
                    }
                }
                DependencyKind::DesktopCapability => {
                    let (status, current_val) = self.evaluate_desktop_capability(&dep.id);
                    let entry = capability_deps.entry(dep.id.clone()).or_insert_with(|| {
                        ResolvedCapabilityDependency {
                            capability: dep.id.clone(),
                            kind: DependencyKind::DesktopCapability,
                            required: dep.required,
                            status,
                            current_value: current_val,
                            description: dep.description.clone(),
                        }
                    });
                    if dep.required {
                        entry.required = true;
                    }
                }
                DependencyKind::RuntimeCapability => {
                    let (status, current_val) = self.evaluate_runtime_capability(&dep.id);
                    let entry = capability_deps.entry(dep.id.clone()).or_insert_with(|| {
                        ResolvedCapabilityDependency {
                            capability: dep.id.clone(),
                            kind: DependencyKind::RuntimeCapability,
                            required: dep.required,
                            status,
                            current_value: current_val,
                            description: dep.description.clone(),
                        }
                    });
                    if dep.required {
                        entry.required = true;
                    }
                }
            }
        }

        adj_list.insert(pkg_id.clone(), child_package_ids.clone());

        let node = ResolvedPackageNode {
            package_id: pkg_id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            version_req,
            required,
            repository_id: repo_id,
            status,
            status_message,
            dependencies: all_deps,
            transitive_packages: child_package_ids,
        };

        packages.insert(pkg_id, node);
        call_stack.pop();
    }

    fn topological_sort(
        &self,
        node: &str,
        adj: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.to_string());

        if let Some(children) = adj.get(node) {
            for child in children {
                self.topological_sort(child, adj, visited, order);
            }
        }

        order.push(node.to_string());
    }

    /// Secure capability check for desktop environment / window manager / display protocol.
    /// Pure environment / system detection without external process execution.
    fn evaluate_desktop_capability(&self, cap: &str) -> (DependencyStatus, Option<String>) {
        let cap_lower = cap.to_lowercase();
        let sys_session = self.system_info.session_type.to_lowercase();
        let sys_wm = self.system_info.window_manager.to_lowercase();
        let sys_de = self.system_info.desktop_environment.to_lowercase();

        // Universal check
        if cap_lower == "universal" || cap_lower == "all" || cap_lower == "any" {
            return (DependencyStatus::Satisfied, Some("universal".to_string()));
        }

        // Display session checks
        if cap_lower == "wayland" || cap_lower == "x11" {
            if sys_session == cap_lower {
                return (
                    DependencyStatus::Satisfied,
                    Some(self.system_info.session_type.clone()),
                );
            } else {
                return (
                    DependencyStatus::Incompatible,
                    Some(self.system_info.session_type.clone()),
                );
            }
        }

        // Window manager / desktop environment checks
        if sys_wm.contains(&cap_lower) || sys_de.contains(&cap_lower) {
            let current = if !sys_wm.is_empty() { sys_wm } else { sys_de };
            return (DependencyStatus::Satisfied, Some(current));
        }

        // Check if environment variables indicate the desktop
        if let Ok(xdg_current) = std::env::var("XDG_CURRENT_DESKTOP") {
            if xdg_current.to_lowercase().contains(&cap_lower) {
                return (DependencyStatus::Satisfied, Some(xdg_current));
            }
        }

        (
            DependencyStatus::Incompatible,
            Some(format!("current WM: '{}', DE: '{}'", sys_wm, sys_de)),
        )
    }

    /// Secure capability check for runtime environments.
    /// Pure filesystem checks without external process execution.
    fn evaluate_runtime_capability(&self, cap: &str) -> (DependencyStatus, Option<String>) {
        let cap_lower = cap.to_lowercase();
        match cap_lower.as_str() {
            "systemd" => {
                if Path::new("/run/systemd/system").exists() {
                    (
                        DependencyStatus::Satisfied,
                        Some("systemd running (/run/systemd/system)".to_string()),
                    )
                } else {
                    (DependencyStatus::Missing, None)
                }
            }
            "dbus" => {
                if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok()
                    || Path::new("/run/user").exists()
                {
                    (
                        DependencyStatus::Satisfied,
                        Some("D-Bus session bus active".to_string()),
                    )
                } else {
                    (DependencyStatus::Missing, None)
                }
            }
            "pipewire" => {
                let (has_bin, bin_path) = check_binary("pipewire");
                if has_bin {
                    (DependencyStatus::Satisfied, bin_path)
                } else {
                    (DependencyStatus::Missing, None)
                }
            }
            "dri" | "opengl" => {
                if Path::new("/dev/dri").exists() {
                    (
                        DependencyStatus::Satisfied,
                        Some("/dev/dri accessible".to_string()),
                    )
                } else {
                    (DependencyStatus::Missing, None)
                }
            }
            "vulkan" => {
                let icd_exists = Path::new("/usr/share/vulkan/icd.d").exists()
                    || Path::new("/etc/vulkan/icd.d").exists();
                if icd_exists {
                    (
                        DependencyStatus::Satisfied,
                        Some("Vulkan ICD drivers found".to_string()),
                    )
                } else {
                    (DependencyStatus::Missing, None)
                }
            }
            "audio" | "sound" => {
                if Path::new("/dev/snd").exists() {
                    (
                        DependencyStatus::Satisfied,
                        Some("/dev/snd available".to_string()),
                    )
                } else {
                    (DependencyStatus::Missing, None)
                }
            }
            _ => {
                // Fallback: check if tool with matching name exists in PATH
                let (has_bin, bin_path) = check_binary(&cap_lower);
                if has_bin {
                    (DependencyStatus::Satisfied, bin_path)
                } else {
                    (DependencyStatus::Unknown, None)
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Command Handlers
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn resolve_package_dependencies(
    package_id: String,
) -> Result<DependencyResolutionReport, String> {
    let provider = RepositoryPackageProvider::new();
    let sys_info = crate::system::detect_system_info();
    let resolver = DependencyResolver::new(&provider, &sys_info);
    resolver.resolve(&package_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// In-Memory Package Provider for Unit Testing
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub struct MockPackageProvider {
    pub manifests: HashMap<String, (RyzoraManifest, Option<String>)>,
}

#[cfg(test)]
impl MockPackageProvider {
    pub fn new() -> Self {
        Self {
            manifests: HashMap::new(),
        }
    }

    pub fn add(&mut self, manifest: RyzoraManifest, repo_id: Option<&str>) {
        self.manifests.insert(
            manifest.id.clone(),
            (manifest, repo_id.map(|s| s.to_string())),
        );
    }
}

#[cfg(test)]
impl PackageProvider for MockPackageProvider {
    fn get_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
        self.manifests
            .get(package_id)
            .map(|(m, _)| m.clone())
            .ok_or_else(|| format!("Package '{}' not found in mock provider", package_id))
    }

    fn get_repository_id(&self, package_id: &str) -> Option<String> {
        self.manifests.get(package_id).and_then(|(_, r)| r.clone())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit Tests (Phase 9)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ManifestCompatibility, PackageType};

    fn make_test_sys() -> SystemInfo {
        SystemInfo {
            distro_name: "Arch Linux".to_string(),
            distro_id: "arch".to_string(),
            distro_family: "arch".to_string(),
            distro_version: "rolling".to_string(),
            kernel_version: "6.8.0".to_string(),
            desktop_environment: "Hyprland".to_string(),
            window_manager: "Hyprland".to_string(),
            session_type: "wayland".to_string(),
            shell: "zsh".to_string(),
            terminal: "kitty".to_string(),
            installed_components: vec![],
        }
    }

    fn make_manifest(
        id: &str,
        version: &str,
        deps: Vec<DependencySpec>,
        req_apps: Vec<&str>,
        opt_apps: Vec<&str>,
    ) -> RyzoraManifest {
        RyzoraManifest {
            id: id.to_string(),
            name: format!("Package {}", id),
            version: version.to_string(),
            ryzora_spec: "1".to_string(),
            author: "Tester".to_string(),
            package_type: PackageType::Rice,
            description: "Test description".to_string(),
            tags: vec![],
            color_palette: vec![],
            compatibility: ManifestCompatibility {
                desktops: vec!["hyprland".to_string()],
                sessions: vec!["wayland".to_string()],
                distros: vec![],
                required: req_apps.into_iter().map(|s| s.to_string()).collect(),
                optional: opt_apps.into_iter().map(|s| s.to_string()).collect(),
            },
            files: vec![],
            dependencies: deps,
        }
    }

    #[test]
    fn test_resolver_simple_satisfied_package() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        let root = make_manifest("root-pkg", "1.0.0", vec![], vec![], vec![]);
        provider.add(root.clone(), Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("root-pkg").unwrap();

        assert!(report.resolved);
        assert_eq!(report.root_package_id, "root-pkg");
        assert_eq!(report.install_order, vec!["root-pkg"]);
        assert!(report.missing_required.is_empty());
        assert!(report.conflicts.is_empty());
        assert!(report.cycles.is_empty());
    }

    #[test]
    fn test_resolver_semver_constraint_matching() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Dep package version 1.2.3
        let dep = make_manifest("dep-theme", "1.2.3", vec![], vec![], vec![]);
        provider.add(dep, Some("community"));

        // Root requires ^1.2.0 (satisfied by 1.2.3)
        let root = make_manifest(
            "root-rice",
            "1.0.0",
            vec![DependencySpec {
                id: "dep-theme".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^1.2.0".to_string()),
                required: true,
                description: Some("Theme".to_string()),
            }],
            vec![],
            vec![],
        );
        provider.add(root, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("root-rice").unwrap();

        assert!(report.resolved);
        assert_eq!(report.packages.len(), 2);
        assert_eq!(report.install_order, vec!["dep-theme", "root-rice"]);
    }

    #[test]
    fn test_resolver_semver_incompatible_rejected() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Dep package version 2.0.0
        let dep = make_manifest("dep-theme", "2.0.0", vec![], vec![], vec![]);
        provider.add(dep, Some("community"));

        // Root requires ^1.0.0 (incompatible with 2.0.0)
        let root = make_manifest(
            "root-rice",
            "1.0.0",
            vec![DependencySpec {
                id: "dep-theme".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^1.0.0".to_string()),
                required: true,
                description: Some("Theme".to_string()),
            }],
            vec![],
            vec![],
        );
        provider.add(root, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("root-rice").unwrap();

        assert!(!report.resolved);
        assert!(!report.conflicts.is_empty());
        assert!(report
            .conflicts
            .iter()
            .any(|c| c.contains("does not satisfy requirement") || c.contains("resolved")));
    }

    #[test]
    fn test_resolver_transitive_multi_level() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Level 3: base-icons
        let c = make_manifest("base-icons", "1.0.0", vec![], vec![], vec![]);
        provider.add(c, Some("community"));

        // Level 2: theme depends on base-icons
        let b = make_manifest(
            "cool-theme",
            "1.1.0",
            vec![DependencySpec {
                id: "base-icons".to_string(),
                kind: DependencyKind::Package,
                version_req: Some(">=1.0.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(b, Some("community"));

        // Level 1: rice depends on cool-theme
        let a = make_manifest(
            "cyber-rice",
            "2.0.0",
            vec![DependencySpec {
                id: "cool-theme".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^1.0.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(a, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("cyber-rice").unwrap();

        assert!(report.resolved);
        assert_eq!(report.packages.len(), 3);
        // Topological order must install base-icons first, then cool-theme, then cyber-rice
        assert_eq!(
            report.install_order,
            vec!["base-icons", "cool-theme", "cyber-rice"]
        );
    }

    #[test]
    fn test_resolver_diamond_dependency_resolution() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Diamond: A -> B -> D and A -> C -> D
        let d = make_manifest("shared-font", "1.5.0", vec![], vec![], vec![]);
        let b = make_manifest(
            "bar-pkg",
            "1.0.0",
            vec![DependencySpec {
                id: "shared-font".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^1.0.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        let c = make_manifest(
            "term-pkg",
            "1.0.0",
            vec![DependencySpec {
                id: "shared-font".to_string(),
                kind: DependencyKind::Package,
                version_req: Some(">=1.2.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        let a = make_manifest(
            "bundle-pkg",
            "1.0.0",
            vec![
                DependencySpec {
                    id: "bar-pkg".to_string(),
                    kind: DependencyKind::Package,
                    version_req: None,
                    required: true,
                    description: None,
                },
                DependencySpec {
                    id: "term-pkg".to_string(),
                    kind: DependencyKind::Package,
                    version_req: None,
                    required: true,
                    description: None,
                },
            ],
            vec![],
            vec![],
        );

        provider.add(d, Some("community"));
        provider.add(b, Some("community"));
        provider.add(c, Some("community"));
        provider.add(a, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("bundle-pkg").unwrap();

        assert!(report.resolved);
        assert_eq!(report.packages.len(), 4);
        assert_eq!(report.install_order[0], "shared-font");
        assert_eq!(report.install_order[3], "bundle-pkg");
    }

    #[test]
    fn test_resolver_diamond_version_conflict_detected() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // D is version 1.5.0
        let d = make_manifest("shared-font", "1.5.0", vec![], vec![], vec![]);
        // B requires D ^1.0.0 (matches 1.5.0)
        let b = make_manifest(
            "bar-pkg",
            "1.0.0",
            vec![DependencySpec {
                id: "shared-font".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^1.0.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        // C requires D ^2.0.0 (conflicts with 1.5.0)
        let c = make_manifest(
            "term-pkg",
            "1.0.0",
            vec![DependencySpec {
                id: "shared-font".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^2.0.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        let a = make_manifest(
            "bundle-pkg",
            "1.0.0",
            vec![
                DependencySpec {
                    id: "bar-pkg".to_string(),
                    kind: DependencyKind::Package,
                    version_req: None,
                    required: true,
                    description: None,
                },
                DependencySpec {
                    id: "term-pkg".to_string(),
                    kind: DependencyKind::Package,
                    version_req: None,
                    required: true,
                    description: None,
                },
            ],
            vec![],
            vec![],
        );

        provider.add(d, Some("community"));
        provider.add(b, Some("community"));
        provider.add(c, Some("community"));
        provider.add(a, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("bundle-pkg").unwrap();

        assert!(!report.resolved);
        assert!(!report.conflicts.is_empty());
        assert!(report.conflicts.iter().any(|c| c.contains("shared-font")));
    }

    #[test]
    fn test_resolver_circular_dependency_detection() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Cycle: A -> B -> C -> A
        let c = make_manifest(
            "pkg-c",
            "1.0.0",
            vec![DependencySpec {
                id: "pkg-a".to_string(),
                kind: DependencyKind::Package,
                version_req: None,
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        let b = make_manifest(
            "pkg-b",
            "1.0.0",
            vec![DependencySpec {
                id: "pkg-c".to_string(),
                kind: DependencyKind::Package,
                version_req: None,
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        let a = make_manifest(
            "pkg-a",
            "1.0.0",
            vec![DependencySpec {
                id: "pkg-b".to_string(),
                kind: DependencyKind::Package,
                version_req: None,
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );

        provider.add(c, Some("community"));
        provider.add(b, Some("community"));
        provider.add(a, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("pkg-a").unwrap();

        assert!(!report.resolved);
        assert!(!report.cycles.is_empty());
        let cycle = &report.cycles[0];
        assert_eq!(cycle[0], cycle[cycle.len() - 1]);
        assert!(report.install_order.is_empty());
    }

    #[test]
    fn test_resolver_missing_required_system_binary() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // A requires nonexistent-binary-xyz
        let a = make_manifest(
            "pkg-with-missing-bin",
            "1.0.0",
            vec![DependencySpec {
                id: "nonexistent_binary_xyz_12345".to_string(),
                kind: DependencyKind::SystemBinary,
                version_req: None,
                required: true,
                description: Some("Crucial tool".to_string()),
            }],
            vec![],
            vec![],
        );
        provider.add(a, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("pkg-with-missing-bin").unwrap();

        assert!(!report.resolved);
        assert!(report
            .missing_required
            .contains(&"nonexistent_binary_xyz_12345".to_string()));
    }

    #[test]
    fn test_resolver_missing_optional_binary_does_not_block_resolved() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        let a = make_manifest(
            "pkg-with-opt-bin",
            "1.0.0",
            vec![DependencySpec {
                id: "optional_tool_xyz_99999".to_string(),
                kind: DependencyKind::SystemBinary,
                version_req: None,
                required: false,
                description: Some("Optional tool".to_string()),
            }],
            vec![],
            vec![],
        );
        provider.add(a, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("pkg-with-opt-bin").unwrap();

        // Since only optional binary is missing, package IS resolved!
        assert!(report.resolved);
        assert!(report.missing_required.is_empty());
        assert!(report
            .missing_optional
            .contains(&"optional_tool_xyz_99999".to_string()));
    }

    #[test]
    fn test_resolver_desktop_and_session_capabilities() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys(); // Hyprland, wayland

        let a = make_manifest(
            "hypr-pkg",
            "1.0.0",
            vec![
                DependencySpec {
                    id: "hyprland".to_string(),
                    kind: DependencyKind::DesktopCapability,
                    version_req: None,
                    required: true,
                    description: None,
                },
                DependencySpec {
                    id: "wayland".to_string(),
                    kind: DependencyKind::DesktopCapability,
                    version_req: None,
                    required: true,
                    description: None,
                },
            ],
            vec![],
            vec![],
        );
        provider.add(a, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("hypr-pkg").unwrap();

        assert!(report.resolved);
        assert_eq!(report.capability_dependencies.len(), 2);
        for cap in report.capability_dependencies {
            assert_eq!(cap.status, DependencyStatus::Satisfied);
        }
    }
}
