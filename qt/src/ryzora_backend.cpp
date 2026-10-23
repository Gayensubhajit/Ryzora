#include <QFileInfo>
#include <QDir>
#include <QUrl>
#include <QSaveFile>
#include <QSettings>
#include <QStandardPaths>
#include <QProcess>
#include "ryzora_backend.hpp"
#include <QDateTime>
#include <QElapsedTimer>
#include <QCryptographicHash>
#include "preview_stubs.hpp"
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
    char* ryzora_list_fastfetch_installed_presets();
    char* ryzora_list_fastfetch_local_emblems();
    char* ryzora_get_fastfetch_preset_config(const char* preset);
    char* ryzora_get_fastfetch_saved_config();
    char* ryzora_run_fastfetch_draft(const char* draft, uint64_t req_id);
    char* ryzora_save_fastfetch_configuration(const char* config, const char* preset_id);
    char* ryzora_reset_fastfetch_default();
    char* ryzora_remove_installed_fastfetch_style(const char* preset_id);
    char* ryzora_apply_fastfetch_preset(const char* preset);
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
    bool ryzora_is_silentsddm_engine_installed();
    char* ryzora_install_silentsddm_engine();
    char* ryzora_uninstall_silentsddm_engine();
}

void RyzoraBackend::updateMonotonicReqId(std::atomic<uint64_t> &target, uint64_t newId) {
    uint64_t current = target.load(std::memory_order_relaxed);
    while (newId > current && !target.compare_exchange_weak(current, newId, std::memory_order_release, std::memory_order_relaxed)) {
        // loop until current >= newId
    }
}

RyzoraBackend::RyzoraBackend(QObject *parent) : QObject(parent) {
    m_threadPool.setMaxThreadCount(12);
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
    loadFastfetchInstalledPresets();
    loadFastfetchLocalEmblems();
    getFastfetchSavedConfig();
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
                self->m_fastfetchHtml = html;
                if (cancelled) {
                    self->m_statusMessage = QString("Cancelled preview for %1").arg(presetName);
                } else if (success) {
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

bool RyzoraBackend::applyFastfetchPreset(const QString &presetName) {
    if (m_isShutdown.load()) return false;
    QByteArray ba = presetName.toUtf8();
    char* raw = ryzora_apply_fastfetch_preset(ba.constData());
    if (!raw) return false;
    QString jsonStr = QString::fromUtf8(raw);
    ryzora_free_string(raw);

    QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
    QJsonObject obj = doc.object();
    bool success = obj.value("success").toBool();
    QString msg = obj.value("message").toString();

    if (success) {
        postNotification("success", "Fastfetch Configuration", msg);
        m_statusMessage = msg;
        emit statusMessageChanged();
        getFastfetchSavedConfig();
        loadLibrary(true);
    } else {
        postNotification("warning", "Fastfetch Error", msg);
        m_statusMessage = QString("Failed to apply preset: %1").arg(msg);
        emit statusMessageChanged();
    }
    return success;
}

bool RyzoraBackend::launchFastfetchTerminal(const QString &presetName) {
    if (m_isShutdown.load()) return false;
    QString preset = presetName.trimmed();
    if (preset.isEmpty()) preset = m_selectedPreset;

    QString cmd = QString("fastfetch -c %1").arg(preset);

    QStringList terms = {"kitty", "foot", "alacritty", "konsole", "xterm", "gnome-terminal", "xfce4-terminal"};
    QString chosenTerm = "";
    for (const QString &t : terms) {
        if (!QStandardPaths::findExecutable(t).isEmpty()) {
            chosenTerm = t;
            break;
        }
    }

    if (chosenTerm.isEmpty()) {
        QString envTerm = qEnvironmentVariable("TERMINAL");
        if (!envTerm.isEmpty() && !QStandardPaths::findExecutable(envTerm).isEmpty()) {
            chosenTerm = envTerm;
        }
    }

    if (chosenTerm.isEmpty()) {
        postNotification("warning", "Terminal Not Found", "No supported terminal emulator (kitty, foot, alacritty, konsole, xterm) found.");
        return false;
    }

    bool spawned = false;
    if (chosenTerm == "kitty") {
        spawned = QProcess::startDetached("kitty", QStringList() << "--hold" << "sh" << "-c" << cmd);
    } else if (chosenTerm == "foot") {
        spawned = QProcess::startDetached("foot", QStringList() << "-H" << "sh" << "-c" << QString("%1; exec bash").arg(cmd));
    } else if (chosenTerm == "alacritty") {
        spawned = QProcess::startDetached("alacritty", QStringList() << "-e" << "sh" << "-c" << QString("%1; exec bash").arg(cmd));
    } else if (chosenTerm == "konsole") {
        spawned = QProcess::startDetached("konsole", QStringList() << "--noclose" << "-e" << "fastfetch" << "-c" << preset);
    } else {
        spawned = QProcess::startDetached(chosenTerm, QStringList() << "-e" << "sh" << "-c" << QString("%1; exec bash").arg(cmd));
    }

    if (spawned) {
        postNotification("info", "Fastfetch Terminal", QString("Launched %1 with preset %2").arg(chosenTerm, preset));
        return true;
    } else {
        postNotification("warning", "Terminal Launch Failed", QString("Could not launch %1").arg(chosenTerm));
        return false;
    }
}

void RyzoraBackend::requestCustomFastfetchPreview(const QString &structure, const QString &logo, const QString &color) {
    QString customPreset = QString("custom:structure=%1;logo=%2;color=%3").arg(structure, logo, color);
    requestFastfetchPreview(customPreset);
}

void RyzoraBackend::loadFastfetchInstalledPresets() {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char* raw = ryzora_list_fastfetch_installed_presets();
        if (!raw) return;
        QString jsonStr = QString::fromUtf8(raw);
        ryzora_free_string(raw);
        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantList list = doc.array().toVariantList();
        QMetaObject::invokeMethod(qApp, [self, list]() {
            if (self) {
                self->m_fastfetchInstalledPresets = list;
                emit self->fastfetchInstalledPresetsChanged();
            }
        });
    });
}

void RyzoraBackend::loadFastfetchLocalEmblems() {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char* raw = ryzora_list_fastfetch_local_emblems();
        if (!raw) return;
        QString jsonStr = QString::fromUtf8(raw);
        ryzora_free_string(raw);
        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QVariantList list = doc.array().toVariantList();
        QMetaObject::invokeMethod(qApp, [self, list]() {
            if (self) {
                self->m_fastfetchLocalEmblems = list;
                emit self->fastfetchLocalEmblemsChanged();
            }
        });
    });
}

QString RyzoraBackend::getFastfetchPresetConfig(const QString &presetId) {
    QByteArray ba = presetId.toUtf8();
    char* raw = ryzora_get_fastfetch_preset_config(ba.constData());
    if (!raw) return QString();
    QString res = QString::fromUtf8(raw);
    ryzora_free_string(raw);
    return res;
}

QString RyzoraBackend::getFastfetchSavedConfig() {
    char* raw = ryzora_get_fastfetch_saved_config();
    if (!raw) return QString();
    QString res = QString::fromUtf8(raw);
    ryzora_free_string(raw);
    m_fastfetchSavedConfig = res;
    emit fastfetchSavedConfigChanged();
    return res;
}

QByteArray RyzoraBackend::fastfetchPreviewCacheKey(const QString &draftJson) const {
    static const QByteArray binIdentity = []() {
        const QString exe = QStandardPaths::findExecutable(QStringLiteral("fastfetch"));
        const QFileInfo fi(exe);
        return (exe + QLatin1Char('|') + QString::number(fi.size()) + QLatin1Char('|')
                + QString::number(fi.lastModified().toMSecsSinceEpoch())).toUtf8();
    }();
    return QCryptographicHash::hash(binIdentity + '\n' + draftJson.toUtf8(), QCryptographicHash::Sha1);
}

void RyzoraBackend::storeFastfetchPreview(const QByteArray &key, const QString &html) {
    FastfetchPreviewCacheEntry e;
    e.html = html;
    e.producedAtMs = QDateTime::currentMSecsSinceEpoch();
    m_fastfetchPreviewCacheOrder.removeAll(key);
    m_fastfetchPreviewCacheOrder.append(key);
    m_fastfetchPreviewCache.insert(key, e);
    while (m_fastfetchPreviewCacheOrder.size() > 48) {
        m_fastfetchPreviewCache.remove(m_fastfetchPreviewCacheOrder.takeFirst());
    }
}

void RyzoraBackend::requestDraftPreview(const QString &draftJson) {
    if (m_isShutdown.load()) return;
    // Cached real output is reused for this many ms before a background refresh is required
    // (system telemetry such as memory/uptime drifts over time).
    constexpr qint64 kFreshMs = 30000;
    const QByteArray key = fastfetchPreviewCacheKey(draftJson);
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    uint64_t reqId = ryzora_allocate_request_id();
    updateMonotonicReqId(m_currentFastfetchReqId, reqId);
    m_fastfetchError = "";

    auto hit = m_fastfetchPreviewCache.constFind(key);
    if (hit != m_fastfetchPreviewCache.constEnd()) {
        // Show the cached real output for this exact configuration immediately.
        m_fastfetchHtml = hit->html;
        const bool fresh = (now - hit->producedAtMs) < kFreshMs;
        m_isFastfetchLoading = !fresh;
        emit fastfetchChanged();
        emit fastfetchErrorChanged();
        emit fastfetchLoadingChanged();
        if (fresh) return;  // identical config, recent output: no process needed
    } else {
        // Keep the last valid preview on screen while the new one is produced.
        m_isFastfetchLoading = true;
        emit fastfetchErrorChanged();
        emit fastfetchLoadingChanged();
    }
    m_isFastfetchCancelled = false;

    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self, draftJson, reqId, key]() {
        QElapsedTimer timer;
        timer.start();
        QByteArray bDraft = draftJson.toUtf8();
        char* rawJson = ryzora_run_fastfetch_draft(bDraft.constData(), reqId);
        QString jsonStr = QString::fromUtf8(rawJson);
        ryzora_free_string(rawJson);

        QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
        QJsonObject obj = doc.object();
        bool success = obj.value("success").toBool();
        bool cancelled = obj.value("cancelled").toBool();
        QString html = obj.value("html_formatted").toString();
        QString err = obj.value("error").toString();
        const qint64 totalMs = timer.elapsed();
        const qint64 execMs = obj.value("execution_time_ms").toInteger();

        QMetaObject::invokeMethod(qApp, [self, reqId, success, cancelled, html, err, key, totalMs, execMs]() {
            if (!self) return;
            if (qEnvironmentVariableIsSet("RYZORA_FF_TIMING")) {
                qInfo().noquote() << QString("[ff-preview] req=%1 total=%2ms fastfetch=%3ms ok=%4 cancelled=%5")
                                         .arg(reqId).arg(totalMs).arg(execMs).arg(success).arg(cancelled);
            }
            if (success && !cancelled) {
                self->storeFastfetchPreview(key, html);
            }
            // Ignore stale results: an older process must never replace a newer preview.
            if (reqId != self->m_currentFastfetchReqId.load()) return;
            self->m_isFastfetchLoading = false;
            self->m_isFastfetchCancelled = cancelled;
            if (success) {
                self->m_fastfetchHtml = html;
                self->m_fastfetchError = "";
            } else if (!cancelled) {
                // Report the error but keep the last valid preview visible.
                self->m_fastfetchError = err.isEmpty() ? "Execution failed" : err;
                if (self->m_fastfetchHtml.isEmpty()) self->m_fastfetchHtml = html;
            }
            emit self->fastfetchLoadingChanged();
            emit self->fastfetchChanged();
            emit self->fastfetchErrorChanged();
        });
    });
}

bool RyzoraBackend::saveFastfetchConfiguration(const QString &configJson, const QString &presetId) {
    if (m_isShutdown.load()) return false;
    QByteArray bConfig = configJson.toUtf8();
    QByteArray bPreset = presetId.toUtf8();
    char* raw = ryzora_save_fastfetch_configuration(bConfig.constData(), bPreset.constData());
    if (!raw) return false;
    QString jsonStr = QString::fromUtf8(raw);
    ryzora_free_string(raw);

    QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
    QJsonObject obj = doc.object();
    bool success = obj.value("success").toBool();
    QString msg = obj.value("message").toString();

    if (success) {
        m_fastfetchSavedConfig = configJson;
        emit fastfetchSavedConfigChanged();
        postNotification("success", "Fastfetch Saved", msg);
        loadLibrary(true);
    } else {
        postNotification("warning", "Fastfetch Save Error", msg);
    }
    return success;
}

bool RyzoraBackend::resetFastfetchDefault() {
    if (m_isShutdown.load()) return false;
    char* raw = ryzora_reset_fastfetch_default();
    if (!raw) return false;
    QString jsonStr = QString::fromUtf8(raw);
    ryzora_free_string(raw);

    QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
    QJsonObject obj = doc.object();
    bool success = obj.value("success").toBool();
    QString msg = obj.value("message").toString();

    if (success) {
        getFastfetchSavedConfig();
        postNotification("info", "Fastfetch Reset", "Reset to default Fastfetch configuration.");
        loadLibrary(true);
    } else {
        postNotification("warning", "Fastfetch Reset Error", msg);
    }
    return success;
}

bool RyzoraBackend::removeInstalledFastfetchStyle(const QString &presetId) {
    if (m_isShutdown.load()) return false;
    QByteArray bPreset = presetId.toUtf8();
    char* raw = ryzora_remove_installed_fastfetch_style(bPreset.constData());
    if (!raw) return false;
    QString jsonStr = QString::fromUtf8(raw);
    ryzora_free_string(raw);

    QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
    QJsonObject obj = doc.object();
    bool success = obj.value("success").toBool();
    QString msg = obj.value("message").toString();

    if (success) {
        getFastfetchSavedConfig();
        postNotification("info", "Fastfetch Style", msg);
        loadLibrary(true);
    } else {
        postNotification("warning", "Fastfetch Error", msg);
    }
    return success;
}

bool RyzoraBackend::launchTerminalWithDraft(const QString &draftJson) {
    QString tempPath = QDir::tempPath() + QString("/ryzora_ff_draft_%1.jsonc").arg(QDateTime::currentMSecsSinceEpoch());
    QFile file(tempPath);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        postNotification("warning", "Terminal Preview", "Failed to write temporary preview config");
        return false;
    }
    file.write(draftJson.toUtf8());
    file.close();

    QStringList candidates = { "kitty", "ghostty", "wezterm", "foot", "alacritty", "ptyxis", "konsole", "gnome-terminal", "xfce4-terminal", "xterm" };
    QString chosenTerm;
    for (const QString &t : candidates) {
        if (!QStandardPaths::findExecutable(t).isEmpty()) {
            chosenTerm = t;
            break;
        }
    }

    if (chosenTerm.isEmpty()) {
        QString envTerm = qEnvironmentVariable("TERMINAL");
        if (!envTerm.isEmpty() && !QStandardPaths::findExecutable(envTerm).isEmpty()) {
            chosenTerm = envTerm;
        }
    }

    if (chosenTerm.isEmpty()) {
        postNotification("warning", "Terminal Not Found", "No supported terminal emulator found.");
        return false;
    }

    QString runCmd = QString("clear; fastfetch -c \"%1\"; echo ''; printf '\\033[2mPress Enter to close preview...\\033[0m'; read _; rm -f \"%1\"").arg(tempPath);
    bool spawned = false;
    if (chosenTerm == "kitty") {
        spawned = QProcess::startDetached("kitty", QStringList() << "-e" << "sh" << "-c" << runCmd);
    } else if (chosenTerm == "ghostty") {
        spawned = QProcess::startDetached("ghostty", QStringList() << "-e" << "sh" << "-c" << runCmd);
    } else if (chosenTerm == "wezterm") {
        spawned = QProcess::startDetached("wezterm", QStringList() << "start" << "--" << "sh" << "-c" << runCmd);
    } else if (chosenTerm == "foot") {
        spawned = QProcess::startDetached("foot", QStringList() << "sh" << "-c" << runCmd);
    } else if (chosenTerm == "alacritty") {
        spawned = QProcess::startDetached("alacritty", QStringList() << "-e" << "sh" << "-c" << runCmd);
    } else if (chosenTerm == "ptyxis") {
        spawned = QProcess::startDetached("ptyxis", QStringList() << "--" << "sh" << "-c" << runCmd);
    } else if (chosenTerm == "konsole") {
        spawned = QProcess::startDetached("konsole", QStringList() << "-e" << "sh" << "-c" << runCmd);
    } else if (chosenTerm == "gnome-terminal") {
        spawned = QProcess::startDetached("gnome-terminal", QStringList() << "--" << "sh" << "-c" << runCmd);
    } else {
        spawned = QProcess::startDetached(chosenTerm, QStringList() << "-e" << "sh" << "-c" << runCmd);
    }

    if (spawned) {
        postNotification("info", "Fastfetch Terminal", QString("Launched %1 with live draft preview").arg(chosenTerm));
        return true;
    }
    postNotification("warning", "Terminal Launch Error", QString("Failed to spawn %1").arg(chosenTerm));
    return false;
}

void RyzoraBackend::openFastfetchConfigInEditor() {
    QString configDir = QDir::homePath() + "/.config/fastfetch";
    QDir().mkpath(configDir);
    QString configPath = configDir + "/config.jsonc";
    if (!QFile::exists(configPath)) {
        resetFastfetchDefault();
    }
    QStringList guiEditors = { "code", "codium", "zed", "kate", "gedit", "mousepad", "gnome-text-editor", "xdg-open" };
    for (const QString &ed : guiEditors) {
        if (!QStandardPaths::findExecutable(ed).isEmpty()) {
            if (QProcess::startDetached(ed, QStringList() << configPath)) {
                postNotification("info", "Fastfetch Config", QString("Opened ~/.config/fastfetch/config.jsonc in %1").arg(ed));
                return;
            }
        }
    }
    postNotification("warning", "Editor Not Found", "Could not find an external editor for ~/.config/fastfetch/config.jsonc");
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
                if (!list.contains(self->m_selectedPreset)) {
                    QString defaultPreset = list.contains("archey") ? "archey"
                                          : (list.contains("all") ? "all"
                                          : (list.contains("fastfetch-preset-2") ? "fastfetch-preset-2"
                                          : list.first().toString()));
                    self->requestFastfetchPreview(defaultPreset);
                }
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
    m_testHomeOverride = path;
    if (path.isEmpty()) {
        qunsetenv("RYZORA_HOME");
        ryzora_set_test_home_override(nullptr);
    } else {
        qputenv("RYZORA_HOME", path.toUtf8());
        QByteArray b = path.toUtf8();
        ryzora_set_test_home_override(b.constData());
    }
    loadLibrary(true);
    filterCatalog("all", "");
    loadSnapshots();
}

void RyzoraBackend::setTestSystemRootOverride(const QString &path) {
    m_testSystemRootOverride = path;
    if (path.isEmpty()) {
        qunsetenv("RYZORA_SYSTEM_ROOT");
        ryzora_set_test_system_root_override(nullptr);
    } else {
        qputenv("RYZORA_SYSTEM_ROOT", path.toUtf8());
        QByteArray b = path.toUtf8();
        ryzora_set_test_system_root_override(b.constData());
    }
}

QString RyzoraBackend::resolveHomePath() const {
    if (!m_testHomeOverride.isEmpty()) return m_testHomeOverride;
    QString envHome = qEnvironmentVariable("RYZORA_HOME");
    if (!envHome.isEmpty()) return envHome;
    return QDir::homePath();
}

QString RyzoraBackend::resolveSystemRoot() const {
    if (!m_testSystemRootOverride.isEmpty()) return m_testSystemRootOverride;
    return qEnvironmentVariable("RYZORA_SYSTEM_ROOT");
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
    if (packageId.startsWith("silentsddm-")) {
        themePreviewMainUrl(packageId);
    }
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
        QString home = resolveHomePath();
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


QString RyzoraBackend::themePreviewMainUrl(const QString &packageId) const {
    // ── Qylock themes ──
    const QString qylockPrefix = QStringLiteral("lockscreen-qylock-");
    if (packageId.startsWith(qylockPrefix)) {
        const QString name = packageId.mid(qylockPrefix.size());
        if (name.isEmpty() || name.contains('/') || name.contains("..")) return QString();
        const QString home = resolveHomePath();
        QStringList candidates;
        candidates << (home + "/.local/share/ryzora/lockscreens/qylock/" + name + "/Main.qml")
                   << (home + "/.local/share/ryzora/installed/" + packageId + "/files/Main.qml")
                   << (QDir::currentPath() + "/repositories/community/packages/" + packageId + "/files/Main.qml")
                   << (QDir::currentPath() + "/dist/packages/" + packageId + "/files/Main.qml");
        for (const QString &c : candidates) {
            if (QFileInfo::exists(c)) return QUrl::fromLocalFile(c).toString();
        }
        return QString();
    }

    // ── SilentSDDM items: build a writable preview-theme dir ──
    const QString silentPrefix = QStringLiteral("silentsddm-");
    if (!packageId.startsWith(silentPrefix)) return QString();

    const QString slug = packageId.mid(silentPrefix.size());
    if (slug.isEmpty() || slug.contains('/') || slug.contains("..")) return QString();

    const QString testSysRoot = resolveSystemRoot();
    const QString silentBase = testSysRoot.isEmpty()
        ? QStringLiteral("/usr/share/sddm/themes/ryzora-silent")
        : (testSysRoot + QStringLiteral("/usr/share/sddm/themes/ryzora-silent"));
    const QString realMain = silentBase + QStringLiteral("/Main.qml");
    if (!QFileInfo::exists(realMain)) return QString();

    const QString home = resolveHomePath();
    if (home.isEmpty()) return QString();

    QString baseConf = silentBase + "/configs/" + slug + ".conf";
    if (!QFileInfo::exists(baseConf))
        baseConf = home + "/.local/share/ryzora/lockscreens/silentsddm/configs/" + slug + ".conf";
    if (!QFileInfo::exists(baseConf))
        baseConf = QDir::currentPath() + "/repositories/community/packages/" + packageId + "/files/configs/" + slug + ".conf";
    if (!QFileInfo::exists(baseConf))
        baseConf = silentBase + "/configs/default.conf";
    if (!QFileInfo::exists(baseConf)) return QString();
    if (home.isEmpty()) return QString();
    const QString previewDir = home + "/.local/share/ryzora/lockscreens/silentsddm/preview-" + slug;
    const QString previewConfsDir = previewDir + "/configs";
    QDir().mkpath(previewConfsDir);

    // Metadata.desktop: point to our preview conf
    {
        QFile mf(previewDir + "/metadata.desktop");
        if (mf.open(QIODevice::WriteOnly | QIODevice::Text)) {
            QTextStream ts(&mf);
            ts << "[SddmGreeterTheme]\nName=SilentSDDM Preview\nType=sddm-theme\n"
               << "MainScript=Main.qml\nQtVersion=6\nConfigFile=configs/ryzora-preview.conf\n";
        }
    }

    // Determine user's stored overrides from configuration.json
    QJsonObject ovr;
    {
        QString ujp = home + "/.local/share/ryzora/lockscreens/silentsddm/configuration.json";
        QFile jf(ujp);
        if (jf.open(QIODevice::ReadOnly)) {
            QJsonDocument jd = QJsonDocument::fromJson(jf.readAll());
            if (jd.isObject()) {
                QJsonObject root = jd.object();
                if (root.contains("themes") && root.value("themes").isObject()) {
                    QJsonObject themes = root.value("themes").toObject();
                    if (themes.contains(packageId)) ovr = themes.value(packageId).toObject();
                } else if (!root.contains("themes")) {
                    ovr = root;
                }
            }
        }
    }

    // Write ryzora-preview.conf: copy base conf and apply overrides
    auto asBool = [](const QJsonValue &v) -> bool {
        if (v.isBool()) return v.toBool();
        QString s = v.toString().trimmed().toLower();
        return (s == "true" || s == "1" || s == "yes");
    };
    {
        QFile baseFile(baseConf);
        QFile outFile(previewConfsDir + "/ryzora-preview.conf");
        if (baseFile.open(QIODevice::ReadOnly | QIODevice::Text) &&
            outFile.open(QIODevice::WriteOnly | QIODevice::Text)) {
            QTextStream in(&baseFile), out(&outFile);
            QString curSec;
            while (!in.atEnd()) {
                QString line = in.readLine();
                QString trimmed = line.trimmed();
                if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
                    curSec = trimmed; out << line << "\n"; continue;
                }
                if (curSec == "[General]") {
                    if (trimmed.startsWith("background-fill-mode") && ovr.contains("fillMode")) {
                        out << "background-fill-mode = \"" << ovr.value("fillMode").toString() << "\"\n"; continue;
                    }
                    if (trimmed.startsWith("enable-animations") && ovr.contains("enableAnimations")) {
                        out << "enable-animations = " << (ovr.value("enableAnimations").toBool() ? "true" : "false") << "\n"; continue;
                    }
                } else if (curSec == "[LockScreen]") {
                    if (trimmed.startsWith("blur =") && ovr.contains("blur")) {
                        out << "blur = " << ovr.value("blur").toInt() << "\n"; continue;
                    }
                    if (trimmed.startsWith("brightness =") && ovr.contains("brightness")) {
                        QString b = ovr.value("brightness").toString();
                        QString bval = (b == "dark" ? "-0.4" : (b == "dim" ? "-0.2" : (b == "bright" ? "0.2" : "0.0")));
                        out << "brightness = " << bval << "\n"; continue;
                    }
                    if (trimmed.startsWith("saturation =") && ovr.contains("saturation")) {
                        QString s = ovr.value("saturation").toString();
                        QString sval = (s == "grayscale" ? "-1.0" : (s == "muted" ? "-0.5" : (s == "vibrant" ? "0.2" : "0.0")));
                        out << "saturation = " << sval << "\n"; continue;
                    }
                } else if (curSec == "[LockScreen.Clock]") {
                    if (trimmed.startsWith("display =") && ovr.contains("clockDisplay")) {
                        out << "display = " << (asBool(ovr.value("clockDisplay")) ? "true" : "false") << "\n"; continue;
                    }
                    if (trimmed.startsWith("position =") && ovr.contains("clockPosition")) {
                        out << "position = \"" << ovr.value("clockPosition").toString() << "\"\n"; continue;
                    }
                    if (trimmed.startsWith("format =") && ovr.contains("clockFormat")) {
                        out << "format = \"" << ovr.value("clockFormat").toString() << "\"\n"; continue;
                    }
                } else if (curSec == "[LockScreen.Date]") {
                    if (trimmed.startsWith("display =") && ovr.contains("dateDisplay")) {
                        out << "display = " << (asBool(ovr.value("dateDisplay")) ? "true" : "false") << "\n"; continue;
                    }
                } else if (curSec == "[LockScreen.Message]") {
                    if (trimmed.startsWith("display =") && ovr.contains("messageDisplay")) {
                        out << "display = " << (asBool(ovr.value("messageDisplay")) ? "true" : "false") << "\n"; continue;
                    }
                    if (trimmed.startsWith("text =") && ovr.contains("messageText")) {
                        out << "text = \"" << ovr.value("messageText").toString() << "\"\n"; continue;
                    }
                } else if (curSec == "[LoginScreen.LoginArea]") {
                    if (trimmed.startsWith("position =") && ovr.contains("loginAreaPosition")) {
                        out << "position = \"" << ovr.value("loginAreaPosition").toString() << "\"\n"; continue;
                    }
                } else if (curSec == "[LoginScreen.LoginArea.Avatar]") {
                    if (trimmed.startsWith("shape =") && ovr.contains("avatarShape")) {
                        out << "shape = \"" << ovr.value("avatarShape").toString() << "\"\n"; continue;
                    }
                }
                out << line << "\n";
            }
        }
    }

    // Load preview config into C++ PreviewConfig stub
    if (m_previewConfig) {
        m_previewConfig->load(previewConfsDir + "/ryzora-preview.conf");
    }

    // Write patched Main.qml to prevent mutual recursion bug and allow video playback
    {
        QFile mainFile(realMain);
        if (mainFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
            QString content = QString::fromUtf8(mainFile.readAll());
            content.replace(
                "onSourceChanged: {\n                updateVideo();\n            }",
                "onTsourceChanged: {\n                updateVideo();\n            }"
            );
            content.replace(
                "if (placeholder.length > 0)\n                        source = \"backgrounds/\" + placeholder;",
                "if (placeholder.length > 0 && source != (\"backgrounds/\" + placeholder))\n                        source = \"backgrounds/\" + placeholder;"
            );
            content.replace(
                "visible: parent.displayColor || (backgroundVideo.visible && parent.placeholder.length === 0)",
                "visible: parent.displayColor && !parent.isVideo"
            );
            content.replace(
                "autoPlay: false",
                "autoPlay: true"
            );
            content.replace(
                "property string tsource: root.state === \"lockState\" ? Config.lockScreenBackground : Config.loginScreenBackground",
                QString("property string tsource: (root.state === \"lockState\" ? Config.lockScreenBackground : Config.loginScreenBackground) || \"%1.mp4\"").arg(slug)
            );
            if (!content.contains("visible: (!backgroundImage.isVideo")) {
                content.replace(
                    "// Background effects\n            id: backgroundEffect\n            source: backgroundImage\n            anchors.fill: parent",
                    "// Background effects\n            id: backgroundEffect\n            source: backgroundImage\n            anchors.fill: parent\n            visible: (!backgroundImage.isVideo || backgroundVideo.playbackState !== MediaPlayer.PlayingState) && !backgroundImage.displayColor"
                );
            }
            const QString targetVis = "visible: (!backgroundImage.isVideo || backgroundVideo.playbackState !== MediaPlayer.PlayingState) && !backgroundImage.displayColor";
            const QString dupVis = targetVis + "\n            " + targetVis;
            while (content.contains(dupVis)) {
                content.replace(dupVis, targetVis);
            }
            QFile outMain(previewDir + "/Main.qml");
            if (outMain.open(QIODevice::WriteOnly | QIODevice::Text)) {
                outMain.write(content.toUtf8());
            }
        }
    }

    // Symlink engine resources into preview dir
    auto ensureLink = [&](const QString &target, const QString &linkPath) {
        if (!QFileInfo::exists(linkPath)) QFile::link(target, linkPath);
    };
    ensureLink(silentBase + "/qmldir", previewDir + "/qmldir");
    for (const QString &sub : {QString("fonts"), QString("icons"), QString("images")}) {
        const QString src = silentBase + "/" + sub;
        if (QFileInfo::exists(src)) ensureLink(src, previewDir + "/" + sub);
    }

    // Isolate components per preview so Config.qml singleton has a unique canonical URL
    // and binds reactively to config.revision for instant live theme switching.
    const QString previewComponentsDir = previewDir + "/components";
    QFileInfo compFi(previewComponentsDir);
    if (compFi.isSymLink()) {
        QFile::remove(previewComponentsDir);
    }
    QDir().mkpath(previewComponentsDir);

    const QString silentComponentsDir = silentBase + "/components";
    QDir scDir(silentComponentsDir);
    for (const QFileInfo &fi : scDir.entryInfoList(QDir::Files | QDir::NoDotAndDotDot)) {
        const QString fileName = fi.fileName();
        const QString targetPath = previewComponentsDir + "/" + fileName;
        if (fileName == QStringLiteral("Config.qml")) {
            QFile srcCfg(fi.absoluteFilePath());
            if (srcCfg.open(QIODevice::ReadOnly | QIODevice::Text)) {
                QFile outCfg(targetPath);
                if (outCfg.open(QIODevice::WriteOnly | QIODevice::Text)) {
                    outCfg.write(srcCfg.readAll());
                }
            }
        } else {
            ensureLink(fi.absoluteFilePath(), targetPath);
        }
    }

    // Backgrounds: create directory and symlink shipped backgrounds + user wallpapers + posters
    const QString previewBgDir = previewDir + "/backgrounds";
    QDir().mkpath(previewBgDir);
    QDir shippedBg(silentBase + "/backgrounds");
    for (const QFileInfo &fi : shippedBg.entryInfoList(QDir::Files | QDir::NoDotAndDotDot)) {
        ensureLink(fi.absoluteFilePath(), previewBgDir + "/" + fi.fileName());
    }
    QDir localBg(home + "/.local/share/ryzora/lockscreens/silentsddm/wallpapers");
    for (const QFileInfo &fi : localBg.entryInfoList(QDir::Files | QDir::NoDotAndDotDot)) {
        ensureLink(fi.absoluteFilePath(), previewBgDir + "/" + fi.fileName());
    }
    QDir localPosters(home + "/.local/share/ryzora/lockscreens/silentsddm/posters");
    for (const QFileInfo &fi : localPosters.entryInfoList(QDir::Files | QDir::NoDotAndDotDot)) {
        ensureLink(fi.absoluteFilePath(), previewBgDir + "/" + fi.fileName());
    }

    return QUrl::fromLocalFile(previewDir + "/Main.qml").toString();
}

QVariantMap RyzoraBackend::getThemeConfig(const QString &packageId) {
    QVariantMap result;
    result["has_settings"] = false;
    result["capabilities"] = QVariantList();
    result["settings"] = QVariantList();
    result["config"] = QVariantMap();
    result["package_id"] = packageId;
    result["is_modified"] = false;

    if (packageId.isEmpty() || packageId.contains("..") || packageId.contains("/") || packageId.contains("\\")) {
        result["error"] = "Invalid package identifier";
        return result;
    }

    QString slug = packageId;
    bool isSilent = false;
    if (slug.startsWith("lockscreen-qylock-")) {
        slug = slug.mid(18);
    } else if (slug.startsWith("lockscreen-")) {
        slug = slug.mid(11);
    } else if (slug.startsWith("silentsddm-")) {
        slug = slug.mid(11);
        isSilent = true;
    } else if (slug == "silentsddm-engine" || slug == "ryzora-silent") {
        isSilent = true;
    }
    result["slug"] = slug;
    result["is_silent"] = isSilent;

    QString home = resolveHomePath();
    QString testSysRoot = resolveSystemRoot();
    QString silentBase = testSysRoot.isEmpty() ? "/usr/share/sddm/themes/ryzora-silent" : (testSysRoot + "/usr/share/sddm/themes/ryzora-silent");

    // Shipped / base configuration candidate resolution
    QStringList confCandidates;
    if (isSilent) {
        confCandidates << (silentBase + "/configs/" + slug + ".conf")
                       << (home + "/.local/share/ryzora/lockscreens/silentsddm/configs/" + slug + ".conf")
                       << (QDir::currentPath() + "/repositories/community/packages/" + packageId + "/files/configs/" + slug + ".conf")
                       << (QDir::currentPath() + "/repositories/community/packages/" + packageId + "/files/" + slug + ".conf")
                       << ("/usr/share/sddm/themes/ryzora-silent/configs/" + slug + ".conf")
                       << (silentBase + "/configs/default.conf")
                       << ("/usr/share/sddm/themes/ryzora-silent/configs/default.conf")
                       << (silentBase + "/configs/ryzora-active.conf")
                       << (silentBase + "/theme.conf");
    } else {
        confCandidates << (home + "/.local/share/ryzora/lockscreens/qylock/" + slug + "/theme.conf")
                       << (home + "/.local/share/ryzora/installed/" + packageId + "/files/theme.conf")
                       << (QDir::currentPath() + "/repositories/community/packages/" + packageId + "/files/theme.conf")
                       << (QDir::currentPath() + "/repositories/community/packages/" + packageId + "/theme.conf");
    }

    QString foundConfPath;
    for (const QString &c : confCandidates) {
        if (QFile::exists(c)) {
            foundConfPath = c;
            break;
        }
    }

    QStringList mainCandidates;
    if (isSilent) {
        mainCandidates << (silentBase + "/Main.qml")
                       << ("/usr/share/sddm/themes/silent/Main.qml");
    } else {
        mainCandidates << (home + "/.local/share/ryzora/lockscreens/qylock/" + slug + "/Main.qml")
                       << (home + "/.local/share/ryzora/installed/" + packageId + "/files/Main.qml")
                       << (QDir::currentPath() + "/repositories/community/packages/" + packageId + "/files/Main.qml")
                       << (QDir::currentPath() + "/repositories/community/packages/" + packageId + "/Main.qml");
    }

    bool hasMainQml = false;
    for (const QString &m : mainCandidates) {
        if (QFile::exists(m)) {
            hasMainQml = true;
            break;
        }
    }

    // For SilentSDDM the engine Main.qml lives in a fixed system path; skip
    // the per-package hasMainQml check — the conf is sufficient.
    if (foundConfPath.isEmpty() || (!isSilent && !hasMainQml)) {
        result["error"] = "Theme files or configuration missing";
        return result;
    }

    result["config_path"] = foundConfPath;

    // Define typed capability model based on verified theme engine integration
    QVariantList settingsList;
    QVariantList capsList;
    bool isModified = false;
    QVariantMap rawConfig;
    if (!isSilent) {
        QString userConf = home + "/.local/share/ryzora/lockscreens/qylock/" + slug + "/theme.conf";
        QString confToRead = QFile::exists(userConf) ? userConf : foundConfPath;
        QSettings settings(confToRead, QSettings::IniFormat);
        settings.beginGroup("General");
        for (const QString &k : settings.allKeys()) {
            rawConfig[k] = settings.value(k);
        }
        settings.endGroup();
        result["config"] = rawConfig;
    }

    if (isSilent) {
        // Robust Section-aware INI parser for SilentSDDM configs
        QMap<QString, QMap<QString, QString>> baseIni;
        QString currentSec = "";
        QFile confFile(foundConfPath);
        if (confFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
            QTextStream in(&confFile);
            while (!in.atEnd()) {
                QString line = in.readLine().trimmed();
                if (line.startsWith('#') || line.startsWith(';') || line.isEmpty()) continue;
                if (line.startsWith('[') && line.endsWith(']')) {
                    currentSec = line;
                    continue;
                }
                int eqIdx = line.indexOf('=');
                if (eqIdx != -1) {
                    QString k = line.left(eqIdx).trimmed();
                    QString v = line.mid(eqIdx + 1).trimmed();
                    if (v.startsWith('"') && v.endsWith('"') && v.length() >= 2) {
                        v = v.mid(1, v.length() - 2);
                    }
                    baseIni[currentSec][k] = v;
                }
            }
            confFile.close();
        }

        // Try reading configuration.json if present
        QJsonObject rootJson;
        QString jsonPath = home + "/.local/share/ryzora/lockscreens/silentsddm/configuration.json";
        QFile jf(jsonPath);
        if (jf.open(QIODevice::ReadOnly)) {
            QJsonDocument jd = QJsonDocument::fromJson(jf.readAll());
            if (jd.isObject()) rootJson = jd.object();
            jf.close();
        }

        QJsonObject userOverrides;
        bool hasThemeSpecific = false;
        if (rootJson.contains("themes") && rootJson.value("themes").isObject()) {
            QJsonObject themesObj = rootJson.value("themes").toObject();
            if (themesObj.contains(packageId) && themesObj.value(packageId).isObject()) {
                userOverrides = themesObj.value(packageId).toObject();
                hasThemeSpecific = true;
            }
        }
        if (!hasThemeSpecific && !rootJson.contains("themes")) {
            userOverrides = rootJson; // legacy fallback
        }

        auto getJsonOverride = [&](const QString &key, const QStringList &nestedPath = QStringList()) -> QVariant {
            if (userOverrides.contains(key)) {
                QJsonValue v = userOverrides.value(key);
                if (v.isBool()) return v.toBool();
                if (v.isDouble()) return v.toDouble();
                if (v.isString()) return v.toString();
            }
            if (!nestedPath.isEmpty()) {
                QJsonObject cur = userOverrides;
                for (int i = 0; i < nestedPath.size() - 1; ++i) {
                    if (cur.contains(nestedPath[i]) && cur.value(nestedPath[i]).isObject()) {
                        cur = cur.value(nestedPath[i]).toObject();
                    } else {
                        return QVariant();
                    }
                }
                if (cur.contains(nestedPath.last())) {
                    QJsonValue v = cur.value(nestedPath.last());
                    if (v.isBool()) return v.toBool();
                    if (v.isDouble()) return v.toDouble();
                    if (v.isString()) return v.toString();
                }
            }
            return QVariant();
        };

        // 1. Wallpaper scaling / fill mode
        capsList.append("fillMode");
        QVariantMap sFill;
        sFill["id"] = "fillMode";
        sFill["label"] = "Wallpaper Scaling";
        sFill["help"] = "Wallpaper scaling and fit mode on SDDM login screen";
        sFill["type"] = "enum";
        sFill["options"] = QVariantList{"fill", "fit", "stretch"};
        QString defFill = baseIni.value("[General]").value("background-fill-mode", "fill");
        if (defFill != "fill" && defFill != "fit" && defFill != "stretch") defFill = "fill";
        sFill["default_value"] = defFill;
        QVariant ovFill = getJsonOverride("fillMode", {"background", "fill_mode"});
        QString curFill = ovFill.isValid() ? ovFill.toString() : defFill;
        if (curFill != "fill" && curFill != "fit" && curFill != "stretch") curFill = defFill;
        sFill["current_value"] = curFill;
        if (curFill != defFill) isModified = true;
        sFill["live_preview_supported"] = true;
        sFill["requires_activation"] = true;
        sFill["category"] = "Background";
        settingsList.append(sFill);

        // Detect video vs image background for capability filtering
        bool wpIsVideo = false;
        {
            QString bg = baseIni.value("[LockScreen]").value("background", "");
            if (bg.isEmpty()) bg = baseIni.value("[General]").value("background", "");
            wpIsVideo = bg.endsWith(".mp4", Qt::CaseInsensitive) || bg.endsWith(".webm", Qt::CaseInsensitive);
        }
        result["is_video"] = wpIsVideo;

        // 2. Enable Animations (video-only capability — image items don't animate)
        if (wpIsVideo) {
            capsList.append("enableAnimations");
            QVariantMap sAnim;
            sAnim["id"] = "enableAnimations";
            sAnim["label"] = "Enable Animations";
            sAnim["help"] = "Enable live video transitions and motion effects in SDDM greeter";
            sAnim["type"] = "bool";
            bool defAnim = (baseIni.value("[General]").value("enable-animations", "true").toLower() != "false");
            sAnim["default_value"] = defAnim;
            QVariant ovAnim = getJsonOverride("enableAnimations");
            bool curAnim = ovAnim.isValid() ? (ovAnim.toString().toLower() != "false" && ovAnim.toBool() != false) : defAnim;
            sAnim["current_value"] = curAnim;
            if (curAnim != defAnim) isModified = true;
            sAnim["live_preview_supported"] = true;
            sAnim["requires_activation"] = true;
            sAnim["category"] = "Background";
            settingsList.append(sAnim);
        }

        // 3. Background Blur
        capsList.append("blur");
        QVariantMap sBlur;
        sBlur["id"] = "blur";
        sBlur["label"] = "Background Blur";
        sBlur["help"] = "Gaussian blur intensity applied to login background (0-100)";
        sBlur["type"] = "int";
        sBlur["min"] = 0;
        sBlur["max"] = 100;
        int defBlur = baseIni.value("[LockScreen]").value("blur", baseIni.value("[LoginScreen]").value("blur", "32")).toInt();
        if (defBlur < 0 || defBlur > 100) defBlur = 32;
        sBlur["default_value"] = defBlur;
        QVariant ovBlur = getJsonOverride("blur", {"lock_screen", "blur"});
        int curBlur = ovBlur.isValid() ? ovBlur.toInt() : defBlur;
        if (curBlur < 0 || curBlur > 100) curBlur = defBlur;
        sBlur["current_value"] = curBlur;
        if (curBlur != defBlur) isModified = true;
        sBlur["live_preview_supported"] = true;
        sBlur["requires_activation"] = true;
        sBlur["category"] = "Background";
        settingsList.append(sBlur);

        // 4. Brightness
        capsList.append("brightness");
        QVariantMap sBright;
        sBright["id"] = "brightness";
        sBright["label"] = "Brightness";
        sBright["help"] = "Visual brightness adjustment of login background";
        sBright["type"] = "enum";
        sBright["options"] = QVariantList{"dark", "dim", "neutral", "bright"};
        float rawBr = baseIni.value("[LockScreen]").value("brightness", baseIni.value("[LoginScreen]").value("brightness", "0.0")).toFloat();
        QString defBright = (rawBr <= -0.3f) ? "dark" : ((rawBr <= -0.05f) ? "dim" : ((rawBr >= 0.1f) ? "bright" : "neutral"));
        sBright["default_value"] = defBright;
        QVariant ovBright = getJsonOverride("brightness", {"lock_screen", "brightness"});
        QString curBright = ovBright.isValid() ? ovBright.toString() : defBright;
        if (!sBright["options"].toList().contains(curBright)) curBright = defBright;
        sBright["current_value"] = curBright;
        if (curBright != defBright) isModified = true;
        sBright["live_preview_supported"] = true;
        sBright["requires_activation"] = true;
        sBright["category"] = "Background";
        settingsList.append(sBright);

        // 5. Saturation
        capsList.append("saturation");
        QVariantMap sSat;
        sSat["id"] = "saturation";
        sSat["label"] = "Saturation";
        sSat["help"] = "Color saturation intensity of login background";
        sSat["type"] = "enum";
        sSat["options"] = QVariantList{"grayscale", "muted", "neutral", "vibrant"};
        float rawSat = baseIni.value("[LockScreen]").value("saturation", baseIni.value("[LoginScreen]").value("saturation", "0.0")).toFloat();
        QString defSat = (rawSat <= -0.8f) ? "grayscale" : ((rawSat <= -0.2f) ? "muted" : ((rawSat >= 0.1f) ? "vibrant" : "neutral"));
        sSat["default_value"] = defSat;
        QVariant ovSat = getJsonOverride("saturation", {"lock_screen", "saturation"});
        QString curSat = ovSat.isValid() ? ovSat.toString() : defSat;
        if (!sSat["options"].toList().contains(curSat)) curSat = defSat;
        sSat["current_value"] = curSat;
        if (curSat != defSat) isModified = true;
        sSat["live_preview_supported"] = true;
        sSat["requires_activation"] = true;
        sSat["category"] = "Background";
        settingsList.append(sSat);

        // 6. Login Area Position
        capsList.append("loginAreaPosition");
        QVariantMap sLoginPos;
        sLoginPos["id"] = "loginAreaPosition";
        sLoginPos["label"] = "Login Box Alignment";
        sLoginPos["help"] = "Horizontal placement of user card and password input";
        sLoginPos["type"] = "enum";
        sLoginPos["options"] = QVariantList{"left", "center", "right"};
        QString defLoginPos = baseIni.value("[LoginScreen.LoginArea]").value("position", "center");
        if (defLoginPos != "left" && defLoginPos != "center" && defLoginPos != "right") defLoginPos = "center";
        sLoginPos["default_value"] = defLoginPos;
        QVariant ovLoginPos = getJsonOverride("loginAreaPosition", {"login_screen", "login_area", "position"});
        QString curLoginPos = ovLoginPos.isValid() ? ovLoginPos.toString() : defLoginPos;
        if (curLoginPos != "left" && curLoginPos != "center" && curLoginPos != "right") curLoginPos = defLoginPos;
        sLoginPos["current_value"] = curLoginPos;
        if (curLoginPos != defLoginPos) isModified = true;
        sLoginPos["live_preview_supported"] = true;
        sLoginPos["requires_activation"] = true;
        sLoginPos["category"] = "Login Panel";
        settingsList.append(sLoginPos);

        // 7. Avatar Shape
        capsList.append("avatarShape");
        QVariantMap sAvShape;
        sAvShape["id"] = "avatarShape";
        sAvShape["label"] = "Avatar Geometry";
        sAvShape["help"] = "User profile picture mask shape";
        sAvShape["type"] = "enum";
        sAvShape["options"] = QVariantList{"circle", "square"};
        QString defAvShape = baseIni.value("[LoginScreen.LoginArea.Avatar]").value("shape", "circle");
        if (defAvShape != "circle" && defAvShape != "square") defAvShape = "circle";
        sAvShape["default_value"] = defAvShape;
        QVariant ovAvShape = getJsonOverride("avatarShape", {"login_screen", "avatar", "shape"});
        QString curAvShape = ovAvShape.isValid() ? ovAvShape.toString() : defAvShape;
        if (curAvShape != "circle" && curAvShape != "square") curAvShape = defAvShape;
        sAvShape["current_value"] = curAvShape;
        if (curAvShape != defAvShape) isModified = true;
        sAvShape["live_preview_supported"] = true;
        sAvShape["requires_activation"] = true;
        sAvShape["category"] = "Login Panel";
        settingsList.append(sAvShape);

        // 8. Clock Display
        capsList.append("clockDisplay");
        QVariantMap sClockDisp;
        sClockDisp["id"] = "clockDisplay";
        sClockDisp["label"] = "Display Clock";
        sClockDisp["help"] = "Show real-time digital clock on lockscreen";
        sClockDisp["type"] = "bool";
        bool defClockDisp = (baseIni.value("[LockScreen.Clock]").value("display", "true").toLower() != "false");
        sClockDisp["default_value"] = defClockDisp;
        QVariant ovClockDisp = getJsonOverride("clockDisplay", {"lock_screen", "clock", "display"});
        bool curClockDisp = ovClockDisp.isValid() ? (ovClockDisp.toString().toLower() != "false" && ovClockDisp.toBool() != false) : defClockDisp;
        sClockDisp["current_value"] = curClockDisp;
        if (curClockDisp != defClockDisp) isModified = true;
        sClockDisp["live_preview_supported"] = true;
        sClockDisp["requires_activation"] = true;
        sClockDisp["category"] = "Clock & Date";
        settingsList.append(sClockDisp);

        // 9. Clock Position
        capsList.append("clockPosition");
        QVariantMap sClockPos;
        sClockPos["id"] = "clockPosition";
        sClockPos["label"] = "Clock Position";
        sClockPos["help"] = "Anchor position of clock on lockscreen";
        sClockPos["type"] = "enum";
        sClockPos["options"] = QVariantList{
            "top-left", "top-center", "top-right",
            "center-left", "center", "center-right",
            "bottom-left", "bottom-center", "bottom-right"
        };
        QString defClockPos = baseIni.value("[LockScreen.Clock]").value("position", "top-center");
        if (!sClockPos["options"].toList().contains(defClockPos)) defClockPos = "top-center";
        sClockPos["default_value"] = defClockPos;
        QVariant ovClockPos = getJsonOverride("clockPosition", {"lock_screen", "clock", "position"});
        QString curClockPos = ovClockPos.isValid() ? ovClockPos.toString() : defClockPos;
        if (!sClockPos["options"].toList().contains(curClockPos)) curClockPos = defClockPos;
        sClockPos["current_value"] = curClockPos;
        if (curClockPos != defClockPos) isModified = true;
        sClockPos["live_preview_supported"] = true;
        sClockPos["requires_activation"] = true;
        sClockPos["category"] = "Clock & Date";
        settingsList.append(sClockPos);

        // 10. Clock Format
        capsList.append("clockFormat");
        QVariantMap sClockFmt;
        sClockFmt["id"] = "clockFormat";
        sClockFmt["label"] = "Clock Format";
        sClockFmt["help"] = "Time formatting display string";
        sClockFmt["type"] = "enum";
        sClockFmt["options"] = QVariantList{"hh:mm", "hh:mm:ss", "hh:mm AP", "hh:mm:ss AP"};
        QString defClockFmt = baseIni.value("[LockScreen.Clock]").value("format", "hh:mm");
        if (!sClockFmt["options"].toList().contains(defClockFmt)) defClockFmt = "hh:mm";
        sClockFmt["default_value"] = defClockFmt;
        QVariant ovClockFmt = getJsonOverride("clockFormat", {"lock_screen", "clock", "format"});
        QString curClockFmt = ovClockFmt.isValid() ? ovClockFmt.toString() : defClockFmt;
        if (!sClockFmt["options"].toList().contains(curClockFmt)) curClockFmt = defClockFmt;
        sClockFmt["current_value"] = curClockFmt;
        if (curClockFmt != defClockFmt) isModified = true;
        sClockFmt["live_preview_supported"] = true;
        sClockFmt["requires_activation"] = true;
        sClockFmt["category"] = "Clock & Date";
        settingsList.append(sClockFmt);

        // 11. Date Display
        capsList.append("dateDisplay");
        QVariantMap sDateDisp;
        sDateDisp["id"] = "dateDisplay";
        sDateDisp["label"] = "Display Date";
        sDateDisp["help"] = "Show calendar date under the clock";
        sDateDisp["type"] = "bool";
        bool defDateDisp = (baseIni.value("[LockScreen.Date]").value("display", "true").toLower() != "false");
        sDateDisp["default_value"] = defDateDisp;
        QVariant ovDateDisp = getJsonOverride("dateDisplay", {"lock_screen", "date", "display"});
        bool curDateDisp = ovDateDisp.isValid() ? (ovDateDisp.toString().toLower() != "false" && ovDateDisp.toBool() != false) : defDateDisp;
        sDateDisp["current_value"] = curDateDisp;
        if (curDateDisp != defDateDisp) isModified = true;
        sDateDisp["live_preview_supported"] = true;
        sDateDisp["requires_activation"] = true;
        sDateDisp["category"] = "Clock & Date";
        settingsList.append(sDateDisp);

        // 12. Date Format
        capsList.append("dateFormat");
        QVariantMap sDateFmt;
        sDateFmt["id"] = "dateFormat";
        sDateFmt["label"] = "Date Format";
        sDateFmt["help"] = "Date representation format pattern";
        sDateFmt["type"] = "enum";
        sDateFmt["options"] = QVariantList{"dddd, MMMM dd, yyyy", "dddd, MMMM d", "MM/dd/yyyy", "dd/MM/yyyy"};
        QString defDateFmt = baseIni.value("[LockScreen.Date]").value("format", "dddd, MMMM dd, yyyy");
        if (!sDateFmt["options"].toList().contains(defDateFmt)) defDateFmt = "dddd, MMMM dd, yyyy";
        sDateFmt["default_value"] = defDateFmt;
        QVariant ovDateFmt = getJsonOverride("dateFormat", {"lock_screen", "date", "format"});
        QString curDateFmt = ovDateFmt.isValid() ? ovDateFmt.toString() : defDateFmt;
        if (!sDateFmt["options"].toList().contains(curDateFmt)) curDateFmt = defDateFmt;
        sDateFmt["current_value"] = curDateFmt;
        if (curDateFmt != defDateFmt) isModified = true;
        sDateFmt["live_preview_supported"] = true;
        sDateFmt["requires_activation"] = true;
        sDateFmt["category"] = "Clock & Date";
        settingsList.append(sDateFmt);

        // 13. Unlock Message Display
        capsList.append("messageDisplay");
        QVariantMap sMsgDisp;
        sMsgDisp["id"] = "messageDisplay";
        sMsgDisp["label"] = "Unlock Hint";
        sMsgDisp["help"] = "Display unlock instruction prompt on lockscreen";
        sMsgDisp["type"] = "bool";
        bool defMsgDisp = (baseIni.value("[LockScreen.Message]").value("display", "true").toLower() != "false");
        sMsgDisp["default_value"] = defMsgDisp;
        QVariant ovMsgDisp = getJsonOverride("messageDisplay", {"lock_screen", "message", "display"});
        bool curMsgDisp = ovMsgDisp.isValid() ? (ovMsgDisp.toString().toLower() != "false" && ovMsgDisp.toBool() != false) : defMsgDisp;
        sMsgDisp["current_value"] = curMsgDisp;
        if (curMsgDisp != defMsgDisp) isModified = true;
        sMsgDisp["live_preview_supported"] = true;
        sMsgDisp["requires_activation"] = true;
        sMsgDisp["category"] = "Unlock Hint";
        settingsList.append(sMsgDisp);

        // 14. Unlock Message Text
        capsList.append("messageText");
        QVariantMap sMsgTxt;
        sMsgTxt["id"] = "messageText";
        sMsgTxt["label"] = "Prompt Text";
        sMsgTxt["help"] = "Message text displayed to unlock screen";
        sMsgTxt["type"] = "enum";
        sMsgTxt["options"] = QVariantList{"Press any key to unlock", "Press any key", "Welcome"};
        QString defMsgTxt = baseIni.value("[LockScreen.Message]").value("text", "Press any key to unlock");
        if (!sMsgTxt["options"].toList().contains(defMsgTxt)) defMsgTxt = "Press any key to unlock";
        sMsgTxt["default_value"] = defMsgTxt;
        QVariant ovMsgTxt = getJsonOverride("messageText", {"lock_screen", "message", "text"});
        QString curMsgTxt = ovMsgTxt.isValid() ? ovMsgTxt.toString() : defMsgTxt;
        if (!sMsgTxt["options"].toList().contains(curMsgTxt)) curMsgTxt = defMsgTxt;
        sMsgTxt["current_value"] = curMsgTxt;
        if (curMsgTxt != defMsgTxt) isModified = true;
        sMsgTxt["live_preview_supported"] = true;
        sMsgTxt["requires_activation"] = true;
        sMsgTxt["category"] = "Unlock Hint";
        settingsList.append(sMsgTxt);
} else if (slug == "clockwork-orbital") {
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
    result["is_modified"] = isModified;
    if (isSilent) {
        QVariantMap resolvedConfig;
        for (const QVariant &s : settingsList) {
            QVariantMap sm = s.toMap();
            resolvedConfig[sm.value("id").toString()] = sm.value("current_value");
        }
        result["config"] = resolvedConfig;
    }
    bool isActiveInSddm = (m_activeConfiguration.value("sddm_login").toString() == packageId ||
                           m_activeConfiguration.value("active_package_id").toString() == packageId);
    result["is_active"] = isActiveInSddm;
    return result;
}

void RyzoraBackend::updatePreviewDraft(const QString &packageId, const QVariantMap &draft) {
    if (!m_previewConfig || packageId.isEmpty()) return;

    auto asBool = [](const QVariant &v) -> bool {
        if (v.typeId() == QMetaType::Bool) return v.toBool();
        QString s = v.toString().trimmed().toLower();
        return (s == "true" || s == "1" || s == "yes");
    };

    if (draft.contains("fillMode")) {
        m_previewConfig->insert("background-fill-mode", draft.value("fillMode").toString());
    }
    if (draft.contains("enableAnimations")) {
        bool en = asBool(draft.value("enableAnimations"));
        m_previewConfig->insert("enable-animations", en ? "true" : "false");
    }
    if (draft.contains("blur")) {
        m_previewConfig->insert("LockScreen/blur", QString::number(draft.value("blur").toInt()));
    }
    if (draft.contains("brightness")) {
        QString b = draft.value("brightness").toString();
        QString bval = (b == "dark" ? "-0.4" : (b == "dim" ? "-0.2" : (b == "bright" ? "0.2" : "0.0")));
        m_previewConfig->insert("LockScreen/brightness", bval);
    }
    if (draft.contains("saturation")) {
        QString s = draft.value("saturation").toString();
        QString sval = (s == "grayscale" ? "-1.0" : (s == "muted" ? "-0.5" : (s == "vibrant" ? "0.2" : "0.0")));
        m_previewConfig->insert("LockScreen/saturation", sval);
    }
    if (draft.contains("clockDisplay")) {
        bool cd = asBool(draft.value("clockDisplay"));
        m_previewConfig->insert("LockScreen.Clock/display", cd ? "true" : "false");
    }
    if (draft.contains("clockPosition")) {
        m_previewConfig->insert("LockScreen.Clock/position", draft.value("clockPosition").toString());
    }
    if (draft.contains("clockFormat")) {
        m_previewConfig->insert("LockScreen.Clock/format", draft.value("clockFormat").toString());
    }
    if (draft.contains("dateDisplay")) {
        bool dd = asBool(draft.value("dateDisplay"));
        m_previewConfig->insert("LockScreen.Date/display", dd ? "true" : "false");
    }
    if (draft.contains("dateFormat")) {
        m_previewConfig->insert("LockScreen.Date/format", draft.value("dateFormat").toString());
    }
    if (draft.contains("loginAreaPosition")) {
        m_previewConfig->insert("LoginScreen.LoginArea/position", draft.value("loginAreaPosition").toString());
    }
    if (draft.contains("avatarShape")) {
        m_previewConfig->insert("LoginScreen.LoginArea.Avatar/shape", draft.value("avatarShape").toString());
    }
    if (draft.contains("messageDisplay")) {
        bool md = asBool(draft.value("messageDisplay"));
        m_previewConfig->insert("LockScreen.Message/display", md ? "true" : "false");
    }
    if (draft.contains("messageText")) {
        m_previewConfig->insert("LockScreen.Message/text", draft.value("messageText").toString());
    }

    if (packageId.startsWith("silentsddm-")) {
        QString slug = packageId.mid(11);
        QString home = resolveHomePath();
        QString previewConfPath = home + "/.local/share/ryzora/lockscreens/silentsddm/preview-" + slug + "/configs/ryzora-preview.conf";
        QFile confFile(previewConfPath);
        if (confFile.exists() && confFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
            QString oldConf = QString::fromUtf8(confFile.readAll());
            confFile.close();
            QStringList fileLines = oldConf.split('\n');
            QString curSec;
            QString result;
            for (const QString &line : fileLines) {
                QString trimmed = line.trimmed();
                if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
                    curSec = trimmed;
                    result += line + "\n";
                    continue;
                }
                if (curSec == "[General]") {
                    if (trimmed.startsWith("background-fill-mode") && draft.contains("fillMode")) {
                        result += "background-fill-mode = \"" + draft.value("fillMode").toString() + "\"\n";
                        continue;
                    }
                    if (trimmed.startsWith("enable-animations") && draft.contains("enableAnimations")) {
                        result += QString("enable-animations = %1\n").arg(asBool(draft.value("enableAnimations")) ? "true" : "false");
                        continue;
                    }
                } else if (curSec == "[LockScreen]") {
                    if (trimmed.startsWith("blur =") && draft.contains("blur")) {
                        result += QString("blur = %1\n").arg(draft.value("blur").toInt());
                        continue;
                    }
                    if (trimmed.startsWith("brightness =") && draft.contains("brightness")) {
                        QString b = draft.value("brightness").toString();
                        QString bval = (b == "dark" ? "-0.4" : (b == "dim" ? "-0.2" : (b == "bright" ? "0.2" : "0.0")));
                        result += QString("brightness = %1\n").arg(bval);
                        continue;
                    }
                    if (trimmed.startsWith("saturation =") && draft.contains("saturation")) {
                        QString s = draft.value("saturation").toString();
                        QString sval = (s == "grayscale" ? "-1.0" : (s == "muted" ? "-0.5" : (s == "vibrant" ? "0.2" : "0.0")));
                        result += QString("saturation = %1\n").arg(sval);
                        continue;
                    }
                } else if (curSec == "[LockScreen.Clock]") {
                    if (trimmed.startsWith("display =") && draft.contains("clockDisplay")) {
                        result += QString("display = %1\n").arg(asBool(draft.value("clockDisplay")) ? "true" : "false");
                        continue;
                    }
                    if (trimmed.startsWith("position =") && draft.contains("clockPosition")) {
                        result += "position = \"" + draft.value("clockPosition").toString() + "\"\n";
                        continue;
                    }
                    if (trimmed.startsWith("format =") && draft.contains("clockFormat")) {
                        result += "format = \"" + draft.value("clockFormat").toString() + "\"\n";
                        continue;
                    }
                } else if (curSec == "[LockScreen.Date]") {
                    if (trimmed.startsWith("display =") && draft.contains("dateDisplay")) {
                        result += QString("display = %1\n").arg(asBool(draft.value("dateDisplay")) ? "true" : "false");
                        continue;
                    }
                    if (trimmed.startsWith("format =") && draft.contains("dateFormat")) {
                        result += "format = \"" + draft.value("dateFormat").toString() + "\"\n";
                        continue;
                    }
                } else if (curSec == "[LockScreen.Message]") {
                    if (trimmed.startsWith("display =") && draft.contains("messageDisplay")) {
                        result += QString("display = %1\n").arg(asBool(draft.value("messageDisplay")) ? "true" : "false");
                        continue;
                    }
                    if (trimmed.startsWith("text =") && draft.contains("messageText")) {
                        result += "text = \"" + draft.value("messageText").toString() + "\"\n";
                        continue;
                    }
                } else if (curSec == "[LoginScreen.LoginArea]") {
                    if (trimmed.startsWith("position =") && draft.contains("loginAreaPosition")) {
                        result += "position = \"" + draft.value("loginAreaPosition").toString() + "\"\n";
                        continue;
                    }
                } else if (curSec == "[LoginScreen.LoginArea.Avatar]") {
                    if (trimmed.startsWith("shape =") && draft.contains("avatarShape")) {
                        result += "shape = \"" + draft.value("avatarShape").toString() + "\"\n";
                        continue;
                    }
                }
                result += line + "\n";
            }
            if (confFile.open(QIODevice::WriteOnly | QIODevice::Text | QIODevice::Truncate)) {
                confFile.write(result.toUtf8());
                confFile.close();
            }
            m_previewConfig->load(previewConfPath);
        }
    }
    m_previewConfig->bumpRevision();
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
        if (!specMap.contains(key)) continue;

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
            validatedUpdates[key] = boolVal;
        } else if (type == "int") {
            int intVal = val.toInt();
            int minVal = spec.value("min").toInt();
            int maxVal = spec.value("max").toInt();
            if (intVal < minVal || intVal > maxVal) {
                res["message"] = QString("Validation Error: '%1' must be an integer between %2 and %3 (got %4)")
                                     .arg(key).arg(minVal).arg(maxVal).arg(intVal);
                return res;
            }
            validatedUpdates[key] = intVal;
        }
    }

    if (validatedUpdates.isEmpty()) {
        res["message"] = "No valid configurable parameters were provided.";
        return res;
    }

    QString slug = themeModel.value("slug").toString();
    bool isSilent = themeModel.value("is_silent").toBool();

    if (isSilent) {
        QString silDir = resolveHomePath() + "/.local/share/ryzora/lockscreens/silentsddm";
        QDir().mkpath(silDir);
        QString jsonPath = silDir + "/configuration.json";
        QJsonObject rootObj;
        QFile jfIn(jsonPath);
        if (jfIn.open(QIODevice::ReadOnly)) {
            QJsonDocument jd = QJsonDocument::fromJson(jfIn.readAll());
            if (jd.isObject()) rootObj = jd.object();
            jfIn.close();
        }

        rootObj["version"] = 1;
        QJsonObject themesObj = rootObj.value("themes").toObject();
        QJsonObject pkgObj = themesObj.value(packageId).toObject();
        for (auto it = validatedUpdates.constBegin(); it != validatedUpdates.constEnd(); ++it) {
            pkgObj[it.key()] = QJsonValue::fromVariant(it.value());
        }
        themesObj[packageId] = pkgObj;
        rootObj["themes"] = themesObj;

        // Also update legacy flat & structured sections for backward compatibility with external tools
        for (auto it = validatedUpdates.constBegin(); it != validatedUpdates.constEnd(); ++it) {
            rootObj[it.key()] = QJsonValue::fromVariant(it.value());
        }
        QJsonObject bgObj = rootObj.value("background").toObject();
        if (validatedUpdates.contains("fillMode")) bgObj["fill_mode"] = validatedUpdates.value("fillMode").toString();
        rootObj["background"] = bgObj;

        QJsonObject lockObj = rootObj.value("lock_screen").toObject();
        if (validatedUpdates.contains("blur")) lockObj["blur"] = validatedUpdates.value("blur").toInt();
        if (validatedUpdates.contains("brightness")) lockObj["brightness"] = validatedUpdates.value("brightness").toString();
        if (validatedUpdates.contains("saturation")) lockObj["saturation"] = validatedUpdates.value("saturation").toString();
        QJsonObject clkObj = lockObj.value("clock").toObject();
        if (validatedUpdates.contains("clockDisplay")) clkObj["display"] = validatedUpdates.value("clockDisplay").toBool();
        if (validatedUpdates.contains("clockPosition")) clkObj["position"] = validatedUpdates.value("clockPosition").toString();
        if (validatedUpdates.contains("clockFormat")) clkObj["format"] = validatedUpdates.value("clockFormat").toString();
        lockObj["clock"] = clkObj;
        QJsonObject dtObj = lockObj.value("date").toObject();
        if (validatedUpdates.contains("dateDisplay")) dtObj["display"] = validatedUpdates.value("dateDisplay").toBool();
        if (validatedUpdates.contains("dateFormat")) dtObj["format"] = validatedUpdates.value("dateFormat").toString();
        lockObj["date"] = dtObj;
        QJsonObject msgObj = lockObj.value("message").toObject();
        if (validatedUpdates.contains("messageDisplay")) msgObj["display"] = validatedUpdates.value("messageDisplay").toBool();
        if (validatedUpdates.contains("messageText")) msgObj["text"] = validatedUpdates.value("messageText").toString();
        lockObj["message"] = msgObj;
        rootObj["lock_screen"] = lockObj;

        QJsonObject lgnObj = rootObj.value("login_screen").toObject();
        if (validatedUpdates.contains("blur")) lgnObj["blur"] = validatedUpdates.value("blur").toInt();
        if (validatedUpdates.contains("brightness")) lgnObj["brightness"] = validatedUpdates.value("brightness").toString();
        if (validatedUpdates.contains("saturation")) lgnObj["saturation"] = validatedUpdates.value("saturation").toString();
        QJsonObject areaObj = lgnObj.value("login_area").toObject();
        if (validatedUpdates.contains("loginAreaPosition")) areaObj["position"] = validatedUpdates.value("loginAreaPosition").toString();
        lgnObj["login_area"] = areaObj;
        QJsonObject avObj = lgnObj.value("avatar").toObject();
        if (validatedUpdates.contains("avatarShape")) avObj["shape"] = validatedUpdates.value("avatarShape").toString();
        lgnObj["avatar"] = avObj;
        rootObj["login_screen"] = lgnObj;

        QSaveFile saveJson(jsonPath);
        if (saveJson.open(QIODevice::WriteOnly)) {
            saveJson.write(QJsonDocument(rootObj).toJson(QJsonDocument::Indented));
            saveJson.commit();
        }

        QString userThemeDir = silDir + "/configs";
        QString userConf = userThemeDir + "/" + slug + ".conf";
        if (QFile::exists(userConf)) {
            QFile::remove(userConf);
        }
    } else {
        QString userThemeDir = resolveHomePath() + "/.local/share/ryzora/lockscreens/qylock/" + slug;
        QDir().mkpath(userThemeDir);
        QString userConf = userThemeDir + "/theme.conf";
        QSettings settings(userConf, QSettings::IniFormat);
        settings.beginGroup("General");
        for (auto it = validatedUpdates.constBegin(); it != validatedUpdates.constEnd(); ++it) {
            settings.setValue(it.key(), it.value());
        }
        settings.endGroup();
        settings.sync();
    }

    bool isActiveInSddm = (m_activeConfiguration.value("sddm_login").toString() == packageId ||
                           m_activeConfiguration.value("active_package_id").toString() == packageId);

    res["success"] = true;
    res["active_in_sddm"] = isActiveInSddm;
    res["live_preview_ready"] = true;
    res["updated_keys"] = validatedUpdates.keys();
    res["message"] = "Theme customization saved to user configuration.";

    postNotification("success", "Customization Saved", res.value("message").toString());
    return res;
}

QVariantMap RyzoraBackend::resetThemeConfig(const QString &packageId) {
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

    QString slug = packageId;
    bool isSilent = false;
    if (slug.startsWith("lockscreen-qylock-")) {
        slug = slug.mid(18);
    } else if (slug.startsWith("lockscreen-")) {
        slug = slug.mid(11);
    } else if (slug.startsWith("silentsddm-")) {
        slug = slug.mid(11);
        isSilent = true;
    } else if (slug == "silentsddm-engine" || slug == "ryzora-silent") {
        isSilent = true;
    }

    if (isSilent) {
        QString silDir = resolveHomePath() + "/.local/share/ryzora/lockscreens/silentsddm";
        QString jsonPath = silDir + "/configuration.json";
        QFile jf(jsonPath);
        if (jf.open(QIODevice::ReadOnly)) {
            QJsonDocument jd = QJsonDocument::fromJson(jf.readAll());
            jf.close();
            if (jd.isObject()) {
                QJsonObject rootObj = jd.object();
                if (rootObj.contains("themes") && rootObj.value("themes").isObject()) {
                    QJsonObject themesObj = rootObj.value("themes").toObject();
                    if (themesObj.contains(packageId)) {
                        themesObj.remove(packageId);
                        rootObj["themes"] = themesObj;
                        QSaveFile sf(jsonPath);
                        if (sf.open(QIODevice::WriteOnly)) {
                            sf.write(QJsonDocument(rootObj).toJson(QJsonDocument::Indented));
                            sf.commit();
                        }
                    }
                }
            }
        }
        QString perThemeConf = silDir + "/configs/" + slug + ".conf";
        if (QFile::exists(perThemeConf)) {
            QFile::remove(perThemeConf);
        }
    } else {
        QString userConf = resolveHomePath() + "/.local/share/ryzora/lockscreens/qylock/" + slug + "/theme.conf";
        if (QFile::exists(userConf)) {
            QFile::remove(userConf);
        }
    }

    // Return the newly resolved default configuration
    res = getThemeConfig(packageId);
    res["success"] = true;
    res["message"] = "Customization reset to theme defaults.";
    postNotification("info", "Reset to Defaults", "Theme settings restored to default values.");
    return res;
}

bool RyzoraBackend::isSilentEngineInstalled() const {
    return ryzora_is_silentsddm_engine_installed();
}

void RyzoraBackend::installSilentEngine() {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char *raw = ryzora_install_silentsddm_engine();
        QString resStr;
        if (raw) {
            resStr = QString::fromUtf8(raw);
            ryzora_free_string(raw);
        }
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        QJsonObject obj = doc.object();
        bool success = obj.value("success").toBool();
        QString msg = obj.value("message").toString();

        QMetaObject::invokeMethod(qApp, [self, success, msg]() {
            if (!self) return;
            emit self->silentEngineInstalled(success, msg);
            emit self->silentEngineInstalledChanged();
            self->postNotification(success ? "success" : "error", "SilentSDDM Engine", msg);
            self->loadLibrary(true);
            self->filterCatalog("all", "");
        });
    });
}

void RyzoraBackend::uninstallSilentEngine() {
    if (m_isShutdown.load()) return;
    QPointer<RyzoraBackend> self = this;
    m_threadPool.start([self]() {
        char *raw = ryzora_uninstall_silentsddm_engine();
        QString resStr;
        if (raw) {
            resStr = QString::fromUtf8(raw);
            ryzora_free_string(raw);
        }
        QJsonDocument doc = QJsonDocument::fromJson(resStr.toUtf8());
        QJsonObject obj = doc.object();
        bool success = obj.value("success").toBool();
        QString msg = obj.value("message").toString();

        QMetaObject::invokeMethod(qApp, [self, success, msg]() {
            if (!self) return;
            emit self->silentEngineInstalledChanged();
            self->postNotification(success ? "info" : "error", "SilentSDDM Engine", msg);
            self->loadLibrary(true);
            self->filterCatalog("all", "");
        });
    });
}

bool RyzoraBackend::acquireDecoderSlot() {
    int current = m_activeDecoderCount.load();
    while (current < 10) {
        if (m_activeDecoderCount.compare_exchange_weak(current, current + 1)) {
            return true;
        }
    }
    return false;
}

void RyzoraBackend::releaseDecoderSlot() {
    int current = m_activeDecoderCount.load();
    while (current > 0) {
        if (m_activeDecoderCount.compare_exchange_weak(current, current - 1)) {
            emit decoderSlotAvailable();
            return;
        }
    }
}
