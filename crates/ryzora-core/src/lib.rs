//! ryzora-core
//!
//! UI-independent domain logic, manifest data models, repository discovery,
//! and specification validation for the Ryzora package manager and desktop integration engine.

pub mod compatibility;
pub mod crypto;
pub mod dependency;
pub mod distribution;
pub mod manifest;
pub mod repository;
pub mod snapshot;

pub use compatibility::CompatibilityRequirements;
pub use crypto::{CryptographicStatus, PackageSignatureMetadata, SignerIdentity};
pub use dependency::{DependencyKind, DependencySpec};
pub use distribution::{ModerationStatus, ReleaseChannel, TrustTier};
pub use manifest::{
    default_true, validate_manifest_internal, ManifestCompatibility, ManifestFile,
    ManifestValidationResult, PackageManifest, PackageType, RyzoraManifest, TargetDefinition,
    TargetEntry,
};
pub use repository::{
    compute_package_tree_hash, discover_local_repositories, filter_catalog_items,
    get_default_repository_search_dirs, validate_path_identifier, validate_remote_url,
    validate_repository_package_entry, validate_sha256_hash, AuthorInfo, FrontendPackageItem,
    LocalRepository, PackageProvenanceSpec, PackageSourceSpec, Repository, RepositoryIndex,
    RepositoryPackageEntry, RepositorySourceConfig, RepositorySummary,
};

pub use snapshot::{create_snapshot_in, restore_snapshot_in, verify_snapshot_in, list_snapshots_in, get_snapshot_in, delete_snapshot_in, apply_retention_policy_in, FileEntry, FileEntryType, RestoreResult, SnapshotMetadata, SnapshotStatus};
