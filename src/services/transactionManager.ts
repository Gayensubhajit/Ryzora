/**
 * TransactionManager — Phase 23F.2
 *
 * Centralized, singleton transaction service for Ryzora.
 * Core Architecture & Lifecycle:
 *  - Explicit lifecycle: IDLE -> STARTING -> RUNNING -> COMPLETED/FAILED -> CLEANUP
 *  - Strict operation integrity: operation (install | uninstall | reinstall) is never mutated or confused.
 *  - Backend-authoritative transaction ID: Rust generates and owns the single transaction ID.
 *  - Event rejection: strictly rejects any event whose transaction_id, operation, or target_package does not match active state.
 *  - Coalesced UI dispatch: throttles high-frequency progress events to ~6-8 updates/sec (120ms)
 *    while dispatching stage transitions, errors, and completions instantly.
 *  - Bounded ring buffer for terminal logs (max 500 lines).
 *  - Cache awareness: detects and preserves isCached flag.
 *  - Auto-cleanup: automatically moves completed transactions to history after 3.5s so active state never lingers indefinitely.
 *  - Authoritative ALPM state refresh exclusively upon completion.
 */

import { isTauri, invokeTauri } from "./tauri.ts";
import { pacmanAppProvider } from "../providers/index.ts";

export type TransactionOperation = "install" | "uninstall" | "reinstall";

export type TransactionStage =
  | "preparing"
  | "resolving"
  | "downloading"
  | "installing"
  | "finalizing"
  | "completed"
  | "failed";

export type TransactionLifecycle =
  | "IDLE"
  | "STARTING"
  | "RUNNING"
  | "COMPLETED"
  | "FAILED"
  | "CLEANUP";

export interface TransactionProgressPayload {
  transaction_id: string;
  operation: string;
  target_package: string;
  stage: string;
  percentage: number | null;
  download_percentage: number | null;
  install_percentage: number | null;
  current_package: string | null;
  current_package_index: number | null;
  total_packages: number | null;
  bytes_downloaded_str: string | null;
  bytes_total_str: string | null;
  download_speed: string | null;
  message: string;
  raw_line: string | null;
  error: string | null;
  is_db_locked: boolean;
  is_auth_cancelled: boolean;
  is_cached?: boolean;
}

export interface TransactionState {
  id: string;
  operation: TransactionOperation;
  packageName: string;
  stage: TransactionStage;
  lifecycle: TransactionLifecycle;
  percentage: number | null;
  downloadPercentage: number | null;
  installPercentage: number | null;
  currentPackage: string | null;
  currentPackageIndex: number | null;
  totalPackages: number | null;
  downloadedSizeStr: string | null;
  totalSizeStr: string | null;
  speedStr: string | null;
  message: string;
  logs: string[];
  error: string | null;
  isDbLocked: boolean;
  isAuthCancelled: boolean;
  isCached: boolean;
  startedAt: number;
  completedAt: number | null;
}

export interface PrivilegedPackageHelperStatus {
  installed: boolean;
  helper_path: string;
  helper_exists: boolean;
  helper_executable: boolean;
  helper_valid: boolean;
  policy_path: string;
  policy_exists: boolean;
  rules_path: string;
  rules_exists: boolean;
  sha256?: string;
  error?: string;
}

interface StartTransactionBackendResponse {
  success: boolean;
  transaction_id: string;
  operation: string;
  package_name: string;
}

const MAX_LOG_LINES = 500;
const UI_COALESCE_INTERVAL_MS = 120; // ~8 visual updates/sec max for high-frequency progress
const AUTO_CLEANUP_DELAY_MS = 3500; // Auto-dismiss completed drawer after 3.5s

export class TransactionManager {
  private static instance: TransactionManager | null = null;
  private currentTransaction: TransactionState | null = null;
  private history: TransactionState[] = [];
  private listeners = new Set<(tx: TransactionState | null) => void>();
  private packageStatusListeners = new Map<
    string,
    Set<(isOperating: boolean, op?: TransactionOperation) => void>
  >();
  private unlistenIpc: (() => void) | null = null;
  private initPromise: Promise<void> | null = null;
  private isListening = false;
  private lastNotifyTime = 0;
  private coalesceTimer: ReturnType<typeof setTimeout> | null = null;
  private autoCleanupTimer: ReturnType<typeof setTimeout> | null = null;

  public static getInstance(): TransactionManager {
    if (!TransactionManager.instance) {
      TransactionManager.instance = new TransactionManager();
    }
    return TransactionManager.instance;
  }

  constructor() {
    this.initPromise = this.initIpcListener();
  }

  public async initialize(): Promise<void> {
    if (this.initPromise) {
      await this.initPromise;
    }
  }

  private async initIpcListener(): Promise<void> {
    if (!isTauri || this.isListening) return;
    try {
      const { listen } = await import("@tauri-apps/api/event");
      this.unlistenIpc = await listen<TransactionProgressPayload>(
        "ryzora:transaction_progress",
        (event) => {
          this.handleProgressPayload(event.payload);
        }
      );
      this.isListening = true;
    } catch (err) {
      console.warn("[TransactionManager] Failed to attach Tauri transaction listener:", err);
    }
  }

  public subscribe(listener: (tx: TransactionState | null) => void): () => void {
    this.listeners.add(listener);
    listener(this.getCurrentTransaction());
    return () => {
      this.listeners.delete(listener);
    };
  }

  /**
   * Coarse subscription: notifies ONLY when an app starts, completes, or changes operations.
   * Passes authoritative operation ("install" | "uninstall" | "reinstall") so the UI never defaults.
   */
  public subscribePackageStatus(
    packageName: string,
    listener: (isOperating: boolean, op?: TransactionOperation) => void
  ): () => void {
    const key = packageName.toLowerCase();
    if (!this.packageStatusListeners.has(key)) {
      this.packageStatusListeners.set(key, new Set());
    }
    this.packageStatusListeners.get(key)!.add(listener);

    const isCurrentActive =
      this.currentTransaction !== null &&
      this.currentTransaction.packageName.toLowerCase() === key &&
      this.currentTransaction.stage !== "completed" &&
      this.currentTransaction.stage !== "failed";

    listener(isCurrentActive, this.currentTransaction?.operation);

    return () => {
      const set = this.packageStatusListeners.get(key);
      if (set) {
        set.delete(listener);
        if (set.size === 0) {
          this.packageStatusListeners.delete(key);
        }
      }
    };
  }

  private notify() {
    const snapshot = this.getCurrentTransaction();
    for (const listener of this.listeners) {
      try {
        listener(snapshot);
      } catch (err) {
        console.error("[TransactionManager] Error in transaction subscriber:", err);
      }
    }
  }

  private notifyPackageStatus(packageName: string, isOperating: boolean, op?: TransactionOperation) {
    const key = packageName.toLowerCase();
    const set = this.packageStatusListeners.get(key);
    if (set) {
      for (const listener of set) {
        try {
          listener(isOperating, op);
        } catch (err) {
          console.error("[TransactionManager] Error in package status subscriber:", err);
        }
      }
    }
  }

  /**
   * Coalesced notification: flushes immediately for stage/package/error changes,
   * throttles high-frequency progress ticks to 120ms intervals.
   */
  private scheduleCoalescedNotify(immediate: boolean) {
    if (immediate) {
      if (this.coalesceTimer) {
        clearTimeout(this.coalesceTimer);
        this.coalesceTimer = null;
      }
      this.lastNotifyTime = Date.now();
      this.notify();
      return;
    }

    const now = Date.now();
    const elapsed = now - this.lastNotifyTime;

    if (elapsed >= UI_COALESCE_INTERVAL_MS) {
      if (this.coalesceTimer) {
        clearTimeout(this.coalesceTimer);
        this.coalesceTimer = null;
      }
      this.lastNotifyTime = now;
      this.notify();
    } else if (!this.coalesceTimer) {
      this.coalesceTimer = setTimeout(() => {
        this.coalesceTimer = null;
        this.lastNotifyTime = Date.now();
        this.notify();
      }, UI_COALESCE_INTERVAL_MS - elapsed);
    }
  }

  public handleProgressPayload(payload: TransactionProgressPayload) {
    if (!this.currentTransaction) {
      return;
    }

    const tx = this.currentTransaction;

    // Strict invariant: verify transaction_id, operation, and package match
    if (
      tx.id !== payload.transaction_id ||
      tx.operation.toLowerCase() !== payload.operation.toLowerCase() ||
      tx.packageName.toLowerCase() !== payload.target_package.toLowerCase()
    ) {
      return;
    }

    const stage = payload.stage as TransactionStage;
    const stageChanged = tx.stage !== stage;
    const isTerminal = stage === "completed" || stage === "failed";

    tx.stage = stage;
    tx.percentage = payload.percentage;
    tx.downloadPercentage = payload.download_percentage;
    tx.installPercentage = payload.install_percentage;
    tx.currentPackage = payload.current_package;
    tx.currentPackageIndex = payload.current_package_index;
    tx.totalPackages = payload.total_packages;
    tx.downloadedSizeStr = payload.bytes_downloaded_str;
    tx.totalSizeStr = payload.bytes_total_str;
    tx.speedStr = payload.download_speed;
    tx.message = payload.message;
    tx.isDbLocked = payload.is_db_locked;
    tx.isAuthCancelled = payload.is_auth_cancelled;
    if (payload.is_cached) {
      tx.isCached = true;
    }

    if (payload.error) {
      tx.error = payload.error;
    }

    if (payload.raw_line) {
      if (tx.logs.length >= MAX_LOG_LINES) {
        tx.logs.shift();
      }
      tx.logs.push(payload.raw_line);
    }

    if (isTerminal) {
      tx.lifecycle = stage === "completed" ? "COMPLETED" : "FAILED";
      tx.completedAt = Date.now();

      // Clear any previous auto-cleanup timer
      if (this.autoCleanupTimer) {
        clearTimeout(this.autoCleanupTimer);
        this.autoCleanupTimer = null;
      }

      // Notify package button that active operation has completed
      this.notifyPackageStatus(tx.packageName, false, tx.operation);

      // Perform authoritative ALPM database refresh
      if (stage === "completed") {
        pacmanAppProvider.discover().catch((err) => {
          console.warn("[TransactionManager] Post-transaction refresh failed:", err);
        });

        // Auto-dismiss completed transaction from active drawer after 3.5s
        this.autoCleanupTimer = setTimeout(() => {
          this.autoCleanupTimer = null;
          this.clearCurrentTransaction();
        }, AUTO_CLEANUP_DELAY_MS);
      }

      this.scheduleCoalescedNotify(true);
      return;
    }

    tx.lifecycle = "RUNNING";
    this.scheduleCoalescedNotify(stageChanged);
  }

  public getCurrentTransaction(): TransactionState | null {
    return this.currentTransaction ? { ...this.currentTransaction } : null;
  }

  public getHistory(): TransactionState[] {
    return [...this.history];
  }

  public isPackageOperating(packageName: string): boolean {
    return Boolean(
      this.currentTransaction &&
      this.currentTransaction.packageName.toLowerCase() === packageName.toLowerCase() &&
      this.currentTransaction.stage !== "completed" &&
      this.currentTransaction.stage !== "failed"
    );
  }

  public getPackageActiveOperation(packageName: string): TransactionOperation | null {
    if (this.isPackageOperating(packageName)) {
      return this.currentTransaction?.operation ?? null;
    }
    return null;
  }

  public clearCurrentTransaction(): void {
    if (this.currentTransaction) {
      if (this.autoCleanupTimer) {
        clearTimeout(this.autoCleanupTimer);
        this.autoCleanupTimer = null;
      }
      if (this.coalesceTimer) {
        clearTimeout(this.coalesceTimer);
        this.coalesceTimer = null;
      }
      this.currentTransaction.lifecycle = "CLEANUP";
      this.history.unshift({ ...this.currentTransaction });
      this.currentTransaction = null;
      this.notify();
    }
  }

  public destroy(): void {
    if (this.unlistenIpc) {
      this.unlistenIpc();
      this.unlistenIpc = null;
    }
    if (this.autoCleanupTimer) {
      clearTimeout(this.autoCleanupTimer);
      this.autoCleanupTimer = null;
    }
    if (this.coalesceTimer) {
      clearTimeout(this.coalesceTimer);
      this.coalesceTimer = null;
    }
    this.isListening = false;
  }

  public async getHelperStatus(): Promise<PrivilegedPackageHelperStatus | null> {
    if (isTauri) {
      try {
        return await invokeTauri<PrivilegedPackageHelperStatus>("get_privileged_package_helper_status");
      } catch (err) {
        console.warn("[TransactionManager] getHelperStatus failed:", err);
      }
    }
    return null;
  }

  public async setupHelper(): Promise<PrivilegedPackageHelperStatus> {
    if (isTauri) {
      return await invokeTauri<PrivilegedPackageHelperStatus>("setup_privileged_package_helper");
    }
    return {
      installed: true,
      helper_path: "/usr/lib/ryzora/ryzora-package-helper",
      helper_exists: true,
      helper_executable: true,
      helper_valid: true,
      policy_path: "/usr/share/polkit-1/actions/io.ryzora.package.policy",
      policy_exists: true,
      rules_path: "/usr/share/polkit-1/rules.d/io.ryzora.package.rules",
      rules_exists: true,
    };
  }

  /**
   * Starts an install, uninstall, or reinstall transaction.
   * Adopts the authoritative backend transaction ID returned from pacman_start_transaction.
   */
  public async runTransaction(
    packageName: string,
    operation: TransactionOperation
  ): Promise<TransactionState> {
    if (
      this.currentTransaction &&
      this.currentTransaction.stage !== "completed" &&
      this.currentTransaction.stage !== "failed"
    ) {
      throw new Error(
        `Another package transaction (${this.currentTransaction.operation} ${this.currentTransaction.packageName}) is currently running.`
      );
    }

    if (this.autoCleanupTimer) {
      clearTimeout(this.autoCleanupTimer);
      this.autoCleanupTimer = null;
    }

    await this.initialize();

    let backendTxnId: string;

    if (isTauri) {
      const resp = await invokeTauri<StartTransactionBackendResponse>(
        "pacman_start_transaction",
        {
          packageName,
          operation,
        }
      );
      backendTxnId = resp.transaction_id;
    } else {
      backendTxnId = `mock-txn-${Date.now()}`;
    }

    const tx: TransactionState = {
      id: backendTxnId,
      operation,
      packageName,
      stage: "preparing",
      lifecycle: "STARTING",
      percentage: 2,
      downloadPercentage: null,
      installPercentage: null,
      currentPackage: packageName,
      currentPackageIndex: null,
      totalPackages: null,
      downloadedSizeStr: null,
      totalSizeStr: null,
      speedStr: null,
      message: `Preparing to ${operation} ${packageName}...`,
      logs: [`Preparing to ${operation} ${packageName}...`],
      error: null,
      isDbLocked: false,
      isAuthCancelled: false,
      isCached: false,
      startedAt: Date.now(),
      completedAt: null,
    };

    this.currentTransaction = tx;
    this.notifyPackageStatus(packageName, true, operation);
    this.notify();

    return { ...this.currentTransaction };
  }
}

export const transactionManager = TransactionManager.getInstance();
