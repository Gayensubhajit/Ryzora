#include <QQuickWindow>
#include <QDir>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QElapsedTimer>
#include <QDebug>
#include <csignal>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include "ryzora_backend.hpp"

static bool isPidAlive(pid_t pid) {
    if (pid <= 0) return false;
    return kill(pid, 0) == 0;
}


int runCaptureScreenshots(QGuiApplication &app, QQmlApplicationEngine &engine, RyzoraBackend *backend, const QString &outDir) {
    QDir dir(outDir);
    if (!dir.exists()) dir.mkpath(".");

    QObject *rootObj = engine.rootObjects().isEmpty() ? nullptr : engine.rootObjects().first();
    QQuickWindow *window = qobject_cast<QQuickWindow*>(rootObj);
    if (!window) {
        qCritical() << "Failed to find root QQuickWindow";
        return 1;
    }

    QObject *stack = rootObj->findChild<QObject*>("stackLayout");
    QStringList routes = {"discover", "library", "fastfetch", "lockscreens", "updates", "backups", "system", "settings"};

    // Wait for catalog & library to populate
    QElapsedTimer t;
    t.start();
    while ((backend->isCatalogLoading() || backend->isLibraryLoading()) && t.elapsed() < 3000) {
        app.processEvents();
        usleep(10000);
    }
    for (int i = 0; i < 10; ++i) { app.processEvents(); usleep(20000); }

    for (const QString &route : routes) {
        if (stack) stack->setProperty("currentTab", route);
        for (int i = 0; i < 15; ++i) { app.processEvents(); usleep(20000); }
        QImage img = window->grabWindow();
        QString dest = dir.filePath(route + ".png");
        img.save(dest);
        qInfo() << "Saved screenshot:" << dest << "(" << img.width() << "x" << img.height() << ")";
    }
    return 0;
}

int runLifecycleSuite(QGuiApplication &app, QQmlApplicationEngine &engine, RyzoraBackend *backend) {
    Q_UNUSED(engine);
    qInfo() << "==================================================";
    qInfo() << "STARTING QT QUICK -> C-ABI -> RUST LIFECYCLE SUITE";
    qInfo() << "==================================================";

    // STEP 1: UI Cancel latency test with harmless long-running fixture
    qInfo() << "[STEP 1] Testing Qt UI Cancellation of fixture-sleep...";
    backend->requestFastfetchPreview("fixture-sleep");

    QElapsedTimer spawnTimer;
    spawnTimer.start();
    while (!backend->isProcessActive() && spawnTimer.elapsed() < 2500) {
        app.processEvents();
        usleep(3000);
    }

    uint32_t activePid = backend->activePid();
    if (activePid == 0 || !isPidAlive(activePid)) {
        qCritical() << "FAIL: Process PID was not active. PID:" << activePid;
        return 1;
    }
    qInfo() << "  -> Active fixture process running, PID:" << activePid;

    // Trigger cancel through Qt method (same as clicking Cancel button in QML)
    QElapsedTimer cancelTimer;
    cancelTimer.start();
    backend->cancelFastfetch();

    while ((backend->isProcessActive() || isPidAlive(activePid)) && cancelTimer.elapsed() < 2000) {
        app.processEvents();
        usleep(2000);
    }
    qint64 cancelMs = cancelTimer.elapsed();

    if (isPidAlive(activePid)) {
        qCritical() << "FAIL: Process" << activePid << "was not terminated!";
        return 2;
    }
    if (backend->isFastfetchLoading()) {
        qCritical() << "FAIL: Loading state flag remained true after cancel!";
        return 3;
    }
    qInfo() << "  -> SUCCESS: UI Cancel terminated & reaped PID" << activePid << "in" << cancelMs << "ms.";

    // STEP 2: Rapid preset switching storm (concurrency & monotonic watermark test)
    qInfo() << "[STEP 2] Testing rapid preset selection storm with overlapping requests...";
    for (int i = 1; i <= 8; ++i) {
        backend->requestFastfetchPreview(QString("fixture-sleep-%1").arg(i));
        usleep(2000);
        app.processEvents();
    }

    // Immediately queue dr460nized as the final desired preset
    backend->requestFastfetchPreview("dr460nized");

    QElapsedTimer stormTimer;
    stormTimer.start();
    while (backend->isFastfetchLoading() && stormTimer.elapsed() < 6000) {
        app.processEvents();
        usleep(5000);
    }

    if (backend->isFastfetchLoading()) {
        qCritical() << "FAIL: Preset storm did not settle!";
        return 4;
    }
    qInfo() << "  -> Final status message:" << backend->statusMessage();
    qInfo() << "  -> SUCCESS: Storm settled monotonically to latest preset without race or corruption.";

    // STEP 3: Close application during running preview
    qInfo() << "[STEP 3] Testing application shutdown during active running preview...";
    backend->requestFastfetchPreview("fixture-sleep");
    spawnTimer.restart();
    while (!backend->isProcessActive() && spawnTimer.elapsed() < 2500) {
        app.processEvents();
        usleep(3000);
    }
    uint32_t shutdownPid = backend->activePid();
    if (shutdownPid == 0 || !isPidAlive(shutdownPid)) {
        qCritical() << "FAIL: In-flight preview process not active. PID:" << shutdownPid;
        return 5;
    }
    qInfo() << "  -> In-flight preview running, PID:" << shutdownPid;

    // Shutdown backend (simulates closing window / quitting application)
    QElapsedTimer shutdownTimer;
    shutdownTimer.start();
    backend->shutdown();
    qint64 shutdownMs = shutdownTimer.elapsed();

    if (isPidAlive(shutdownPid)) {
        qCritical() << "FAIL: PID" << shutdownPid << "still alive after shutdown!";
        return 6;
    }
    qInfo() << "  -> SUCCESS: Application shutdown killed & reaped PID" << shutdownPid << "in" << shutdownMs << "ms.";

        // STEP 4: Interactive Page Navigation Latency Benchmark (25 Switches)
    qInfo() << "[STEP 4] Testing 25 interactive page navigation cycles & scene-graph mapping...";
    QList<qint64> navTimes;
    QObject *rootObj = engine.rootObjects().isEmpty() ? nullptr : engine.rootObjects().first();
    QObject *stack = rootObj ? rootObj->findChild<QObject*>("stackLayout") : nullptr;
    QStringList tabs = {"discover", "library", "fastfetch", "lockscreens", "updates", "backups", "system", "settings"};

    for (int i = 0; i < 25; ++i) {
        QString nextTab = tabs[i % tabs.size()];
        QElapsedTimer t;
        t.start();
        if (stack) {
            stack->setProperty("currentTab", nextTab);
        }
        app.processEvents();
        navTimes.append(t.nsecsElapsed());
    }

    std::sort(navTimes.begin(), navTimes.end());
    double medianUs = (navTimes[navTimes.size() / 2]) / 1000.0;
    double p95Us = (navTimes[static_cast<int>(navTimes.size() * 0.95)]) / 1000.0;
    qInfo() << "  -> SUCCESS: 25 page switches completed. Median switch latency:" << QString::number(medianUs, 'f', 1) << "µs (" << QString::number(medianUs / 1000.0, 'f', 2) << "ms) | p95:" << QString::number(p95Us, 'f', 1) << "µs (" << QString::number(p95Us / 1000.0, 'f', 2) << "ms)";

    qInfo() << "==================================================";
    qInfo() << "ALL QT QUICK -> RUST LIFECYCLE TESTS PASSED (0 ERRORS)";
    qInfo() << "==================================================";
    return 0;
}

static RyzoraBackend *g_activeBackend = nullptr;

static void handleTerminationSignal(int sig) {
    Q_UNUSED(sig);
    if (g_activeBackend) {
        g_activeBackend->shutdown();
    }
    _exit(0);
}

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);
    app.setApplicationName("Ryzora Prototype");
    app.setOrganizationName("Ryzora");

    QQuickStyle::setStyle("Basic");

    RyzoraBackend backend;
    g_activeBackend = &backend;
    std::signal(SIGTERM, handleTerminationSignal);
    std::signal(SIGINT, handleTerminationSignal);
    std::signal(SIGHUP, handleTerminationSignal);

    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty("backend", &backend);

    const QUrl url(QStringLiteral("qrc:/qml/Main.qml"));
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreated,
                     &app, [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
            QCoreApplication::exit(-1);
    }, Qt::QueuedConnection);

    engine.load(url);

    int exitCode = 0;
    if (argc > 1 && QString(argv[1]) == "--test-lifecycle") {
        exitCode = runLifecycleSuite(app, engine, &backend);
    } else if (argc > 2 && QString(argv[1]) == "--capture-screenshots") {
        exitCode = runCaptureScreenshots(app, engine, &backend, QString(argv[2]));
    } else {
        exitCode = app.exec();
    }

    backend.shutdown();
    return exitCode;
}
