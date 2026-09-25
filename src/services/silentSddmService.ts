export { yieldFrame } from "./frameYield.ts";

import { invokeTauri } from "./tauri.ts";
import type {
  SilentSddmLifecycleState,
  PackageTransaction,
  SilentSddmHostReport,
  SilentSddmCustomVideoValidation,
  SilentSddmEngineManifest,
  SilentSddmCachedAsset,
  UpstreamWallpaperInstallRequest,
  SilentSddmWallpaper,
  SilentSddmActivationManifest,
  SilentSddmConfiguration,
} from "../types/index.ts";
import { SILENTSDDM_WALLPAPERS } from "../providers/silentSddmDiscovery.ts";

export class SilentSddmService {
  /**
   * Retrieves the read-only host report for SilentSDDM capabilities and engine state.
   */
  static async getHostReport(): Promise<SilentSddmHostReport> {
    return invokeTauri<SilentSddmHostReport>("silentsddm_get_host_report");
  }

  /**
   * Validates a custom media file path (photo or video).
   */
  static async validateCustomMedia(path: string): Promise<SilentSddmCustomVideoValidation> {
    return invokeTauri<SilentSddmCustomVideoValidation>("silentsddm_validate_custom_media", {
      path,
    });
  }

  /**
   * Validates a custom video or image file path (legacy alias).
   */
  static async validateCustomPath(path: string): Promise<SilentSddmCustomVideoValidation> {
    return this.validateCustomMedia(path);
  }

  /**
   * Opens the native file picker dialog for custom media files (photos or videos).
   * Returns selected file path or null if cancelled / unavailable.
   */
  static async pickCustomMediaFile(): Promise<string | null> {
    try {
      return await invokeTauri<string | null>("silentsddm_pick_custom_media_file");
    } catch {
      return null;
    }
  }

  /**
   * Installs the SilentSDDM theme engine transactionally.
   * Phase S3: Installs engine files only; backgrounds/ are excluded.
   * Does NOT activate the theme in SDDM (activation is strictly S4).
   */
  static async installEngine(): Promise<SilentSddmEngineManifest> {
    return invokeTauri<SilentSddmEngineManifest>("silentsddm_install_engine");
  }

  /**
   * Uninstalls the SilentSDDM theme engine.
   * Refuses if SDDM is currently configured to use ryzora-silent.
   */
  static async uninstallEngine(): Promise<void> {
    return invokeTauri<void>("silentsddm_uninstall_engine");
  }

  /**
   * Installs an upstream wallpaper: verifies SHA-256 in CAS and places a regular file copy.
   */
  static async installWallpaper(
    req: UpstreamWallpaperInstallRequest
  ): Promise<SilentSddmCachedAsset> {
    return invokeTauri<SilentSddmCachedAsset>("silentsddm_install_wallpaper", { req });
  }

  /**
   * Uninstalls an upstream wallpaper and cleans up CAS blob if unreferenced.
   */
  static async uninstallWallpaper(wallpaperId: string): Promise<void> {
    return invokeTauri<void>("silentsddm_uninstall_wallpaper", { wallpaperId });
  }

  /**
   * Imports a user-selected custom media (photo or video) into Ryzora's managed directory.
   */
  static async importCustomMedia(
    path: string,
    displayName?: string
  ): Promise<SilentSddmCachedAsset> {
    return invokeTauri<SilentSddmCachedAsset>("silentsddm_import_custom_media", {
      path,
      displayName: displayName || null,
    });
  }

  /**
   * Imports a user-selected custom video or image (legacy alias).
   */
  static async importCustomVideo(path: string): Promise<SilentSddmCachedAsset> {
    return this.importCustomMedia(path);
  }

  /**
   * Removes an imported custom media asset by ID.
   */
  static async removeCustomMedia(customId: string): Promise<void> {
    return invokeTauri<void>("silentsddm_remove_custom_media", { customId });
  }

  /**
   * Removes an imported custom video or image (legacy alias).
   */
  static async removeCustomVideo(customId: string): Promise<void> {
    return this.removeCustomMedia(customId);
  }

  /**
   * Returns the catalog of all upstream wallpapers.
   */
  static getCatalog(): SilentSddmWallpaper[] {
    return SILENTSDDM_WALLPAPERS;
  }

  /**
   * Retrieves the current SilentSDDM activation manifest, if active.
   */
  static async getActivationManifest(): Promise<SilentSddmActivationManifest | null> {
    return invokeTauri<SilentSddmActivationManifest | null>("silentsddm_get_activation_manifest");
  }

  /**
   * Applies an installed SilentSDDM wallpaper or custom media to the SDDM login screen.
   */
  static async applyWallpaper(assetId: string): Promise<SilentSddmActivationManifest> {
    return invokeTauri<SilentSddmActivationManifest>("silentsddm_apply_wallpaper", { assetId });
  }

  /**
   * Deactivates the SilentSDDM login screen theme, restoring the previous configuration.
   */
  /**
   * Retrieves the saved or default SilentSDDM configuration.
   */
  static async getConfiguration(): Promise<SilentSddmConfiguration> {
    return invokeTauri<SilentSddmConfiguration>("silentsddm_get_configuration");
  }

  /**
   * Persists user-customized SilentSDDM configuration.
   */
  static async saveConfiguration(config: SilentSddmConfiguration): Promise<void> {
    return invokeTauri<void>("silentsddm_save_configuration", { config });
  }

  /**
   * Applies the full SilentSDDM configuration to SDDM.
   */
  static async applyConfiguration(config: SilentSddmConfiguration): Promise<SilentSddmActivationManifest> {
    return invokeTauri<SilentSddmActivationManifest>("silentsddm_apply_configuration", { config });
  }

  static async deactivate(): Promise<void> {
    return invokeTauri<void>("silentsddm_deactivate");
  }

  /**
   * Derives the authoritative lifecycle state of a SilentSDDM wallpaper
   * from authoritative installed state, active manifest, and transaction.
   */
  static deriveLifecycleState(params: {
    installed: boolean;
    activeAssetId?: string | null;
    packageId: string;
    transaction?: PackageTransaction | null;
  }): SilentSddmLifecycleState {
    const { installed, activeAssetId, packageId, transaction } = params;

    if (transaction && transaction.packageId === packageId) {
      if (transaction.operation === "install") return "installing";
      if (transaction.operation === "apply") return "applying";
      if (transaction.operation === "deactivate") return "deactivating";
      if (transaction.operation === "uninstall") return "uninstalling";
    }

    if (!installed) {
      return "not-installed";
    }

    if (activeAssetId === packageId) {
      return "active";
    }

    return "installed";
  }
}
