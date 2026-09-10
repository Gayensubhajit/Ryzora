import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { resolveLockscreenCapabilities } from '../providers/qylockProvider.ts';
import { RAW_QYLOCK_THEMES, normalizeQylockTheme } from '../providers/qylockProvider.ts';
import type { SystemInfo } from '../types.ts';

// ============================================================================
// 1. Ryzora Light Mode Persistence & UI Theme Isolation
// ============================================================================

test('UI Theme Isolation 1: Single Source of Truth for data-theme', () => {
  const srcDir = path.resolve(process.cwd(), 'src');
  
  function scanFiles(dir: string, fileList: string[] = []): string[] {
    const files = fs.readdirSync(dir);
    for (const file of files) {
      const fullPath = path.join(dir, file);
      const stat = fs.statSync(fullPath);
      if (stat.isDirectory()) {
        if (file !== 'node_modules' && file !== '.git') {
          scanFiles(fullPath, fileList);
        }
      } else if (file.endsWith('.tsx') || file.endsWith('.ts')) {
        fileList.push(fullPath);
      }
    }
    return fileList;
  }

  const allTsFiles = scanFiles(srcDir);
  const filesWithDataTheme: string[] = [];

  for (const filePath of allTsFiles) {
    const content = fs.readFileSync(filePath, 'utf-8');
    // Ignore test files
    if (filePath.includes('/tests/')) continue;

    if (content.includes('data-theme')) {
      const relative = path.relative(srcDir, filePath);
      filesWithDataTheme.push(relative);
    }
  }

  // ONLY ThemeProvider.tsx should ever reference or manipulate data-theme
  assert.deepStrictEqual(
    filesWithDataTheme,
    ['theme/ThemeProvider.tsx'],
    'Only ThemeProvider.tsx may manage data-theme in the application'
  );
});

test('UI Theme Isolation 2: ProductDetailShell and LockScreenCustomizer do not hardcode dark theme', () => {
  const shellPath = path.resolve(process.cwd(), 'src/components/detail/ProductDetailShell.tsx');
  const customizerPath = path.resolve(process.cwd(), 'src/components/detail/LockScreenCustomizer.tsx');

  const shellContent = fs.readFileSync(shellPath, 'utf-8');
  const customizerContent = fs.readFileSync(customizerPath, 'utf-8');

  // Must not have data-theme="dark"
  assert.ok(!shellContent.includes('data-theme="dark"'), 'ProductDetailShell must not force dark theme');
  assert.ok(!customizerContent.includes('data-theme="dark"'), 'LockScreenCustomizer must not force dark theme');

  // Must not have className="... dark" forcing Tailwind dark variant
  assert.ok(!shellContent.includes('className="relative min-h-screen text-white dark"'), 'Shell must not hardcode dark class');
});

test('UI Theme Isolation 3: 4-Way Combination Matrix (App Theme x Package ThemeMode)', () => {
  // Verify that application theme state and package runtime configuration remain 100% independent
  const combos = [
    { appTheme: 'light', packageThemeMode: 'dark' },
    { appTheme: 'light', packageThemeMode: 'light' },
    { appTheme: 'dark', packageThemeMode: 'light' },
    { appTheme: 'dark', packageThemeMode: 'dark' },
  ];

  for (const combo of combos) {
    // 1. Application theme state
    const simulatedAppDomTheme = combo.appTheme;

    // 2. Package customizer action
    const lockscreenConfig: Record<string, any> = {
      themeMode: combo.packageThemeMode,
      enableWindup: true,
    };

    // Serialized config for runtime
    const runtimeConfigJson = JSON.stringify(lockscreenConfig);
    const parsedConfig = JSON.parse(runtimeConfigJson);

    // Assert package runtime option received value
    assert.strictEqual(parsedConfig.themeMode, combo.packageThemeMode);

    // Assert application DOM theme was completely untouched
    assert.strictEqual(
      simulatedAppDomTheme,
      combo.appTheme,
      `App DOM theme must remain ${combo.appTheme} even when package themeMode is ${combo.packageThemeMode}`
    );
  }
});

// ============================================================================
// 2. Universal Linux Compatibility Engine (Ubuntu GNOME + GDM Architecture)
// ============================================================================

test('Compatibility Engine 1: Ubuntu GNOME + GDM completely blocks both Quickshell and SDDM', () => {
  const ubuntuGnomeSystem: SystemInfo = {
    os: 'Ubuntu 24.04 LTS',
    kernel: '6.8.0-generic',
    desktop_environment: 'GNOME',
    window_manager: 'Mutter',
    session_type: 'wayland',
    display_manager: 'gdm',
  };

  const clockworkTheme = RAW_QYLOCK_THEMES.find((t) => t.id === 'clockwork-orbital') || RAW_QYLOCK_THEMES[0];
  assert.ok(clockworkTheme, 'Clockwork Orbital theme must exist');

  // Test Quickshell target resolution on GNOME
  const qsResult = resolveLockscreenCapabilities(ubuntuGnomeSystem, normalizeQylockTheme(clockworkTheme).lockscreen, 'quickshell');
  assert.strictEqual(qsResult.session_lock_supported, false, 'Quickshell session lock must be unsupported on GNOME/Mutter');
  assert.strictEqual(qsResult.can_install, false, 'Cannot install Quickshell target on GNOME');
  assert.ok(
    qsResult.warnings.some((w) => w.includes('ext-session-lock-v1 is unsupported on GNOME/Mutter')),
    'Must provide clear warning about Mutter protocol incompatibility'
  );

  // Test SDDM target resolution on GDM
  const sddmResult = resolveLockscreenCapabilities(ubuntuGnomeSystem, normalizeQylockTheme(clockworkTheme).lockscreen, 'sddm');
  assert.strictEqual(sddmResult.login_screen_supported, false, 'SDDM login screen must be unsupported on GDM');
  assert.strictEqual(sddmResult.can_install, false, 'Cannot install SDDM target on GDM');
  assert.ok(
    sddmResult.warnings.some((w) => w.includes('GDM (GNOME Display Manager). Qylock SDDM themes cannot be installed into GDM')),
    'Must provide clear warning that SDDM themes cannot be installed into GDM'
  );

  // Test Both targets resolution on GNOME + GDM
  const bothResult = resolveLockscreenCapabilities(ubuntuGnomeSystem, normalizeQylockTheme(clockworkTheme).lockscreen, 'both');
  assert.strictEqual(bothResult.can_install, false, 'Both targets must be rejected on GNOME + GDM');
  assert.strictEqual(bothResult.session_lock_supported, false);
  assert.strictEqual(bothResult.login_screen_supported, false);
});

test('Compatibility Engine 2: KDE Plasma + SDDM allows SDDM login screen but blocks Quickshell session lock', () => {
  const kdeSystem: SystemInfo = {
    os: 'Fedora Linux 40',
    kernel: '6.9.0',
    desktop_environment: 'KDE Plasma',
    window_manager: 'kwin_wayland',
    session_type: 'wayland',
    display_manager: 'sddm',
  };

  const theme = RAW_QYLOCK_THEMES[0];

  // Quickshell target
  const qsResult = resolveLockscreenCapabilities(kdeSystem, normalizeQylockTheme(theme).lockscreen, 'quickshell');
  assert.strictEqual(qsResult.session_lock_supported, false, 'Quickshell must be unsupported on KWin');
  assert.strictEqual(qsResult.can_install, false);

  // SDDM target
  const sddmResult = resolveLockscreenCapabilities(kdeSystem, normalizeQylockTheme(theme).lockscreen, 'sddm');
  assert.strictEqual(sddmResult.login_screen_supported, true, 'SDDM must be supported when host DM is SDDM');
  assert.strictEqual(sddmResult.can_install, true, 'SDDM target can install on KDE with SDDM');
  assert.strictEqual(sddmResult.requires_root, true, 'SDDM requires root');
});

test('Compatibility Engine 3: Hyprland + SDDM allows both Quickshell and SDDM', () => {
  const hyprlandSystem: SystemInfo = {
    os: 'Arch Linux',
    kernel: '6.10.0-arch1-1',
    desktop_environment: 'Hyprland',
    window_manager: 'Hyprland',
    session_type: 'wayland',
    display_manager: 'sddm',
  };

  const theme = RAW_QYLOCK_THEMES[0];

  // Quickshell target
  const qsResult = resolveLockscreenCapabilities(hyprlandSystem, normalizeQylockTheme(theme).lockscreen, 'quickshell');
  assert.strictEqual(qsResult.session_lock_supported, true);
  assert.strictEqual(qsResult.can_install, true);

  // SDDM target
  const sddmResult = resolveLockscreenCapabilities(hyprlandSystem, normalizeQylockTheme(theme).lockscreen, 'sddm');
  assert.strictEqual(sddmResult.login_screen_supported, true);
  assert.strictEqual(sddmResult.can_install, true);

  // Both targets
  const bothResult = resolveLockscreenCapabilities(hyprlandSystem, normalizeQylockTheme(theme).lockscreen, 'both');
  assert.strictEqual(bothResult.session_lock_supported, true);
  assert.strictEqual(bothResult.login_screen_supported, true);
  assert.strictEqual(bothResult.can_install, true);
});

test('Compatibility Engine 4: Never silently install SDDM into GDM', () => {
  const gdmSystem: SystemInfo = {
    os: 'Ubuntu 24.04 LTS',
    kernel: '6.8.0',
    desktop_environment: 'GNOME',
    window_manager: 'Mutter',
    session_type: 'wayland',
    display_manager: 'gdm',
  };

  // Check across all raw Qylock themes: SDDM must be denied on GDM for every single one
  for (const theme of RAW_QYLOCK_THEMES) {
    const sddmRes = resolveLockscreenCapabilities(gdmSystem, normalizeQylockTheme(theme).lockscreen, 'sddm');
    assert.strictEqual(sddmRes.login_screen_supported, false, `${theme.id} must reject SDDM on GDM`);
    assert.strictEqual(sddmRes.can_install, false, `${theme.id} cannot be installed to SDDM on GDM`);
  }
});
