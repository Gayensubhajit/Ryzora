use super::ProviderItem;
use crate::compatibility::CompatibilityRequirements;
use crate::distribution::{ModerationStatus, ReleaseChannel, TrustTier};
use crate::repository::{
    FrontendComponentSpec, FrontendDependencies, FrontendPackageItem, FrontendSafetyAudit,
};

/// Normalizes a raw ProviderItem into a frontend-facing FrontendPackageItem.
/// Crucial Security Invariant: External items imported from third-party providers
/// are ALWAYS assigned `TrustTier::Community` (unvetted). They can never bypass
/// trust verification or claim Official status.
pub fn normalize_provider_item(item: ProviderItem) -> FrontendPackageItem {
    let mut tags = item.tags.clone();
    let provider_tag = format!("provider:{}", item.provenance.provider_id);
    if !tags.contains(&provider_tag) {
        tags.insert(0, provider_tag);
    }
    if let Some(ref spdx) = item.provenance.license_spdx {
        let license_tag = format!("license:{}", spdx);
        if !tags.contains(&license_tag) {
            tags.push(license_tag);
        }
    }

    let components: Vec<FrontendComponentSpec> = item
        .files
        .iter()
        .map(|f| FrontendComponentSpec {
            name: f.source.clone(),
            component_type: format!("{:?}", item.package_type).to_lowercase(),
            target_path: f.target.clone(),
            description: format!("Provided by {}", item.provenance.provider_id),
        })
        .collect();

    let files_count = item.files.len();

    FrontendPackageItem {
        id: item.id.clone(),
        title: item.title,
        subtitle: item.subtitle,
        description: item.description,
        version: item.version,
        author: item.author,
        category: item.category,
        package_type: item.package_type,
        tags,
        supported_desktops: item.supported_desktops.clone(),
        supported_display: item.supported_display.clone(),
        rating: item.rating.unwrap_or(0.0),
        rating_count: item.rating_count.unwrap_or(0) as u32,
        downloads: item.downloads.unwrap_or(0),
        hero_image: item.hero_image.unwrap_or_default(),
        screenshots: item.screenshots,
        featured: Some(false),
        trending: Some(false),
        color_palette: Vec::new(),
        safety_audit: FrontendSafetyAudit {
            rating: "unverified".to_string(),
            changes_system_files: false,
            requires_root: false,
            sandbox_compatible: true,
            files_modified_count: files_count,
        },
        dependencies: FrontendDependencies {
            packages: Vec::new(),
            optional: Vec::new(),
        },
        components,
        compatibility: CompatibilityRequirements {
            supported_distros: Vec::new(),
            supported_desktops: item.supported_desktops.clone(),
            supported_sessions: item.supported_display.clone(),
            required_binaries: Vec::new(),
            optional_binaries: Vec::new(),
        },
        manifest: None,
        repository_id: Some(format!("provider:{}", item.provenance.provider_id)),
        content_hash: None,
        package_size_bytes: None,
        integrity_status: "unverified".to_string(),
        is_cached: false,
        release_channel: Some(ReleaseChannel::Stable),
        trust_tier: Some(TrustTier::Community),
        moderation_status: Some(ModerationStatus::PendingReview),
        trending_score: None,
        maintainer: Some(item.provenance.original_author),
        release_notes: None,
        signature: None,
        cryptographic_status: None,
        provider: None,
        targets: None,
        preview_video: None,
        preview_animated: None,
        source: None,
        provenance: None,
    }
}
