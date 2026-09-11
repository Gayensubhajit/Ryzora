import test from "node:test";
import assert from "node:assert/strict";
import {
  transactionManager,
} from "../services/transactionManager.ts";
import type {
  TransactionProgressPayload,
} from "../services/transactionManager.ts";

test("TransactionManager 1: Initial state is null and history is empty", () => {
  assert.equal(transactionManager.getCurrentTransaction(), null);
  assert.ok(Array.isArray(transactionManager.getHistory()));
});

test("TransactionManager 2: Strict operation integrity across install, uninstall, reinstall", async () => {
  // 1. Install lifecycle
  let observedOp: string | undefined;
  const unsubInstall = transactionManager.subscribePackageStatus("app-x", (isOp, op) => {
    if (isOp) observedOp = op;
  });

  const tx1 = await transactionManager.runTransaction("app-x", "install");
  assert.equal(tx1.operation, "install");
  assert.equal(observedOp, "install");

  transactionManager.handleProgressPayload({
    transaction_id: tx1.id,
    operation: "install",
    target_package: "app-x",
    stage: "completed",
    percentage: 100,
    download_percentage: 100,
    install_percentage: 100,
    current_package: "app-x",
    current_package_index: 1,
    total_packages: 1,
    bytes_downloaded_str: null,
    bytes_total_str: null,
    download_speed: null,
    message: "Completed install",
    raw_line: null,
    error: null,
    is_db_locked: false,
    is_auth_cancelled: false,
  });

  unsubInstall();
  transactionManager.clearCurrentTransaction();

  // 2. Uninstall lifecycle: must NEVER be confused with install
  let uninstallObservedOp: string | undefined;
  const unsubUninstall = transactionManager.subscribePackageStatus("app-x", (isOp, op) => {
    if (isOp) uninstallObservedOp = op;
  });

  const tx2 = await transactionManager.runTransaction("app-x", "uninstall");
  assert.equal(tx2.operation, "uninstall");
  assert.equal(uninstallObservedOp, "uninstall", "Uninstall operation must be reported accurately");

  transactionManager.handleProgressPayload({
    transaction_id: tx2.id,
    operation: "uninstall",
    target_package: "app-x",
    stage: "completed",
    percentage: 100,
    download_percentage: null,
    install_percentage: 100,
    current_package: "app-x",
    current_package_index: 1,
    total_packages: 1,
    bytes_downloaded_str: null,
    bytes_total_str: null,
    download_speed: null,
    message: "Completed uninstall",
    raw_line: null,
    error: null,
    is_db_locked: false,
    is_auth_cancelled: false,
  });

  unsubUninstall();
  transactionManager.clearCurrentTransaction();

  // 3. Reinstall lifecycle
  const tx3 = await transactionManager.runTransaction("app-x", "reinstall");
  assert.equal(tx3.operation, "reinstall");
  transactionManager.clearCurrentTransaction();
});

test("TransactionManager 3: Enforces single active transaction (rejects concurrent)", async () => {
  await transactionManager.runTransaction("pkg-a", "install");

  await assert.rejects(
    async () => {
      await transactionManager.runTransaction("pkg-b", "install");
    },
    {
      message: /Another package transaction \(install pkg-a\) is currently running/,
    }
  );

  transactionManager.clearCurrentTransaction();
});

test("TransactionManager 4: Rejects mismatched transaction ID and operation events", async () => {
  const tx = await transactionManager.runTransaction("pkg-secure", "install");

  // Send event with an alien/mismatched transaction ID
  transactionManager.handleProgressPayload({
    transaction_id: "alien-txn-999",
    operation: "install",
    target_package: "pkg-secure",
    stage: "downloading",
    percentage: 99,
    download_percentage: 99,
    install_percentage: null,
    current_package: "pkg-secure",
    current_package_index: 1,
    total_packages: 1,
    bytes_downloaded_str: null,
    bytes_total_str: null,
    download_speed: null,
    message: "Fake update",
    raw_line: null,
    error: null,
    is_db_locked: false,
    is_auth_cancelled: false,
  });

  const current = transactionManager.getCurrentTransaction();
  assert.equal(current?.id, tx.id);
  assert.notEqual(current?.stage, "downloading", "Mismatched ID event must be dropped");

  // Send event with an alien operation
  transactionManager.handleProgressPayload({
    transaction_id: tx.id,
    operation: "uninstall", // Active is install!
    target_package: "pkg-secure",
    stage: "downloading",
    percentage: 99,
    download_percentage: 99,
    install_percentage: null,
    current_package: "pkg-secure",
    current_package_index: 1,
    total_packages: 1,
    bytes_downloaded_str: null,
    bytes_total_str: null,
    download_speed: null,
    message: "Fake update",
    raw_line: null,
    error: null,
    is_db_locked: false,
    is_auth_cancelled: false,
  });

  assert.notEqual(transactionManager.getCurrentTransaction()?.stage, "downloading", "Mismatched op must be dropped");

  transactionManager.clearCurrentTransaction();
});

test("TransactionManager 5: Handles isCached flag accurately", async () => {
  const tx = await transactionManager.runTransaction("cached-app", "install");

  transactionManager.handleProgressPayload({
    transaction_id: tx.id,
    operation: "install",
    target_package: "cached-app",
    stage: "preparing",
    percentage: 5,
    download_percentage: null,
    install_percentage: null,
    current_package: "cached-app",
    current_package_index: null,
    total_packages: 1,
    bytes_downloaded_str: null,
    bytes_total_str: null,
    download_speed: null,
    message: "Using cached package (verifying local archive)...",
    raw_line: "checking package integrity...",
    error: null,
    is_db_locked: false,
    is_auth_cancelled: false,
    is_cached: true,
  });

  const current = transactionManager.getCurrentTransaction();
  assert.equal(current?.isCached, true, "isCached flag must be true");
  assert.match(current!.message, /Using cached package/);

  transactionManager.clearCurrentTransaction();
});

test("TransactionManager 6: Caps logs to ring buffer limit (500 lines)", async () => {
  const tx = await transactionManager.runTransaction("log-stress", "install");

  for (let i = 0; i < 600; i++) {
    transactionManager.handleProgressPayload({
      transaction_id: tx.id,
      operation: "install",
      target_package: "log-stress",
      stage: "downloading",
      percentage: 50,
      download_percentage: 50,
      install_percentage: null,
      current_package: "log-stress",
      current_package_index: 1,
      total_packages: 1,
      bytes_downloaded_str: null,
      bytes_total_str: null,
      download_speed: null,
      message: "Streaming...",
      raw_line: `log line ${i}`,
      error: null,
      is_db_locked: false,
      is_auth_cancelled: false,
    });
  }

  const current = transactionManager.getCurrentTransaction();
  assert.equal(current?.logs.length, 500, "Log buffer must be capped at MAX_LOG_LINES = 500");
  assert.equal(current?.logs[current.logs.length - 1], "log line 599");

  transactionManager.clearCurrentTransaction();
});

test("TransactionManager 7: Detects database lock and flags isDbLocked cleanly", async () => {
  const tx = await transactionManager.runTransaction("vlc", "install");

  const payload: TransactionProgressPayload = {
    transaction_id: tx.id,
    operation: "install",
    target_package: "vlc",
    stage: "failed",
    percentage: null,
    download_percentage: null,
    install_percentage: null,
    current_package: "vlc",
    current_package_index: null,
    total_packages: null,
    bytes_downloaded_str: null,
    bytes_total_str: null,
    download_speed: null,
    message: "Another package manager is currently using the database.",
    raw_line: "error: failed to init transaction (unable to lock database)",
    error: "Pacman database is currently locked by another process.",
    is_db_locked: true,
    is_auth_cancelled: false,
  };

  transactionManager.handleProgressPayload(payload);
  const current = transactionManager.getCurrentTransaction();
  assert.ok(current);
  assert.equal(current.stage, "failed");
  assert.equal(current.isDbLocked, true);
  assert.equal(current.isAuthCancelled, false);
  assert.match(current.message, /Another package manager/);

  transactionManager.clearCurrentTransaction();
});
