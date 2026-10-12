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
    void ryzora_spike_init();
    uint64_t ryzora_spike_allocate_request_id();
    bool ryzora_spike_is_process_active();
    uint32_t ryzora_spike_active_pid();
    char* ryzora_spike_query_catalog(const char* repo_root, const char* category, const char* search);
    char* ryzora_spike_load_library(bool force_refresh);
    char* ryzora_spike_run_fastfetch(const char* preset, uint64_t req_id);
    bool ryzora_spike_cancel_fastfetch();
    void ryzora_spike_shutdown();
    void ryzora_spike_free_string(char* s);

    char* ryzora_spike_load_snapshots();
    char* ryzora_spike_load_settings();
    bool ryzora_spike_save_settings(const char* settings_json);
    char* ryzora_spike_reset_settings();
    char* ryzora_spike_load_appearance();
    bool ryzora_spike_save_appearance(const char* appearance_json);
    char* ryzora_spike_detect_system_appearance();
    char* ryzora_spike_load_compatibility_profiles();
    char* ryzora_spike_load_system_info();
    char* ryzora_spike_load_updates();
    char* ryzora_spike_list_presets();
    char* ryzora_spike_install_package(const char* package_id);
    char* ryzora_spike_uninstall_package(const char* package_id);
    char* ryzora_spike_deactivate_target(const char* target);
    char* ryzora_spike_create_snapshot(const char* reason);
    char* ryzora_spike_restore_snapshot(const char* id);
    char* ryzora_spike_delete_snapshot(const char* id);
    char* ryzora_spike_apply_update(const char* package_id);
    char* ryzora_spike_apply_all_updates();
    char* ryzora_spike_load_active_configuration();
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

    ryzora_spike_init();
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
    ryzora_spike_shutdown();
    m_threadPool.clear();
    m_threadPool.waitForDone(300);
}

bool RyzoraBackend::isProcessActive() const {
    return ryzora_spike_is_process_active();
}

uint32_t RyzoraBackend::activePid() const {
    return ryzora_spike_active_pid();
}

void RyzoraBackend::filterCatalog(const QString &category, const QString &query) {
    if (m_isShutdown.load()) return;
    uint64_t reqId = ryzora_spike_allocate_request_id();
    updateMonotonicReqId(m_currentCatalogReqId, reqId);
    m_isCatalogLoading = true;
    emit catalogLoadingChanged();

    QPointer<RyzoraBackend> self = this;
    QString repoRoot = QDir::currentPath();

    m_threadPool.start([self, repoRoot, category, query, reqId]() {
        QByteArray bRoot = repoRoot.toUtf8();
        QByteArray bCat = category.toUtf8();
        QByteArray bQuery = query.toUtf8();

        char* rawJson = ryzora_spike_query_catalog(bRoot.constData(), bCat.constData(), bQuery.constData());
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_spike_free_string(rawJson);

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
    uint64_t reqId = ryzora_spike_allocate_request_id();
    updateMonotonicReqId(m_currentLibraryReqId, reqId);
    m_isLibraryLoading = true;
    m_libraryError = "";
    emit libraryLoadingChanged();
    emit libraryErrorChanged();

    QPointer<RyzoraBackend> self = this;

    m_threadPool.start([self, forceRefresh, reqId]() {
        char* rawJson = ryzora_spike_load_library(forceRefresh);
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_spike_free_string(rawJson);

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

        char *rawConfig = ryzora_spike_load_active_configuration();
        QString configStr = QString::fromUtf8(rawConfig);
        ryzora_spike_free_string(rawConfig);
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
    uint64_t reqId = ryzora_spike_allocate_request_id();
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
        char* rawJson = ryzora_spike_run_fastfetch(bPreset.constData(), reqId);
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_spike_free_string(rawJson);

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
    ryzora_spike_cancel_fastfetch();
    emit fastfetchLoadingChanged();
    emit statusMessageChanged();
}

void RyzoraBackend::loadSnapshots() {
    if (m_isShutdown.load()) return;
    m_isSnapshotsLoading = true;
    emit snapshotsLoadingChanged();

    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char* rawJson = ryzora_spike_load_snapshots();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_spike_free_string(rawJson);

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
        char* rawJson = ryzora_spike_load_updates();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_spike_free_string(rawJson);

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
        char* rawJson = ryzora_spike_load_settings();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_spike_free_string(rawJson);

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
        bool ok = ryzora_spike_save_settings(jsonBytes.constData());
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
    char* rawJson = ryzora_spike_reset_settings();
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
    char* rawJson = ryzora_spike_load_appearance();
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
    ryzora_spike_save_appearance(jsonBytes.constData());
}

QString RyzoraBackend::detectSystemAppearance() {
    char* str = ryzora_spike_detect_system_appearance();
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
        char* rawJson = ryzora_spike_list_presets();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_spike_free_string(rawJson);

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
        char* rawJson = ryzora_spike_load_compatibility_profiles();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_spike_free_string(rawJson);

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
        char* rawJson = ryzora_spike_load_system_info();
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_spike_free_string(rawJson);

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

void RyzoraBackend::installPackage(const QString &packageId) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self, packageId]() {
        QByteArray idBytes = packageId.toUtf8();
        char* rawRes = ryzora_spike_install_package(idBytes.constData());
        if (rawRes) {
            QString resStr = QString::fromUtf8(rawRes);
            ryzora_spike_free_string(rawRes);
            QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
            bool success = doc.object().value("success").toBool();
            QString msg = doc.object().value("message").toString();

            QMetaObject::invokeMethod(qApp, [self, success, msg]() {
                if (self) {
                    self->postNotification(success ? "success" : "error",
                                           success ? "Installation Complete" : "Installation Failed",
                                           msg);
                    self->loadLibrary(true);
                    self->filterCatalog("all", "");
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
        char* rawRes = ryzora_spike_uninstall_package(idBytes.constData());
        if (rawRes) {
            QString resStr = QString::fromUtf8(rawRes);
            ryzora_spike_free_string(rawRes);
            QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
            bool success = doc.object().value("success").toBool();
            QString msg = doc.object().value("message").toString();

            QMetaObject::invokeMethod(qApp, [self, success, msg]() {
                if (self) {
                    self->postNotification(success ? "info" : "error",
                                           success ? "Uninstalled" : "Uninstall Refused",
                                           msg);
                    self->loadLibrary(true);
                    self->filterCatalog("all", "");
                }
            });
        }
    });
}

void RyzoraBackend::deactivateTarget(const QString &target) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self, target]() {
        QByteArray targetBa = target.toUtf8();
        char *rawRes = ryzora_spike_deactivate_target(targetBa.constData());
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_spike_free_string(rawRes);
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        bool success = doc.object().value("success").toBool();
        QString msg = doc.object().value("message").toString();

        QMetaObject::invokeMethod(qApp, [self, success, msg]() {
            if (self) {
                self->postNotification(success ? "success" : "error",
                                       success ? "Target Deactivated" : "Deactivation Failed",
                                       msg);
                self->loadLibrary(true);
            }
        });
    });
}

void RyzoraBackend::createSnapshot(const QString &reason) {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self(this);
    m_threadPool.start([self, reason]() {
        QByteArray reasonBa = reason.toUtf8();
        char *rawRes = ryzora_spike_create_snapshot(reasonBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_spike_free_string(rawRes);
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
        char *rawRes = ryzora_spike_restore_snapshot(idBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_spike_free_string(rawRes);
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
        char *rawRes = ryzora_spike_delete_snapshot(idBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_spike_free_string(rawRes);
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
        char *rawRes = ryzora_spike_apply_update(idBa.constData());
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_spike_free_string(rawRes);
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
        char *rawRes = ryzora_spike_apply_all_updates();
        if (!rawRes) return;
        QString resStr = QString::fromUtf8(rawRes);
        ryzora_spike_free_string(rawRes);
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
