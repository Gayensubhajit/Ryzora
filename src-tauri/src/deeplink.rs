//! Deep Link (`ryzora://`) Grammar, Parsing & Routing.
//!
//! Enforces:
//! - Strict prefix `ryzora://`
//! - Explicit grammar:
//!     ryzora://package/<safe-id>
//!     ryzora://repository/<safe-id>
//!     ryzora://creator/<safe-id>
//!     ryzora://category/<safe-id>
//! - Zero automatic execution: Deep links strictly map to UI navigation routes or review modals.
//!   They NEVER execute installation, mutations, or CLI commands.
//! - Rejection of traversal (`..`), file/exec schemes, shell commands, or null bytes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload", rename_all = "snake_case")]
pub enum DeepLinkAction {
    ViewPackage { package_id: String },
    ViewRepository { repo_id: String },
    ViewCreator { creator_id: String },
    ViewCategory { category: String },
}

/// Parses a deep link URL according to Ryzora's strict navigation grammar.
pub fn parse_deeplink_url(url_str: &str) -> Result<DeepLinkAction, String> {
    let trimmed = url_str.trim();
    if !trimmed.starts_with("ryzora://") {
        return Err(format!(
            "Invalid URL scheme: '{}' must start with 'ryzora://'",
            trimmed
        ));
    }

    let decoded = percent_encoding::percent_decode_str(trimmed).decode_utf8_lossy();
    if decoded.contains("..")
        || decoded.contains('\0')
        || trimmed.contains("..")
        || trimmed.contains('\0')
    {
        return Err(format!(
            "Forbidden path traversal sequence or null byte detected in deep link: '{}'",
            trimmed
        ));
    }

    let without_scheme = &trimmed["ryzora://".len()..];
    let path = without_scheme.trim_start_matches('/');

    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid deep link structure: '{}'. Expected ryzora://<entity>/<identifier>",
            trimmed
        ));
    }

    let entity = parts[0].to_lowercase();
    let identifier = parts[1];

    validate_safe_identifier(identifier)?;

    match entity.as_str() {
        "package" | "pkg" => Ok(DeepLinkAction::ViewPackage {
            package_id: identifier.to_string(),
        }),
        "repository" | "repo" => Ok(DeepLinkAction::ViewRepository {
            repo_id: identifier.to_string(),
        }),
        "creator" | "author" => Ok(DeepLinkAction::ViewCreator {
            creator_id: identifier.to_string(),
        }),
        "category" | "cat" => Ok(DeepLinkAction::ViewCategory {
            category: identifier.to_string(),
        }),
        other => Err(format!(
            "Prohibited or unknown deep link entity: '{}'. Allowed entities: package, repository, creator, category",
            other
        )),
    }
}

fn validate_safe_identifier(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Deep link identifier cannot be empty".to_string());
    }
    if id.len() > 128 {
        return Err(format!("Identifier exceeds maximum length (128): '{}'", id));
    }
    if id.contains("..") || id.contains('/') || id.contains('\\') || id.contains('\0') {
        return Err(format!("Security violation: Deep link identifier contains path traversal or invalid characters: '{}'", id));
    }
    if id.starts_with('.') || id.starts_with('-') {
        return Err(format!(
            "Security violation: Deep link identifier cannot start with '.' or '-': '{}'",
            id
        ));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(format!(
            "Security violation: Deep link identifier contains prohibited characters: '{}'",
            id
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn handle_deeplink(url: String) -> Result<DeepLinkAction, String> {
    parse_deeplink_url(&url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_package_deeplink() {
        let res = parse_deeplink_url("ryzora://package/fastfetch-catppuccin-mocha").unwrap();
        assert_eq!(
            res,
            DeepLinkAction::ViewPackage {
                package_id: "fastfetch-catppuccin-mocha".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_valid_repo_deeplink() {
        let res = parse_deeplink_url("ryzora://repository/official-curated").unwrap();
        assert_eq!(
            res,
            DeepLinkAction::ViewRepository {
                repo_id: "official-curated".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_valid_creator_deeplink() {
        let res = parse_deeplink_url("ryzora://creator/catppuccin-dev").unwrap();
        assert_eq!(
            res,
            DeepLinkAction::ViewCreator {
                creator_id: "catppuccin-dev".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_valid_category_deeplink() {
        let res = parse_deeplink_url("ryzora://category/wallpapers").unwrap();
        assert_eq!(
            res,
            DeepLinkAction::ViewCategory {
                category: "wallpapers".to_string(),
            }
        );
    }

    #[test]
    fn test_rejects_path_traversal_in_deeplink() {
        let res = parse_deeplink_url("ryzora://package/../../etc/passwd");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("path traversal"));
    }

    #[test]
    fn test_rejects_dangerous_entities_in_deeplink() {
        assert!(parse_deeplink_url("ryzora://exec/rm").is_err());
        assert!(parse_deeplink_url("ryzora://file/home").is_err());
        assert!(parse_deeplink_url("ryzora://shell/bash").is_err());
    }

    #[test]
    fn test_rejects_invalid_scheme() {
        assert!(parse_deeplink_url("https://package/test").is_err());
        assert!(parse_deeplink_url("file:///package/test").is_err());
    }

    #[test]
    fn test_deep_link_security_vector_matrix() {
        // 1. valid-id -> accepted
        let res = parse_deeplink_url("ryzora://package/valid-id");
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            DeepLinkAction::ViewPackage {
                package_id: "valid-id".to_string()
            }
        );

        // 2. ../secret -> rejected
        assert!(parse_deeplink_url("ryzora://package/../secret").is_err());

        // 3. %2e%2e/secret -> rejected
        assert!(parse_deeplink_url("ryzora://package/%2e%2e/secret").is_err());

        // 4. exec -> rejected
        assert!(parse_deeplink_url("ryzora://exec/arbitrary-cmd").is_err());

        // 5. shell -> rejected
        assert!(parse_deeplink_url("ryzora://shell/bash").is_err());

        // 6. file -> rejected
        assert!(parse_deeplink_url("ryzora://file/etc/shadow").is_err());

        // 7. null byte -> rejected
        assert!(parse_deeplink_url("ryzora://package/test\0evil").is_err());

        // 8. oversized-id -> rejected (len > 128)
        let oversized = "a".repeat(129);
        assert!(parse_deeplink_url(&format!("ryzora://package/{}", oversized)).is_err());
    }
}
