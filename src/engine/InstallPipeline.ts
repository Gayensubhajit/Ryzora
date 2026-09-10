/**
 * Ryzora Install Pipeline — Phase 22
 *
 * Typed TypeScript wrapper for the Rust engine's transactional install/uninstall
 * pipeline commands. Provides stage-progress callbacks so UI components can
 * display real-time pipeline progress without knowing anything about the
 * underlying Rust implementation.
 *
 * All actual work is done in Rust. This module is purely a typed bridge.
 */

import { invoke } from "@tauri-apps/api/core";

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline stage types
// ─────────────────────────────────────────────────────────────────────────────

/** The nine canonical pipeline stages, matching engine.rs PipelineStage. */
export type PipelineStageId =
  | "resolve"
  | "validate"
  | "check_compat"
  | "resolve_deps"
  | "download"
  | "verify"
  | "stage"
  | "materialize"
  | "integrate";

export interface PipelineStageInfo {
  id: PipelineStageId;
  label: string;
}

export interface PipelineProgress {
  stage: PipelineStageId;
  status: "pending" | "running" | "done" | "failed";
  message?: string;
}

export interface PipelineStageResult {
  stage: string;
  success: boolean;
  message: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Install result types
// ─────────────────────────────────────────────────────────────────────────────

export interface PipelineInstallResult {
  success: boolean;
  package_id: string;
  version: string;
  snapshot_id: string;
  installed_files: string[];
  ownership_claimed: string[];
  stage_results: PipelineStageResult[];
  errors: string[];
  rolled_back: boolean;
}

export interface PipelineUninstallResult {
  success: boolean;
  package_id: string;
  removed_files: string[];
  ownership_released: string[];
  stage_results: PipelineStageResult[];
  errors: string[];
  rolled_back: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Stage info (from Rust)
// ─────────────────────────────────────────────────────────────────────────────

let _cachedStages: PipelineStageInfo[] | null = null;

/** Returns the ordered list of pipeline stage definitions from the Rust backend. */
export async function getPipelineStages(): Promise<PipelineStageInfo[]> {
  if (_cachedStages) return _cachedStages;
  const stages = await invoke<PipelineStageInfo[]>("get_pipeline_stages");
  _cachedStages = stages;
  return stages;
}

// ─────────────────────────────────────────────────────────────────────────────
// Ownership ledger queries
// ─────────────────────────────────────────────────────────────────────────────

export interface OwnedFileInfo {
  package_id: string;
  installed_at: number;
  sha256: string;
}

export interface ConflictReport {
  path: string;
  owned_by: string;
  requested_by: string;
}

/** Returns the owner record for a target path, or null if unowned. */
export async function queryFileOwner(target: string): Promise<OwnedFileInfo | null> {
  return invoke<OwnedFileInfo | null>("query_file_owner", { target });
}

/** Returns all file paths owned by a package. */
export async function listOwnedFiles(packageId: string): Promise<string[]> {
  return invoke<string[]>("list_owned_files", { packageId });
}

/** Pre-flight check: returns any ownership conflicts for the given package + targets. */
export async function checkOwnershipConflicts(
  packageId: string,
  targets: string[],
): Promise<ConflictReport[]> {
  return invoke<ConflictReport[]>("check_ownership_conflicts", { packageId, targets });
}

// ─────────────────────────────────────────────────────────────────────────────
// Install pipeline
// ─────────────────────────────────────────────────────────────────────────────

export interface InstallPipelineOptions {
  target?: string;
  force?: boolean;
  /** Callback called when each stage completes. */
  onProgress?: (progress: PipelineProgress) => void;
}

/**
 * Runs the full install pipeline for a package through the Rust engine.
 *
 * Stage progress is synthesised from the `stage_results` array returned
 * by the backend since Tauri commands are currently request/response only.
 * When Tauri events are available, this can be upgraded to real-time streaming.
 */
export async function runInstallPipeline(
  packageId: string,
  options: InstallPipelineOptions = {},
): Promise<PipelineInstallResult> {
  const { target, force = false, onProgress } = options;

  // Emit a synthetic "running" event for the first visible stage
  onProgress?.({ stage: "resolve", status: "running" });

  const result = await invoke<PipelineInstallResult>("engine_install_package", {
    packageId,
    target: target ?? null,
    force,
  });

  // Replay stage results as progress events
  if (onProgress) {
    for (const sr of result.stage_results) {
      const stageId = normalizeStageId(sr.stage);
      if (stageId) {
        onProgress({
          stage: stageId,
          status: sr.success ? "done" : "failed",
          message: sr.message,
        });
      }
    }
  }

  return result;
}

// ─────────────────────────────────────────────────────────────────────────────
// Uninstall pipeline
// ─────────────────────────────────────────────────────────────────────────────

export interface UninstallPipelineOptions {
  onProgress?: (progress: PipelineProgress) => void;
}

/**
 * Runs the full uninstall pipeline for a package through the Rust engine.
 */
export async function runUninstallPipeline(
  packageId: string,
  options: UninstallPipelineOptions = {},
): Promise<PipelineUninstallResult> {
  const { onProgress } = options;

  onProgress?.({ stage: "resolve", status: "running" });

  const result = await invoke<PipelineUninstallResult>("engine_uninstall_package", {
    packageId,
  });

  if (onProgress) {
    for (const sr of result.stage_results) {
      const stageId = normalizeStageId(sr.stage);
      if (stageId) {
        onProgress({
          stage: stageId,
          status: sr.success ? "done" : "failed",
          message: sr.message,
        });
      }
    }
  }

  return result;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function normalizeStageId(raw: string): PipelineStageId | null {
  const map: Record<string, PipelineStageId> = {
    resolve: "resolve",
    validate: "validate",
    checkcompat: "check_compat",
    check_compat: "check_compat",
    resolvedeps: "resolve_deps",
    resolve_deps: "resolve_deps",
    download: "download",
    verify: "verify",
    stage: "stage",
    materialize: "materialize",
    integrate: "integrate",
  };
  return map[raw.toLowerCase().replace(/\s+/g, "_")] ?? null;
}
