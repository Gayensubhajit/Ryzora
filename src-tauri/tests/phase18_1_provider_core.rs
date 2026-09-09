use ryzora_lib::distribution::TrustTier;
use ryzora_lib::manifest::PackageType;
use ryzora_lib::providers::normalizer::normalize_provider_item;
use ryzora_lib::providers::synthesizer::ManifestSynthesizer;
use ryzora_lib::providers::{
    ContentProvider, ProviderCapabilities, ProviderError, ProviderFileSpec, ProviderItem,
    ProviderManager, ProviderProvenance, ProviderQuery, ProviderStatus, ProviderType,
};
use ryzora_lib::repository::AuthorInfo;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Mock Providers for Deterministic Testing
// ─────────────────────────────────────────────────────────────────────────────

struct MockProvider {
    id: &'static str,
    name: &'static str,
    ptype: ProviderType,
    enabled: bool,
    items: Vec<ProviderItem>,
    should_fail: Option<ProviderError>,
}

impl MockProvider {
    fn new(id: &'static str, name: &'static str, ptype: ProviderType) -> Self {
        Self {
            id,
            name,
            ptype,
            enabled: true,
            items: Vec::new(),
            should_fail: None,
        }
    }
}

impl ContentProvider for MockProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn name(&self) -> &str {
        self.name
    }

    fn provider_type(&self) -> ProviderType {
        self.ptype.clone()
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn search(&self, query: &ProviderQuery) -> Result<Vec<ProviderItem>, ProviderError> {
        if let Some(ref err) = self.should_fail {
            return Err(err.clone());
        }

        let q = query.query.to_lowercase();
        let matches = self
            .items
            .iter()
            .filter(|item| {
                if q.is_empty() {
                    true
                } else {
                    item.title.to_lowercase().contains(&q)
                        || item.description.to_lowercase().contains(&q)
                        || item.tags.iter().any(|t| t.to_lowercase().contains(&q))
                }
            })
            .cloned()
            .collect();
        Ok(matches)
    }

    fn fetch_item(&self, id: &str) -> Result<ProviderItem, ProviderError> {
        if let Some(ref err) = self.should_fail {
            return Err(err.clone());
        }
        self.items
            .iter()
            .find(|item| item.id == id)
            .cloned()
            .ok_or_else(|| ProviderError::NotFound(id.to_string()))
    }

    fn stage_payload(&self, _item: &ProviderItem, staging_dir: &Path) -> Result<PathBuf, String> {
        Ok(staging_dir.to_path_buf())
    }
}

fn create_sample_item(id: &str, title: &str, provider_id: &str, source_url: &str) -> ProviderItem {
    ProviderItem {
        id: id.to_string(),
        title: title.to_string(),
        subtitle: "A clean Linux rice".to_string(),
        description: "Customized desktop layout with Waybar".to_string(),
        version: "1.0.0".to_string(),
        author: AuthorInfo {
            name: "Alex Designer".to_string(),
            avatar: String::new(),
            verified: false,
        },
        category: "rices".to_string(),
        package_type: PackageType::Rice,
        tags: vec!["hyprland".to_string(), "waybar".to_string()],
        supported_desktops: vec!["hyprland".to_string()],
        supported_display: vec!["wayland".to_string()],
        rating: Some(4.8),
        rating_count: Some(42),
        downloads: Some(1337),
        hero_image: Some("https://example.com/preview.png".to_string()),
        screenshots: vec!["https://example.com/shot1.png".to_string()],
        provenance: ProviderProvenance {
            provider_id: provider_id.to_string(),
            source_url: source_url.to_string(),
            repository: Some(source_url.to_string()),
            commit_or_tag: Some("v1.0.0".to_string()),
            license_spdx: Some("MIT".to_string()),
            original_author: "Alex Designer".to_string(),
            fetched_at: "1773000000".to_string(),
        },
        files: vec![
            ProviderFileSpec {
                source: "files/hyprland.conf".to_string(),
                target: "~/.config/hypr/hyprland.conf".to_string(),
            },
            ProviderFileSpec {
                source: "files/waybar.jsonc".to_string(),
                target: "~/.config/waybar/config.jsonc".to_string(),
            },
        ],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Provider Aggregator & Multi-Provider Search
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_provider_manager_registration_and_list() {
    let mut manager = ProviderManager::new();

    let p1 = Arc::new(MockProvider::new(
        "community",
        "Community Catalog",
        ProviderType::Community,
    ));
    let p2 = Arc::new(MockProvider::new(
        "github",
        "GitHub Search",
        ProviderType::GitHub,
    ));

    manager.register_provider(p1);
    manager.register_provider(p2);

    let list = manager.list_providers();
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|p| p.id == "community" && p.enabled));
    assert!(list.iter().any(|p| p.id == "github" && p.enabled));
}

#[test]
fn test_provider_aggregator_search_and_deduplication() {
    let mut manager = ProviderManager::new();

    let mut p1 = MockProvider::new("community", "Community Catalog", ProviderType::Community);
    p1.items.push(create_sample_item(
        "catppuccin-rice",
        "Catppuccin Rice",
        "community",
        "https://github.com/ryzora/catppuccin",
    ));
    p1.items.push(create_sample_item(
        "dracula-rice",
        "Dracula Rice",
        "community",
        "https://github.com/ryzora/dracula",
    ));

    let mut p2 = MockProvider::new("github", "GitHub Search", ProviderType::GitHub);
    // Duplicate item from same source URL
    p2.items.push(create_sample_item(
        "catppuccin-rice",
        "Catppuccin Rice",
        "github",
        "https://github.com/ryzora/catppuccin",
    ));
    // Distinct item
    p2.items.push(create_sample_item(
        "tokyo-night-rice",
        "Tokyo Night Rice",
        "github",
        "https://github.com/other/tokyo-night",
    ));

    manager.register_provider(Arc::new(p1));
    manager.register_provider(Arc::new(p2));

    let query = ProviderQuery {
        query: "rice".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 20,
    };

    let response = manager.search_all(&query);

    // 4 items found across providers, but 1 is a duplicate (catppuccin-rice from same source)
    assert_eq!(response.total_results, 3);
    assert_eq!(response.items.len(), 3);
    assert!(response.items.iter().any(|i| i.id == "catppuccin-rice"));
    assert!(response.items.iter().any(|i| i.id == "dracula-rice"));
    assert!(response.items.iter().any(|i| i.id == "tokyo-night-rice"));

    // Verify both providers report Online
    assert_eq!(
        response.provider_statuses.get("community"),
        Some(&ProviderStatus::Online)
    );
    assert_eq!(
        response.provider_statuses.get("github"),
        Some(&ProviderStatus::Online)
    );
}

#[test]
fn test_provider_resilience_partial_failure_does_not_break_search() {
    let mut manager = ProviderManager::new();

    let mut p1 = MockProvider::new("community", "Community Catalog", ProviderType::Community);
    p1.items.push(create_sample_item(
        "nord-rice",
        "Nord Theme",
        "community",
        "https://github.com/ryzora/nord",
    ));

    let mut p2 = MockProvider::new("github", "GitHub Search", ProviderType::GitHub);
    p2.should_fail = Some(ProviderError::RateLimitExceeded {
        reset_seconds: 60,
        message: "GitHub API rate limit exceeded".to_string(),
    });

    let mut p3 = MockProvider::new("fastfetch", "Fastfetch Presets", ProviderType::Fastfetch);
    p3.should_fail = Some(ProviderError::NetworkError(
        "Connection refused".to_string(),
    ));

    manager.register_provider(Arc::new(p1));
    manager.register_provider(Arc::new(p2));
    manager.register_provider(Arc::new(p3));

    let query = ProviderQuery {
        query: "nord".to_string(),
        ..Default::default()
    };

    // The aggregator must succeed overall, returning results from p1 while reporting error statuses for p2 and p3
    let response = manager.search_all(&query);
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].id, "nord-rice");

    assert_eq!(
        response.provider_statuses.get("community"),
        Some(&ProviderStatus::Online)
    );
    assert_eq!(
        response.provider_statuses.get("github"),
        Some(&ProviderStatus::RateLimited { reset_seconds: 60 })
    );
    match response.provider_statuses.get("fastfetch") {
        Some(ProviderStatus::Error(msg)) => assert!(msg.contains("Connection refused")),
        other => panic!("Expected ProviderStatus::Error, got {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Normalizer Trust Boundary
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_normalizer_enforces_community_trust_tier() {
    let item = create_sample_item(
        "awesome-rice",
        "Awesome Rice",
        "github",
        "https://github.com/user/awesome",
    );
    let normalized = normalize_provider_item(item);

    // CRITICAL: External items can NEVER claim Official or AuthorVerified
    assert_eq!(normalized.trust_tier, Some(TrustTier::Community));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Official));

    // Tags contain provider provenance and license
    assert!(normalized.tags.contains(&"provider:github".to_string()));
    assert!(normalized.tags.contains(&"license:MIT".to_string()));

    // Safety audit defaults
    assert_eq!(normalized.safety_audit.rating, "unverified");
    assert_eq!(normalized.safety_audit.changes_system_files, false);
    assert_eq!(normalized.safety_audit.requires_root, false);
    assert_eq!(normalized.safety_audit.sandbox_compatible, true);
    assert_eq!(normalized.safety_audit.files_modified_count, 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Manifest Synthesizer Security Boundary
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_manifest_synthesizer_valid_package() {
    let item = create_sample_item(
        "clean-rice",
        "Clean Rice",
        "github",
        "https://github.com/user/clean",
    );
    let manifest =
        ManifestSynthesizer::synthesize(&item).expect("Synthesizer must accept valid item");

    assert_eq!(manifest.ryzora_spec, "1");
    assert_eq!(manifest.id, "clean-rice");
    assert_eq!(manifest.name, "Clean Rice");
    assert_eq!(manifest.files.len(), 2);
    assert_eq!(manifest.files[0].target, "~/.config/hypr/hyprland.conf");
    assert_eq!(manifest.files[1].target, "~/.config/waybar/config.jsonc");
}

#[test]
fn test_manifest_synthesizer_wallpaper_placement_allowed() {
    let mut item = create_sample_item(
        "nature-wallpaper",
        "Nature 4K",
        "wallpapers",
        "https://wallpapers.example/nature",
    );
    item.package_type = PackageType::Wallpaper;
    item.files = vec![ProviderFileSpec {
        source: "nature.png".to_string(),
        target: "~/Pictures/Wallpapers/nature.png".to_string(),
    }];

    let manifest = ManifestSynthesizer::synthesize(&item)
        .expect("Synthesizer must allow ~/Pictures/Wallpapers/");
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].target, "~/Pictures/Wallpapers/nature.png");
}

#[test]
fn test_manifest_synthesizer_theme_placement_allowed() {
    let mut item = create_sample_item(
        "dracula-gtk",
        "Dracula GTK",
        "kde",
        "https://themes.example/dracula",
    );
    item.package_type = PackageType::Theme;
    item.files = vec![
        ProviderFileSpec {
            source: "dracula/index.theme".to_string(),
            target: "~/.themes/dracula/index.theme".to_string(),
        },
        ProviderFileSpec {
            source: "icons/index.theme".to_string(),
            target: "~/.icons/dracula-icons/index.theme".to_string(),
        },
    ];

    let manifest = ManifestSynthesizer::synthesize(&item)
        .expect("Synthesizer must allow ~/.themes/ and ~/.icons/");
    assert_eq!(manifest.files.len(), 2);
}

#[test]
fn test_manifest_synthesizer_strips_executable_scripts() {
    let mut item = create_sample_item(
        "script-rice",
        "Script Rice",
        "github",
        "https://github.com/evil/rice",
    );
    item.files = vec![
        ProviderFileSpec {
            source: "install.sh".to_string(),
            target: "~/.config/hypr/install.sh".to_string(),
        },
        ProviderFileSpec {
            source: "setup.bash".to_string(),
            target: "~/.config/setup.bash".to_string(),
        },
        ProviderFileSpec {
            source: "safe.conf".to_string(),
            target: "~/.config/hypr/safe.conf".to_string(),
        },
    ];

    let manifest = ManifestSynthesizer::synthesize(&item)
        .expect("Synthesizer should strip scripts without crashing");
    // install.sh and setup.bash must be dropped
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].target, "~/.config/hypr/safe.conf");
}

#[test]
fn test_manifest_synthesizer_rejects_dangerous_targets() {
    let dangerous_targets = [
        "~/.ssh/authorized_keys",
        "~/.bashrc",
        "~/.profile",
        "~/.zshrc",
        "~/.config/fish/config.fish",
        "/etc/shadow",
        "~/../../etc/passwd",
    ];

    for dangerous in &dangerous_targets {
        let mut item = create_sample_item(
            "exploit-rice",
            "Exploit",
            "github",
            "https://github.com/evil/exploit",
        );
        item.files = vec![ProviderFileSpec {
            source: "exploit.conf".to_string(),
            target: dangerous.to_string(),
        }];

        let result = ManifestSynthesizer::synthesize(&item);
        assert!(
            result.is_err(),
            "Synthesizer must reject dangerous target: {}",
            dangerous
        );
    }
}

#[test]
fn test_manifest_synthesizer_rejects_source_escaping() {
    let escaping_sources = [
        "../secret.conf",
        "/etc/shadow",
        "nested/../../outside.conf",
        "foo\0bar",
    ];

    for escaping in &escaping_sources {
        let mut item = create_sample_item(
            "escape-rice",
            "Escape",
            "github",
            "https://github.com/evil/escape",
        );
        item.files = vec![ProviderFileSpec {
            source: escaping.to_string(),
            target: "~/.config/hypr/hyprland.conf".to_string(),
        }];

        let result = ManifestSynthesizer::synthesize(&item);
        assert!(
            result.is_err(),
            "Synthesizer must reject escaping source: {}",
            escaping
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Static Subprocess & Privilege Audit for Providers Module
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_providers_zero_subprocess_and_privilege_audit() {
    let providers_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/providers");
    assert!(providers_dir.is_dir());

    for entry in fs::read_dir(&providers_dir).unwrap().flatten() {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(entry.path()).unwrap();
            assert!(
                !content.contains("Command::new("),
                "No Command::new allowed in {:?}",
                entry.path()
            );
            assert!(
                !content.contains("std::process::Command"),
                "No std::process::Command allowed in {:?}",
                entry.path()
            );
            assert!(
                !content.contains("\"sudo\"") && !content.contains("\"sudo "),
                "No sudo invocations in {:?}",
                entry.path()
            );
            assert!(
                !content.contains("\"pkexec\"") && !content.contains("\"pkexec "),
                "No pkexec invocations in {:?}",
                entry.path()
            );
        }
    }
}
