#include <QSettings>
#include "ryzora_backend.hpp"
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QMetaObject>
#include <QCoreApplication>
#include <QDir>
#include <QDebug>

// Declarations of Rust C-ABI functions
extern "C" {
    void ryzora_init();
    void ryzora_set_test_home_override(const char* path);
    void ryzora_set_test_system_root_override(const char* path);
    uint64_t ryzora_allocate_request_id();
    bool ryzora_is_process_active();
    uint32_t ryzora_active_pid();
    char* ryzora_query_catalog(const char* repo_root, const char* category, const char* search);
    char* ryzora_load_library(bool force_refresh);
    char* ryzora_run_fastfetch(const char* preset, uint64_t req_id);
    bool ryzora_cancel_fastfetch();
    void ryzora_shutdown();
    void ryzora_free_string(char* s);

    char* ryzora_load_snapshots();
    char* ryzora_load_settings();
    bool ryzora_save_settings(const char* settings_json);
    char* ryzora_reset_settings();
    char* ryzora_load_appearance();
    bool ryzora_save_appearance(const char* appearance_json);
    char* ryzora_detect_system_appearance();
    char* ryzora_load_compatibility_profiles();
    char* ryzora_load_system_info();
    char* ryzora_load_updates();
    char* ryzora_list_presets();
    char* ryzora_install_package(const char* package_id);
    char* ryzora_test_package(const char* package_id);
    char* ryzora_uninstall_package(const char* package_id);
    char* ryzora_apply_target(const char* package_id, const char* target);
    char* ryzora_deactivate_target(const char* target);
    char* ryzora_create_snapshot(const char* reason);
    char* ryzora_restore_snapshot(const char* id);
    char* ryzora_delete_snapshot(const char* id);
    char* ryzora_apply_update(const char* package_id);
    char* ryzora_apply_all_updates();
    char* ryzora_load_active_configuration();
}

void RyzoraBackend::updateMonotonicReqId(std::atomic<uint64_t> &target, uint64_t newId) {
    uint64_t current = target.load(std::memory_order_relaxed);
    while (newId > current && !target.compare_exchange_weak(current, newId, std::memory_order_release, std::memory_order_relaxed)) {
        // loop until current >= newId
    }
}

RyzoraBackend::RyzoraBackend(QObject *parent) : QObject(parent) {
    m_threadPool.setMaxThreadCount(4);
    m_availablePresets = QVariantList{
        "dr460nized",
        "fastfetch-preset-2",
        "fastfetch-preset-3",
        "fastfetch-preset-6",
        "fastfetch-rorfetch",
        "neofetch",
        "paleofetch",
        "archey"
    };

    ryzora_init();
    filterCatalog("all", "");
    loadLibrary(false);
    requestFastfetchPreview("dr460nized");
    loadSnapshots();
    loadUpdates();
    loadSettings();
    loadSystemInfo();
    loadCompatibilityProfiles();
    loadPresets();
}

RyzoraBackend::~RyzoraBackend() {
    shutdown();
}

void RyzoraBackend::shutdown() {
    if (m_isShutdown.exchange(true)) {
        return;
    }
    cancelFastfetch();
    ryzora_shutdown();
    m_threadPool.clear();
    m_threadPool.waitForDone(300);
}

bool RyzoraBackend::isProcessActive() const {
    return ryzora_is_process_active();
}

uint32_t RyzoraBackend::activePid() const {
    return ryzora_active_pid();
}

void RyzoraBackend::filterCatalog(const QString &category, const QString &query) {
    if (m_isShutdown.load()) return;
    uint64_t reqId = ryzora_allocate_request_id();
    updateMonotonicReqId(m_currentCatalogReqId, reqId);
    m_isCatalogLoading = true;
    emit catalogLoadingChanged();

    QPointer<RyzoraBackend> self = this;
    QString repoRoot = QDir::currentPath();

    m_threadPool.start([self, repoRoot, category, query, reqId]() {
        QByteArray bRoot = repoRoot.toUtf8();
        QByteArray bCat = category.toUtf8();
        QByteArray bQuery = query.toUtf8();

        char* rawJson = ryzora_query_catalog(bRoot.constData(), bCat.constData(), bQuery.constData());
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantList list;
        if (doc.isArray()) {
            list = doc.array().toVariantList();
        }

        QMetaObject::invokeMethod(qApp, [self, list, reqId]() {
            if (self && reqId == self->m_currentCatalogReqId.load()) {
                self->m_catalogList = list;
                self->m_isCatalogLoading = false;
                emit self->catalogChanged();
                emit self->catalogLoadingChanged();
            }
        });
    });
}

void RyzoraBackend::loadLibrary(bool forceRefresh) {
    if (m_isShutdown.load()) return;
    uint64_t reqId = ryzora_allocate_request_id();
    updateMonotonicReqId(m_currentLibraryReqId, reqId);
    m_isLibraryLoading = true;
    m_libraryError = "";
    emit libraryLoadingChanged();
    emit libraryErrorChanged();

    QPointer<RyzoraBackend> self = this;

    m_threadPool.start([self, forceRefresh, reqId]() {
        char* rawJson = ryzora_load_library(forceRefresh);
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantList list;
        QString err = "";
        int activeCount = 0;
        if (doc.isArray()) {
            list = doc.array().toVariantList();
            for (const QVariant &item : list) {
                if (item.toMap().value("is_active").toBool()) {
                    activeCount++;
                }
            }
        } else {
            err = "Failed to parse installed packages JSON payload.";
        }

        char *rawConfig = ryzora_load_active_configuration();
        QString configStr = QString::fromUtf8(rawConfig);
        ryzora_free_string(rawConfig);
        QJsonDocument cfgDoc = QJsonDocument::fromJson(configStr.toUtf8());
        QVariantMap activeCfg = cfgDoc.object().toVariantMap();

        QMetaObject::invokeMethod(qApp, [self, list, activeCount, activeCfg, err, reqId]() {
            if (self && reqId == self->m_currentLibraryReqId.load()) {
                self->m_libraryList = list;
                self->m_activeCount = activeCount;
                self->m_activeConfiguration = activeCfg;
                self->m_libraryError = err;
                self->m_isLibraryLoading = false;
                emit self->libraryChanged();
                emit self->activeConfigurationChanged();
                emit self->libraryLoadingChanged();
                emit self->libraryErrorChanged();
            }
        });
    });
}

void RyzoraBackend::requestFastfetchPreview(const QString &presetName) {
    if (m_isShutdown.load()) return;
    uint64_t reqId = ryzora_allocate_request_id();
    updateMonotonicReqId(m_currentFastfetchReqId, reqId);

    m_selectedPreset = presetName;
    m_isFastfetchLoading = true;
    m_isFastfetchCancelled = false;
    m_fastfetchError = "";
    m_statusMessage = QString("Running preview for %1…").arg(presetName);
    emit selectedPresetChanged();
    emit fastfetchLoadingChanged();
    emit fastfetchErrorChanged();
    emit statusMessageChanged();

    QPointer<RyzoraBackend> self = this;

    m_threadPool.start([self, presetName, reqId]() {
        QByteArray bPreset = presetName.toUtf8();
        char* rawJson = ryzora_run_fastfetch(bPreset.constData(), reqId);
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QJsonObject obj = doc.object();

        bool success = obj.value("success").toBool();
        bool cancelled = obj.value("cancelled").toBool();
        QString html = obj.value("html_formatted").toString();
        QString err = obj.value("error").toString();
        double execMs = obj.value("execution_time_ms").toDouble();

        QMetaObject::invokeMethod(qApp, [self, reqId, presetName, success, cancelled, html, err, execMs]() {
            if (self && reqId == self->m_currentFastfetchReqId.load()) {
                self->m_isFastfetchLoading = false;
                self->m_isFastfetchCancelled = cancelled;
                if (cancelled) {
                    self->m_statusMessage = QString("Cancelled preview for %1").arg(presetName);
                } else if (success) {
                    self->m_fastfetchHtml = html;
                    self->m_statusMessage = QString("Rendered %1 in %2 ms").arg(presetName).arg(QString::number(execMs, 'f', 1));
                } else {
                    self->m_fastfetchError = err.isEmpty() ? "Execution failed" : err;
                    self->m_statusMessage = QString("Failed: %1").arg(self->m_fastfetchError);
                }
                emit self->fastfetchChanged();
                emit self->fastfetchLoadingChanged();
                emit self->fastfetchErrorChanged();
                emit self->statusMessageChanged();
            }
        });
    });
}

void RyzoraBackend::cancelFastfetch() {
    m_isFastfetchCancelled = true;
    m_isFastfetchLoading = false;
    m_statusMessage = "Preview cancelled by user";
    ryzora_cancel_fastfetch();
    emit fastfetchLoadingChanged();
    emit statusMessageChanged();
}

void RyzoraBackend::loadSnapshots() {
    if (m_isShutdown.load()) return;
    m_isSnapshotsLoading = true;
    emit snapshotsLoadingChanged();

    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char* rawJson = ryzora_load_snapshots();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantList list;
        if (doc.isArray()) {
            list = doc.array().toVariantList();
        }

        QMetaObject::invokeMethod(qApp, [self, list]() {
            if (self) {
                self->m_snapshotsList = list;
                self->m_isSnapshotsLoading = false;
                emit self->snapshotsChanged();
                emit self->snapshotsLoadingChanged();
            }
        });
    });
}

void RyzoraBackend::loadUpdates() {
    if (m_isShutdown.load()) return;
    m_isUpdatesLoading = true;
    emit updatesLoadingChanged();

    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char* rawJson = ryzora_load_updates();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantList list;
        if (doc.isArray()) {
            list = doc.array().toVariantList();
        }

        QMetaObject::invokeMethod(qApp, [self, list]() {
            if (self) {
                self->m_updatesList = list;
                self->m_isUpdatesLoading = false;
                emit self->updatesChanged();
                emit self->updatesLoadingChanged();
            }
        });
    });
}

void RyzoraBackend::loadSettings() {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char* rawJson = ryzora_load_settings();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantMap map;
        if (doc.isObject()) {
            map = doc.object().toVariantMap();
        }

        QMetaObject::invokeMethod(qApp, [self, map]() {
            if (self) {
                self->m_settingsData = map;
                emit self->settingsChanged();
            }
        });
    });
}

void RyzoraBackend::saveSettings(const QVariantMap &settings) {
    if (m_isShutdown.load()) return;
    m_settingsData = settings;
    emit settingsChanged();

    QJsonDocument doc = QJsonDocument::fromVariant(settings);
    QByteArray jsonBytes = doc.toJson(QJsonDocument::Indented);

    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self, jsonBytes]() {
        bool ok = ryzora_save_settings(jsonBytes.constData());
        QMetaObject::invokeMethod(qApp, [self, ok]() {
            if (self) {
                if (ok) {
                    self->postNotification("success", "Settings Saved", "Your application preferences have been persisted.");
                } else {
                    self->postNotification("error", "Error", "Failed to persist settings to disk.");
                }
            }
        });
    });
}

void RyzoraBackend::resetSettings() {
    if (m_isShutdown.load()) return;
    char* rawJson = ryzora_reset_settings();
    if (rawJson) {
        QByteArray jsonBytes(rawJson);
        free(rawJson);
        QJsonDocument doc = QJsonDocument::fromJson(jsonBytes);
        if (doc.isObject()) {
            QJsonObject obj = doc.object();
            if (obj.value("success").toBool()) {
                m_settingsData = obj.value("settings").toObject().toVariantMap();
                emit settingsChanged();
                postNotification("info", "Settings Reset", "All preferences have been restored to default values.");
                return;
            }
        }
    }
    postNotification("error", "Error", "Failed to reset settings.");
}

QVariantMap RyzoraBackend::loadAppearance() {
    char* rawJson = ryzora_load_appearance();
    if (rawJson) {
        QByteArray jsonBytes(rawJson);
        free(rawJson);
        QJsonDocument doc = QJsonDocument::fromJson(jsonBytes);
        if (doc.isObject()) {
            return doc.object().toVariantMap();
        }
    }
    return QVariantMap();
}

void RyzoraBackend::saveAppearance(const QVariantMap &app) {
    if (m_isShutdown.load()) return;
    QJsonDocument doc = QJsonDocument::fromVariant(app);
    QByteArray jsonBytes = doc.toJson(QJsonDocument::Compact);
    ryzora_save_appearance(jsonBytes.constData());
}

QString RyzoraBackend::detectSystemAppearance() {
    char* str = ryzora_detect_system_appearance();
    if (str) {
        QString res = QString::fromUtf8(str);
        free(str);
        return res;
    }
    return QStringLiteral("dark");
}


void RyzoraBackend::loadPresets() {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char* rawJson = ryzora_list_presets();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantList list;
        if (doc.isArray()) {
            list = doc.array().toVariantList();
        }

        QMetaObject::invokeMethod(qApp, [self, list]() {
            if (self && !list.isEmpty()) {
                self->m_availablePresets = list;
                emit self->availablePresetsChanged();
            }
        });
    });
}

void RyzoraBackend::loadCompatibilityProfiles() {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char* rawJson = ryzora_load_compatibility_profiles();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantList list;
        if (doc.isArray()) {
            list = doc.array().toVariantList();
        }

        QMetaObject::invokeMethod(qApp, [self, list]() {
            if (self) {
                self->m_compatibilityProfiles = list;
                emit self->compatibilityProfilesChanged();
            }
        });
    });
}

void RyzoraBackend::loadSystemInfo() {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char* rawJson = ryzora_load_system_info();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantMap map;
        if (doc.isObject()) {
            map = doc.object().toVariantMap();
        }

        QMetaObject::invokeMethod(qApp, [self, map]() {
            if (self) {
                self->m_systemInfo = map;
                emit self->systemInfoChanged();
            }
        });
    });
}

void RyzoraBackend::postNotification(const QString &type, const QString &title, const QString &message) {
    emit notificationPosted(type, title, message);
}

void RyzoraBackend::setTestHomeOverride(const QString &path) {
    if (path.isEmpty()) {
        ryzora_set_test_home_override(nullptr);
    } else {
        QByteArray b = path.toUtf8();
        ryzora_set_test_home_override(b.constData());
    }
    loadLibrary(true);
    filterCatalog("all", "");
    loadSnapshots();
}

void RyzoraBackend::setTestSystemRootOverride(const QString &path) {
    if (path.isEmpty()) {
        ryzora_set_test_system_root_override(nullptr);
    } else {
        QByteArray b = path.toUtf8();
        ryzora_set_test_system_root_override(b.constData());
    }
}

struct FreshStatePayload {
    QVariantList catalogList;
    QVariantList libraryList;
    int activeCount{0};
    QVariantMap activeConfig;
};

static FreshStatePayload fetchFreshStatePayload() {
    FreshStatePayload payload;

    char *rawLib = ryzora_load_library(true);
    if (rawLib) {
        QString libStr = QString::fromUtf8(rawLib);
        ryzora_free_string(rawLib);
        QJsonDocument libDoc = QJsonDocument::fromJson(libStr.toUtf8());
        if (libDoc.isArray()) {
            payload.libraryList = libDoc.array().toVariantList();
            for (const QVariant &item : payload.libraryList) {
                if (item.toMap().value("is_active").toBool()) {
                    payload.activeCount++;
                }
            }
        }
    }

    char *rawCfg = ryzora_load_active_configuration();
    if (rawCfg) {
        QString cfgStr = QString::fromUtf8(rawCfg);
        ryzora_free_string(rawCfg);
        QJsonDocument cfgDoc = QJsonDocument::fromJson(cfgStr.toUtf8());
        payload.activeConfig = cfgDoc.object().toVariantMap();
    }

    QString repoRoot = QDir::currentPath();
    QByteArray bRoot = repoRoot.toUtf8();
    char *rawCat = ryzora_query_catalog(bRoot.constData(), "all", "");
    if (rawCat) {
        QString catStr = QString::fromUtf8(rawCat);
        ryzora_free_string(rawCat);
        QJsonDocument catDoc = QJsonDocument::fromJson(catStr.toUtf8());
        if (catDoc.isArray()) {
            payload.catalogList = catDoc.array().toVariantList();
        }
    }

    return payload;
}

void RyzoraBackend::installPackage(const QString &packageId) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self, packageId]() {
        QByteArray idBytes = packageId.toUtf8();
        char* rawRes = ryzora_install_package(idBytes.constData());
        if (rawRes) {
            QString resStr = QString::fromUtf8(rawRes);
            ryzora_free_string(rawRes);
            QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
            bool success = doc.object().value("success").toBool();
            QString msg = doc.object().value("message").toString();

            FreshStatePayload fresh;
            if (success) {
                fresh = fetchFreshStatePayload();
            }

            QMetaObject::invokeMethod(qApp, [self, success, msg, packageId, fresh]() {
                if (self) {
                    if (success) {
                        self->m_libraryList = fresh.libraryList;
                        self->m_catalogList = fresh.catalogList;
                        self->m_activeCount = fresh.activeCount;
                        self->m_activeConfiguration = fresh.activeConfig;
                        emit self->catalogChanged();
                        emit self->libraryChanged();
                        emit self->activeConfigurationChanged();
                    }
                    self->postNotification(success ? "success" : "error",
                                           success ? "Installation Complete" : "Installation Failed",
                                           msg);
                    emit self->packageInstalled(packageId, success, msg);
                }
            });
        }
    });
}

void RyzoraBackend::uninstallPackage(const QString &packageId) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self, packageId]() {
        QByteArray idBytes = packageId.toUtf8();
        char* rawRes = ryzora_uninstall_package(idBytes.constData());
        if (rawRes) {
            QString resStr = QString::fromUtf8(rawRes);
            ryzora_free_string(rawRes);
            QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
            bool success = doc.object().value("success").toBool();
            QString msg = doc.object().value("message").toString();

            FreshStatePayload fresh;
            if (success) {
                fresh = fetchFreshStatePayload();
            }

            QMetaObject::invokeMethod(qApp, [self, success, msg, packageId, fresh]() {
                if (self) {
                    if (success) {
                        self->m_libraryList = fresh.libraryList;
                        self->m_catalogList = fresh.catalogList;
                        self->m_activeCount = fresh.activeCount;
                        self->m_activeConfiguration = fresh.activeConfig;
                        emit self->catalogChanged();
                        emit self->libraryChanged();
                        emit self->activeConfigurationChanged();
                    }
                    self->postNotification(success ? "info" : "error",
                                           success ? "Uninstalled" : "Uninstall Refused",
                                           msg);
                    emit self->packageUninstalled(packageId, success, msg);
                }
            });
        }
    });
}

void RyzoraBackend::testPackage(const QString &packageId) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self, packageId]() {
        QByteArray idBa = packageId.toUtf8();
        char *rawRes = ryzora_test_package(idBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_free_string(rawRes);
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        bool success = doc.object().value("success").toBool();
        QString msg = doc.object().value("message").toString();

        FreshStatePayload fresh;
        if (success) {
            fresh = fetchFreshStatePayload();
        }

        QMetaObject::invokeMethod(qApp, [self, success, msg, packageId, fresh]() {
            if (self) {
                if (success) {
                    self->m_libraryList = fresh.libraryList;
                    self->m_catalogList = fresh.catalogList;
                    emit self->catalogChanged();
                    emit self->libraryChanged();
                }
                self->postNotification(success ? "success" : "error",
                                       success ? "Preview Launched" : "Preview Failed",
                                       msg);
                emit self->packageTested(packageId, success, msg);
            }
        });
    });
}

void RyzoraBackend::applyTarget(const QString &packageId, const QString &target) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self, packageId, target]() {
        QByteArray idBa = packageId.toUtf8();
        QByteArray targetBa = target.toUtf8();
        char *rawRes = ryzora_apply_target(idBa.constData(), targetBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_free_string(rawRes);
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        bool success = doc.object().value("success").toBool();
        QString msg = doc.object().value("message").toString();

        FreshStatePayload fresh;
        if (success) {
            fresh = fetchFreshStatePayload();
        }

        QMetaObject::invokeMethod(qApp, [self, success, msg, packageId, target, fresh]() {
            if (self) {
                if (success) {
                    self->m_libraryList = fresh.libraryList;
                    self->m_catalogList = fresh.catalogList;
                    self->m_activeCount = fresh.activeCount;
                    self->m_activeConfiguration = fresh.activeConfig;
                    emit self->catalogChanged();
                    emit self->libraryChanged();
                    emit self->activeConfigurationChanged();
                }
                self->postNotification(success ? "success" : "error",
                                       success ? "Target Activated" : "Activation Failed",
                                       msg);
                emit self->targetActivated(packageId, target, success, msg);
            }
        });
    });
}

void RyzoraBackend::deactivateTarget(const QString &target) {
    qInfo() << "[BACKEND C++] deactivateTarget called for target:" << target;
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self, target]() {
        QByteArray targetBa = target.toUtf8();
        char *rawRes = ryzora_deactivate_target(targetBa.constData());
qInfo() << "[BACKEND C++] ryzora_deactivate_target finished. rawRes =" << (rawRes ? rawRes : "NULL");
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_free_string(rawRes);
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        bool success = doc.object().value("success").toBool();
        QString msg = doc.object().value("message").toString();

        FreshStatePayload fresh;
        if (success) {
            fresh = fetchFreshStatePayload();
        }

        QMetaObject::invokeMethod(qApp, [self, success, msg, target, fresh]() {
            if (self) {
                if (success) {
                    self->m_libraryList = fresh.libraryList;
                    self->m_catalogList = fresh.catalogList;
                    self->m_activeCount = fresh.activeCount;
                    self->m_activeConfiguration = fresh.activeConfig;
                    emit self->catalogChanged();
                    emit self->libraryChanged();
                    emit self->activeConfigurationChanged();
                }
                self->postNotification(success ? "success" : "error",
                                       success ? "Target Deactivated" : "Deactivation Failed",
                                       msg);
                emit self->targetDeactivated(target, success, msg);
            }
        });
    });
}

void RyzoraBackend::createSnapshot(const QString &reason) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self, reason]() {
        QByteArray reasonBa = reason.toUtf8();
        char *rawRes = ryzora_create_snapshot(reasonBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_free_string(rawRes);
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        bool success = doc.object().value("success").toBool();
        QString msg = doc.object().value("message").toString();

        QMetaObject::invokeMethod(qApp, [self, success, msg]() {
            if (self) {
                self->postNotification(success ? "success" : "error",
                                       success ? "Snapshot Created" : "Snapshot Failed",
                                       msg);
                self->loadSnapshots();
            }
        });
    });
}

void RyzoraBackend::restoreSnapshot(const QString &id) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self, id]() {
        QByteArray idBa = id.toUtf8();
        char *rawRes = ryzora_restore_snapshot(idBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_free_string(rawRes);
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        bool success = doc.object().value("success").toBool();
        QString msg = doc.object().value("message").toString();

        QMetaObject::invokeMethod(qApp, [self, success, msg]() {
            if (self) {
                self->postNotification(success ? "success" : "error",
                                       success ? "Snapshot Restored" : "Restore Failed",
                                       msg);
                self->loadLibrary(true);
                self->loadSnapshots();
            }
        });
    });
}

void RyzoraBackend::deleteSnapshot(const QString &id) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self, id]() {
        QByteArray idBa = id.toUtf8();
        char *rawRes = ryzora_delete_snapshot(idBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_free_string(rawRes);
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        bool success = doc.object().value("success").toBool();
        QString msg = doc.object().value("message").toString();

        QMetaObject::invokeMethod(qApp, [self, success, msg]() {
            if (self) {
                self->postNotification(success ? "info" : "error",
                                       success ? "Snapshot Deleted" : "Deletion Failed",
                                       msg);
                self->loadSnapshots();
            }
        });
    });
}

void RyzoraBackend::applyUpdate(const QString &packageId) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self, packageId]() {
        QByteArray idBa = packageId.toUtf8();
        char *rawRes = ryzora_apply_update(idBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_free_string(rawRes);
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        bool success = doc.object().value("success").toBool();
        QString msg = doc.object().value("message").toString();

        QMetaObject::invokeMethod(qApp, [self, success, msg]() {
            if (self) {
                self->postNotification(success ? "success" : "error",
                                       success ? "Package Updated" : "Update Failed",
                                       msg);
                self->loadUpdates();
                self->loadLibrary(true);
            }
        });
    });
}

void RyzoraBackend::applyAllUpdates() {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self]() {
        char *rawRes = ryzora_apply_all_updates();
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_free_string(rawRes);
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        bool success = doc.object().value("success").toBool();
        QString msg = doc.object().value("message").toString();

        QMetaObject::invokeMethod(qApp, [self, success, msg]() {
            if (self) {
                self->postNotification(success ? "success" : "error",
                                       success ? "Updates Completed" : "Updates Failed",
                                       msg);
                self->loadUpdates();
                self->loadLibrary(true);
            }
        });
    });
}

QString RyzoraBackend::exportManifest(const QString &manifestJson, const QString &packageId, const QString &customPath) {
    if (manifestJson.trimmed().isEmpty() || packageId.trimmed().isEmpty()) {
        return QString("Failed: Manifest content or package ID cannot be empty.");
    }
    QJsonParseError parseErr;
    QJsonDocument doc = QJsonDocument::fromJson(manifestJson.toUtf8(), &parseErr);
    if (parseErr.error != QJsonParseError::NoError) {
        return QString("Validation Error: Invalid JSON syntax (%1)").arg(parseErr.errorString());
    }
    QJsonObject obj = doc.object();
    if (!obj.contains("id") || !obj.contains("name") || !obj.contains("version") || !obj.contains("target")) {
        return QString("Validation Error: Missing required manifest keys (id, name, version, target).");
    }

    QString filePath;
    if (!customPath.trimmed().isEmpty()) {
        filePath = customPath.trimmed();
        if (filePath.startsWith("file://")) {
            filePath = QUrl(filePath).toLocalFile();
        }
        QFileInfo fi(filePath);
        QDir targetDir = fi.dir();
        if (!targetDir.exists()) {
            targetDir.mkpath(".");
        }
    } else {
        QString home = QDir::homePath();
        QDir draftDir(home + "/.local/share/ryzora/drafts");
        if (!draftDir.exists()) {
            draftDir.mkpath(".");
        }
        QString safeSlug = packageId.trimmed().toLower();
        safeSlug.replace(QRegularExpression("[^a-z0-9_-]"), "-");
        filePath = draftDir.filePath(QString("%1-manifest.json").arg(safeSlug));
    }

    QFile file(filePath);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        return QString("Filesystem Error: Could not write to %1 (%2)").arg(filePath, file.errorString());
    }
    file.write(doc.toJson(QJsonDocument::Indented));
    file.close();
    return QString("OK:%1").arg(filePath);
}


QVariantMap RyzoraBackend::getThemeConfig(const QString &packageId) {
    QVariantMap result;
    result["has_settings"] = false;
    result["capabilities"] = QVariantList();
    result["settings"] = QVariantList();
    result["config"] = QVariantMap();
    result["package_id"] = packageId;

    if (packageId.isEmpty() || packageId.contains("..") || packageId.contains("/") || packageId.contains("\\")) {
        result["error"] = "Invalid package identifier";
        return result;
    }

    QString slug = packageId;
    if (slug.startsWith("lockscreen-qylock-")) {
        slug = slug.mid(18);
    } else if (slug.startsWith("lockscreen-")) {
        slug = slug.mid(11);
    }
    result["slug"] = slug;

    QString home = QDir::homePath();
    QStringList confCandidates = {
        home + "/.local/share/ryzora/lockscreens/qylock/" + slug + "/theme.conf",
        home + "/.local/share/ryzora/installed/" + packageId + "/files/theme.conf",
        QDir::currentPath() + "/repositories/community/packages/" + packageId + "/files/theme.conf",
        QDir::currentPath() + "/repositories/community/packages/" + packageId + "/theme.conf"
    };

    QString foundConfPath;
    for (const QString &c : confCandidates) {
        if (QFile::exists(c)) {
            foundConfPath = c;
            break;
        }
    }

    QStringList mainCandidates = {
        home + "/.local/share/ryzora/lockscreens/qylock/" + slug + "/Main.qml",
        home + "/.local/share/ryzora/installed/" + packageId + "/files/Main.qml",
        QDir::currentPath() + "/repositories/community/packages/" + packageId + "/files/Main.qml",
        QDir::currentPath() + "/repositories/community/packages/" + packageId + "/Main.qml"
    };

    bool hasMainQml = false;
    for (const QString &m : mainCandidates) {
        if (QFile::exists(m)) {
            hasMainQml = true;
            break;
        }
    }

    if (!hasMainQml || foundConfPath.isEmpty()) {
        result["error"] = "Theme files or configuration missing";
        return result;
    }

    // Parse existing theme.conf while preserving all raw key-value pairs
    QSettings settings(foundConfPath, QSettings::IniFormat);
    settings.beginGroup("General");
    QStringList keys = settings.allKeys();
    QVariantMap rawConfig;
    for (const QString &k : keys) {
        rawConfig[k] = settings.value(k);
    }
    settings.endGroup();
    result["config"] = rawConfig;
    result["config_path"] = foundConfPath;

    // Define typed capability model based on verified theme engine integration
    QVariantList settingsList;
    QVariantList capsList;

    if (slug == "clockwork-orbital") {
        capsList.append("themeMode");
        capsList.append("enableWindup");

        QVariantMap sMode;
        sMode["id"] = "themeMode";
        sMode["label"] = "Color Mode";
        sMode["help"] = "Choose dark or light background and dial styling";
        sMode["type"] = "enum";
        sMode["options"] = QVariantList{"dark", "light"};
        sMode["default_value"] = "dark";
        QString curMode = rawConfig.value("themeMode", "dark").toString();
        if (curMode != "dark" && curMode != "light") curMode = "dark";
        sMode["current_value"] = curMode;
        sMode["live_preview_supported"] = true;
        sMode["requires_activation"] = true;
        sMode["category"] = "Appearance";
        settingsList.append(sMode);

        QVariantMap sWindup;
        sWindup["id"] = "enableWindup";
        sWindup["label"] = "Dial Windup Animation";
        sWindup["help"] = "Play dynamic sweep animation when the login screen initializes";
        sWindup["type"] = "bool";
        sWindup["default_value"] = true;
        bool curWindup = true;
        if (rawConfig.contains("enableWindup")) {
            QString v = rawConfig.value("enableWindup").toString().toLower();
            curWindup = (v != "false" && v != "0");
        }
        sWindup["current_value"] = curWindup;
        sWindup["live_preview_supported"] = true;
        sWindup["requires_activation"] = true;
        sWindup["category"] = "Animation";
        settingsList.append(sWindup);
    } else if (slug == "clockwork-tape") {
        capsList.append("themeMode");

        QVariantMap sMode;
        sMode["id"] = "themeMode";
        sMode["label"] = "Color Mode";
        sMode["help"] = "Choose dark or light background and cassette tape styling";
        sMode["type"] = "enum";
        sMode["options"] = QVariantList{"dark", "light"};
        sMode["default_value"] = "dark";
        QString curMode = rawConfig.value("themeMode", "dark").toString();
        if (curMode != "dark" && curMode != "light") curMode = "dark";
        sMode["current_value"] = curMode;
        sMode["live_preview_supported"] = true;
        sMode["requires_activation"] = true;
        sMode["category"] = "Appearance";
        settingsList.append(sMode);
    } else if (slug == "genshin") {
        capsList.append("background_mode");
        capsList.append("background_index");

        QVariantMap sBgMode;
        sBgMode["id"] = "background_mode";
        sBgMode["label"] = "Wallpaper Cycle Mode";
        sBgMode["help"] = "Cycle scenic backdrops according to real-world time or randomly";
        sBgMode["type"] = "enum";
        sBgMode["options"] = QVariantList{"time", "random"};
        sBgMode["default_value"] = "time";
        QString curMode = rawConfig.value("background_mode", "time").toString();
        if (curMode != "time" && curMode != "random") curMode = "time";
        sBgMode["current_value"] = curMode;
        sBgMode["live_preview_supported"] = true;
        sBgMode["requires_activation"] = true;
        sBgMode["category"] = "Wallpaper";
        settingsList.append(sBgMode);

        QVariantMap sBgIdx;
        sBgIdx["id"] = "background_index";
        sBgIdx["label"] = "Scene Selection";
        sBgIdx["help"] = "Manual scene index (1: Morning, 2: Day, 3: Sunset, 4: Night, 5: Late Night, 6: Dawn)";
        sBgIdx["type"] = "int";
        sBgIdx["min"] = 1;
        sBgIdx["max"] = 6;
        sBgIdx["default_value"] = 2;
        int curIdx = rawConfig.value("background_index", 2).toInt();
        if (curIdx < 1 || curIdx > 6) curIdx = 2;
        sBgIdx["current_value"] = curIdx;
        sBgIdx["live_preview_supported"] = true;
        sBgIdx["requires_activation"] = true;
        sBgIdx["category"] = "Wallpaper";
        settingsList.append(sBgIdx);
    } else if (slug == "terraria") {
        capsList.append("background_mode");
        capsList.append("background_index");

        QVariantMap sBgMode;
        sBgMode["id"] = "background_mode";
        sBgMode["label"] = "Wallpaper Cycle Mode";
        sBgMode["help"] = "Cycle biome backdrops randomly or maintain manual selection";
        sBgMode["type"] = "enum";
        sBgMode["options"] = QVariantList{"random", "manual"};
        sBgMode["default_value"] = "random";
        QString curMode = rawConfig.value("background_mode", "random").toString();
        if (curMode != "random" && curMode != "manual") curMode = "random";
        sBgMode["current_value"] = curMode;
        sBgMode["live_preview_supported"] = true;
        sBgMode["requires_activation"] = true;
        sBgMode["category"] = "Wallpaper";
        settingsList.append(sBgMode);

        QVariantMap sBgIdx;
        sBgIdx["id"] = "background_index";
        sBgIdx["label"] = "Biome Scene";
        sBgIdx["help"] = "Selected Terraria biome background (1 to 5)";
        sBgIdx["type"] = "int";
        sBgIdx["min"] = 1;
        sBgIdx["max"] = 5;
        sBgIdx["default_value"] = 1;
        int curIdx = rawConfig.value("background_index", 1).toInt();
        if (curIdx < 1 || curIdx > 5) curIdx = 1;
        sBgIdx["current_value"] = curIdx;
        sBgIdx["live_preview_supported"] = true;
        sBgIdx["requires_activation"] = true;
        sBgIdx["category"] = "Wallpaper";
        settingsList.append(sBgIdx);
    } else if (slug == "osu" || slug == "osumania") {
        capsList.append("gameMode");

        QVariantMap sGm;
        sGm["id"] = "gameMode";
        sGm["label"] = "Interactive Mode";
        sGm["help"] = "Playable rhythm mini-game or clean static display";
        sGm["type"] = "enum";
        sGm["options"] = QVariantList{"game", "menu"};
        sGm["default_value"] = "game";
        QString curMode = rawConfig.value("gameMode", "game").toString();
        if (curMode != "game" && curMode != "menu") curMode = "game";
        sGm["current_value"] = curMode;
        sGm["live_preview_supported"] = true;
        sGm["requires_activation"] = true;
        sGm["category"] = "Gameplay";
        settingsList.append(sGm);
    }

    result["has_settings"] = !settingsList.isEmpty();
    result["capabilities"] = capsList;
    result["settings"] = settingsList;
    return result;
}

QVariantMap RyzoraBackend::saveThemeConfig(const QString &packageId, const QVariantMap &config) {
    QVariantMap res;
    res["success"] = false;
    if (m_isShutdown.load()) {
        res["message"] = "Application is shutting down.";
        return res;
    }

    if (packageId.isEmpty() || packageId.contains("..") || packageId.contains("/") || packageId.contains("\\")) {
        res["message"] = "Invalid package identifier.";
        return res;
    }

    QVariantMap themeModel = getThemeConfig(packageId);
    if (!themeModel.value("has_settings").toBool()) {
        res["message"] = QString("Theme '%1' does not declare configurable capabilities.").arg(packageId);
        return res;
    }

    QVariantList settingsList = themeModel.value("settings").toList();
    QMap<QString, QVariantMap> specMap;
    for (const QVariant &s : settingsList) {
        QVariantMap sm = s.toMap();
        specMap[sm.value("id").toString()] = sm;
    }

    // Validate all submitted fields against their capability schema
    QVariantMap validatedUpdates;
    for (auto it = config.constBegin(); it != config.constEnd(); ++it) {
        QString key = it.key();
        if (!specMap.contains(key)) continue; // ignore non-capability keys

        QVariantMap spec = specMap[key];
        QString type = spec.value("type").toString();
        QVariant val = it.value();

        if (type == "enum") {
            QString strVal = val.toString();
            QStringList allowed;
            for (const QVariant &opt : spec.value("options").toList()) {
                allowed.append(opt.toString());
            }
            if (!allowed.contains(strVal)) {
                res["message"] = QString("Validation Error: '%1' is not a valid option for '%2' (allowed: %3)")
                                     .arg(strVal, key, allowed.join(", "));
                return res;
            }
            validatedUpdates[key] = strVal;
        } else if (type == "bool") {
            bool boolVal = (val.toString().toLower() != "false" && val.toBool() != false);
            validatedUpdates[key] = boolVal ? "true" : "false";
        } else if (type == "int") {
            int intVal = val.toInt();
            int minVal = spec.value("min").toInt();
            int maxVal = spec.value("max").toInt();
            if (intVal < minVal || intVal > maxVal) {
                res["message"] = QString("Validation Error: '%1' must be an integer between %2 and %3 (got %4)")
                                     .arg(key).arg(minVal).arg(maxVal).arg(intVal);
                return res;
            }
            validatedUpdates[key] = QString::number(intVal);
        }
    }

    if (validatedUpdates.isEmpty()) {
        res["message"] = "No valid configurable parameters were provided.";
        return res;
    }

    QString slug = themeModel.value("slug").toString();
    QString userThemeDir = QDir::homePath() + "/.local/share/ryzora/lockscreens/qylock/" + slug;
    QString userConf = userThemeDir + "/theme.conf";
    QFileInfo fi(userConf);
    if (!fi.dir().exists()) {
        fi.dir().mkpath(".");
    }

    // Preserve all existing configuration without evaluating arbitrary code
    QSettings settings(userConf, QSettings::IniFormat);
    settings.beginGroup("General");
    for (auto it = validatedUpdates.constBegin(); it != validatedUpdates.constEnd(); ++it) {
        settings.setValue(it.key(), it.value());
    }
    settings.endGroup();
    settings.sync();

    if (settings.status() != QSettings::NoError) {
        res["message"] = QString("Filesystem Error: Failed to write configuration to '%1'").arg(userConf);
        return res;
    }

    // Check if this theme is currently active in SDDM
    bool isActiveInSddm = false;
    QString activeSddmPkg = m_activeConfiguration.value("sddm_login").toString();
    if (activeSddmPkg == packageId || m_activeConfiguration.value("active_package_id").toString() == packageId) {
        isActiveInSddm = true;
    }

    if (isActiveInSddm) {
        // Re-apply to update live SDDM theme staging safely via the privileged helper
        applyTarget(packageId, "sddm_login");
        res["message"] = "Theme customization saved. Preview updated immediately; active SDDM login screen re-synchronized.";
    } else {
        res["message"] = "Theme customization saved. Preview updated immediately.";
    }

    res["success"] = true;
    res["active_in_sddm"] = isActiveInSddm;
    res["live_preview_ready"] = true;
    res["updated_keys"] = validatedUpdates.keys();

    postNotification("success", "Customization Saved", res.value("message").toString());
    return res;
}

