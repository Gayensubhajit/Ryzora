import test, { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { LockScreenActivationState, AuthoritativeLockscreenState } from "../types/index.ts";

function computePackageActivation(
  pkgId: string,
  authState: AuthoritativeLockscreenState
): LockScreenActivationState {
  const normalize = (id: string | null | undefined) => {
    if (!id) return "";
    return id.replace(/^lockscreen-(qylock-)?/, "").replace(/^ryzora-/, "").toLowerCase();
  };
  const targetId = normalize(pkgId);
  const sddmLoginTarget = normalize(authState.sddm_login);

  return {
    sddmLogin: Boolean(sddmLoginTarget && sddmLoginTarget === targetId),
  };
}

describe("Lock-Screen State Model — Authoritative SDDM Login Target", () => {
  it("Requirement 1: SilentSDDM Login active sets sddmLogin true", () => {
    const backendState: AuthoritativeLockscreenState = {
      sddm_login: "silentsddm-silvia",
    };

    const silvia = computePackageActivation("silentsddm-silvia", backendState);
    assert.equal(silvia.sddmLogin, true, "SilentSDDM login must be active");

    const dogSamurai = computePackageActivation("dog-samurai", backendState);
    assert.equal(dogSamurai.sddmLogin, false);
  });

  it("Requirement 2: Changing Login updates authoritative active state", () => {
    let backendState: AuthoritativeLockscreenState = {
      sddm_login: "silentsddm-silvia",
    };

    backendState = {
      sddm_login: "silentsddm-ken",
    };

    const silvia = computePackageActivation("silentsddm-silvia", backendState);
    assert.equal(silvia.sddmLogin, false, "Silvia should no longer be active login");

    const ken = computePackageActivation("silentsddm-ken", backendState);
    assert.equal(ken.sddmLogin, true, "Ken is now active login");
  });

  it("Requirement 3: Deactivating Login sets sddmLogin false", () => {
    let backendState: AuthoritativeLockscreenState = {
      sddm_login: "silentsddm-ken",
    };

    backendState = {
      sddm_login: null,
    };

    const ken = computePackageActivation("silentsddm-ken", backendState);
    assert.equal(ken.sddmLogin, false, "Ken is deactivated");
  });

  it("Requirement 4: Winter is NOT active when backend has Samurai Dog", () => {
    const backendState: AuthoritativeLockscreenState = {
      sddm_login: "lockscreen-qylock-dog-samurai",
    };

    const winter = computePackageActivation("winter", backendState);
    assert.equal(winter.sddmLogin, false, "Winter sddmLogin must be false");

    const dogSamurai = computePackageActivation("dog-samurai", backendState);
    assert.equal(dogSamurai.sddmLogin, true, "Dog Samurai must be authoritatively marked active login screen");
  });
});
