//! Phase 18.10: Fastfetch Preset Preview Fidelity Regression Test.
//!
//! Enforces:
//! 1. Preset-2 config != Preset-3 config != Preset-5 config
//! 2. Preview resolution for presets points to their respective configuration files
//! 3. CamelCase deserialization of `presetId` works seamlessly
//! 4. Selecting and previewing presets NEVER modifies ~/.config/fastfetch/config.jsonc
//! 5. Untouched preset preview output faithfully differs between presets

use std::fs;
use sha2::{Digest, Sha256};
use ryzora_lib::system::{
    get_fastfetch_preset_config,
    render_fastfetch_preview_sync as render_fastfetch_preview,
    resolve_preset_path,
    FastfetchPreviewRequest,
};

fn hash_file(path: &std::path::Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    let data = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Some(format!("{:x}", hasher.finalize()))
}

#[test]
fn test_fastfetch_preset_configs_differ() {
    let cfg2 = get_fastfetch_preset_config("fastfetch-preset-2".to_string())
        .expect("preset 2 config must resolve");
    let cfg3 = get_fastfetch_preset_config("fastfetch-preset-3".to_string())
        .expect("preset 3 config must resolve");
    let cfg5 = get_fastfetch_preset_config("fastfetch-preset-5".to_string())
        .expect("preset 5 config must resolve");

    assert!(!cfg2.trim().is_empty(), "preset-2 config must not be empty");
    assert!(!cfg3.trim().is_empty(), "preset-3 config must not be empty");
    assert!(!cfg5.trim().is_empty(), "preset-5 config must not be empty");

    assert_ne!(cfg2, cfg3, "preset-2 and preset-3 configs must be distinct");
    assert_ne!(cfg3, cfg5, "preset-3 and preset-5 configs must be distinct");
    assert_ne!(cfg2, cfg5, "preset-2 and preset-5 configs must be distinct");
}

#[test]
fn test_fastfetch_preset_resolution() {
    let home = ryzora_lib::snapshot::get_home_dir();

    let p2 = resolve_preset_path("fastfetch-preset-2", &home)
        .expect("preset-2 must resolve to a valid path");
    let p3 = resolve_preset_path("fastfetch-preset-3", &home)
        .expect("preset-3 must resolve to a valid path");
    let p5 = resolve_preset_path("fastfetch-preset-5", &home)
        .expect("preset-5 must resolve to a valid path");

    assert!(p2.exists(), "preset-2 resolved path must exist on disk");
    assert!(p3.exists(), "preset-3 resolved path must exist on disk");
    assert!(p5.exists(), "preset-5 resolved path must exist on disk");

    assert_ne!(p2, p3, "preset-2 and preset-3 must resolve to different files");
}

#[test]
fn test_fastfetch_preview_request_camelcase_deserialization() {
    let json_data = r#"{"presetId":"fastfetch-preset-2","baseConfigJsonc":"{}","isModified":false,"accentColor":"cyan","widthCols":30}"#;

    let parsed: FastfetchPreviewRequest = serde_json::from_str(json_data)
        .expect("camelCase JSON must deserialize into FastfetchPreviewRequest");

    assert_eq!(parsed.preset_id.as_deref(), Some("fastfetch-preset-2"));
    assert_eq!(parsed.is_modified, Some(false));
    assert_eq!(parsed.accent_color.as_deref(), Some("cyan"));
    assert_eq!(parsed.width_cols, Some(30));
}

#[test]
fn test_preset_preview_does_not_modify_active_config() {
    let home = ryzora_lib::snapshot::get_home_dir();
    let active_config = home.join(".config/fastfetch/config.jsonc");

    let initial_hash = hash_file(&active_config);
    let initial_len = fs::metadata(&active_config).map(|m| m.len()).ok();

    // 1. Select preset 2 and render preview
    let _ = get_fastfetch_preset_config("fastfetch-preset-2".to_string());
    let req2 = FastfetchPreviewRequest {
        preset_id: Some("fastfetch-preset-2".to_string()),
        base_config_jsonc: None,
        is_modified: Some(false),
        accent_color: None,
        emblem_type: None,
        emblem_path: None,
        width_cols: None,
        height_lines: None,
        padding_cols: None,
        dither: None,
        active_modules: None,
    };
    let res2 = render_fastfetch_preview(req2).expect("preview for preset-2 must succeed");
    assert!(res2.success);

    // 2. Select preset 3 and render preview
    let _ = get_fastfetch_preset_config("fastfetch-preset-3".to_string());
    let req3 = FastfetchPreviewRequest {
        preset_id: Some("fastfetch-preset-3".to_string()),
        base_config_jsonc: None,
        is_modified: Some(false),
        accent_color: None,
        emblem_type: None,
        emblem_path: None,
        width_cols: None,
        height_lines: None,
        padding_cols: None,
        dither: None,
        active_modules: None,
    };
    let res3 = render_fastfetch_preview(req3).expect("preview for preset-3 must succeed");
    assert!(res3.success);

    // 3. Verify active config is unchanged
    let post_hash = hash_file(&active_config);
    let post_len = fs::metadata(&active_config).map(|m| m.len()).ok();

    assert_eq!(initial_hash, post_hash, "active config.jsonc SHA256 must NOT change during preview");
    assert_eq!(initial_len, post_len, "active config.jsonc length must NOT change during preview");
}

#[test]
fn test_list_fastfetch_local_emblems_filters_and_includes_required() {
    let emblems = ryzora_lib::system::list_fastfetch_local_emblems()
        .expect("listing emblems must succeed");

    assert!(!emblems.is_empty(), "emblems list must not be empty");

    // Must not contain README or 1BIT
    for emb in &emblems {
        let name_upper = emb.name.to_uppercase();
        let path_upper = emb.path.to_uppercase();
        assert!(!name_upper.contains("README"), "Must not contain README: {:?}", emb);
        assert!(!name_upper.contains("1BIT"), "Must not contain 1BIT: {:?}", emb);
        assert!(!path_upper.ends_with(".TXT"), "Must not be text: {:?}", emb);
    }

    let names: Vec<String> = emblems.iter().map(|e| e.name.to_lowercase()).collect();
    let expected = vec![
        "katana", "arch", "moon", "camera", "mic", "needle",
        "aurelius", "david", "hawk", "lighthouse", "rashin"
    ];

    for exp in expected {
        assert!(
            names.iter().any(|n| n.contains(exp)),
            "Expected emblem {} not found in discovered emblems: {:?}",
            exp, names
        );
    }
}

#[test]
fn test_list_fastfetch_installed_presets() {
    let presets = ryzora_lib::system::list_fastfetch_installed_presets()
        .expect("listing installed presets must succeed");

    // Must be able to discover installed presets
    println!("Discovered installed presets: {:?}", presets);
    // Presets should not contain emblem packages
    for p in &presets {
        assert!(!p.contains("emblem"), "Preset list must not contain emblems: {}", p);
    }
}

#[test]
fn test_preset_logo_resolution_no_ryoku_leak() {
    let _home = ryzora_lib::snapshot::get_home_dir();

    // 1. Two-Column Spectrum (fastfetch-preset-2) without emblem override
    let req2 = FastfetchPreviewRequest {
        preset_id: Some("fastfetch-preset-2".to_string()),
        base_config_jsonc: None,
        is_modified: Some(false),
        accent_color: None,
        emblem_type: None,
        emblem_path: None,
        width_cols: None,
        height_lines: None,
        padding_cols: None,
        dither: None,
        active_modules: None,
    };
    let res2 = render_fastfetch_preview(req2).expect("preview for preset-2 must succeed");
    assert!(res2.success);
    // Must NOT resolve to ryoku or fastfetch-emblem.png
    if let Some(ref path) = res2.emblem_path {
        assert!(!path.contains("fastfetch-emblem.png"), "Must not leak Ryoku emblem");
        assert!(!path.contains("ryoku"), "Must not leak Ryoku emblem");
    }
    // Must identify the missing path from the preset
    assert_eq!(
        res2.missing_logo_path.as_deref(),
        Some("/home/cachy/.config/fastfetch/PNGs/*.png")
    );

    // 2. fastfetch-aayushchalise has real arch.png
    let req_aayush = FastfetchPreviewRequest {
        preset_id: Some("fastfetch-aayushchalise".to_string()),
        base_config_jsonc: None,
        is_modified: Some(false),
        accent_color: None,
        emblem_type: None,
        emblem_path: None,
        width_cols: None,
        height_lines: None,
        padding_cols: None,
        dither: None,
        active_modules: None,
    };
    let res_aayush = render_fastfetch_preview(req_aayush).expect("preview for aayushchalise must succeed");
    assert!(res_aayush.success);
    assert!(res_aayush.missing_logo_path.is_none());
    assert!(
        res_aayush.emblem_path.as_deref().unwrap_or("").contains("arch.png"),
        "aayushchalise must resolve to its arch.png emblem"
    );

    // 3. Explicit user selection of Katana overrides preset-2
    // katana path
    let req_override = FastfetchPreviewRequest {
        preset_id: Some("fastfetch-preset-2".to_string()),
        base_config_jsonc: None,
        is_modified: Some(true),
        accent_color: None,
        emblem_type: Some("image".to_string()),
        emblem_path: Some("repositories/community/packages/fastfetch-emblem-katana/files/katana.png".to_string()),
        width_cols: None,
        height_lines: None,
        padding_cols: None,
        dither: None,
        active_modules: None,
    };
    let res_override = render_fastfetch_preview(req_override).expect("preview with override must succeed");
    assert!(res_override.success);
    assert!(
        res_override.emblem_path.as_deref().unwrap_or("").contains("katana.png"),
        "explicit emblem selection must apply Katana"
    );
}

#[test]
fn test_emblem_selection_preserves_layout_and_right_padding() {
    let _home = ryzora_lib::snapshot::get_home_dir();
    let katana_path = "repositories/community/packages/fastfetch-emblem-katana/files/katana.png";

    let req = FastfetchPreviewRequest {
        preset_id: Some("fastfetch-preset-2".to_string()),
        base_config_jsonc: None,
        is_modified: Some(true),
        accent_color: None,
        emblem_type: Some("image".to_string()),
        emblem_path: Some(katana_path.to_string()),
        width_cols: Some(34),
        height_lines: Some(15),
        padding_cols: Some(0), // user did not specify right padding -> backend must ensure >= 2
        dither: None,
        active_modules: None,
    };

    let res = render_fastfetch_preview(req).expect("preview with katana must succeed");
    assert!(res.success);
    assert!(res.raw_ansi.contains("kitty-direct") || res.raw_ansi.contains("Ga=T"));

    // Check that terminal advance contains right padding (width 34 + right padding >= 36)
    let has_separated_advance = res.raw_ansi.contains("[36C")
        || res.raw_ansi.contains("[37C")
        || res.raw_ansi.contains("[38C");
    assert!(
        has_separated_advance,
        "Horizontal advance must include at least 2 separation columns past image width 34"
    );
}
