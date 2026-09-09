use crate::manifest::RyzoraManifest;
use crate::system::{check_binary, SystemInfo};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Hardening Limits (Phase 9.1)
// ─────────────────────────────────────────────────────────────────────────────

pub const MAX_DEPENDENCY_DEPTH: usize = 32;
pub const MAX_RESOLVED_NODES: usize = 128;

// ─────────────────────────────────────────────────────────────────────────────
// Dependency Data Model (Phase 9 & 9.1)
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
    #[serde(default = "default_true")]
    pub effective_required: bool,
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

    /// Retrieve all candidate manifests available for this package ID across all repositories,
    /// paired with their repository ID.
    fn get_candidates(&self, package_id: &str) -> Vec<(RyzoraManifest, String)> {
        match self.get_manifest(package_id) {
            Ok(m) => {
                let repo = self
                    .get_repository_id(package_id)
                    .unwrap_or_else(|| "unknown".to_string());
                vec![(m, repo)]
            }
            Err(_) => vec![],
        }
    }

    /// Return a list of offline or error repository descriptions.
    fn get_unavailable_repositories(&self) -> Vec<String> {
        vec![]
    }
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

    fn get_candidates(&self, package_id: &str) -> Vec<(RyzoraManifest, String)> {
        let mut candidates = Vec::new();
        for repo in self.manager.repositories() {
            if let Ok(entries) = repo.list_entries() {
                if entries.iter().any(|e| e.id == package_id) {
                    if let Ok(m) = repo.get_package_manifest(package_id) {
                        candidates.push((m, repo.id().to_string()));
                    }
                }
            }
        }
        candidates
    }

    fn get_unavailable_repositories(&self) -> Vec<String> {
        let mut unavail = Vec::new();
        for repo in self.manager.repositories() {
            if repo.status() != "online" {
                unavail.push(format!("{} ({})", repo.id(), repo.status()));
            }
        }
        unavail
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
        let root_repo_id = self.provider.get_repository_id(&root_id);
        let mut call_stack: Vec<String> = Vec::new();
        self.traverse_package(
            root_manifest,
            None,
            root_repo_id,
            true, // root package is directly required
            true, // effective required
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

        // 4. Summarize missing dependencies with transitive optional awareness
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
            if pkg.status == DependencyStatus::Missing {
                if pkg.effective_required {
                    missing_required.push(format!("package:{}", pkg.package_id));
                } else {
                    missing_optional.push(format!("package:{}", pkg.package_id));
                }
            } else if pkg.status == DependencyStatus::Incompatible {
                if pkg.effective_required {
                    conflicts.push(format!(
                        "Package '{}' is incompatible: {}",
                        pkg.package_id,
                        pkg.status_message.as_deref().unwrap_or("unknown reason")
                    ));
                } else {
                    missing_optional.push(format!(
                        "package:{} (incompatible: {})",
                        pkg.package_id,
                        pkg.status_message.as_deref().unwrap_or("unknown reason")
                    ));
                }
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
        repo_id: Option<String>,
        direct_required: bool,
        effective_required: bool,
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

        // Safety Limit 1: Max recursion depth
        if call_stack.len() >= MAX_DEPENDENCY_DEPTH {
            conflicts.push(format!(
                "Maximum dependency depth ({}) exceeded at package '{}' — possible deep recursion or malicious graph",
                MAX_DEPENDENCY_DEPTH, pkg_id
            ));
            return;
        }

        // Safety Limit 2: Max resolved node count
        if packages.len() >= MAX_RESOLVED_NODES {
            conflicts.push(format!(
                "Maximum resolved dependency count ({}) exceeded — graph too large",
                MAX_RESOLVED_NODES
            ));
            return;
        }

        // Check for cycle in current recursion path
        if let Some(pos) = call_stack.iter().position(|id| id == &pkg_id) {
            let mut cycle = call_stack[pos..].to_vec();
            cycle.push(pkg_id.clone());
            cycles.push(cycle);
            return;
        }

        call_stack.push(pkg_id.clone());

        // Check if package was already resolved
        if let Some(existing) = packages.get_mut(&pkg_id) {
            // If another path requires this package, promote effective_required to true
            if effective_required {
                existing.effective_required = true;
            }
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

        let repo_id = repo_id.or_else(|| self.provider.get_repository_id(&pkg_id));
        let all_deps = manifest.all_dependencies();
        let mut child_package_ids = Vec::new();

        // Process all dependencies declared by this package
        for dep in &all_deps {
            let child_effective_required = effective_required && dep.required;

            match dep.kind {
                DependencyKind::Package => {
                    child_package_ids.push(dep.id.clone());
                    if let Some(ref req_str) = dep.version_req {
                        version_requirements
                            .entry(dep.id.clone())
                            .or_default()
                            .push((pkg_id.clone(), req_str.clone()));
                    }

                    // Phase 9.1: Multi-repository candidate inspection & deterministic version selection
                    let candidates = self.provider.get_candidates(&dep.id);

                    if candidates.is_empty() {
                        let unavail = self.provider.get_unavailable_repositories();
                        let error_msg = if !unavail.is_empty() {
                            format!(
                                "Package '{}' not found, but repository sources [{}] are offline/error",
                                dep.id,
                                unavail.join(", ")
                            )
                        } else {
                            format!("Package '{}' not found in any repository", dep.id)
                        };

                        let child_node = ResolvedPackageNode {
                            package_id: dep.id.clone(),
                            name: dep.id.clone(),
                            version: "unknown".to_string(),
                            version_req: dep.version_req.clone(),
                            required: dep.required,
                            effective_required: child_effective_required,
                            repository_id: None,
                            status: DependencyStatus::Missing,
                            status_message: Some(error_msg),
                            dependencies: Vec::new(),
                            transitive_packages: Vec::new(),
                        };
                        packages.insert(dep.id.clone(), child_node);
                    } else {
                        // Filter candidates matching version_req
                        let mut matching: Vec<(RyzoraManifest, String)> = Vec::new();
                        let mut all_versions: Vec<String> = Vec::new();

                        for (cand_manifest, cand_repo) in candidates {
                            all_versions
                                .push(format!("v{} ({})", cand_manifest.version, cand_repo));

                            let satisfies = match &dep.version_req {
                                Some(req_str) => match VersionReq::parse(req_str) {
                                    Ok(req) => match Version::parse(&cand_manifest.version) {
                                        Ok(v) => req.matches(&v),
                                        Err(_) => false,
                                    },
                                    Err(_) => false,
                                },
                                None => true,
                            };

                            if satisfies {
                                matching.push((cand_manifest, cand_repo));
                            }
                        }

                        if matching.is_empty() {
                            let req_display = dep.version_req.as_deref().unwrap_or("any");
                            let msg = format!(
                                "Package '{}' has available versions [{}], but none satisfy requirement '{}'",
                                dep.id,
                                all_versions.join(", "),
                                req_display
                            );

                            let child_node = ResolvedPackageNode {
                                package_id: dep.id.clone(),
                                name: dep.id.clone(),
                                version: "incompatible".to_string(),
                                version_req: dep.version_req.clone(),
                                required: dep.required,
                                effective_required: child_effective_required,
                                repository_id: None,
                                status: DependencyStatus::Incompatible,
                                status_message: Some(msg),
                                dependencies: Vec::new(),
                                transitive_packages: Vec::new(),
                            };
                            packages.insert(dep.id.clone(), child_node);
                        } else {
                            // Deterministic candidate selection:
                            // 1. Highest SemVer version first
                            // 2. Local repository preferred over remote
                            // 3. Alphabetical tie-breaker on repository ID
                            matching.sort_by(|(m_a, repo_a), (m_b, repo_b)| {
                                let v_a = Version::parse(&m_a.version).ok();
                                let v_b = Version::parse(&m_b.version).ok();
                                match (v_a, v_b) {
                                    (Some(a), Some(b)) => {
                                        let cmp = b.cmp(&a);
                                        if cmp != std::cmp::Ordering::Equal {
                                            return cmp;
                                        }
                                    }
                                    _ => {}
                                }

                                let is_local_a = repo_a.starts_with("local");
                                let is_local_b = repo_b.starts_with("local");
                                if is_local_a != is_local_b {
                                    return is_local_b.cmp(&is_local_a);
                                }

                                repo_a.cmp(repo_b)
                            });

                            let (best_manifest, best_repo_id) = matching.remove(0);

                            self.traverse_package(
                                &best_manifest,
                                dep.version_req.clone(),
                                Some(best_repo_id),
                                dep.required,
                                child_effective_required,
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
                            required: child_effective_required,
                            status: dep_status,
                            path,
                            description: dep.description.clone(),
                        }
                    });

                    if child_effective_required {
                        entry.required = true;
                    }
                }
                DependencyKind::DesktopCapability => {
                    let (status, current_val) = self.evaluate_desktop_capability(&dep.id);
                    let entry = capability_deps.entry(dep.id.clone()).or_insert_with(|| {
                        ResolvedCapabilityDependency {
                            capability: dep.id.clone(),
                            kind: DependencyKind::DesktopCapability,
                            required: child_effective_required,
                            status,
                            current_value: current_val,
                            description: dep.description.clone(),
                        }
                    });
                    if child_effective_required {
                        entry.required = true;
                    }
                }
                DependencyKind::RuntimeCapability => {
                    let (status, current_val) = self.evaluate_runtime_capability(&dep.id);
                    let entry = capability_deps.entry(dep.id.clone()).or_insert_with(|| {
                        ResolvedCapabilityDependency {
                            capability: dep.id.clone(),
                            kind: DependencyKind::RuntimeCapability,
                            required: child_effective_required,
                            status,
                            current_value: current_val,
                            description: dep.description.clone(),
                        }
                    });
                    if child_effective_required {
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
            required: direct_required,
            effective_required,
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

    /// Secure capability check for runtime environments and distro requirements.
    /// Pure filesystem checks without external process execution.
    fn evaluate_runtime_capability(&self, cap: &str) -> (DependencyStatus, Option<String>) {
        let cap_lower = cap.to_lowercase();

        // Distro capability check (e.g. distro:arch, distro:ubuntu, arch, ubuntu)
        if cap_lower.starts_with("distro:")
            || [
                "arch", "ubuntu", "debian", "fedora", "void", "gentoo", "nixos",
            ]
            .contains(&cap_lower.as_str())
        {
            let target = cap_lower.trim_start_matches("distro:");
            let sys_id = self.system_info.distro_id.to_lowercase();
            let sys_fam = self.system_info.distro_family.to_lowercase();
            if target == "all" || target == "universal" || sys_id == target || sys_fam == target {
                return (
                    DependencyStatus::Satisfied,
                    Some(self.system_info.distro_name.clone()),
                );
            } else {
                return (
                    DependencyStatus::Incompatible,
                    Some(format!("Active system is {}", self.system_info.distro_name)),
                );
            }
        }

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
    pub manifests: HashMap<String, Vec<(RyzoraManifest, String)>>,
    pub unavailable_repositories: Vec<String>,
}

#[cfg(test)]
impl MockPackageProvider {
    pub fn new() -> Self {
        Self {
            manifests: HashMap::new(),
            unavailable_repositories: Vec::new(),
        }
    }

    pub fn add(&mut self, manifest: RyzoraManifest, repo_id: Option<&str>) {
        let r = repo_id.unwrap_or("local").to_string();
        self.manifests
            .entry(manifest.id.clone())
            .or_default()
            .push((manifest, r));
    }

    pub fn add_unavailable_repo(&mut self, repo_name: &str) {
        self.unavailable_repositories.push(repo_name.to_string());
    }
}

#[cfg(test)]
impl PackageProvider for MockPackageProvider {
    fn get_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
        self.manifests
            .get(package_id)
            .and_then(|list| list.first().map(|(m, _)| m.clone()))
            .ok_or_else(|| format!("Package '{}' not found in mock provider", package_id))
    }

    fn get_repository_id(&self, package_id: &str) -> Option<String> {
        self.manifests
            .get(package_id)
            .and_then(|list| list.first().map(|(_, r)| r.clone()))
    }

    fn get_candidates(&self, package_id: &str) -> Vec<(RyzoraManifest, String)> {
        self.manifests.get(package_id).cloned().unwrap_or_default()
    }

    fn get_unavailable_repositories(&self) -> Vec<String> {
        self.unavailable_repositories.clone()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit Tests (Phase 9 & 9.1 Hardening)
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
            .any(|c| c.contains("does not satisfy requirement")
                || c.contains("resolved")
                || c.contains("none satisfy")));
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
        assert_eq!(
            report.install_order,
            vec!["base-icons", "cool-theme", "cyber-rice"]
        );
    }

    #[test]
    fn test_resolver_diamond_dependency_resolution() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

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

        assert!(report.resolved);
        assert!(report.missing_required.is_empty());
        assert!(report
            .missing_optional
            .contains(&"optional_tool_xyz_99999".to_string()));
    }

    #[test]
    fn test_resolver_desktop_and_session_capabilities() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

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

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 9.1 Hardening Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_multi_repo_candidate_selection_selects_highest_compatible() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Repo 1 has v1.0.0
        let dep_v1 = make_manifest("dep-tool", "1.0.0", vec![], vec![], vec![]);
        provider.add(dep_v1, Some("official-repo"));

        // Repo 2 has v2.1.0
        let dep_v2 = make_manifest("dep-tool", "2.1.0", vec![], vec![], vec![]);
        provider.add(dep_v2, Some("community-repo"));

        // Repo 3 has v2.5.0
        let dep_v25 = make_manifest("dep-tool", "2.5.0", vec![], vec![], vec![]);
        provider.add(dep_v25, Some("edge-repo"));

        // Root requires ^2.0.0 -> should deterministically select v2.5.0
        let root = make_manifest(
            "root-app",
            "1.0.0",
            vec![DependencySpec {
                id: "dep-tool".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^2.0.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(root, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("root-app").unwrap();

        assert!(report.resolved);
        let dep_node = report
            .packages
            .iter()
            .find(|p| p.package_id == "dep-tool")
            .unwrap();
        assert_eq!(dep_node.version, "2.5.0");
        assert_eq!(dep_node.repository_id.as_deref(), Some("edge-repo"));
    }

    #[test]
    fn test_deterministic_version_selection_tie_breaker_prefers_local() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Remote repo has v1.0.0
        let dep_remote = make_manifest("shared-theme", "1.0.0", vec![], vec![], vec![]);
        provider.add(dep_remote, Some("remote-store"));

        // Local repo has v1.0.0
        let dep_local = make_manifest("shared-theme", "1.0.0", vec![], vec![], vec![]);
        provider.add(dep_local, Some("local-user"));

        let root = make_manifest(
            "root-app",
            "1.0.0",
            vec![DependencySpec {
                id: "shared-theme".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^1.0.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(root, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("root-app").unwrap();

        assert!(report.resolved);
        let dep_node = report
            .packages
            .iter()
            .find(|p| p.package_id == "shared-theme")
            .unwrap();
        assert_eq!(dep_node.version, "1.0.0");
        // Local repository must be preferred on identical versions!
        assert_eq!(dep_node.repository_id.as_deref(), Some("local-user"));
    }

    #[test]
    fn test_unavailable_version_diagnostics() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        let dep1 = make_manifest("widget-lib", "1.0.0", vec![], vec![], vec![]);
        let dep2 = make_manifest("widget-lib", "1.2.0", vec![], vec![], vec![]);
        provider.add(dep1, Some("repo-a"));
        provider.add(dep2, Some("repo-b"));

        // Request ^2.0.0 (neither satisfies)
        let root = make_manifest(
            "root-app",
            "1.0.0",
            vec![DependencySpec {
                id: "widget-lib".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^2.0.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(root, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("root-app").unwrap();

        assert!(!report.resolved);
        assert!(!report.conflicts.is_empty());
        let conflict = &report.conflicts[0];
        assert!(conflict.contains("available versions"));
        assert!(conflict.contains("widget-lib"));
        assert!(conflict.contains("^2.0.0"));
    }

    #[test]
    fn test_offline_repository_distinction() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        provider.add_unavailable_repo("arch-community (offline)");

        let root = make_manifest(
            "root-app",
            "1.0.0",
            vec![DependencySpec {
                id: "unreachable-package".to_string(),
                kind: DependencyKind::Package,
                version_req: None,
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(root, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("root-app").unwrap();

        assert!(!report.resolved);
        let unreach = report
            .packages
            .iter()
            .find(|p| p.package_id == "unreachable-package")
            .unwrap();
        assert!(unreach
            .status_message
            .as_ref()
            .unwrap()
            .contains("offline/error"));
    }

    #[test]
    fn test_transitive_optional_dependency_propagation() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Tool C is a system binary that does NOT exist
        // Package B (optional) declares Tool C as a required binary
        let b = make_manifest(
            "opt-package-b",
            "1.0.0",
            vec![DependencySpec {
                id: "missing_system_tool_xyz".to_string(),
                kind: DependencyKind::SystemBinary,
                version_req: None,
                required: true, // required for B, but B is optional for A!
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(b, Some("community"));

        // Package A requires B optionally
        let a = make_manifest(
            "pkg-a",
            "1.0.0",
            vec![DependencySpec {
                id: "opt-package-b".to_string(),
                kind: DependencyKind::Package,
                version_req: None,
                required: false, // OPTIONAL
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(a, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("pkg-a").unwrap();

        // Since B is optional, missing Tool C MUST NOT block resolution of A!
        assert!(
            report.resolved,
            "Optional missing dependency should not block report.resolved"
        );
        assert!(
            report.missing_required.is_empty(),
            "missing_required should be empty"
        );
        assert!(report
            .missing_optional
            .contains(&"missing_system_tool_xyz".to_string()));
    }

    #[test]
    fn test_optional_dependency_cycle_does_not_hang() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Cycle in optional path: A -> B (opt) -> C (opt) -> B
        let c = make_manifest(
            "pkg-c",
            "1.0.0",
            vec![DependencySpec {
                id: "pkg-b".to_string(),
                kind: DependencyKind::Package,
                version_req: None,
                required: false,
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
                required: false,
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
                required: false,
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

        // Should detect cycle cleanly and not hang or overflow stack
        assert!(!report.cycles.is_empty());
    }

    #[test]
    fn test_prerelease_semver_constraint_matching() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        let beta_pkg = make_manifest("dep-beta", "1.0.0-beta.2", vec![], vec![], vec![]);
        provider.add(beta_pkg, Some("community"));

        // 1. Stable requirement does not match prerelease under standard SemVer 2.0
        let root_stable = make_manifest(
            "root-stable",
            "1.0.0",
            vec![DependencySpec {
                id: "dep-beta".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^1.0.0".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(root_stable, Some("local"));

        // 2. Explicit prerelease requirement matches
        let root_beta = make_manifest(
            "root-beta",
            "1.0.0",
            vec![DependencySpec {
                id: "dep-beta".to_string(),
                kind: DependencyKind::Package,
                version_req: Some("^1.0.0-beta".to_string()),
                required: true,
                description: None,
            }],
            vec![],
            vec![],
        );
        provider.add(root_beta, Some("local"));

        let resolver = DependencyResolver::new(&provider, &sys);
        let report_stable = resolver.resolve("root-stable").unwrap();
        assert!(!report_stable.resolved);

        let report_beta = resolver.resolve("root-beta").unwrap();
        assert!(report_beta.resolved);
    }

    #[test]
    fn test_max_dependency_depth_guard() {
        let mut provider = MockPackageProvider::new();
        let sys = make_test_sys();

        // Chain of 35 packages: pkg-0 -> pkg-1 -> ... -> pkg-34
        for i in 0..35 {
            let next_dep = if i < 34 {
                vec![DependencySpec {
                    id: format!("chain-pkg-{}", i + 1),
                    kind: DependencyKind::Package,
                    version_req: None,
                    required: true,
                    description: None,
                }]
            } else {
                vec![]
            };

            let pkg = make_manifest(
                &format!("chain-pkg-{}", i),
                "1.0.0",
                next_dep,
                vec![],
                vec![],
            );
            provider.add(pkg, Some("local"));
        }

        let resolver = DependencyResolver::new(&provider, &sys);
        let report = resolver.resolve("chain-pkg-0").unwrap();

        // Depth guard must trigger conflict rather than stack overflow
        assert!(!report.resolved);
        assert!(report
            .conflicts
            .iter()
            .any(|c| c.contains("Maximum dependency depth")));
    }

    #[test]
    fn test_distro_capability_detection() {
        let provider = MockPackageProvider::new();
        let sys = make_test_sys(); // Arch Linux

        let resolver = DependencyResolver::new(&provider, &sys);

        // Arch should satisfy distro:arch
        let (arch_status, _) = resolver.evaluate_runtime_capability("distro:arch");
        assert_eq!(arch_status, DependencyStatus::Satisfied);

        // Arch should be incompatible with distro:ubuntu
        let (ubuntu_status, _) = resolver.evaluate_runtime_capability("distro:ubuntu");
        assert_eq!(ubuntu_status, DependencyStatus::Incompatible);
    }
}
