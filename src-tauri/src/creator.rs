use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::crypto::{compute_key_fingerprint, TrustStore};
use crate::distribution::{ReleaseChannel, TrustTier};
use crate::repository::{create_default_manager, RepositoryPackageEntry};
use ed25519_dalek::VerifyingKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorProfile {
    pub id: String, // normalized author slug, e.g. "archcraft-team"
    pub display_name: String,
    pub avatar: String,
    pub verified: bool,
    pub trust_tier: TrustTier,
    pub public_key_fingerprint: Option<String>,
    pub total_packages: usize,
    pub total_downloads: u64,
    pub average_rating: Option<f64>,
    pub package_ids: Vec<String>,
    pub recent_packages: Vec<RepositoryPackageEntry>,
    pub release_channels: Vec<ReleaseChannel>,
}

pub fn normalize_creator_id(name: &str) -> String {
    let slug: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    // Collapse multiple consecutive hyphens
    let mut clean_slug = String::new();
    let mut last_was_hyphen = false;
    for c in slug.chars() {
        if c == '-' {
            if !last_was_hyphen && !clean_slug.is_empty() {
                clean_slug.push(c);
                last_was_hyphen = true;
            }
        } else {
            clean_slug.push(c);
            last_was_hyphen = false;
        }
    }
    let trimmed = clean_slug.trim_end_matches('-').to_string();
    if trimmed.is_empty() {
        "anonymous-creator".to_string()
    } else {
        trimmed
    }
}

#[tauri::command]
pub fn list_creator_profiles() -> Result<Vec<CreatorProfile>, String> {
    let manager = create_default_manager();
    let trust_store = TrustStore::load_default();

    // Gather all package entries across repositories
    let mut all_entries: Vec<RepositoryPackageEntry> = Vec::new();
    for repo in manager.repositories() {
        if let Ok(entries) = repo.list_entries() {
            all_entries.extend(entries);
        }
    }

    Ok(build_creator_profiles(&all_entries, &trust_store))
}

#[tauri::command]
pub fn get_creator_profile(creator_id: String) -> Result<CreatorProfile, String> {
    let profiles = list_creator_profiles()?;
    profiles
        .into_iter()
        .find(|p| p.id == creator_id)
        .ok_or_else(|| format!("Creator profile '{}' not found", creator_id))
}

pub fn build_creator_profiles(
    entries: &[RepositoryPackageEntry],
    trust_store: &TrustStore,
) -> Vec<CreatorProfile> {
    let mut groups: HashMap<String, Vec<RepositoryPackageEntry>> = HashMap::new();
    let mut display_names: HashMap<String, String> = HashMap::new();
    let mut avatars: HashMap<String, String> = HashMap::new();

    for entry in entries {
        let raw_name = entry.author.name.trim();
        if raw_name.is_empty() {
            continue;
        }
        let slug = normalize_creator_id(raw_name);

        groups.entry(slug.clone()).or_default().push(entry.clone());

        if !display_names.contains_key(&slug) || display_names[&slug].len() < raw_name.len() {
            display_names.insert(slug.clone(), raw_name.to_string());
        }

        if !entry.author.avatar.is_empty() && !avatars.contains_key(&slug) {
            avatars.insert(slug.clone(), entry.author.avatar.clone());
        }
    }

    let trusted_keys = trust_store.list_trusted_keys();
    let mut profiles = Vec::new();

    for (slug, pkgs) in groups {
        let display_name = display_names
            .get(&slug)
            .cloned()
            .unwrap_or_else(|| slug.clone());
        let avatar = avatars.get(&slug).cloned().unwrap_or_default();

        let total_packages = pkgs.len();
        let total_downloads: u64 = pkgs.iter().map(|p| p.downloads.unwrap_or(0)).sum();

        let ratings: Vec<f64> = pkgs.iter().filter_map(|p| p.rating).collect();
        let average_rating = if !ratings.is_empty() {
            Some(ratings.iter().sum::<f64>() / ratings.len() as f64)
        } else {
            None
        };

        let package_ids: Vec<String> = pkgs.iter().map(|p| p.id.clone()).collect();

        let mut release_channels: Vec<ReleaseChannel> = Vec::new();
        for p in &pkgs {
            if let Some(ch) = p.release_channel {
                if !release_channels.contains(&ch) {
                    release_channels.push(ch);
                }
            }
        }
        release_channels.sort_by_key(|c| match c {
            ReleaseChannel::Stable => 0,
            ReleaseChannel::Beta => 1,
            ReleaseChannel::Nightly => 2,
        });

        // Resolve Trust Authoritatively:
        // NEVER trust unvetted JSON metadata! Check against TrustStore.
        let mut verified = false;
        let mut trust_tier = TrustTier::Community;
        let mut fingerprint: Option<String> = None;

        // 1. Check if any package has a signature with a recognized trusted key
        for p in &pkgs {
            if let Some(ref sig) = p.signature {
                let key_id = &sig.key_id;
                let pub_key_hex = &sig.public_key;

                if !trust_store.is_revoked(key_id, pub_key_hex) {
                    if trust_store.is_official_core_key(pub_key_hex) {
                        verified = true;
                        trust_tier = TrustTier::Official;
                        if let Ok(bytes) = hex::decode(pub_key_hex) {
                            if let Ok(vk) = VerifyingKey::from_bytes(
                                bytes.as_slice().try_into().unwrap_or(&[0u8; 32]),
                            ) {
                                fingerprint = Some(compute_key_fingerprint(&vk));
                            }
                        }
                        break;
                    } else if trust_store.is_trusted_author(key_id, pub_key_hex) {
                        verified = true;
                        trust_tier = TrustTier::Verified;
                        if let Ok(bytes) = hex::decode(pub_key_hex) {
                            if let Ok(vk) = VerifyingKey::from_bytes(
                                bytes.as_slice().try_into().unwrap_or(&[0u8; 32]),
                            ) {
                                fingerprint = Some(compute_key_fingerprint(&vk));
                            }
                        }
                        break;
                    }
                }
            }
        }

        // 2. If not signed, check if creator name directly matches a trusted key entry
        if !verified {
            if let Some(tk) = trusted_keys.iter().find(|k| {
                k.author_name.eq_ignore_ascii_case(&display_name)
                    || normalize_creator_id(&k.author_name) == slug
            }) {
                if !trust_store.is_revoked(&tk.key_id, &tk.public_key) {
                    if trust_store.is_official_core_key(&tk.public_key) {
                        verified = true;
                        trust_tier = TrustTier::Official;
                    } else {
                        verified = true;
                        trust_tier = TrustTier::Verified;
                    }
                    if let Ok(bytes) = hex::decode(&tk.public_key) {
                        if let Ok(vk) = VerifyingKey::from_bytes(
                            bytes.as_slice().try_into().unwrap_or(&[0u8; 32]),
                        ) {
                            fingerprint = Some(compute_key_fingerprint(&vk));
                        }
                    }
                }
            }
        }

        let mut recent_packages = pkgs;
        recent_packages.sort_by(|a, b| b.downloads.unwrap_or(0).cmp(&a.downloads.unwrap_or(0)));

        profiles.push(CreatorProfile {
            id: slug,
            display_name,
            avatar,
            verified,
            trust_tier,
            public_key_fingerprint: fingerprint,
            total_packages,
            total_downloads,
            average_rating,
            package_ids,
            recent_packages,
            release_channels,
        });
    }

    // Sort profiles: Verified/Official first, then by total downloads descending
    profiles.sort_by(|a, b| {
        let tier_weight = |t: TrustTier| match t {
            TrustTier::Official => 0,
            TrustTier::Verified => 1,
            TrustTier::Community => 2,
            TrustTier::Untrusted => 3,
        };
        tier_weight(a.trust_tier)
            .cmp(&tier_weight(b.trust_tier))
            .then_with(|| b.total_downloads.cmp(&a.total_downloads))
            .then_with(|| a.display_name.cmp(&b.display_name))
    });

    profiles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution::ReleaseChannel;
    use crate::repository::AuthorInfo;

    #[test]
    fn test_creator_profiles_normalization_prevents_duplicates() {
        assert_eq!(normalize_creator_id("Archcraft Team"), "archcraft-team");
        assert_eq!(
            normalize_creator_id("  Archcraft   Team  "),
            "archcraft-team"
        );
        assert_eq!(normalize_creator_id("archcraft-team"), "archcraft-team");
        assert_eq!(
            normalize_creator_id("Creator #1 / Linux"),
            "creator-1-linux"
        );
        assert_eq!(normalize_creator_id(""), "anonymous-creator");
    }

    #[test]
    fn test_creator_profiles_aggregation_from_catalog() {
        let p1 = RepositoryPackageEntry {
            id: "pkg-1".to_string(),
            name: "Theme Alpha".to_string(),
            version: "1.0.0".to_string(),
            downloads: Some(1500),
            rating: Some(4.8),
            author: AuthorInfo {
                name: "Alice Dev".to_string(),
                avatar: "https://example.com/alice.png".to_string(),
                verified: false,
            },
            release_channel: Some(ReleaseChannel::Stable),
            ..Default::default()
        };

        let p2 = RepositoryPackageEntry {
            id: "pkg-2".to_string(),
            name: "Theme Beta".to_string(),
            version: "2.0.0".to_string(),
            downloads: Some(2500),
            rating: Some(4.6),
            author: AuthorInfo {
                name: "Alice Dev".to_string(),
                avatar: "".to_string(),
                verified: false,
            },
            release_channel: Some(ReleaseChannel::Beta),
            ..Default::default()
        };

        let trust_store = TrustStore::load_default();
        let profiles = build_creator_profiles(&[p1, p2], &trust_store);

        assert_eq!(profiles.len(), 1);
        let alice = &profiles[0];
        assert_eq!(alice.id, "alice-dev");
        assert_eq!(alice.display_name, "Alice Dev");
        assert_eq!(alice.avatar, "https://example.com/alice.png");
        assert_eq!(alice.total_packages, 2);
        assert_eq!(alice.total_downloads, 4000);
        assert!((alice.average_rating.unwrap() - 4.7).abs() < 0.01);
        assert_eq!(alice.package_ids, vec!["pkg-1", "pkg-2"]);
        assert_eq!(alice.release_channels.len(), 2);
    }

    #[test]
    fn test_creator_profiles_unvetted_author_unverified() {
        // Author claims verified: true in untrusted repository JSON
        let unvetted = RepositoryPackageEntry {
            id: "unvetted-pkg".to_string(),
            name: "Unvetted Pkg".to_string(),
            version: "1.0.0".to_string(),
            downloads: Some(500),
            author: AuthorInfo {
                name: "Rogue Imposter".to_string(),
                avatar: "".to_string(),
                verified: true, // UNTRUSTED ADVISORY CLAIM
            },
            ..Default::default()
        };

        let trust_store = TrustStore::load_default();
        let profiles = build_creator_profiles(&[unvetted], &trust_store);

        assert_eq!(profiles.len(), 1);
        let rogue = &profiles[0];
        assert!(!rogue.verified, "Advisory verified claim MUST be discarded");
        assert_eq!(rogue.trust_tier, TrustTier::Community);
        assert!(rogue.public_key_fingerprint.is_none());
    }

    #[test]
    fn test_creator_profiles_verified_status_from_keyring() {
        let trust_store = TrustStore::load_default();
        let trusted_keys = trust_store.list_trusted_keys();

        if let Some(tk) = trusted_keys.first() {
            let pkg = RepositoryPackageEntry {
                id: "verified-pkg".to_string(),
                name: "Verified Package".to_string(),
                version: "1.0.0".to_string(),
                downloads: Some(10000),
                author: AuthorInfo {
                    name: tk.author_name.clone(),
                    avatar: "".to_string(),
                    verified: false, // Even if repo metadata omits verified
                },
                ..Default::default()
            };

            let profiles = build_creator_profiles(&[pkg], &trust_store);
            assert_eq!(profiles.len(), 1);
            let verified_creator = &profiles[0];
            assert!(
                verified_creator.verified,
                "Keyring match must mark author as verified"
            );
            assert!(
                verified_creator.trust_tier == TrustTier::Verified
                    || verified_creator.trust_tier == TrustTier::Official
            );
            assert!(verified_creator.public_key_fingerprint.is_some());
            let fp = verified_creator.public_key_fingerprint.as_ref().unwrap();
            assert!(fp.starts_with("ed25519:"));
        }
    }

    #[test]
    fn test_creator_profiles_public_key_fingerprint_lookup() {
        use crate::crypto::compute_key_fingerprint;
        use ed25519_dalek::SigningKey;
        use rand_core::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let fp = compute_key_fingerprint(&verifying_key);

        assert!(fp.starts_with("ed25519:"));
        assert_eq!(fp.len(), "ed25519:".len() + 16);
    }
}
