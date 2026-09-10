import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { HostCapabilities, LockscreenTargetCapability } from '../types';

test('HostCapabilities data contract & adapter intersection', () => {
  const mockCapabilities: HostCapabilities = {
    os: 'Arch Linux',
    distro_id: 'arch',
    distro_name: 'Arch Linux',
    desktop_environment: 'Hyprland',
    compositor: 'Hyprland',
    compositor_version: '0.55.x',
    session_type: 'wayland',
    session_lock_protocol: 'ext-session-lock-v1',
    display_manager: 'sddm',
    display_manager_service: 'sddm.service',
    display_manager_theme: 'winter',
    active_lockscreen: {
      session_lock_type: 'hyprlock',
      session_lock_name: '~/.config/hypr/hyprlock_themes/006_stacked_clock/hyprlock.conf',
      session_lock_config: '/home/silentbyte/.config/hypr/hypridle.conf',
      login_screen_type: 'sddm',
      login_screen_theme: 'winter',
      login_screen_config: 'sddm.service',
      managed_by: 'Dusky',
    },
    installed_commands: {
      quickshell: true,
      hyprlock: true,
      swaylock: false,
      sddm: true,
      pkexec: true,
    },
    supported_adapters: [
      {
        adapter: 'quickshell',
        name: 'Quickshell Session Lock',
        category: 'session_lock',
        supported: true,
        reason: 'Fully compatible with your Wayland compositor and ext-session-lock-v1',
        required_privilege: 'user',
        runtime_binary: 'quickshell',
        binary_installed: true,
        protocol: 'ext-session-lock-v1',
      },
      {
        adapter: 'sddm',
        name: 'SDDM Login Screen',
        category: 'login_screen',
        supported: true,
        reason: 'SDDM is your active system display manager',
        required_privilege: 'administrator',
        runtime_binary: 'sddm',
        binary_installed: true,
        protocol: 'sddm-greeter',
      },
    ],
  };

  assert.equal(mockCapabilities.os, 'Arch Linux');
  assert.equal(mockCapabilities.session_lock_protocol, 'ext-session-lock-v1');
  assert.equal(mockCapabilities.active_lockscreen.managed_by, 'Dusky');

  const qs = mockCapabilities.supported_adapters.find(a => a.adapter === 'quickshell');
  assert.ok(qs);
  assert.equal(qs?.supported, true);
  assert.equal(qs?.required_privilege, 'user');

  const sddm = mockCapabilities.supported_adapters.find(a => a.adapter === 'sddm');
  assert.ok(sddm);
  assert.equal(sddm?.supported, true);
  assert.equal(sddm?.required_privilege, 'administrator');
});
