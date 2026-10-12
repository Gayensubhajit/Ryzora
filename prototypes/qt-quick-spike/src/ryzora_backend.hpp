#pragma once

#include <QObject>
#include <QVariantList>
#include <QVariantMap>
#include <QString>
#include <QPointer>
#include <QThreadPool>
#include <atomic>

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

    // Fastfetch
    Q_PROPERTY(QString fastfetchHtml READ fastfetchHtml NOTIFY fastfetchChanged)
    Q_PROPERTY(bool isFastfetchLoading READ isFastfetchLoading NOTIFY fastfetchLoadingChanged)
    Q_PROPERTY(bool isFastfetchCancelled READ isFastfetchCancelled NOTIFY fastfetchChanged)
    Q_PROPERTY(QString fastfetchError READ fastfetchError NOTIFY fastfetchErrorChanged)
    Q_PROPERTY(QString selectedPreset READ selectedPreset NOTIFY selectedPresetChanged)
    Q_PROPERTY(QVariantList availablePresets READ availablePresets NOTIFY availablePresetsChanged)

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

    QString fastfetchHtml() const { return m_fastfetchHtml; }
    bool isFastfetchLoading() const { return m_isFastfetchLoading; }
    bool isFastfetchCancelled() const { return m_isFastfetchCancelled; }
    QString fastfetchError() const { return m_fastfetchError; }
    QString selectedPreset() const { return m_selectedPreset; }
    QVariantList availablePresets() const { return m_availablePresets; }

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
    Q_INVOKABLE void loadLibrary(bool forceRefresh = false);
    Q_INVOKABLE void loadPresets();
    Q_INVOKABLE void requestFastfetchPreview(const QString &presetName);
    Q_INVOKABLE void cancelFastfetch();
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
    Q_INVOKABLE void deactivateTarget(const QString &target);
    Q_INVOKABLE void createSnapshot(const QString &reason);
    Q_INVOKABLE void restoreSnapshot(const QString &id);
    Q_INVOKABLE void deleteSnapshot(const QString &id);
    Q_INVOKABLE void applyUpdate(const QString &packageId);
    Q_INVOKABLE void applyAllUpdates();
    Q_INVOKABLE void postNotification(const QString &type, const QString &title, const QString &message);
    Q_INVOKABLE bool isProcessActive() const;
    Q_INVOKABLE uint32_t activePid() const;
    Q_INVOKABLE void shutdown();

signals:
    void catalogChanged();
    void libraryChanged();
    void activeConfigurationChanged();
    void catalogLoadingChanged();
    void libraryLoadingChanged();
    void libraryErrorChanged();

    void fastfetchChanged();
    void fastfetchLoadingChanged();
    void fastfetchErrorChanged();
    void selectedPresetChanged();
    void availablePresetsChanged();

    void snapshotsChanged();
    void snapshotsLoadingChanged();

    void updatesChanged();
    void updatesLoadingChanged();

    void settingsChanged();
    void systemInfoChanged();
    void compatibilityProfilesChanged();
    void statusMessageChanged();

    void notificationPosted(const QString &type, const QString &title, const QString &message);

private:
    void updateMonotonicReqId(std::atomic<uint64_t> &target, uint64_t newId);

    QVariantList m_catalogList;
    QVariantList m_libraryList;
    bool m_isCatalogLoading{false};
    bool m_isLibraryLoading{false};
    QString m_libraryError{""};
    int m_activeCount{0};
    QVariantMap m_activeConfiguration;

    QString m_fastfetchHtml;
    bool m_isFastfetchLoading{false};
    bool m_isFastfetchCancelled{false};
    QString m_fastfetchError{""};
    QString m_selectedPreset{"dr460nized"};
    QVariantList m_availablePresets;

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
    std::atomic<bool> m_isShutdown{false};
};
