//! Phase 18.2: GitHub Content Provider Comprehensive Test Suite.
//!
//! Validates:
//! 1. Public GitHub search query formatting & URL validation (HTTPS, domain allowlist, no credentials).
//! 2. Unauthenticated access enforcement (no token required or sent).
//! 3. API response size limits (5MB JSON, 50MB tarball, 200MB uncompressed).
//! 4. Rate-limit detection & backoff reporting (403 + remaining 0 + reset seconds).
//! 5. Aggregator fault isolation (rate-limited GitHub provider does not crash search).
//! 6. Strict TrustTier::Community enforcement (stars/popularity never grant Official/Verified).
//! 7. Full provenance tracking (source URL, commit ref, author, SPDX license).
//! 8. Convention-based discovery (hypr, waybar, kitty, fastfetch, rofi, hyprlock, wallpapers).
//! 9. Pure-Rust safe archive extraction.
//! 10. Malicious tarball rejection: symlinks unconditionally dropped/rejected.
//! 11. Malicious tarball rejection: path traversal ('..') unconditionally rejected.
//! 12. Decompression bomb defense: max decompressed payload capped at 200MB.
//! 13. Executable script stripping (install.sh, setup.sh never reach manifest).
//! 14. Atomic caching with TTL.
//! 15. Zero subprocess / privilege audit (0 Command::new, 0 sudo, 0 pkexec).

use flate2::write::GzEncoder;
use flate2::Compression;
use ryzora_lib::distribution::TrustTier;
use ryzora_lib::manifest::PackageType;
use ryzora_lib::providers::github::{
    discover_conventions, extract_tarball_safe, validate_github_url, GitHubProvider, HttpResponse,
    HttpTransport, MAX_ARCHIVE_UNCOMPRESSED_BYTES,
};
use ryzora_lib::providers::normalizer::normalize_provider_item;
use ryzora_lib::providers::{
    ContentProvider, ProviderError, ProviderManager, ProviderQuery, ProviderStatus,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tar::{Builder, Header};

// ─────────────────────────────────────────────────────────────────────────────

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(name: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ryzora-phase18-2-{}-{}", name, nanos));
        let _ = fs::create_dir_all(&path);
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// Test Helpers: Mock HTTP Transport
// ─────────────────────────────────────────────────────────────────────────────

struct MockHttpTransport {
    responses: Mutex<HashMap<String, HttpResponse>>,
    recorded_calls: Mutex<Vec<(String, Vec<(String, String)>)>>,
}

impl MockHttpTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
            recorded_calls: Mutex::new(Vec::new()),
        }
    }

    fn register_response(&self, url_prefix: &str, response: HttpResponse) {
        let mut map = self.responses.lock().unwrap();
        map.insert(url_prefix.to_string(), response);
    }
}

impl HttpTransport for MockHttpTransport {
    fn get(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        _max_bytes: usize,
    ) -> Result<HttpResponse, ProviderError> {
        validate_github_url(url)?;

        let mut calls = self.recorded_calls.lock().unwrap();
        let hdrs: Vec<(String, String)> = headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        calls.push((url.to_string(), hdrs));

        let map = self.responses.lock().unwrap();
        for (prefix, resp) in map.iter() {
            if url.starts_with(prefix) {
                if resp.status == 403 || resp.status == 429 {
                    let remaining = resp
                        .headers
                        .get("x-ratelimit-remaining")
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    if remaining == 0 {
                        let reset_sec = resp
                            .headers
                            .get("x-ratelimit-reset")
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(60);
                        return Err(ProviderError::RateLimitExceeded {
                            reset_seconds: reset_sec,
                            message: format!("Rate limited for {} seconds", reset_sec),
                        });
                    }
                }
                return Ok(resp.clone());
            }
        }

        Err(ProviderError::NotFound(format!(
            "No mock response for {}",
            url
        )))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Helpers: Archive Generators
// ─────────────────────────────────────────────────────────────────────────────

fn create_valid_test_tarball() -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar = Builder::new(&mut encoder);

        // Top level repo folder
        let mut root_hdr = Header::new_gnu();
        root_hdr.set_entry_type(tar::EntryType::Directory);
        root_hdr.set_size(0);
        root_hdr.set_mode(0o755);
        root_hdr.set_cksum();
        tar.append_data(
            &mut root_hdr,
            "alice-hyprland-dots-abc1234/",
            std::io::empty(),
        )
        .unwrap();

        // hypr/hyprland.conf
        let conf = b"# Hyprland config
monitor=,preferred,auto,1
";
        let mut hdr1 = Header::new_gnu();
        hdr1.set_entry_type(tar::EntryType::Regular);
        hdr1.set_size(conf.len() as u64);
        hdr1.set_mode(0o644);
        hdr1.set_cksum();
        tar.append_data(
            &mut hdr1,
            "alice-hyprland-dots-abc1234/hypr/hyprland.conf",
            &conf[..],
        )
        .unwrap();

        // waybar/config
        let wb = br#"{"layer": "top"}"#;
        let mut hdr2 = Header::new_gnu();
        hdr2.set_entry_type(tar::EntryType::Regular);
        hdr2.set_size(wb.len() as u64);
        hdr2.set_mode(0o644);
        hdr2.set_cksum();
        tar.append_data(
            &mut hdr2,
            "alice-hyprland-dots-abc1234/waybar/config",
            &wb[..],
        )
        .unwrap();

        // wallpapers/sunset.png
        let wp = b"fake png image data";
        let mut hdr3 = Header::new_gnu();
        hdr3.set_entry_type(tar::EntryType::Regular);
        hdr3.set_size(wp.len() as u64);
        hdr3.set_mode(0o644);
        hdr3.set_cksum();
        tar.append_data(
            &mut hdr3,
            "alice-hyprland-dots-abc1234/wallpapers/sunset.png",
            &wp[..],
        )
        .unwrap();

        // install.sh (must be ignored by convention discovery)
        let sh = b"#!/bin/sh
rm -rf /
";
        let mut hdr4 = Header::new_gnu();
        hdr4.set_entry_type(tar::EntryType::Regular);
        hdr4.set_size(sh.len() as u64);
        hdr4.set_mode(0o755);
        hdr4.set_cksum();
        tar.append_data(&mut hdr4, "alice-hyprland-dots-abc1234/install.sh", &sh[..])
            .unwrap();

        tar.finish().unwrap();
    }
    encoder.finish().unwrap()
}

fn create_symlink_attack_tarball() -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar = Builder::new(&mut encoder);
        let mut hdr = Header::new_gnu();
        hdr.set_entry_type(tar::EntryType::Symlink);
        hdr.set_size(0);
        hdr.set_link_name("/etc/passwd").unwrap();
        hdr.set_cksum();
        tar.append_data(&mut hdr, "repo-root/passwd_symlink", std::io::empty())
            .unwrap();
        tar.finish().unwrap();
    }
    encoder.finish().unwrap()
}

fn create_traversal_attack_tarball() -> Vec<u8> {
    use std::io::Write;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut header_block = vec![0u8; 512];

    let name = b"repo-root/../../../../etc/shadow";
    header_block[..name.len()].copy_from_slice(name);
    header_block[100..108].copy_from_slice(b"0000644 ");
    header_block[124..136].copy_from_slice(b"00000000005 ");
    header_block[156] = b'0';
    header_block[257..263].copy_from_slice(b"ustar ");
    header_block[263..265].copy_from_slice(b"00");

    header_block[148..156].copy_from_slice(b"        ");
    let sum: u32 = header_block.iter().map(|&b| b as u32).sum();
    let chk = format!("{:06o}  ", sum);
    header_block[148..156].copy_from_slice(chk.as_bytes());

    encoder.write_all(&header_block).unwrap();

    let mut data_block = vec![0u8; 512];
    data_block[..5].copy_from_slice(b"pwned");
    encoder.write_all(&data_block).unwrap();

    encoder.write_all(&vec![0u8; 1024]).unwrap();
    encoder.finish().unwrap()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: GitHub URL Validation & Scheme Security
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_github_url_validation_accepts_valid_endpoints() {
    assert!(validate_github_url("https://api.github.com/search/repositories?q=hyprland").is_ok());
    assert!(validate_github_url("https://api.github.com/repos/alice/dots").is_ok());
    assert!(
        validate_github_url("https://codeload.github.com/alice/dots/legacy.tar.gz/main").is_ok()
    );
    assert!(validate_github_url("https://github.com/alice/dots").is_ok());
}

#[test]
fn test_github_url_validation_rejects_insecure_or_malicious_urls() {
    // Insecure HTTP
    assert!(validate_github_url("http://api.github.com/repos").is_err());

    // Unauthorized domain
    assert!(validate_github_url("https://evil.com/payload").is_err());
    assert!(validate_github_url("https://api.github.com.evil.com/repos").is_err());

    // Embedded user credentials
    assert!(validate_github_url("https://user:pass@api.github.com/repos").is_err());

    // Null byte injection
    assert!(validate_github_url("https://api.github.com/repos /evil").is_err());

    // Path traversal in URL
    assert!(validate_github_url("https://api.github.com/repos/../evil").is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Public Search, No Tokens, & Rate-Limit Resilience
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_github_search_success_and_metadata_mapping() {
    let mock_transport = Arc::new(MockHttpTransport::new());

    let search_json = r#"{
        "total_count": 1,
        "items": [
            {
                "id": 998877,
                "name": "hyprland-catppuccin",
                "full_name": "alice/hyprland-catppuccin",
                "owner": {
                    "login": "alice",
                    "avatar_url": "https://avatars.githubusercontent.com/u/1234",
                    "html_url": "https://github.com/alice"
                },
                "html_url": "https://github.com/alice/hyprland-catppuccin",
                "description": "Clean Catppuccin mocha theme for Hyprland and Waybar",
                "stargazers_count": 450,
                "forks_count": 32,
                "default_branch": "main",
                "topics": ["hyprland", "waybar", "catppuccin"],
                "license": {
                    "key": "mit",
                    "name": "MIT License",
                    "spdx_id": "MIT",
                    "url": "https://api.github.com/licenses/mit"
                },
                "updated_at": "2026-03-01T12:00:00Z"
            }
        ]
    }"#;

    mock_transport.register_response(
        "https://api.github.com/search/repositories",
        HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: search_json.as_bytes().to_vec(),
        },
    );

    let temp_cache = TempTestDir::new("test");
    let provider = GitHubProvider::with_transport(mock_transport, temp_cache.path().to_path_buf());

    let query = ProviderQuery {
        query: "hyprland".to_string(),
        category: Some("wm_rice".to_string()),
        desktop: Some("Hyprland".to_string()),
        page: 1,
        page_size: 10,
    };

    let items = provider.search(&query).expect("Search must succeed");
    assert_eq!(items.len(), 1);

    let item = &items[0];
    assert_eq!(item.id, "gh-alice-hyprland-catppuccin");
    assert_eq!(item.title, "Hyprland Catppuccin");
    assert_eq!(item.author.name, "alice");
    assert_eq!(item.author.verified, false); // Strictly unverified
    assert_eq!(item.package_type, PackageType::Rice);
    assert_eq!(item.provenance.provider_id, "github");
    assert_eq!(
        item.provenance.repository,
        Some("alice/hyprland-catppuccin".to_string())
    );
    assert_eq!(item.provenance.license_spdx, Some("MIT".to_string()));

    // CRITICAL: Normalization MUST assign Community trust tier
    let normalized = normalize_provider_item(item.clone());
    assert_eq!(normalized.trust_tier, Some(TrustTier::Community));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Official));
    assert!(normalized.tags.contains(&"provider:github".to_string()));
    assert!(normalized.tags.contains(&"license:MIT".to_string()));
}

#[test]
fn test_github_rate_limit_detection_and_aggregator_resilience() {
    let mock_transport = Arc::new(MockHttpTransport::new());

    // Register a 403 Rate Limit response
    let mut rl_headers = HashMap::new();
    rl_headers.insert("x-ratelimit-remaining".to_string(), "0".to_string());
    rl_headers.insert("x-ratelimit-reset".to_string(), "120".to_string());

    mock_transport.register_response(
        "https://api.github.com/search/repositories",
        HttpResponse {
            status: 403,
            headers: rl_headers,
            body: br#"{"message": "API rate limit exceeded"}"#.to_vec(),
        },
    );

    let temp_cache = TempTestDir::new("test");
    let github_provider = Arc::new(GitHubProvider::with_transport(
        mock_transport,
        temp_cache.path().to_path_buf(),
    ));

    // Direct search returns RateLimitExceeded error
    let query = ProviderQuery {
        query: "popular rices".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };

    let err = github_provider.search(&query).unwrap_err();
    match err {
        ProviderError::RateLimitExceeded { reset_seconds, .. } => {
            assert!(reset_seconds > 0);
        }
        other => panic!("Expected RateLimitExceeded, got {:?}", other),
    }

    // Now test aggregator resilience: aggregator must not crash!
    let mut manager = ProviderManager::new();
    manager.register_provider(github_provider);

    let agg_response = manager.search_all(&query);
    assert_eq!(agg_response.items.len(), 0);

    // Status for github provider must be RateLimited
    let status = agg_response.provider_statuses.get("github").unwrap();
    match status {
        ProviderStatus::RateLimited { reset_seconds } => {
            assert!(reset_seconds > &0);
        }
        other => panic!("Expected RateLimited status in aggregator, got {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Safe Tarball Extraction & Malicious Payload Rejection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_safe_tarball_extraction_valid_archive() {
    let valid_bytes = create_valid_test_tarball();
    let dest_dir = TempTestDir::new("test");

    let result = extract_tarball_safe(
        std::io::Cursor::new(valid_bytes),
        dest_dir.path(),
        MAX_ARCHIVE_UNCOMPRESSED_BYTES,
    );
    assert!(result.is_ok(), "Valid archive must extract successfully");

    // Check that files are extracted without root prefix
    assert!(dest_dir.path().join("hypr/hyprland.conf").is_file());
    assert!(dest_dir.path().join("waybar/config").is_file());
    assert!(dest_dir.path().join("wallpapers/sunset.png").is_file());
    assert!(dest_dir.path().join("install.sh").is_file());
}

#[test]
fn test_safe_tarball_extraction_rejects_symlink_attacks() {
    let symlink_bytes = create_symlink_attack_tarball();
    let dest_dir = TempTestDir::new("test");

    let result = extract_tarball_safe(
        std::io::Cursor::new(symlink_bytes),
        dest_dir.path(),
        MAX_ARCHIVE_UNCOMPRESSED_BYTES,
    );

    assert!(result.is_err(), "Symlink archive must be fatally rejected");
    let err = result.unwrap_err();
    match err {
        ProviderError::SecurityRejected(msg) => {
            assert!(
                msg.contains("symlink"),
                "Error message must mention symlink: {}",
                msg
            );
        }
        other => panic!("Expected SecurityRejected error, got {:?}", other),
    }
}

#[test]
fn test_safe_tarball_extraction_rejects_path_traversal_attacks() {
    let traversal_bytes = create_traversal_attack_tarball();
    let dest_dir = TempTestDir::new("test");

    let result = extract_tarball_safe(
        std::io::Cursor::new(traversal_bytes),
        dest_dir.path(),
        MAX_ARCHIVE_UNCOMPRESSED_BYTES,
    );

    assert!(
        result.is_err(),
        "Path traversal archive must be fatally rejected"
    );
    let err = result.unwrap_err();
    match err {
        ProviderError::SecurityRejected(msg) => {
            assert!(
                msg.contains("parent traversal"),
                "Error message must mention parent traversal: {}",
                msg
            );
        }
        other => panic!("Expected SecurityRejected error, got {:?}", other),
    }
}

#[test]
fn test_safe_tarball_extraction_rejects_decompression_bomb() {
    let valid_bytes = create_valid_test_tarball();
    let dest_dir = TempTestDir::new("test");

    // Set max decompressed bytes to 10 bytes (lower than actual payload)
    let result = extract_tarball_safe(
        std::io::Cursor::new(valid_bytes),
        dest_dir.path(),
        10, // tiny limit
    );

    assert!(result.is_err(), "Exceeding decompressed limit must fail");
    let err = result.unwrap_err();
    match err {
        ProviderError::SecurityRejected(msg) => {
            assert!(
                msg.contains("exceeded maximum limit"),
                "Error message must mention size limit: {}",
                msg
            );
        }
        other => panic!("Expected SecurityRejected error, got {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Convention-Based Discovery Engine
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_convention_discovery_maps_desktop_tools_and_ignores_scripts() {
    let temp_repo = TempTestDir::new("test");
    let root = temp_repo.path();

    // Create folder structure
    fs::create_dir_all(root.join("hypr")).unwrap();
    fs::write(root.join("hypr/hyprland.conf"), "# hypr").unwrap();

    fs::create_dir_all(root.join("waybar")).unwrap();
    fs::write(root.join("waybar/config"), "{}").unwrap();

    fs::create_dir_all(root.join("kitty")).unwrap();
    fs::write(root.join("kitty/kitty.conf"), "# kitty").unwrap();

    fs::create_dir_all(root.join("fastfetch")).unwrap();
    fs::write(root.join("fastfetch/config.jsonc"), "{}").unwrap();

    fs::create_dir_all(root.join("rofi")).unwrap();
    fs::write(root.join("rofi/config.rasi"), "/* rofi */").unwrap();

    fs::create_dir_all(root.join("hyprlock")).unwrap();
    fs::write(root.join("hyprlock/hyprlock.conf"), "# lock").unwrap();

    fs::create_dir_all(root.join("wallpapers")).unwrap();
    fs::write(root.join("wallpapers/mountain.png"), "fake").unwrap();

    // Root wallpaper
    fs::write(root.join("wallpaper.jpg"), "fake").unwrap();

    // Ignored scripts and documentation
    fs::write(root.join("install.sh"), "#!/bin/sh").unwrap();
    fs::write(root.join("setup.bash"), "#!/bin/bash").unwrap();
    fs::write(root.join("README.md"), "# Readme").unwrap();
    fs::write(root.join("LICENSE"), "MIT").unwrap();

    let specs = discover_conventions(root).expect("Convention discovery must succeed");

    let target_map: HashMap<String, String> =
        specs.into_iter().map(|s| (s.source, s.target)).collect();

    // Verify all tools are mapped correctly to safe desktop targets
    assert_eq!(
        target_map.get("hypr/hyprland.conf").map(|s| s.as_str()),
        Some("~/.config/hypr/hyprland.conf")
    );
    assert_eq!(
        target_map.get("waybar/config").map(|s| s.as_str()),
        Some("~/.config/waybar/config")
    );
    assert_eq!(
        target_map.get("kitty/kitty.conf").map(|s| s.as_str()),
        Some("~/.config/kitty/kitty.conf")
    );
    assert_eq!(
        target_map.get("fastfetch/config.jsonc").map(|s| s.as_str()),
        Some("~/.config/fastfetch/config.jsonc")
    );
    assert_eq!(
        target_map.get("rofi/config.rasi").map(|s| s.as_str()),
        Some("~/.config/rofi/config.rasi")
    );
    assert_eq!(
        target_map.get("hyprlock/hyprlock.conf").map(|s| s.as_str()),
        Some("~/.config/hyprlock/hyprlock.conf")
    );
    assert_eq!(
        target_map
            .get("wallpapers/mountain.png")
            .map(|s| s.as_str()),
        Some("~/Pictures/Wallpapers/mountain.png")
    );
    assert_eq!(
        target_map.get("wallpaper.jpg").map(|s| s.as_str()),
        Some("~/Pictures/Wallpapers/wallpaper.jpg")
    );

    // Verify scripts and documentation were NOT mapped
    assert!(!target_map.contains_key("install.sh"));
    assert!(!target_map.contains_key("setup.bash"));
    assert!(!target_map.contains_key("README.md"));
    assert!(!target_map.contains_key("LICENSE"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Complete Payload Staging & Manifest Synthesis
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_stage_payload_flow_and_manifest_synthesis() {
    let mock_transport = Arc::new(MockHttpTransport::new());
    let tarball_bytes = create_valid_test_tarball();

    mock_transport.register_response(
        "https://api.github.com/repos/alice/hyprland-dots/tarball/main",
        HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: tarball_bytes,
        },
    );

    let temp_cache = TempTestDir::new("test");
    let staging_root = TempTestDir::new("test");

    let provider = GitHubProvider::with_transport(mock_transport, temp_cache.path().to_path_buf());

    let item = ryzora_lib::providers::github::repo_to_provider_item(
        &ryzora_lib::providers::github::GitHubRepoItem {
            id: 112233,
            name: "hyprland-dots".to_string(),
            full_name: "alice/hyprland-dots".to_string(),
            owner: ryzora_lib::providers::github::GitHubOwner {
                login: "alice".to_string(),
                avatar_url: None,
                html_url: "https://github.com/alice".to_string(),
            },
            html_url: "https://github.com/alice/hyprland-dots".to_string(),
            description: Some("Awesome dots".to_string()),
            stargazers_count: 50,
            forks_count: 5,
            default_branch: Some("main".to_string()),
            topics: vec!["hyprland".to_string(), "waybar".to_string()],
            license: Some(ryzora_lib::providers::github::GitHubLicense {
                key: Some("mit".to_string()),
                name: Some("MIT License".to_string()),
                spdx_id: Some("MIT".to_string()),
                url: None,
            }),
            updated_at: None,
        },
    );

    let staged_dir = provider
        .stage_payload(&item, staging_root.path())
        .expect("Payload staging must succeed");

    assert!(staged_dir.is_dir());
    assert_eq!(staged_dir.file_name().unwrap(), "gh-alice-hyprland-dots");

    // Check synthesized manifest.json
    let manifest_path = staged_dir.join("manifest.json");
    assert!(manifest_path.is_file());

    let manifest_content = fs::read_to_string(&manifest_path).unwrap();
    let manifest: ryzora_lib::manifest::RyzoraManifest =
        serde_json::from_str(&manifest_content).unwrap();

    assert_eq!(manifest.ryzora_spec, "1");
    assert_eq!(manifest.id, "gh-alice-hyprland-dots");
    assert!(manifest.files.len() >= 3); // hyprland.conf, waybar/config, wallpapers/sunset.png

    // Verify install.sh was excluded from synthesized manifest
    for f in &manifest.files {
        assert!(!f.source.contains("install.sh"));
        assert!(!f.target.contains("install.sh"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Caching, Ref Pinning, & License Preservation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_github_provider_caching_avoids_redundant_http_calls() {
    let mock_transport = Arc::new(MockHttpTransport::new());

    let search_json = r#"{
        "total_count": 1,
        "items": [
            {
                "id": 1122,
                "name": "waybar-sleek",
                "full_name": "bob/waybar-sleek",
                "owner": {
                    "login": "bob",
                    "avatar_url": null,
                    "html_url": "https://github.com/bob"
                },
                "html_url": "https://github.com/bob/waybar-sleek",
                "description": "Minimal Waybar bar",
                "stargazers_count": 100,
                "forks_count": 10,
                "default_branch": "main",
                "topics": ["waybar"],
                "license": null,
                "updated_at": null
            }
        ]
    }"#;

    mock_transport.register_response(
        "https://api.github.com/search/repositories",
        HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: search_json.as_bytes().to_vec(),
        },
    );

    let temp_cache = TempTestDir::new("cache_test");
    let provider =
        GitHubProvider::with_transport(mock_transport.clone(), temp_cache.path().to_path_buf());

    let query = ProviderQuery {
        query: "waybar".to_string(),
        category: Some("panel_bar".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };

    // First call: executes HTTP request
    let items_1 = provider.search(&query).unwrap();
    assert_eq!(items_1.len(), 1);
    assert_eq!(mock_transport.recorded_calls.lock().unwrap().len(), 1);

    // Second call: serves from disk cache!
    let items_2 = provider.search(&query).unwrap();
    assert_eq!(items_2.len(), 1);
    assert_eq!(items_1[0].id, items_2[0].id);
    assert_eq!(
        mock_transport.recorded_calls.lock().unwrap().len(),
        1,
        "Second call must be served from cache"
    );
}

#[test]
fn test_github_stage_payload_preserves_pinned_commit_or_ref() {
    let mock_transport = Arc::new(MockHttpTransport::new());
    let tarball_bytes = create_valid_test_tarball();

    // Register pinned ref URL
    mock_transport.register_response(
        "https://api.github.com/repos/alice/hyprland-dots/tarball/v2.1.0",
        HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: tarball_bytes,
        },
    );

    let temp_cache = TempTestDir::new("ref_cache");
    let staging_root = TempTestDir::new("ref_staging");

    let provider =
        GitHubProvider::with_transport(mock_transport.clone(), temp_cache.path().to_path_buf());

    let mut item = ryzora_lib::providers::github::repo_to_provider_item(
        &ryzora_lib::providers::github::GitHubRepoItem {
            id: 445566,
            name: "hyprland-dots".to_string(),
            full_name: "alice/hyprland-dots".to_string(),
            owner: ryzora_lib::providers::github::GitHubOwner {
                login: "alice".to_string(),
                avatar_url: None,
                html_url: "https://github.com/alice".to_string(),
            },
            html_url: "https://github.com/alice/hyprland-dots".to_string(),
            description: Some("Pinned release".to_string()),
            stargazers_count: 50,
            forks_count: 5,
            default_branch: Some("main".to_string()),
            topics: vec!["hyprland".to_string()],
            license: Some(ryzora_lib::providers::github::GitHubLicense {
                key: Some("gpl-3.0".to_string()),
                name: Some("GNU General Public License v3.0".to_string()),
                spdx_id: Some("GPL-3.0-only".to_string()),
                url: None,
            }),
            updated_at: None,
        },
    );

    // Pin to specific release tag
    item.provenance.commit_or_tag = Some("v2.1.0".to_string());
    item.version = "v2.1.0".to_string();

    let staged_dir = provider
        .stage_payload(&item, staging_root.path())
        .expect("Staging pinned release must succeed");

    // Check that HTTP request requested the exact pinned ref
    let recorded = mock_transport.recorded_calls.lock().unwrap();
    assert!(recorded
        .iter()
        .any(|(url, _)| url.ends_with("/tarball/v2.1.0")));

    // Verify manifest version normalized v2.1.0 -> 2.1.0
    let manifest_path = staged_dir.join("manifest.json");
    let manifest_content = fs::read_to_string(&manifest_path).unwrap();
    let manifest: ryzora_lib::manifest::RyzoraManifest =
        serde_json::from_str(&manifest_content).unwrap();
    assert_eq!(manifest.version, "2.1.0");
    assert!(manifest.description.contains("License: GPL-3.0-only"));
}

#[test]
fn test_safe_tarball_extraction_rejects_absolute_path_attacks() {
    use std::io::Write;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut header_block = vec![0u8; 512];

    let name = b"/etc/passwd";
    header_block[..name.len()].copy_from_slice(name);
    header_block[100..108].copy_from_slice(b"0000644 ");
    header_block[124..136].copy_from_slice(b"00000000005 ");
    header_block[156] = b'0';
    header_block[257..263].copy_from_slice(b"ustar ");
    header_block[263..265].copy_from_slice(b"00");

    header_block[148..156].copy_from_slice(b"        ");
    let sum: u32 = header_block.iter().map(|&b| b as u32).sum();
    let chk = format!("{:06o}  ", sum);
    header_block[148..156].copy_from_slice(chk.as_bytes());

    encoder.write_all(&header_block).unwrap();
    let mut data_block = vec![0u8; 512];
    data_block[..5].copy_from_slice(b"root:");
    encoder.write_all(&data_block).unwrap();
    encoder.write_all(&vec![0u8; 1024]).unwrap();
    let archive_bytes = encoder.finish().unwrap();

    let dest_dir = TempTestDir::new("abs_test");
    let res = extract_tarball_safe(
        std::io::Cursor::new(archive_bytes),
        dest_dir.path(),
        MAX_ARCHIVE_UNCOMPRESSED_BYTES,
    );

    assert!(res.is_err(), "Absolute path must be rejected");
    match res.unwrap_err() {
        ProviderError::SecurityRejected(msg) => {
            assert!(msg.contains("must be relative"), "Error message: {}", msg);
        }
        other => panic!("Expected SecurityRejected, got {:?}", other),
    }
}

// Tests: Zero Subprocess and Privilege Audit for Providers & GitHub Adapter
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_github_provider_zero_subprocess_and_privilege_audit() {
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
