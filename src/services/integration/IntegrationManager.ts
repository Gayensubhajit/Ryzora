import { invokeTauri } from '../tauri.ts';
import type {
  IntegrationManifest,
  ArtifactVerificationResult,
  DisableReport,
  SessionLockStatus,
} from './IntegrationTypes.ts';

type Invoker = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

/**
 * Ryzora Reversible Integration Manager
 *
 * Frontend service gateway to the backend Reversible Integration Subsystem.
 * Enforces the contract:
 * 1. Only feature/integration lifecycle requests are dispatched.
 * 2. No arbitrary filesystem paths are accepted from the frontend.
 * 3. All verification, ownership rules, and safe unlinks are enforced by the core engine.
 */
export class IntegrationManager {
  private static customInvoker: Invoker | null = null;

  /**
   * Sets a custom invoker for testing or headless execution.
   */
  static setInvoker(invoker: Invoker | null): void {
    IntegrationManager.customInvoker = invoker;
  }

  private static async invoke<T>(
    cmd: string,
    args: Record<string, unknown> = {}
  ): Promise<T> {
    if (IntegrationManager.customInvoker) {
      return IntegrationManager.customInvoker<T>(cmd, args);
    }
    return invokeTauri<T>(cmd, args);
  }

  /**
   * Lists all recorded Ryzora integration manifests on the system.
   */
  static async list(): Promise<IntegrationManifest[]> {
    return IntegrationManager.invoke<IntegrationManifest[]>('integration_list');
  }

  /**
   * Retrieves a specific integration manifest by its integration ID.
   */
  static async get(integrationId: string): Promise<IntegrationManifest | null> {
    return IntegrationManager.invoke<IntegrationManifest | null>(
      'integration_get',
      { integrationId }
    );
  }

  /**
   * Inspects and authoritatively verifies all artifacts tracked by an integration.
   */
  static async verify(
    integrationId: string
  ): Promise<ArtifactVerificationResult[]> {
    return IntegrationManager.invoke<ArtifactVerificationResult[]>(
      'integration_verify',
      { integrationId }
    );
  }

  /**
   * Disables an integration, returning the system to its native state.
   *
   * Invariant:
   * - Only the integrationId is provided.
   * - Ryzora removes ONLY verified Ryzora-owned artifacts.
   * - User-modified files are strictly preserved.
   * - Missing artifacts are handled idempotently.
   * - Only empty directories created by Ryzora are cleaned up.
   */
  static async disable(integrationId: string): Promise<DisableReport> {
    return IntegrationManager.invoke<DisableReport>('integration_disable', {
      integrationId,
    });
  }
  /**
   * Gets the live status of the Hyprland session lock integration.
   */
  static async getSessionLockStatus(): Promise<SessionLockStatus> {
    return IntegrationManager.invoke<SessionLockStatus>('session_lock_get_status');
  }

  /**
   * Reversibly enables the Hyprland session lock integration.
   */
  static async enableSessionLock(): Promise<SessionLockStatus> {
    return IntegrationManager.invoke<SessionLockStatus>('session_lock_enable');
  }

  /**
   * Reversibly disables the Hyprland session lock integration.
   */
  static async disableSessionLock(): Promise<DisableReport> {
    return IntegrationManager.invoke<DisableReport>('session_lock_disable');
  }
}
