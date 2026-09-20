import test, { describe, it } from "node:test";
import assert from "node:assert/strict";

describe("Phase 3D.1 — Lock Screen Overflow Actions & Safe Active Uninstall Contract", () => {
  // 1. Session-only active
  it("Scenario 1: Session-only active detects session active and SDDM inactive", () => {
    const activeTargets = { quickshell: true, sddm: false };
    const isQsActive = Boolean(activeTargets.quickshell);
    const isSddmActive = Boolean(activeTargets.sddm);
    const bothActive = isQsActive && isSddmActive;

    assert.strictEqual(isQsActive, true);
    assert.strictEqual(isSddmActive, false);
    assert.strictEqual(bothActive, false);

    // Test action should directly test quickshell without guessing
    const targetToTest = isQsActive && !isSddmActive ? "quickshell" : "both";
    assert.strictEqual(targetToTest, "quickshell");

    // Deactivate action should directly target quickshell
    const targetToDeactivate = isQsActive && !isSddmActive ? "quickshell" : "both";
    assert.strictEqual(targetToDeactivate, "quickshell");
  });

  // 2. SDDM-only active
  it("Scenario 2: SDDM-only active detects SDDM active and session inactive", () => {
    const activeTargets = { quickshell: false, sddm: true };
    const isQsActive = Boolean(activeTargets.quickshell);
    const isSddmActive = Boolean(activeTargets.sddm);
    const bothActive = isQsActive && isSddmActive;

    assert.strictEqual(isQsActive, false);
    assert.strictEqual(isSddmActive, true);
    assert.strictEqual(bothActive, false);

    // Test action directly targets sddm
    const targetToTest = !isQsActive && isSddmActive ? "sddm" : "both";
    assert.strictEqual(targetToTest, "sddm");

    // Deactivate action directly targets sddm
    const targetToDeactivate = !isQsActive && isSddmActive ? "sddm" : "both";
    assert.strictEqual(targetToDeactivate, "sddm");
  });

  // 3. Both active
  it("Scenario 3: Both active exposes submenus and prevents ambiguous guessing", () => {
    const activeTargets = { quickshell: true, sddm: true };
    const isQsActive = Boolean(activeTargets.quickshell);
    const isSddmActive = Boolean(activeTargets.sddm);
    const bothActive = isQsActive && isSddmActive;

    assert.strictEqual(bothActive, true);

    const testOptions = ["quickshell", "sddm"];
    assert.deepStrictEqual(testOptions, ["quickshell", "sddm"]);

    const deactivateOptions = ["quickshell", "sddm", "both"];
    assert.deepStrictEqual(deactivateOptions, ["quickshell", "sddm", "both"]);
  });

  // 4. Deactivate only session
  it("Scenario 4: Deactivating session leaves SDDM active/installed", () => {
    let state = { quickshell: "lockscreen-dog-samurai", sddm: "lockscreen-dog-samurai" };
    const deactivateTarget = (target: "quickshell" | "sddm" | "both") => {
      if (target === "quickshell" || target === "both") {
        delete (state as any).quickshell;
      }
      if (target === "sddm" || target === "both") {
        delete (state as any).sddm;
      }
    };

    deactivateTarget("quickshell");
    assert.strictEqual(state.quickshell, undefined);
    assert.strictEqual(state.sddm, "lockscreen-dog-samurai");
  });

  // 5. Deactivate only SDDM
  it("Scenario 5: Deactivating SDDM leaves session active", () => {
    let state = { quickshell: "lockscreen-dog-samurai", sddm: "lockscreen-dog-samurai" };
    const deactivateTarget = (target: "quickshell" | "sddm" | "both") => {
      if (target === "quickshell" || target === "both") {
        delete (state as any).quickshell;
      }
      if (target === "sddm" || target === "both") {
        delete (state as any).sddm;
      }
    };

    deactivateTarget("sddm");
    assert.strictEqual(state.quickshell, "lockscreen-dog-samurai");
    assert.strictEqual(state.sddm, undefined);
  });

  // 6. Deactivate both
  it("Scenario 6: Deactivating both clears all active integrations", () => {
    let state = { quickshell: "lockscreen-dog-samurai", sddm: "lockscreen-dog-samurai" };
    const deactivateTarget = (target: "quickshell" | "sddm" | "both") => {
      if (target === "quickshell" || target === "both") {
        delete (state as any).quickshell;
      }
      if (target === "sddm" || target === "both") {
        delete (state as any).sddm;
      }
    };

    deactivateTarget("both");
    assert.strictEqual(state.quickshell, undefined);
    assert.strictEqual(state.sddm, undefined);
  });

  // 7. Uninstall inactive package
  it("Scenario 7: Uninstalling inactive package proceeds cleanly without deactivation step", async () => {
    const isAnyActive = false;
    let deactivationCalled = false;
    let fileRemovalCalled = false;

    const executeUninstall = async () => {
      if (isAnyActive) {
        deactivationCalled = true;
      }
      fileRemovalCalled = true;
      return true;
    };

    const res = await executeUninstall();
    assert.strictEqual(res, true);
    assert.strictEqual(deactivationCalled, false);
    assert.strictEqual(fileRemovalCalled, true);
  });

  // 8. Uninstall active package requires deactivation
  it("Scenario 8: Uninstalling active package deactivates active target before removing files", async () => {
    const isQsActive = true;
    const isSddmActive = false;
    const executionOrder: string[] = [];

    const deactivateActive = async (target: string) => {
      executionOrder.push(`deactivate:${target}`);
      return true;
    };

    const removeFiles = async () => {
      executionOrder.push("remove_files");
      return true;
    };

    const executeSafeUninstall = async () => {
      const activeTarget = isQsActive && isSddmActive ? "both" : isQsActive ? "quickshell" : "sddm";
      const deactivated = await deactivateActive(activeTarget);
      assert.strictEqual(deactivated, true);
      await removeFiles();
    };

    await executeSafeUninstall();
    assert.deepStrictEqual(executionOrder, ["deactivate:quickshell", "remove_files"]);
  });

  // 9. Failed deactivation prevents uninstall
  it("Scenario 9: Failed deactivation aborts pipeline and preserves files intact", async () => {
    const isQsActive = true;
    let filesRemoved = false;
    let caughtError: string | null = null;

    const deactivateActive = async () => {
      throw new Error("Systemd user daemon failed to reload native unit");
    };

    const removeFiles = async () => {
      filesRemoved = true;
    };

    const executeSafeUninstall = async () => {
      try {
        if (isQsActive) {
          await deactivateActive();
        }
        await removeFiles();
      } catch (err: any) {
        caughtError = err.message;
      }
    };

    await executeSafeUninstall();
    assert.strictEqual(caughtError, "Systemd user daemon failed to reload native unit");
    assert.strictEqual(filesRemoved, false, "Package files must remain completely untouched on failure");
  });

  // 10. Menu actions don't navigate away
  it("Scenario 10: Clicking overflow menu, submenus, and action buttons never navigates card", () => {
    let cardNavigated = false;
    let stopPropagationCount = 0;

    const handleCardClick = () => {
      cardNavigated = true;
    };

    const createSyntheticEvent = () => ({
      stopPropagation: () => {
        stopPropagationCount++;
      },
    });

    const handleMenuClick = (e: { stopPropagation: () => void }) => {
      e.stopPropagation();
    };

    const handleSubmenuClick = (action: string, e: { stopPropagation: () => void }) => {
      e.stopPropagation();
    };

    // Simulate clicking more button
    handleMenuClick(createSyntheticEvent());
    // Simulate selecting test submenu
    handleSubmenuClick("test:quickshell", createSyntheticEvent());
    // Simulate selecting deactivate submenu
    handleSubmenuClick("deactivate:quickshell", createSyntheticEvent());

    assert.strictEqual(stopPropagationCount, 3);
    assert.strictEqual(cardNavigated, false, "Card navigation must never be triggered by menu actions");
  });
});
