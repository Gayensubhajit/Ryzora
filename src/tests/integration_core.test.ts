import test, { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { IntegrationManager } from '../services/integration/IntegrationManager.ts';
import type {
  IntegrationManifest,
  ArtifactVerificationResult,
  DisableReport,
} from '../services/integration/IntegrationTypes.ts';

describe('Phase 1 — Ryzora Reversible Integration Core Contract', () => {
  beforeEach(() => {
    IntegrationManager.setInvoker(null);
  });

  afterEach(() => {
    IntegrationManager.setInvoker(null);
  });

  it('Contract 1: Refuses IPC in non-Tauri environment when unmocked', async () => {
    await assert.rejects(
      async () => {
        await IntegrationManager.list();
      },
      /Tauri IPC is not available in non-Tauri environments/
    );
  });

  it('Contract 2: IntegrationManager.list() invokes integration_list without extra parameters', async () => {
    let invokedCmd = '';
    let invokedArgs: any = null;

    IntegrationManager.setInvoker(async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
      invokedCmd = cmd;
      invokedArgs = args;
      const mockManifests: IntegrationManifest[] = [
        {
          feature_id: 'session-lock',
          integration_id: 'hyprlock-shim',
          version: 1,
          enabled: true,
          artifacts: [
            {
              path: '/home/user/.local/bin/hyprlock',
              artifact_type: 'wrapper_shim',
              policy: 'immutable',
              sha256: 'abc123hash',
              marker: '# Ryzora-Managed-Integration: hyprlock-shim',
              created_at: 1726000000,
              permissions: 0o755,
            },
          ],
          created_directories: [],
          metadata: { compositor: 'hyprland' },
          updated_at: 1726000000,
        },
      ];
      return mockManifests as unknown as T;
    });

    const list = await IntegrationManager.list();
    assert.strictEqual(invokedCmd, 'integration_list');
    assert.deepStrictEqual(invokedArgs, {});
    assert.strictEqual(list.length, 1);
    assert.strictEqual(list[0].feature_id, 'session-lock');
    assert.strictEqual(list[0].artifacts[0].policy, 'immutable');
  });

  it('Contract 3: IntegrationManager.get(id) supplies typed integrationId parameter', async () => {
    let invokedCmd = '';
    let invokedArgs: any = null;

    IntegrationManager.setInvoker(async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
      invokedCmd = cmd;
      invokedArgs = args;
      if (args?.integrationId === 'idle-management') {
        const manifest: IntegrationManifest = {
          feature_id: 'idle-management',
          integration_id: 'idle-management',
          version: 1,
          enabled: true,
          artifacts: [],
          created_directories: [],
          metadata: {},
          updated_at: 1726000000,
        };
        return manifest as unknown as T;
      }
      return null as unknown as T;
    });

    const found = await IntegrationManager.get('idle-management');
    assert.strictEqual(invokedCmd, 'integration_get');
    assert.deepStrictEqual(invokedArgs, { integrationId: 'idle-management' });
    assert.ok(found !== null);
    assert.strictEqual(found?.feature_id, 'idle-management');

    const missing = await IntegrationManager.get('non-existent');
    assert.strictEqual(missing, null);
  });

  it('Contract 4: IntegrationManager.verify(id) validates ownership status responses', async () => {
    let invokedCmd = '';
    let invokedArgs: any = null;

    IntegrationManager.setInvoker(async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
      invokedCmd = cmd;
      invokedArgs = args;
      const results: ArtifactVerificationResult[] = [
        {
          path: '/home/user/.config/systemd/user/hypridle.service.d/override.conf',
          status: 'verified_ryzora_owned',
        },
        {
          path: '/home/user/.config/ryzora/user-tuned.conf',
          status: 'modified_by_user',
          details: 'Hash mismatch: user edited content',
        },
      ];
      return results as unknown as T;
    });

    const res = await IntegrationManager.verify('session-lock');
    assert.strictEqual(invokedCmd, 'integration_verify');
    assert.deepStrictEqual(invokedArgs, { integrationId: 'session-lock' });
    assert.strictEqual(res.length, 2);
    assert.strictEqual(res[0].status, 'verified_ryzora_owned');
    assert.strictEqual(res[1].status, 'modified_by_user');
  });

  it('Contract 5: IntegrationManager.disable(id) passes ONLY integrationId, never arbitrary file paths', async () => {
    let invokedCmd = '';
    let invokedArgs: any = null;

    IntegrationManager.setInvoker(async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
      invokedCmd = cmd;
      invokedArgs = args;
      const report: DisableReport = {
        integration_id: (args?.integrationId as string) || '',
        removed_artifacts: ['/home/user/.config/systemd/user/hypridle.service.d/override.conf'],
        preserved_artifacts: ['/home/user/.config/ryzora/user-tuned.conf'],
        missing_artifacts: [],
        removed_directories: ['/home/user/.config/systemd/user/hypridle.service.d'],
        preserved_directories: [],
        errors: [],
        fully_reverted: false, // false because user-edited file was preserved
      };
      return report as unknown as T;
    });

    const report = await IntegrationManager.disable('session-lock');
    assert.strictEqual(invokedCmd, 'integration_disable');
    // Safety verification: The arguments sent to the backend MUST contain ONLY integrationId!
    assert.deepStrictEqual(Object.keys(invokedArgs), ['integrationId']);
    assert.strictEqual(invokedArgs.integrationId, 'session-lock');
    assert.strictEqual(report.removed_artifacts.length, 1);
    assert.strictEqual(report.preserved_artifacts.length, 1);
    assert.strictEqual(report.fully_reverted, false);
  });

  it('Phase 2 - 1: IntegrationManager.getSessionLockStatus() dispatches typed session_lock_get_status', async () => {
    let invokedCmd = '';
    IntegrationManager.setInvoker(async <T>(cmd: string): Promise<T> => {
      invokedCmd = cmd;
      return {
        enabled: true,
        dropin_active: true,
        service_active: true,
        native_config_path: '/home/user/.config/hypr/hypridle.conf',
        overlay_config_path: '/home/user/.config/ryzora/session-lock/hypridle.conf',
        dropin_path: '/home/user/.config/systemd/user/hypridle.service.d/zz-ryzora-session-lock.conf',
        audit_native_config_hash: 'abc123audit',
      } as unknown as T;
    });

    const status = await IntegrationManager.getSessionLockStatus();
    assert.strictEqual(invokedCmd, 'session_lock_get_status');
    assert.strictEqual(status.enabled, true);
    assert.strictEqual(status.dropin_active, true);
    assert.strictEqual(status.service_active, true);
  });

  it('Phase 2 - 2: IntegrationManager.enableSessionLock() dispatches session_lock_enable without arbitrary paths', async () => {
    let invokedCmd = '';
    let invokedArgs: any = null;
    IntegrationManager.setInvoker(async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
      invokedCmd = cmd;
      invokedArgs = args;
      return {
        enabled: true,
        dropin_active: true,
        service_active: true,
        native_config_path: '/home/user/.config/hypr/hypridle.conf',
        overlay_config_path: '/home/user/.config/ryzora/session-lock/hypridle.conf',
        dropin_path: '/home/user/.config/systemd/user/hypridle.service.d/zz-ryzora-session-lock.conf',
      } as unknown as T;
    });

    const res = await IntegrationManager.enableSessionLock();
    assert.strictEqual(invokedCmd, 'session_lock_enable');
    assert.deepStrictEqual(invokedArgs, {});
    assert.strictEqual(res.enabled, true);
  });

  it('Phase 2 - 3: IntegrationManager.disableSessionLock() dispatches session_lock_disable without arbitrary paths', async () => {
    let invokedCmd = '';
    let invokedArgs: any = null;
    IntegrationManager.setInvoker(async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
      invokedCmd = cmd;
      invokedArgs = args;
      return {
        integration_id: 'hyprland-session-lock',
        removed_artifacts: [
          '/home/user/.config/ryzora/session-lock/hypridle.conf',
          '/home/user/.config/systemd/user/hypridle.service.d/zz-ryzora-session-lock.conf',
        ],
        preserved_artifacts: [],
        missing_artifacts: [],
        removed_directories: ['/home/user/.config/ryzora/session-lock'],
        preserved_directories: [],
        errors: [],
        fully_reverted: true,
      } as unknown as T;
    });

    const rep = await IntegrationManager.disableSessionLock();
    assert.strictEqual(invokedCmd, 'session_lock_disable');
    assert.deepStrictEqual(invokedArgs, {});
    assert.strictEqual(rep.fully_reverted, true);
    assert.strictEqual(rep.removed_artifacts.length, 2);
  });
});
