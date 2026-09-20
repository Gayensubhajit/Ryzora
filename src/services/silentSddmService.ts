import { invokeTauri } from "./tauri";
import {
  SilentSddmHostReport,
  SilentSddmCustomVideoValidation,
  SilentSddmEngineManifest,
  SilentSddmCachedAsset,
  UpstreamWallpaperInstallRequest,
  SilentSddmWallpaper,
} from "../types";
import { SILENTSDDM_WALLPAPERS } from "../providers/silentSddmDiscovery";

export class SilentSddmService {
  /**
   * Retrieves the read-only host report for SilentSDDM capabilities and engine state.
   */
  static async getHostReport(): Promise<SilentSddmHostReport> {
    return invokeTauri<SilentSddmHostReport>("silentsddm_get_host_report");
  }

  /**
   * Validates a custom video or image file path.
   */
  static async validateCustomPath(path: string): Promise<SilentSddmCustomVideoValidation> {
    return invokeTauri<SilentSddmCustomVideoValidation>("silentsddm_validate_custom_path", {
      path,
    });
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
   * Imports a user-selected custom video or image into Ryzora's managed directory.
   */
  static async importCustomVideo(path: string): Promise<SilentSddmCachedAsset> {
    return invokeTauri<SilentSddmCachedAsset>("silentsddm_import_custom_video", { path });
  }

  /**
   * Removes an imported custom video or image.
   */
  static async removeCustomVideo(customId: string): Promise<void> {
    return invokeTauri<void>("silentsddm_remove_custom_video", { customId });
  }

  /**
   * Returns the catalog of all upstream wallpapers.
   */
  static getCatalog(): SilentSddmWallpaper[] {
    return SILENTSDDM_WALLPAPERS;
  }
}
