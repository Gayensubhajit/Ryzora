use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::system::SystemInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityRequirements {
    pub supported_distros: Vec<String>,
    pub supported_desktops: Vec<String>,
    pub supported_sessions: Vec<String>,
    pub required_binaries: Vec<String>,
    pub optional_binaries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompatibilityLevel {
    Compatible,
    MissingDependencies,
    IncompatibleSession,
    IncompatibleDesktop,
    IncompatibleDistro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityIssue {
    pub severity: String, // "error" | "warning" | "info"
    pub code: String,
    pub message: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub level: CompatibilityLevel,
    pub score: u32,
    pub summary_label: String,
    pub session_compatible: bool,
    pub desktop_compatible: bool,
    pub distro_compatible: bool,
    pub satisfied_apps: Vec<String>,
    pub missing_required_apps: Vec<String>,
    pub missing_optional_apps: Vec<String>,
    pub issues: Vec<CompatibilityIssue>,
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn evaluate_compatibility(
    sys: &SystemInfo,
    reqs: &CompatibilityRequirements,
) -> CompatibilityReport {
    let mut issues = Vec::new();
    let mut satisfied_apps = Vec::new();
    let mut missing_required_apps = Vec::new();
    let mut missing_optional_apps = Vec::new();

    // 1. Distro Check
    let distro_id = sys.distro_id.to_lowercase();
    let distro_family = sys.distro_family.to_lowercase();
    let distro_compatible = reqs.supported_distros.is_empty()
        || reqs.supported_distros.iter().any(|d| {
            let dl = d.to_lowercase();
            dl == "all" || dl == "universal" || dl == distro_id || dl == distro_family
        });

    if !distro_compatible {
        issues.push(CompatibilityIssue {
            severity: "error".to_string(),
            code: "DISTRO_MISMATCH".to_string(),
            message: format!(
                "Package supports distros [{}] but active system is {}",
                reqs.supported_distros.join(", "),
                sys.distro_name
            ),
            target: Some(sys.distro_id.clone()),
        });
    }

    // 2. Session Type Check (Wayland vs X11)
    let sys_session = sys.session_type.to_lowercase();
    let session_compatible = reqs.supported_sessions.is_empty()
        || reqs.supported_sessions.iter().any(|s| {
            let sl = s.to_lowercase();
            sl == "any" || sl == "all" || sl == sys_session
        });

    if !session_compatible {
        let req_str = reqs.supported_sessions.join(", ");
        issues.push(CompatibilityIssue {
            severity: "error".to_string(),
            code: "SESSION_MISMATCH".to_string(),
            message: format!(
                "Package requires session [{}] but current session is {}",
                req_str,
                sys.session_type
            ),
            target: Some(sys.session_type.clone()),
        });
    }

    // 3. Desktop / WM Check
    let wm = sys.window_manager.to_lowercase();
    let de = sys.desktop_environment.to_lowercase();
    let desktop_compatible = reqs.supported_desktops.is_empty()
        || reqs.supported_desktops.iter().any(|d| {
            let dl = d.to_lowercase();
            dl == "universal" || dl == "all" || dl == wm || dl == de || wm.contains(&dl) || de.contains(&dl)
        });

    if !desktop_compatible {
        issues.push(CompatibilityIssue {
            severity: "error".to_string(),
            code: "DESKTOP_MISMATCH".to_string(),
            message: format!(
                "Package designed for [{}] but active desktop is {}",
                reqs.supported_desktops.join(", "),
                sys.window_manager
            ),
            target: Some(sys.window_manager.clone()),
        });
    }

    // 4. Required & Optional Binaries Check
    let installed_map: std::collections::HashSet<String> = sys
        .installed_components
        .iter()
        .filter(|c| c.installed)
        .map(|c| c.binary.to_lowercase())
        .collect();

    for req_bin in &reqs.required_binaries {
        let bin_lower = req_bin.to_lowercase();
        let found = installed_map.contains(&bin_lower)
            || (bin_lower == "rofi-wayland" && installed_map.contains("rofi"))
            || (bin_lower == "rofi" && installed_map.contains("rofi-wayland"));

        if found {
            satisfied_apps.push(req_bin.clone());
        } else {
            missing_required_apps.push(req_bin.clone());
            issues.push(CompatibilityIssue {
                severity: "warning".to_string(),
                code: "MISSING_REQUIRED_BIN".to_string(),
                message: format!("Required tool '{}' is not installed in PATH", req_bin),
                target: Some(req_bin.clone()),
            });
        }
    }

    for opt_bin in &reqs.optional_binaries {
        let bin_lower = opt_bin.to_lowercase();
        let found = installed_map.contains(&bin_lower);
        if found {
            satisfied_apps.push(opt_bin.clone());
        } else {
            missing_optional_apps.push(opt_bin.clone());
            issues.push(CompatibilityIssue {
                severity: "info".to_string(),
                code: "MISSING_OPTIONAL_BIN".to_string(),
                message: format!("Optional tool '{}' is not installed in PATH", opt_bin),
                target: Some(opt_bin.clone()),
            });
        }
    }

    // 5. Determine Overall Level & Quiet Summary Label
    let (level, score, summary_label) = if !desktop_compatible {
        let first_target = reqs.supported_desktops.first().cloned().unwrap_or_else(|| "other".to_string());
        (
            CompatibilityLevel::IncompatibleDesktop,
            20,
            format!("Requires {}", capitalize(&first_target)),
        )
    } else if !session_compatible {
        let req_sess = reqs.supported_sessions.first().cloned().unwrap_or_else(|| "Wayland".to_string());
        (
            CompatibilityLevel::IncompatibleSession,
            30,
            format!("Requires {}", capitalize(&req_sess)),
        )
    } else if !distro_compatible {
        (
            CompatibilityLevel::IncompatibleDistro,
            40,
            "Distro mismatch".to_string(),
        )
    } else if !missing_required_apps.is_empty() {
        let missing_first = &missing_required_apps[0];
        (
            CompatibilityLevel::MissingDependencies,
            75,
            format!("Missing: {}", missing_first),
        )
    } else {
        (
            CompatibilityLevel::Compatible,
            100,
            "Compatible".to_string(),
        )
    };

    CompatibilityReport {
        level,
        score,
        summary_label,
        session_compatible,
        desktop_compatible,
        distro_compatible,
        satisfied_apps,
        missing_required_apps,
        missing_optional_apps,
        issues,
    }
}

#[tauri::command]
pub fn evaluate_package_compatibility(
    requirements: CompatibilityRequirements,
) -> CompatibilityReport {
    let sys = crate::system::detect_system_info();
    evaluate_compatibility(&sys, &requirements)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPackageInput {
    pub id: String,
    pub requirements: CompatibilityRequirements,
}

#[tauri::command]
pub fn evaluate_batch_compatibility(
    packages: Vec<BatchPackageInput>,
) -> HashMap<String, CompatibilityReport> {
    let sys = crate::system::detect_system_info();
    let mut results = HashMap::new();
    for pkg in packages {
        let report = evaluate_compatibility(&sys, &pkg.requirements);
        results.insert(pkg.id, report);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::InstalledComponent;

    fn mock_system() -> SystemInfo {
        SystemInfo {
            distro_name: "Garuda Linux".to_string(),
            distro_id: "garuda".to_string(),
            distro_family: "arch".to_string(),
            distro_version: "Rolling".to_string(),
            kernel_version: "6.18.50".to_string(),
            desktop_environment: "Hyprland".to_string(),
            window_manager: "Hyprland".to_string(),
            session_type: "wayland".to_string(),
            shell: "zsh".to_string(),
            terminal: "kitty".to_string(),
            installed_components: vec![
                InstalledComponent {
                    name: "Hyprland".to_string(),
                    binary: "hyprland".to_string(),
                    installed: true,
                    path: Some("/usr/bin/hyprland".to_string()),
                    category: "WM".to_string(),
                },
                InstalledComponent {
                    name: "Waybar".to_string(),
                    binary: "waybar".to_string(),
                    installed: true,
                    path: Some("/usr/bin/waybar".to_string()),
                    category: "Bar".to_string(),
                },
                InstalledComponent {
                    name: "Kitty".to_string(),
                    binary: "kitty".to_string(),
                    installed: true,
                    path: Some("/usr/bin/kitty".to_string()),
                    category: "Terminal".to_string(),
                },
            ],
        }
    }

    #[test]
    fn test_fully_compatible_package() {
        let sys = mock_system();
        let reqs = CompatibilityRequirements {
            supported_distros: vec!["arch".to_string()],
            supported_desktops: vec!["hyprland".to_string()],
            supported_sessions: vec!["wayland".to_string()],
            required_binaries: vec!["hyprland".to_string(), "waybar".to_string()],
            optional_binaries: vec!["fastfetch".to_string()],
        };

        let report = evaluate_compatibility(&sys, &reqs);
        assert_eq!(report.level, CompatibilityLevel::Compatible);
        assert_eq!(report.summary_label, "Compatible");
        assert!(report.missing_required_apps.is_empty());
        assert_eq!(report.missing_optional_apps, vec!["fastfetch"]);
    }

    #[test]
    fn test_missing_required_tool() {
        let sys = mock_system();
        let reqs = CompatibilityRequirements {
            supported_distros: vec!["all".to_string()],
            supported_desktops: vec!["hyprland".to_string()],
            supported_sessions: vec!["wayland".to_string()],
            required_binaries: vec!["rofi".to_string()],
            optional_binaries: vec![],
        };

        let report = evaluate_compatibility(&sys, &reqs);
        assert_eq!(report.level, CompatibilityLevel::MissingDependencies);
        assert_eq!(report.summary_label, "Missing: rofi");
        assert_eq!(report.missing_required_apps, vec!["rofi"]);
    }

    #[test]
    fn test_incompatible_desktop() {
        let sys = mock_system();
        let reqs = CompatibilityRequirements {
            supported_distros: vec!["all".to_string()],
            supported_desktops: vec!["kde".to_string()],
            supported_sessions: vec!["any".to_string()],
            required_binaries: vec![],
            optional_binaries: vec![],
        };

        let report = evaluate_compatibility(&sys, &reqs);
        assert_eq!(report.level, CompatibilityLevel::IncompatibleDesktop);
        assert_eq!(report.summary_label, "Requires Kde");
    }

    #[test]
    fn test_incompatible_session() {
        let sys = mock_system();
        let reqs = CompatibilityRequirements {
            supported_distros: vec!["all".to_string()],
            supported_desktops: vec!["hyprland".to_string()],
            supported_sessions: vec!["x11".to_string()],
            required_binaries: vec![],
            optional_binaries: vec![],
        };

        let report = evaluate_compatibility(&sys, &reqs);
        assert_eq!(report.level, CompatibilityLevel::IncompatibleSession);
        assert_eq!(report.summary_label, "Requires X11");
    }

    #[test]
    fn test_incompatible_distro() {
        let sys = mock_system();
        let reqs = CompatibilityRequirements {
            supported_distros: vec!["fedora".to_string(), "rhel".to_string()],
            supported_desktops: vec!["hyprland".to_string()],
            supported_sessions: vec!["wayland".to_string()],
            required_binaries: vec![],
            optional_binaries: vec![],
        };

        let report = evaluate_compatibility(&sys, &reqs);
        assert_eq!(report.level, CompatibilityLevel::IncompatibleDistro);
        assert_eq!(report.summary_label, "Distro mismatch");
        assert!(!report.distro_compatible);
    }
}
