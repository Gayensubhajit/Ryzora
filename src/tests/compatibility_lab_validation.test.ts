import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  resolveLockscreenCapabilities,
  RAW_QYLOCK_THEMES,
  normalizeQylockTheme,
} from '../providers/qylockProvider.ts';
import type { SystemInfo, HostProfileFixture } from '../types/index.ts';

// Get a representative configurable theme (Clockwork Orbital) and static theme (Dog Samurai)
const orbitalTheme = normalizeQylockTheme(
  RAW_QYLOCK_THEMES.find((t) => t.id === 'clockwork-orbital') || RAW_QYLOCK_THEMES[0]
);
const dogSamuraiTheme = normalizeQylockTheme(
  RAW_QYLOCK_THEMES.find((t) => t.id === 'dog-samurai') || RAW_QYLOCK_THEMES[1]
);

// ============================================================================
// Phase 21 Compatibility Lab: Capability Resolver Invariants
// ============================================================================

test('Lab 1: Simulated Ubuntu 24.04 (GNOME / Mutter / GDM) rejects both Quickshell and SDDM', () => {
  const simulatedUbuntu: SystemInfo = {
    os: 'Ubuntu 24.04 LTS',
    kernel: '6.8.0-generic',
    desktop_environment: 'GNOME',
    window_manager: 'Mutter',
    session_type: 'wayland',
    display_manager: 'gdm',
  };

  const result = resolveLockscreenCapabilities(simulatedUbuntu, orbitalTheme.lockscreen, 'both');

  // Both must be rejected
  assert.strictEqual(result.session_lock_supported, false, 'Quickshell must be unsupported on GNOME/Mutter');
  assert.strictEqual(result.login_screen_supported, false, 'SDDM must be unsupported on GDM');
  assert.strictEqual(result.can_install, false, 'Cannot install on Ubuntu GNOME/GDM');

  // Detailed evaluations breakdown
  const evals = result.evaluations;
  assert.ok(evals, 'Detailed evaluations must be populated');
  assert.ok(evals.quickshell, 'Quickshell evaluation must exist');
  assert.ok(evals.sddm, 'SDDM evaluation must exist');

  // Quickshell checks
  assert.strictEqual(evals.quickshell.supported, false);
  assert.strictEqual(evals.quickshell.package_provided, true);
  assert.strictEqual(evals.quickshell.checks.compositor_protocol.passed, false);
  assert.strictEqual(evals.quickshell.checks.compositor_protocol.detected, 'compositor-native (Mutter)');
  assert.ok(evals.quickshell.reason.includes('Mutter'));
  assert.strictEqual(evals.quickshell.system_changes_made, false);

  // SDDM checks
  assert.strictEqual(evals.sddm.supported, false);
  assert.strictEqual(evals.sddm.package_provided, true);
  assert.strictEqual(evals.sddm.checks.display_manager?.passed, false);
  assert.strictEqual(evals.sddm.checks.display_manager?.detected, 'gdm');
  assert.ok(evals.sddm.reason.includes('GDM'));
  assert.strictEqual(evals.sddm.system_changes_made, false);
});

test('Lab 2: Simulated Fedora 40 (KDE Plasma 6 / KWin / SDDM) allows SDDM but rejects Quickshell', () => {
  const simulatedFedoraKde: SystemInfo = {
    os: 'Fedora Linux 40',
    kernel: '6.9.0',
    desktop_environment: 'KDE Plasma',
    window_manager: 'KWin',
    session_type: 'wayland',
    display_manager: 'sddm',
  };

  const result = resolveLockscreenCapabilities(simulatedFedoraKde, orbitalTheme.lockscreen, 'both');

  assert.strictEqual(result.session_lock_supported, false, 'Quickshell must be unsupported on KDE/KWin');
  assert.strictEqual(result.login_screen_supported, true, 'SDDM must be supported when active DM is SDDM');

  const evals = result.evaluations;
  assert.ok(evals?.quickshell);
  assert.ok(evals?.sddm);

  // Quickshell check
  assert.strictEqual(evals.quickshell.supported, false);
  assert.strictEqual(evals.quickshell.checks.compositor_protocol.passed, false);
  assert.strictEqual(evals.quickshell.checks.compositor_protocol.detected, 'compositor-native (KWin)');
  assert.ok(evals.quickshell.reason.includes('KWin'));

  // SDDM check
  assert.strictEqual(evals.sddm.supported, true);
  assert.strictEqual(evals.sddm.checks.display_manager?.passed, true);
  assert.strictEqual(evals.sddm.checks.privilege_boundary.required, 'administrator');
});

test('Lab 3: Simulated Arch Linux (Hyprland / SDDM) allows both Quickshell and SDDM', () => {
  const simulatedArchHypr: SystemInfo = {
    os: 'Arch Linux',
    kernel: '6.10.0-arch1-1',
    desktop_environment: 'Hyprland',
    window_manager: 'Hyprland',
    session_type: 'wayland',
    display_manager: 'sddm',
  };

  const result = resolveLockscreenCapabilities(simulatedArchHypr, orbitalTheme.lockscreen, 'both');

  assert.strictEqual(result.session_lock_supported, true, 'Quickshell must be supported on Hyprland');
  assert.strictEqual(result.login_screen_supported, true, 'SDDM must be supported on SDDM');
  assert.strictEqual(result.can_install, true);

  const evals = result.evaluations;
  assert.ok(evals?.quickshell);
  assert.ok(evals?.sddm);

  assert.strictEqual(evals.quickshell.supported, true);
  assert.strictEqual(evals.quickshell.checks.compositor_protocol.passed, true);
  assert.strictEqual(evals.quickshell.checks.compositor_protocol.detected, 'ext-session-lock-v1');

  assert.strictEqual(evals.sddm.supported, true);
  assert.strictEqual(evals.sddm.checks.display_manager?.passed, true);
});

test('Lab 4: Simulated Arch Linux (Sway / GDM) allows Quickshell but rejects SDDM', () => {
  const simulatedArchSway: SystemInfo = {
    os: 'Arch Linux',
    kernel: '6.10.0-arch1-1',
    desktop_environment: 'Sway',
    window_manager: 'Sway',
    session_type: 'wayland',
    display_manager: 'gdm',
  };

  const result = resolveLockscreenCapabilities(simulatedArchSway, orbitalTheme.lockscreen, 'both');

  // Quickshell is supported on Sway because Sway implements ext-session-lock-v1
  assert.strictEqual(result.session_lock_supported, true, 'Quickshell supported on Sway');
  // SDDM is rejected because GDM is active
  assert.strictEqual(result.login_screen_supported, false, 'SDDM rejected on GDM');

  const evals = result.evaluations;
  assert.ok(evals?.quickshell);
  assert.ok(evals?.sddm);

  assert.strictEqual(evals.quickshell.supported, true);
  assert.strictEqual(evals.sddm.supported, false);
  assert.ok(evals.sddm.reason.includes('GDM'));
});

test('Lab 5: Simulated Debian 12 (X11 / LightDM / XFCE) rejects Quickshell and SDDM', () => {
  const simulatedDebian: SystemInfo = {
    os: 'Debian GNU/Linux 12 (bookworm)',
    kernel: '6.1.0-21-amd64',
    desktop_environment: 'XFCE',
    window_manager: 'xfwm4',
    session_type: 'x11',
    display_manager: 'lightdm',
  };

  const result = resolveLockscreenCapabilities(simulatedDebian, orbitalTheme.lockscreen, 'both');

  assert.strictEqual(result.session_lock_supported, false, 'Quickshell requires Wayland session');
  assert.strictEqual(result.login_screen_supported, false, 'SDDM not supported on LightDM');

  const evals = result.evaluations;
  assert.ok(evals?.quickshell);
  assert.ok(evals?.sddm);

  assert.strictEqual(evals.quickshell.supported, false);
  assert.ok(evals.quickshell.reason.includes('Wayland'));

  assert.strictEqual(evals.sddm.supported, false);
  assert.ok(evals.sddm.reason.includes('LightDM'));
});

test('Lab 6: Full Compatibility Matrix Across All Discovered Qylock Themes', () => {
  const testSystems: { name: string; info: SystemInfo; expectQs: boolean; expectSddm: boolean }[] = [
    {
      name: 'Ubuntu GNOME/GDM',
      info: { os: 'Ubuntu', desktop_environment: 'GNOME', window_manager: 'Mutter', session_type: 'wayland', display_manager: 'gdm' },
      expectQs: false,
      expectSddm: false,
    },
    {
      name: 'Fedora KDE/SDDM',
      info: { os: 'Fedora', desktop_environment: 'KDE Plasma', window_manager: 'KWin', session_type: 'wayland', display_manager: 'sddm' },
      expectQs: false,
      expectSddm: true,
    },
    {
      name: 'Arch Hyprland/SDDM',
      info: { os: 'Arch Linux', desktop_environment: 'Hyprland', window_manager: 'Hyprland', session_type: 'wayland', display_manager: 'sddm' },
      expectQs: true,
      expectSddm: true,
    },
    {
      name: 'Debian X11/LightDM',
      info: { os: 'Debian', desktop_environment: 'XFCE', window_manager: 'xfwm4', session_type: 'x11', display_manager: 'lightdm' },
      expectQs: false,
      expectSddm: false,
    },
  ];

  for (const rawTheme of RAW_QYLOCK_THEMES) {
    const pkg = normalizeQylockTheme(rawTheme);
    for (const sys of testSystems) {
      const res = resolveLockscreenCapabilities(sys.info, pkg.lockscreen, 'both');
      assert.strictEqual(
        res.session_lock_supported,
        sys.expectQs,
        `${pkg.id} on ${sys.name}: session_lock_supported mismatch`
      );
      assert.strictEqual(
        res.login_screen_supported,
        sys.expectSddm,
        `${pkg.id} on ${sys.name}: login_screen_supported mismatch`
      );
      // Ensure zero side effects guaranteed
      assert.strictEqual(res.evaluations?.quickshell?.system_changes_made, false);
      assert.strictEqual(res.evaluations?.sddm?.system_changes_made, false);
    }
  }
});
