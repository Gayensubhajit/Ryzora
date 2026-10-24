#pragma once

#include <QObject>
#include <QVariantList>
#include <QVariantMap>
#include <QString>
#include <QPointer>
#include <QThreadPool>
#include <QHash>
#include <QMutex>
#include <QElapsedTimer>
#include <atomic>

class PreviewConfig;

class RyzoraBackend : public QObject {
    Q_OBJECT

    // Catalog & Library
    Q_PROPERTY(QVariantList catalogList READ catalogList NOTIFY catalogChanged)
    Q_PROPERTY(QVariantList libraryList READ libraryList NOTIFY libraryChanged)
    Q_PROPERTY(int catalogCount READ catalogCount NOTIFY catalogChanged)
    Q_PROPERTY(int libraryCount READ libraryCount NOTIFY libraryChanged)
    Q_PROPERTY(int activeCount READ activeCount NOTIFY libraryChanged)
    Q_PROPERTY(QVariantMap activeConfiguration READ activeConfiguration NOTIFY activeConfigurationChanged)
    Q_PROPERTY(bool isCatalogLoading READ isCatalogLoading NOTIFY catalogLoadingChanged)
    Q_PROPERTY(bool isLibraryLoading READ isLibraryLoading NOTIFY libraryLoadingChanged)
    Q_PROPERTY(QString libraryError READ libraryError NOTIFY libraryErrorChanged)

    // Software Applications Catalog (Distinct from Customization Catalog)
    Q_PROPERTY(QVariantList appsList READ appsList NOTIFY appsChanged)
    Q_PROPERTY(int appsCount READ appsCount NOTIFY appsChanged)
    Q_PROPERTY(bool isAppsLoading READ isAppsLoading NOTIFY appsLoadingChanged)

    // Fastfetch
    Q_PROPERTY(QString fastfetchHtml READ fastfetchHtml NOTIFY fastfetchChanged)
    Q_PROPERTY(bool isFastfetchLoading READ isFastfetchLoading NOTIFY fastfetchLoadingChanged)
    Q_PROPERTY(bool isFastfetchCancelled READ isFastfetchCancelled NOTIFY fastfetchChanged)
    Q_PROPERTY(QString fastfetchError READ fastfetchError NOTIFY fastfetchErrorChanged)
    Q_PROPERTY(QString selectedPreset READ selectedPreset NOTIFY selectedPresetChanged)
    Q_PROPERTY(QVariantList availablePresets READ availablePresets NOTIFY availablePresetsChanged)
    Q_PROPERTY(QVariantList fastfetchInstalledPresets READ fastfetchInstalledPresets NOTIFY fastfetchInstalledPresetsChanged)
    Q_PROPERTY(QVariantList fastfetchLocalEmblems READ fastfetchLocalEmblems NOTIFY fastfetchLocalEmblemsChanged)
    Q_PROPERTY(QString fastfetchSavedConfig READ fastfetchSavedConfig NOTIFY fastfetchSavedConfigChanged)

    // Snapshots / Backups
    Q_PROPERTY(QVariantList snapshotsList READ snapshotsList NOTIFY snapshotsChanged)
    Q_PROPERTY(int snapshotsCount READ snapshotsCount NOTIFY snapshotsChanged)
    Q_PROPERTY(bool isSnapshotsLoading READ isSnapshotsLoading NOTIFY snapshotsLoadingChanged)

    // Updates
    Q_PROPERTY(QVariantList updatesList READ updatesList NOTIFY updatesChanged)
    Q_PROPERTY(int updatesCount READ updatesCount NOTIFY updatesChanged)
    Q_PROPERTY(bool isUpdatesLoading READ isUpdatesLoading NOTIFY updatesLoadingChanged)

    // Settings & Host System
    Q_PROPERTY(QVariantMap settingsData READ settingsData NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap systemInfo READ systemInfo NOTIFY systemInfoChanged)
    Q_PROPERTY(QVariantList compatibilityProfiles READ compatibilityProfiles NOTIFY compatibilityProfilesChanged)
    Q_PROPERTY(QString statusMessage READ statusMessage NOTIFY statusMessageChanged)
    Q_PROPERTY(bool isSilentEngineInstalled READ isSilentEngineInstalled NOTIFY silentEngineInstalledChanged)

public:
    explicit RyzoraBackend(QObject *parent = nullptr);
    ~RyzoraBackend() override;

    // Getters
    QVariantList catalogList() const { return m_catalogList; }
    QVariantList libraryList() const { return m_libraryList; }
    int catalogCount() const { return m_catalogList.size(); }
    int libraryCount() const { return m_libraryList.size(); }
    int activeCount() const { return m_activeCount; }
    QVariantMap activeConfiguration() const { return m_activeConfiguration; }
    bool isCatalogLoading() const { return m_isCatalogLoading; }
    bool isLibraryLoading() const { return m_isLibraryLoading; }
    QString libraryError() const { return m_libraryError; }

    QVariantList appsList() const { return m_appsList; }
    int appsCount() const { return m_appsList.size(); }
    bool isAppsLoading() const { return m_isAppsLoading; }

    QString fastfetchHtml() const { return m_fastfetchHtml; }
    bool isFastfetchLoading() const { return m_isFastfetchLoading; }
    bool isFastfetchCancelled() const { return m_isFastfetchCancelled; }
    QString fastfetchError() const { return m_fastfetchError; }
    QString selectedPreset() const { return m_selectedPreset; }
    QVariantList availablePresets() const { return m_availablePresets; }
    QVariantList fastfetchInstalledPresets() const { return m_fastfetchInstalledPresets; }
    QVariantList fastfetchLocalEmblems() const { return m_fastfetchLocalEmblems; }
    QString fastfetchSavedConfig() const { return m_fastfetchSavedConfig; }

    QVariantList snapshotsList() const { return m_snapshotsList; }
    int snapshotsCount() const { return m_snapshotsList.size(); }
    bool isSnapshotsLoading() const { return m_isSnapshotsLoading; }

    QVariantList updatesList() const { return m_updatesList; }
    int updatesCount() const { return m_updatesList.size(); }
    bool isUpdatesLoading() const { return m_isUpdatesLoading; }

    QVariantMap settingsData() const { return m_settingsData; }
    QVariantMap systemInfo() const { return m_systemInfo; }
    QVariantList compatibilityProfiles() const { return m_compatibilityProfiles; }
    QString statusMessage() const { return m_statusMessage; }

    // Invokable Methods
    Q_INVOKABLE void filterCatalog(const QString &category, const QString &query);
    Q_INVOKABLE void filterApps(const QString &category, const QString &query);
    Q_INVOKABLE void installApp(const QString &appId);
    Q_INVOKABLE void cancelAppOperation(const QString &appId);
    Q_INVOKABLE bool isAppOperating(const QString &appId) const;
    Q_INVOKABLE QString getAppActiveOperation(const QString &appId) const;
    Q_INVOKABLE void uninstallApp(const QString &appId);
    Q_INVOKABLE void launchApp(const QString &appId);
    Q_INVOKABLE QVariantList resolveAppProviders(const QString &appId, const QString &displayName = QString());
    Q_INVOKABLE void loadLibrary(bool forceRefresh = false);
    Q_INVOKABLE void loadPresets();
    Q_INVOKABLE void requestFastfetchPreview(const QString &presetName);
    Q_INVOKABLE void cancelFastfetch();
    Q_INVOKABLE bool applyFastfetchPreset(const QString &presetName);
    Q_INVOKABLE bool launchFastfetchTerminal(const QString &presetName);
    Q_INVOKABLE void requestCustomFastfetchPreview(const QString &structure, const QString &logo, const QString &color);
    Q_INVOKABLE void loadFastfetchInstalledPresets();
    Q_INVOKABLE void loadFastfetchLocalEmblems();
    Q_INVOKABLE QString getFastfetchPresetConfig(const QString &presetId);
    Q_INVOKABLE QString getFastfetchSavedConfig();
    Q_INVOKABLE void requestDraftPreview(const QString &draftJson);
    Q_INVOKABLE bool saveFastfetchConfiguration(const QString &configJson, const QString &presetId);
    Q_INVOKABLE bool resetFastfetchDefault();
    Q_INVOKABLE bool removeInstalledFastfetchStyle(const QString &presetId);
    Q_INVOKABLE bool launchTerminalWithDraft(const QString &draftJson);
    Q_INVOKABLE void openFastfetchConfigInEditor();
    Q_INVOKABLE void loadSnapshots();
    Q_INVOKABLE void loadUpdates();
    Q_INVOKABLE void loadSettings();
    Q_INVOKABLE void saveSettings(const QVariantMap &settings);
    Q_INVOKABLE void resetSettings();
    Q_INVOKABLE void saveAppearance(const QVariantMap &app);
    Q_INVOKABLE QVariantMap loadAppearance();
    Q_INVOKABLE QString detectSystemAppearance();
    Q_INVOKABLE void loadSystemInfo();
    Q_INVOKABLE void loadCompatibilityProfiles();
    Q_INVOKABLE void installPackage(const QString &packageId);
    Q_INVOKABLE void uninstallPackage(const QString &packageId);
    Q_INVOKABLE void testPackage(const QString &packageId);
    Q_INVOKABLE void applyTarget(const QString &packageId, const QString &target);
    Q_INVOKABLE void deactivateTarget(const QString &target);
    Q_INVOKABLE void createSnapshot(const QString &reason);
    Q_INVOKABLE void restoreSnapshot(const QString &id);
    Q_INVOKABLE void deleteSnapshot(const QString &id);
    Q_INVOKABLE void applyUpdate(const QString &packageId);
    Q_INVOKABLE void applyAllUpdates();
    Q_INVOKABLE void postNotification(const QString &type, const QString &title, const QString &message);
    Q_INVOKABLE void setTestHomeOverride(const QString &path);
    Q_INVOKABLE void setTestSystemRootOverride(const QString &path);
    Q_INVOKABLE QString exportManifest(const QString &manifestJson, const QString &packageId, const QString &customPath = QString());
    Q_INVOKABLE QVariantMap getThemeConfig(const QString &packageId);
    // file:// URL of an installed real SDDM theme Main.qml usable for in-app preview, or "".
    Q_INVOKABLE QString themePreviewMainUrl(const QString &packageId) const;
    PreviewConfig *previewConfig() const { return m_previewConfig; }
    void setPreviewConfig(PreviewConfig *cfg) { m_previewConfig = cfg; }
    Q_INVOKABLE void updatePreviewDraft(const QString &packageId, const QVariantMap &draft);
    Q_INVOKABLE QVariantMap saveThemeConfig(const QString &packageId, const QVariantMap &config);
    Q_INVOKABLE QVariantMap resetThemeConfig(const QString &packageId);
    Q_INVOKABLE bool isProcessActive() const;
    Q_INVOKABLE uint32_t activePid() const;
    Q_INVOKABLE void shutdown();
    Q_INVOKABLE bool isSilentEngineInstalled() const;
    Q_INVOKABLE void installSilentEngine();
    Q_INVOKABLE void uninstallSilentEngine();

    Q_INVOKABLE bool acquireDecoderSlot();
    Q_INVOKABLE void releaseDecoderSlot();
    Q_INVOKABLE int activeDecoderCount() const { return m_activeDecoderCount.load(); }

    Q_INVOKABLE QString resolveHomePath() const;

signals:
    void catalogChanged();
    void appsChanged();
    void appsLoadingChanged();
    void appInstalled(const QString &appId, bool success, const QString &message);
    void appInstallProgress(const QString &appId, const QString &stage, int percentage, bool isDeterminate, const QString &message, const QString &bytesDownloaded, const QString &bytesTotal, const QString &downloadSpeed);
    void appUninstalled(const QString &appId, bool success, const QString &message);
    void appLaunched(const QString &appId, bool success, const QString &message);
    void libraryChanged();
    void activeConfigurationChanged();
    void decoderSlotAvailable();
    void catalogLoadingChanged();
    void libraryLoadingChanged();
    void libraryErrorChanged();

    void fastfetchChanged();
    void fastfetchLoadingChanged();
    void fastfetchErrorChanged();
    void selectedPresetChanged();
    void availablePresetsChanged();
    void fastfetchInstalledPresetsChanged();
    void fastfetchLocalEmblemsChanged();
    void fastfetchSavedConfigChanged();

    void snapshotsChanged();
    void snapshotsLoadingChanged();

    void updatesChanged();
    void updatesLoadingChanged();

    void settingsChanged();
    void systemInfoChanged();
    void compatibilityProfilesChanged();
    void statusMessageChanged();

    void notificationPosted(const QString &type, const QString &title, const QString &message);

    void targetDeactivated(const QString &target, bool success, const QString &message);
    void packageTested(const QString &packageId, bool success, const QString &message);
    void targetActivated(const QString &packageId, const QString &target, bool success, const QString &message);
    void packageInstalled(const QString &packageId, bool success, const QString &message);
    void packageUninstalled(const QString &packageId, bool success, const QString &message);
    void silentEngineInstalled(bool success, const QString &message);
    void silentEngineInstalledChanged();

private:
    void updateMonotonicReqId(std::atomic<uint64_t> &target, uint64_t newId);
    QString resolveSystemRoot() const;

    QVariantList m_catalogList;
    QVariantList m_appsList;
    bool m_isAppsLoading{false};
    std::atomic<uint64_t> m_currentAppsReqId{0};
    QString m_currentAppsCategory{"all"};
    QString m_currentAppsQuery{""};
    QVariantList m_libraryList;
    bool m_isCatalogLoading{false};
    bool m_isLibraryLoading{false};
    QString m_libraryError{""};
    int m_activeCount{0};
    QVariantMap m_activeConfiguration;

    QString m_fastfetchHtml;
    // Draft-preview output cache: key = sha1(effective draft + fastfetch binary identity).
    // Display-only; never used as evidence of a successful save/apply.
    struct FastfetchPreviewCacheEntry { QString html; qint64 producedAtMs = 0; };
    QHash<QByteArray, FastfetchPreviewCacheEntry> m_fastfetchPreviewCache;
    QList<QByteArray> m_fastfetchPreviewCacheOrder;
    QByteArray fastfetchPreviewCacheKey(const QString &draftJson) const;
    void storeFastfetchPreview(const QByteArray &key, const QString &html);
    bool m_isFastfetchLoading{false};
    bool m_isFastfetchCancelled{false};
    QString m_fastfetchError{""};
    QString m_selectedPreset{"dr460nized"};
    QVariantList m_availablePresets;
    QVariantList m_fastfetchInstalledPresets;
    QVariantList m_fastfetchLocalEmblems;
    QString m_fastfetchSavedConfig;

    QVariantList m_snapshotsList;
    bool m_isSnapshotsLoading{false};

    QVariantList m_updatesList;
    bool m_isUpdatesLoading{false};

    QVariantMap m_settingsData;
    QVariantMap m_systemInfo;
    QVariantList m_compatibilityProfiles;
    QString m_statusMessage{"Ready"};

    // Request synchronization
    std::atomic<uint64_t> m_currentFastfetchReqId{0};
    std::atomic<uint64_t> m_currentCatalogReqId{0};
    std::atomic<uint64_t> m_currentLibraryReqId{0};

    QThreadPool m_threadPool;
    std::atomic<int> m_activeDecoderCount{0};
    std::atomic<bool> m_isShutdown{false};
    mutable QMutex m_appOpMutex;
    QHash<QString, QString> m_activeAppOperations;
    bool isAppOperatingLocked(const QString &clean) const;
    QString m_testHomeOverride;
    QString m_testSystemRootOverride;
    mutable PreviewConfig *m_previewConfig = nullptr;
};
