//! Ryzora distribution and store trust models.

use serde::{Deserialize, Serialize};

/// Supported release channels for Ryzora packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    Stable,
    Beta,
    Nightly,
}

impl Default for ReleaseChannel {
    fn default() -> Self {
        ReleaseChannel::Stable
    }
}

impl std::fmt::Display for ReleaseChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseChannel::Stable => write!(f, "stable"),
            ReleaseChannel::Beta => write!(f, "beta"),
            ReleaseChannel::Nightly => write!(f, "nightly"),
        }
    }
}

/// Store trust tier evaluated from repository origin, author verification, and hash validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustTier {
    /// Curated and officially maintained by the Ryzora Core team.
    Official,
    /// Author identity and cryptographic signatures verified against trusted registry.
    Verified,
    /// Contributed by community member; passes all declarative sandbox pre-flight checks.
    Community,
    /// Untrusted: unverified source, checksum mismatch, or unapproved modifications.
    Untrusted,
}

impl Default for TrustTier {
    fn default() -> Self {
        TrustTier::Community
    }
}

/// Package curation and moderation status in the store catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationStatus {
    /// Approved for public listing in the store catalog.
    Approved,
    /// Submitted and waiting for community PR review or maintainer audit.
    PendingReview,
    /// Flagged by automated heuristics or reported by users.
    Flagged,
    /// Deprecated in favor of an upgraded package or no longer maintained.
    Deprecated,
}

impl Default for ModerationStatus {
    fn default() -> Self {
        ModerationStatus::Approved
    }
}
