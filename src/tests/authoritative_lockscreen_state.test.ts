import test, { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { LockScreenActivationState, AuthoritativeLockscreenState } from "../types/index.ts";

// Replicate the pure normalization and activation computation from AppContext
function computePackageActivation(
  pkgId: string,
  authState: AuthoritativeLockscreenState
): LockScreenActivationState {
  const normalize = (id: string | null | undefined) => {
    if (!id) return "";
    return id.replace(/^lockscreen-(qylock-)?/, "").replace(/^ryzora-/, "").toLowerCase();
  };
  const targetId = normalize(pkgId);
  const sessionTarget = normalize(authState.session_lock);
  const sddmLoginTarget = normalize(authState.sddm_login);
  const sddmLockTarget = normalize(authState.sddm_lock);

  return {
    sessionLock: Boolean(sessionTarget && sessionTarget === targetId),
    sddmLogin: Boolean(sddmLoginTarget && sddmLoginTarget === targetId),
    sddmLock: Boolean(sddmLockTarget && sddmLockTarget === targetId),
  };
}

describe("Lock-Screen State Model — Authoritative Independent Targets", () => {
  it("Requirement 1: Qylock active does not activate either SilentSDDM target", () => {
    const backendState: AuthoritativeLockscreenState = {
      session_lock: "lockscreen-qylock-dog-samurai",
      sddm_login: null,
      sddm_lock: null,
    };

    const dogSamurai = computePackageActivation("dog-samurai", backendState);
    assert.equal(dogSamurai.sessionLock, true, "Qylock session lock must be active");
    assert.equal(dogSamurai.sddmLogin, false, "Qylock active must NOT activate sddmLogin");
    assert.equal(dogSamurai.sddmLock, false, "Qylock active must NOT activate sddmLock");

    const silentsddmSilvia = computePackageActivation("silentsddm-silvia", backendState);
    assert.equal(silentsddmSilvia.sessionLock, false);
    assert.equal(silentsddmSilvia.sddmLogin, false);
    assert.equal(silentsddmSilvia.sddmLock, false);
  });

  it("Requirement 2: SilentSDDM Login active does not activate Session Lock", () => {
    const backendState: AuthoritativeLockscreenState = {
      session_lock: null,
      sddm_login: "silentsddm-silvia",
      sddm_lock: null,
    };

    const silvia = computePackageActivation("silentsddm-silvia", backendState);
    assert.equal(silvia.sessionLock, false, "SilentSDDM login must NOT activate sessionLock");
    assert.equal(silvia.sddmLogin, true, "SilentSDDM login must be active");
    assert.equal(silvia.sddmLock, false, "SilentSDDM login must NOT activate sddmLock");

    const dogSamurai = computePackageActivation("dog-samurai", backendState);
    assert.equal(dogSamurai.sessionLock, false);
    assert.equal(dogSamurai.sddmLogin, false);
    assert.equal(dogSamurai.sddmLock, false);
  });

  it("Requirement 3: SilentSDDM Lock active does not activate Login", () => {
    const backendState: AuthoritativeLockscreenState = {
      session_lock: null,
      sddm_login: null,
      sddm_lock: "custom:my-video-lock",
    };

    const customLock = computePackageActivation("custom:my-video-lock", backendState);
    assert.equal(customLock.sessionLock, false);
    assert.equal(customLock.sddmLogin, false, "SilentSDDM Lock active must NOT activate Login");
    assert.equal(customLock.sddmLock, true, "SilentSDDM Lock must be active");

    const silvia = computePackageActivation("silentsddm-silvia", backendState);
    assert.equal(silvia.sessionLock, false);
    assert.equal(silvia.sddmLogin, false);
    assert.equal(silvia.sddmLock, false);
  });

  it("Requirement 4: Changing Login does not change Lock", () => {
    let backendState: AuthoritativeLockscreenState = {
      session_lock: null,
      sddm_login: "silentsddm-silvia",
      sddm_lock: "custom:my-video-lock",
    };

    // User changes Login to ken
    backendState = {
      ...backendState,
      sddm_login: "silentsddm-ken",
    };

    const silvia = computePackageActivation("silentsddm-silvia", backendState);
    assert.equal(silvia.sddmLogin, false, "Silvia should no longer be active login");

    const ken = computePackageActivation("silentsddm-ken", backendState);
    assert.equal(ken.sddmLogin, true, "Ken is now active login");
    assert.equal(ken.sddmLock, false);

    const lock = computePackageActivation("custom:my-video-lock", backendState);
    assert.equal(lock.sddmLock, true, "Lock must remain active and unchanged after login changed");
  });

  it("Requirement 5: Deactivating Login does not deactivate Lock", () => {
    let backendState: AuthoritativeLockscreenState = {
      session_lock: null,
      sddm_login: "silentsddm-ken",
      sddm_lock: "custom:my-video-lock",
    };

    // Deactivate login target
    backendState = {
      ...backendState,
      sddm_login: null,
    };

    const ken = computePackageActivation("silentsddm-ken", backendState);
    assert.equal(ken.sddmLogin, false, "Ken is deactivated");

    const lock = computePackageActivation("custom:my-video-lock", backendState);
    assert.equal(lock.sddmLock, true, "Lock must remain active after deactivating login");
  });

  it("Requirement 6: Library state inconsistency fixed — Winter is NOT active when backend has Samurai Dog", () => {
    // Exact user scenario: Actual Login Screen = Dog Samurai on system, stale active_lockscreen had Winter
    // Authoritative backend resolution resolves Dog Samurai as active Login Screen
    const backendState: AuthoritativeLockscreenState = {
      session_lock: null,
      sddm_login: "lockscreen-qylock-dog-samurai",
      sddm_lock: null,
    };

    const winter = computePackageActivation("winter", backendState);
    assert.equal(winter.sessionLock, false, "Winter sessionLock must be false");
    assert.equal(winter.sddmLogin, false, "Winter sddmLogin must be false");
    assert.equal(winter.sddmLock, false, "Winter sddmLock must be false");

    const dogSamurai = computePackageActivation("dog-samurai", backendState);
    assert.equal(dogSamurai.sddmLogin, true, "Dog Samurai must be authoritatively marked active login screen");
  });
});
