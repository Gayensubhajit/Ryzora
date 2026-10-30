import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "components"
import "views"

ApplicationWindow {
    id: window
    width: Qt.application.arguments.indexOf("--width") !== -1 ? parseInt(Qt.application.arguments[Qt.application.arguments.indexOf("--width") + 1]) : 1240
    height: Qt.application.arguments.indexOf("--height") !== -1 ? parseInt(Qt.application.arguments[Qt.application.arguments.indexOf("--height") + 1]) : 820
    minimumWidth: 640
    minimumHeight: 500
    visible: true
    title: "Ryzora — Lock Screen & Theme Ecosystem"
    color: theme.bg

    // Theme Singleton Component
    Theme {
        id: theme
        objectName: "theme"
    }

    // Shared video decoder pool counter to prevent GPU NVDEC out-of-memory
    property int activeDecoderCount: 0

    // Navigation History Stack
    property var navHistory: ["discover"]
    property int navHistoryIndex: 0
    readonly property bool canGoBack: navHistoryIndex > 0 || (stackLayout.currentTab === "discover" && discoverView.selectedPackage !== null) || (stackLayout.currentTab === "apps" && appsView.selectedPackage !== null) || (stackLayout.currentTab === "lockscreens" && lockscreenView.selectedPackage !== null) || (stackLayout.currentTab === "studio" && studioView && (studioView.activeDetailTheme !== null || studioView.currentSubpage !== "lockscreen"))
    readonly property bool canGoForward: navHistoryIndex < navHistory.length - 1

    function showPackageDetail(pkg, originTab) {
        var tab = originTab || stackLayout.currentTab;
        if (tab === "apps" && appsView) {
            if (discoverView) discoverView.setSelectedPackage(null);
            if (lockscreenView) lockscreenView.setSelectedPackage(null);
            appsView.setSelectedPackage(pkg);
        } else if (tab === "lockscreens" && lockscreenView) {
            if (discoverView) discoverView.setSelectedPackage(null);
            if (appsView) appsView.setSelectedPackage(null);
            lockscreenView.setSelectedPackage(pkg);
        } else {
            if (lockscreenView) lockscreenView.setSelectedPackage(null);
            if (appsView) appsView.setSelectedPackage(null);
            discoverView.setSelectedPackage(pkg);
            navigateTo("discover");
        }
    }

    function openStudioWithRoute(subpage, pkg, returnRoute) {
        if (studioView) {
            studioView.returnRoute = returnRoute || "";
            studioView.selectSubpage(subpage || "lockscreen", pkg || null);
        }
        if (stackLayout.currentTab !== "studio") {
            if (navHistoryIndex < navHistory.length - 1) {
                navHistory = navHistory.slice(0, navHistoryIndex + 1);
            }
            if (navHistory.length === 0 || navHistory[navHistory.length - 1] !== "studio") {
                navHistory.push("studio");
            }
            navHistoryIndex = navHistory.length - 1;
            stackLayout.currentTab = "studio";
        }
    }

    function openStudio(subpage, pkg) {
        openStudioWithRoute(subpage, pkg, "");
    }

    Connections {
        target: backend
        function onNotificationPosted(type, title, message) {
            toast.show(type, title, message)
        }
        function onPackageInstalled(packageId, success, message) {
            if (success) {
                backend.loadFastfetchLocalEmblems();
                backend.loadFastfetchInstalledPresets();
            }
        }
        function onPackageUninstalled(packageId, success, message) {
            if (success) {
                backend.loadFastfetchLocalEmblems();
                backend.loadFastfetchInstalledPresets();
            }
        }
        function onAppInstalled(appId, success, message) {
            // Backend already refreshes appsList after install
        }
        function onAppUninstalled(appId, success, message) {
            // Backend already refreshes appsList after uninstall
        }
        function onSetupWizardRequested() {
            setupWizardModal.open()
        }
    }

    Component.onCompleted: {
        if (backend.isFirstRun) {
            setupWizardModal.open()
        }
    }

    function handlePackageAction(action, data, targetOverride) {
        console.log("[QML Main] handlePackageAction CALLED: action=" + action + " pkg=" + (data ? data.id : "null") + " override=" + targetOverride + " active_targets=" + JSON.stringify(data ? data.active_targets : {}));
        if (!data) return;
        if (action === "install") {
            backend.installPackage(data.id);
        } else if (action === "apply") {
            var cat = String(data.category || "").toLowerCase();
            var target = targetOverride;
            if (target === "session_lock" || target === "quickshell") {
                console.warn("[QML Main] Desktop session lock is unmanaged by Ryzora (Dusky/Hyprlock independent)");
                backend.postNotification("warning", "Desktop Lock Unmanaged", "Desktop session lock is controlled independently by Dusky/Hyprlock.");
                return;
            }
            if (!target) {
                if (cat === "lockscreens" || cat === "sddm") {
                    target = "sddm_login";
                } else if (data.supported_targets && data.supported_targets.length > 0) {
                    target = data.supported_targets[0];
                }
            }
            if (target) {
                backend.applyTarget(data.id, target);
            }
        } else if (action === "deactivate") {
            var deactTarget = targetOverride;
            if (deactTarget === "session_lock" || deactTarget === "quickshell") {
                console.warn("[QML Main] Deactivating session_lock is a no-op: desktop lock is unmanaged");
                return;
            }
            if (!deactTarget) {
                var catDeact = String(data.category || "").toLowerCase();
                if (data.active_targets && data.active_targets.sddm_login) {
                    deactTarget = "sddm_login";
                } else if (catDeact === "lockscreens" || catDeact === "sddm") {
                    deactTarget = "sddm_login";
                }
            }
            if (deactTarget) {
                backend.deactivateTarget(deactTarget);
            }
        } else if (action === "test") {
            backend.testPackage(data.id);
        } else if (action === "uninstall") {
            backend.uninstallPackage(data.id);
        } else if (action === "open_fastfetch") {
            detailDrawer.isOpen = false;
            var ffOrigin = stackLayout ? stackLayout.currentTab : "discover";
            discoverView.setSelectedPackage(null);
            if (appsView) appsView.setSelectedPackage(null);
            window.openStudioWithRoute("fastfetch", data ? data.id : null, ffOrigin);
            if (fastfetchView && data && data.id && typeof fastfetchView.loadPackage === "function") {
                fastfetchView.loadPackage(data.id);
            }
        } else if (action === "open_lockscreen") {
            detailDrawer.isOpen = false;
            discoverView.setSelectedPackage(null);
            if (appsView) appsView.setSelectedPackage(null);
            var lockOrigin = stackLayout ? stackLayout.currentTab : "discover";
            window.openStudioWithRoute("lockscreen", data ? data.id : null, lockOrigin);
        } else if (action === "open_studio") {
            detailDrawer.isOpen = false;
            discoverView.setSelectedPackage(null);
            if (appsView) appsView.setSelectedPackage(null);
            var origin = stackLayout ? stackLayout.currentTab : "discover";
            var cat = String(data.category || "").toLowerCase();
            var pt = String(data.package_type || "").toLowerCase();
            var sf = String(data.subfilter || "").toLowerCase();
            var id = String(data.id || "");
            var isSilent = sf === "silentsddm" || id.indexOf("silentsddm-") === 0;
            var isCustom = sf === "custom_media" || id.indexOf("custom:") === 0 || (data.tags && data.tags.indexOf("custom") !== -1);

            if (isSilent || cat === "lockscreens" || cat === "lockscreen" || cat === "sddm" || pt === "lockscreen" || pt === "lockscreens" || data.supports_sddm) {
                window.openStudioWithRoute("lockscreen", data.id, origin);
            } else if (cat === "fastfetch" || pt === "fastfetch") {
                window.openStudioWithRoute("fastfetch", data.id, origin);
            } else if (isCustom || cat === "wallpapers" || cat === "wallpaper" || pt === "wallpaper" || cat === "rices" || cat === "themes") {
                window.openStudioWithRoute("wallpapers", data.id, origin);
            } else {
                window.openStudioWithRoute("lockscreen", data.id, origin);
            }
        } else if (action === "open_library") {
            detailDrawer.isOpen = false;
            discoverView.setSelectedPackage(null);
            if (appsView) appsView.setSelectedPackage(null);
            window.navigateTo("library");
        } else {
            backend.postNotification("info", "Package Details", "Package " + data.name + " is selected.");
        }
    }

    function navigateTo(tab) {
        if (tab === "compat_lab") {
            systemView.currentSubTab = 1;
            tab = "system";
        } else if (tab === "integrity") {
            systemView.currentSubTab = 2;
            tab = "system";
        } else if (tab === "activity") {
            systemView.currentSubTab = 3;
            tab = "system";
        } else if (tab === "author" || tab === "pkg_creator") {
            systemView.currentSubTab = 4;
            tab = "system";
        } else if (tab === "collections") {
            discoverView.secondaryMode = "collections";
            tab = "discover";
        } else if (tab === "creators") {
            discoverView.secondaryMode = "creators";
            tab = "discover";
        } else if (tab === "repositories") {
            discoverView.secondaryMode = "repositories";
            tab = "discover";
        }

        if (tab === "studio") {
            if (studioView) {
                studioView.returnRoute = "";
                studioView.selectSubpage("lockscreen", null);
            }
        }

        if (stackLayout.currentTab === tab) return;
        // Truncate forward history if navigating to a new destination
        if (navHistoryIndex < navHistory.length - 1) {
            navHistory = navHistory.slice(0, navHistoryIndex + 1);
        }
        if (navHistory.length === 0 || navHistory[navHistory.length - 1] !== tab) {
            navHistory.push(tab);
        }
        navHistoryIndex = navHistory.length - 1;
        stackLayout.currentTab = tab;
        if (topBar) {
            if (tab === "apps") {
                topBar.searchQuery = appsView ? appsView.searchQuery : "";
            } else if (tab === "discover") {
                topBar.searchQuery = discoverView ? discoverView.searchQuery : "";
            } else {
                topBar.searchQuery = "";
            }
        }
    }

    function goBack() {
        if (stackLayout.currentTab === "apps" && appsView.selectedPackage !== null) {
            appsView.setSelectedPackage(null);
            return;
        }
        if (stackLayout.currentTab === "discover" && discoverView.selectedPackage !== null) {
            discoverView.setSelectedPackage(null);
            return;
        }
        if (stackLayout.currentTab === "lockscreens" && lockscreenView.selectedPackage !== null) {
            lockscreenView.setSelectedPackage(null);
            return;
        }
        if (stackLayout.currentTab === "studio" && studioView) {
            if (studioView.activeDetailTheme !== null) {
                if (studioView.returnRoute !== "") {
                    var r = studioView.returnRoute;
                    studioView.returnRoute = "";
                    studioView.activeDetailTheme = null;
                    navigateTo(r);
                    return;
                }
                studioView.activeDetailTheme = null;
                return;
            }
            if (studioView.currentSubpage !== "lockscreen") {
                studioView.selectSubpage("lockscreen", null);
                return;
            }
        }
        if (navHistoryIndex > 0) {
            navHistoryIndex--;
            stackLayout.currentTab = navHistory[navHistoryIndex];
        }
    }

    function goForward() {
        if (canGoForward) {
            navHistoryIndex++;
            stackLayout.currentTab = navHistory[navHistoryIndex];
        }
    }

    // Global Shortcuts
    Shortcut {
        sequence: "Ctrl+B"
        onActivated: sidebar.isCollapsed = !sidebar.isCollapsed
    }
    Shortcut {
        sequence: "Alt+Left"
        onActivated: goBack()
    }
    Shortcut {
        sequence: "Alt+Right"
        onActivated: goForward()
    }
    Shortcut {
        sequence: "Ctrl+1"
        onActivated: navigateTo("hub")
    }
    Shortcut {
        sequence: "Ctrl+2"
        onActivated: navigateTo("discover")
    }
    Shortcut {
        sequence: "Ctrl+3"
        onActivated: navigateTo("library")
    }
    Shortcut {
        sequence: "Ctrl+4"
        onActivated: navigateTo("updates")
    }
    Shortcut {
        sequence: "Ctrl+5"
        onActivated: navigateTo("backups")
    }
    Shortcut {
        sequence: "Ctrl+6"
        onActivated: navigateTo("system")
    }
    Shortcut {
        sequence: "Ctrl+7"
        onActivated: navigateTo("settings")
    }
    Shortcut {
        sequence: "Escape"
        onActivated: {
            if (aboutModal.visible) {
                aboutModal.visible = false
            } else if (setupWizardModal.visible) {
                setupWizardModal.dismiss()
            } else if (detailDrawer.isOpen) {
                detailDrawer.isOpen = false
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // ── 1. Full Sidebar ──
        SideBar {
            id: sidebar
            objectName: "sidebarItem"
            currentTab: stackLayout.currentTab
            Layout.fillHeight: true
            onTabSelected: function(tab) {
                navigateTo(tab)
            }
            onOpenAbout: {
                aboutModal.visible = true
            }
        }

        // ── 2. Main Content Area ──
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // Polished Application Header
            TopBar {
                id: topBar
                Layout.fillWidth: true
                currentTab: stackLayout.currentTab
                canGoBack: window.canGoBack
                canGoForward: window.canGoForward
                onNavigateBack: window.goBack()
                onNavigateForward: window.goForward()
                onLibraryClicked: window.navigateTo("library")
                onSearchChanged: function(q) {
                    if (stackLayout.currentTab === "discover") {
                        discoverView.searchQuery = q
                        discoverView.updateFilter()
                    } else if (stackLayout.currentTab === "apps") {
                        appsView.searchQuery = q
                        backend.filterApps(appsView.catalogMode === "all" ? "all_packages" : appsView.selectedCategory, q, appsView.selectedProvider)
                    }
                }
                onRefreshClicked: {
                    if (stackLayout.currentTab === "discover") {
                        discoverView.updateFilter()
                    } else if (stackLayout.currentTab === "apps") {
                        backend.filterApps(appsView.catalogMode === "all" ? "all_packages" : appsView.selectedCategory, appsView.searchQuery, appsView.selectedProvider)
                    } else if (stackLayout.currentTab === "library") {
                        backend.loadLibrary(true)
                    } else if (stackLayout.currentTab === "backups") {
                        backend.loadSnapshots()
                    } else if (stackLayout.currentTab === "updates") {
                        backend.loadUpdates()
                    } else if (stackLayout.currentTab === "system" || stackLayout.currentTab === "compat_lab") {
                        backend.loadSystemInfo()
                        backend.loadCompatibilityProfiles()
                    } else if (stackLayout.currentTab === "hub") {
                        backend.filterCatalog("all", "")
                        backend.loadLibrary(true)
                    }
                }
            }

            // View Stack
            Item {
                id: stackLayout
                objectName: "stackLayout"
                Layout.fillWidth: true
                Layout.fillHeight: true
                property string currentTab: Qt.application.arguments.indexOf("--tab") !== -1 ? Qt.application.arguments[Qt.application.arguments.indexOf("--tab") + 1] : "discover"
                property int initialSubTab: Qt.application.arguments.indexOf("--subtab") !== -1 ? parseInt(Qt.application.arguments[Qt.application.arguments.indexOf("--subtab") + 1]) : 0

                // ── Store Group ──
                HubView {
                    id: hubView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "hub"
                    onNavigateRequested: function(tab) {
                        window.navigateTo(tab)
                    }
                    onPackageSelected: function(pkg) {
                        window.showPackageDetail(pkg)
                    }
                }

                DiscoverView {
                    id: discoverView
                    objectName: "discoverView"
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "discover"
                    onPackageSelected: function(pkg) {
                        if (pkg && (pkg.package_type === "desktop_app" || pkg.appData || pkg.is_application)) {
                            window.navigateTo("apps")
                            if (appsView) {
                                appsView.setSelectedApp(pkg.appData || pkg)
                            }
                        } else {
                            discoverView.setSelectedPackage(pkg)
                        }
                    }
                    onActionRequested: function(action, data, targetOverride) {
                        window.handlePackageAction(action, data, targetOverride)
                    }
                    onSearchResetRequested: {
                        topBar.searchQuery = ""
                    }
                }

                AppsView {
                    id: appsView
                    objectName: "appsView"
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "apps"
                    onAppSelected: function(app) {
                        appsView.setSelectedApp(app)
                    }
                    onPackageSelected: function(app) {
                        appsView.setSelectedApp(app)
                    }
                    onInstallRequested: function(appId) {
                        backend.installApp(appId)
                    }
                    onUninstallRequested: function(appId) {
                        backend.uninstallApp(appId)
                    }
                    onLaunchRequested: function(appId) {
                        backend.launchApp(appId)
                    }
                    onCategorySelected: function(cat) {
                        // Category filtering reuses the loaded in-memory catalog without redundant backend queries
                    }
                    onClearSearchRequested: {
                        topBar.searchQuery = ""
                        backend.filterApps(appsView.catalogMode === "all" ? "all_packages" : appsView.selectedCategory, "", appsView.selectedProvider)
                    }
                }

                LibraryView {
                    id: libraryView
                    objectName: "libraryView"
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "library"
                    onPackageSelected: function(pkg) {
                        if (pkg && (pkg.package_type === "desktop_app" || pkg.appData)) {
                            window.navigateTo("apps")
                            if (appsView) {
                                appsView.setSelectedApp(pkg.appData || pkg)
                            }
                        } else {
                            window.showPackageDetail(pkg)
                        }
                    }
                    onCustomizeRequested: function(subpage, pkg) {
                        window.openStudio(subpage, pkg)
                    }
                    onNavigateRequested: function(tabName) {
                        window.navigateTo(tabName)
                    }
                }

                UpdatesView {
                    id: updatesView
                    objectName: "updatesView"
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "updates"
                    onAppSelected: function(app) {
                        window.navigateTo("apps")
                        if (appsView) {
                            appsView.setSelectedApp(app)
                        }
                    }
                }

                CollectionsView {
                    id: collectionsView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "collections"
                    onPackageSelected: function(pkg) {
                        detailDrawer.packageData = pkg
                        detailDrawer.isOpen = true
                    }
                }

                // ── Community Group ──
                RepositoryView {
                    id: repositoryView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "repositories"
                }

                CreatorsView {
                    id: creatorsView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "creators"
                    onCreatorSelected: function(author) {
                        window.navigateTo("discover")
                        discoverView.searchQuery = author
                        discoverView.updateFilter()
                    }
                }

                // ── Tools Group ──
                BackupsView {
                    id: backupsView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "backups"
                }

                IntegrityView {
                    id: integrityView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "integrity"
                }

                SystemView {
                    id: systemView
                    objectName: "systemView"
                    anchors.fill: parent
                    currentSubTab: stackLayout.currentTab === "compat_lab" ? 1 : stackLayout.initialSubTab
                    visible: stackLayout.currentTab === "system" || stackLayout.currentTab === "compat_lab"
                }

                ActivityView {
                    id: activityView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "activity"
                }

                AuthorView {
                    id: authorView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "author"
                }

                SettingsView {
                    id: settingsView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "settings"
                    onLaunchLabRequested: {
                        window.navigateTo("compat_lab")
                    }
                    onLaunchCreatorRequested: {
                        window.navigateTo("author")
                    }
                }

                // ── Preserved Dedicated Product Views ──
                FastfetchView {
                    id: fastfetchView
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "fastfetch"
                }

                LockScreenView {
                    id: lockscreenView
                    objectName: "lockscreenView"
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "lockscreens"
                }

                // ── Personalization Studio Hub ──
                StudioView {
                    id: studioView
                    objectName: "studioView"
                    anchors.fill: parent
                    visible: stackLayout.currentTab === "studio"
                    onNavigateRequested: function(tab) {
                        window.navigateTo(tab)
                    }
                }
            }
        }
    }

    // Modal Package Detail Drawer Overlay
    PackageDetailDrawer {
        id: detailDrawer
        objectName: "detailDrawer"
        onCloseRequested: detailDrawer.isOpen = false
        onActionRequested: function(action, data) {
            window.handlePackageAction(action, data, null)
        }
    }

    // About Ryzora Modal Dialog
    AboutModal {
        id: aboutModal
        onCloseRequested: aboutModal.visible = false
    }

    // Setup Wizard Modal Dialog
    SetupWizardModal {
        id: setupWizardModal
        objectName: "setupWizardModal"
        onCompleted: {
            console.log("[Main.qml] Setup wizard completed.")
        }
        onDismissed: {
            console.log("[Main.qml] Setup wizard dismissed.")
        }
    }

    // Toast Notification Banner (Floating top-right)
    ToastNotification {
        id: toast
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: 20
    }
}
