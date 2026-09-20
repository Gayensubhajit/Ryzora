/**
 * Ryzora Reversible Integration Core — Frontend Types
 *
 * Reflects the backend contracts in src-tauri/src/integration/types.rs
 */

export type ArtifactType =
  | 'systemd_dropin'
  | 'wrapper_shim'
  | 'config_overlay'
  | 'environment_file'
  | 'session_script';

export type ArtifactOwnershipPolicy =
  | 'immutable'
  | 'ryzora_managed'
  | 'user_editable';

export interface IntegrationArtifact {
  path: string;
  artifact_type: ArtifactType;
  policy: ArtifactOwnershipPolicy;
  sha256: string;
  marker: string;
  created_at: number;
  permissions?: number;
}

export interface IntegrationManifest {
  feature_id: string;
  integration_id: string;
  version: number;
  enabled: boolean;
  artifacts: IntegrationArtifact[];
  created_directories: string[];
  metadata: Record<string, string>;
  updated_at: number;
}

export type ArtifactVerificationStatus =
  | 'verified_ryzora_owned'
  | 'modified_by_user'
  | 'missing'
  | 'foreign_or_unowned'
  | 'symlink_target_untrusted';

export interface ArtifactVerificationResult {
  path: string;
  status: ArtifactVerificationStatus;
  details?: string;
}

export interface DisableReport {
  integration_id: string;
  removed_artifacts: string[];
  preserved_artifacts: string[];
  missing_artifacts: string[];
  removed_directories: string[];
  preserved_directories: string[];
  errors: string[];
  fully_reverted: boolean;
}

export interface SessionLockStatus {
  enabled: boolean;
  dropin_active: boolean;
  service_active: boolean;
  native_config_path: string;
  overlay_config_path: string;
  dropin_path: string;
  audit_native_config_hash?: string;
}
