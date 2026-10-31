import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    property string selectedCategory: "All"
    property string searchQuery: ""
    property string sortBy: "name" // "name" | "category" | "status"
    property var selectedApp: null
    property alias selectedPackage: root.selectedApp
    property int featuredIndex: 0
    property int selectedScreenshotIndex: 0
    property bool lightboxOpen: false
    property int lightboxIndex: 0
    property string pendingAppId: ""
    property string pendingAction: ""
    property string actionErrorMessage: ""
    property string installStage: ""
    property int installPercentage: -1
    property bool installIsDeterminate: false
    property string installMessage: ""
    property string installBytesDownloaded: ""
    property string installBytesTotal: ""
    property string installDownloadSpeed: ""
    property int installCurrentStep: 0
    property int installTotalSteps: 0
    property string installCurrentItem: ""
    property string installEtaStr: ""
    property string installDiagnosticDetails: ""
    property bool showInstallDetails: false
    property string selectedProviderId: ""
    property string selectedTargetId: ""
    property string catalogMode: "apps" // "apps" | "all_packages"
    property string selectedProvider: "all" // "all" | "pacman" | "aur" | "flatpak"
    property real savedBrowseScrollY: 0
    property var masterAppsList: []

    // ── STORE EXPERIENCE 3.0 STATE ──
    property string storeMode: "storefront" // "storefront" | "collection" | "explore_all"
    property string collectionId: ""
    property string collectionTitle: ""
    property string collectionSubtitle: ""
    property string collectionCategory: "all"
    property string collectionSearchQuery: ""
    property string collectionProvider: "all"
    property string collectionSortBy: "name"
    property int collectionPage: 0
    property int collectionTotalCount: 0
    property var collectionItems: []

    property string exploreCategory: "all"
    property string exploreSearchQuery: ""
    property string exploreProvider: "all"
    property string exploreSortBy: "name"
    property int explorePage: 0
    property int exploreTotalCount: 0
    property var exploreItems: []

    readonly property int pageSize: 24
    property var curatedCollections: []

    function openCollection(id, title, subtitle, category) {
        root.collectionId = id || ""
        root.collectionTitle = title || "Applications"
        root.collectionSubtitle = subtitle || ""
        root.collectionCategory = category || "all"
        root.collectionSearchQuery = ""
        root.collectionProvider = "all"
        root.collectionSortBy = "name"
        root.collectionPage = 0
        root.storeMode = "collection"
        root.updateCollectionSlice()
        if (typeof collectionScroll !== "undefined" && collectionScroll) collectionScroll.contentY = 0
    }

    function openExploreAll(mode) {
        if (mode) root.catalogMode = mode
        root.exploreCategory = "all"
        root.exploreSearchQuery = ""
        root.exploreProvider = "all"
        root.exploreSortBy = "name"
        root.explorePage = 0
        root.storeMode = "explore_all"
        root.updateExploreSlice()
        if (typeof exploreScroll !== "undefined" && exploreScroll) exploreScroll.contentY = 0
    }

    function backToStorefront() {
        root.storeMode = "storefront"
        root.searchQuery = ""
        root.collectionSearchQuery = ""
        root.exploreSearchQuery = ""
        root.clearSearchRequested()
        if (typeof mainScroll !== "undefined" && mainScroll) mainScroll.contentY = 0
    }

    function updateCollectionSlice() {
        if (!backend) return
        var res = backend.queryVisibleCatalogSlice(
            root.collectionCategory,
            root.collectionSearchQuery,
            root.collectionProvider,
            root.collectionSortBy,
            "apps",
            [],
            root.collectionPage * root.pageSize,
            root.pageSize
        )
        root.collectionItems = res.items || []
        root.collectionTotalCount = (res.totalCount !== undefined) ? res.totalCount : 0
    }

    function updateExploreSlice() {
        if (!backend) return
        var res = backend.queryVisibleCatalogSlice(
            root.exploreCategory,
            root.exploreSearchQuery,
            root.exploreProvider,
            root.exploreSortBy,
            root.catalogMode,
            [],
            root.explorePage * root.pageSize,
            root.pageSize
        )
        root.exploreItems = res.items || []
        root.exploreTotalCount = (res.totalCount !== undefined) ? res.totalCount : 0
    }

    function refreshCuratedCollections() {
        if (backend) {
            root.curatedCollections = backend.getCuratedCollectionsOverview()
        }
    }

    onSelectedProviderChanged: {
        if (root.catalogMode === "all_packages" && backend) {
            backend.filterApps("all_packages", root.searchQuery, root.selectedProvider)
        }
    }

    readonly property int heroResponsiveHeight: {
        if (!heroPanel.visible) return 0
        if (root.height < 800 || root.width < 1400) return 290
        return 350
    }

    // Dynamic responsive column count: guarantees zero horizontal overflow and comfortable card widths
    readonly property int responsiveColumns: {
        var w = root.width - (root.width < 600 ? 24 : 40)
        var spacing = 12
        var minCardWidth = 240
        var cols = Math.floor((w + spacing) / (minCardWidth + spacing))
        var maxCols = (w >= 1800) ? 6 : ((w >= 1400) ? 5 : 4)
        return Math.max(1, Math.min(maxCols, cols))
    }

    // Curated row slice: returns exactly the number of cards that fit the current responsive column count
    function getCuratedRowSlice(apps, maxCount) {
        if (!apps || apps.length === 0) return []
        var n = Math.min(apps.length, Math.max(1, maxCount))
        return apps.slice(0, n)
    }

    signal appSelected(var appData)
    signal packageSelected(var appData)
    signal installRequested(string appId)
    signal uninstallRequested(string appId)
    signal launchRequested(string appId)
    signal categorySelected(string category)
    signal clearSearchRequested()

    onSearchQueryChanged: {
        var q = root.searchQuery.trim()
        if (q.length > 0) {
            if (root.selectedApp !== null) {
                root.setSelectedApp(null)
            }
            if (root.storeMode === "storefront") {
                root.collectionId = "search"
                root.collectionTitle = "Search Results"
                root.collectionSubtitle = 'Applications matching "' + q + '"'
                root.collectionCategory = "all"
                root.collectionSearchQuery = q
                root.collectionPage = 0
                root.storeMode = "collection"
                root.updateCollectionSlice()
            } else if (root.storeMode === "collection") {
                root.collectionSearchQuery = q
                root.collectionPage = 0
                root.updateCollectionSlice()
            } else if (root.storeMode === "explore_all") {
                root.exploreSearchQuery = q
                root.explorePage = 0
                root.updateExploreSlice()
            }
        } else {
            if (root.storeMode === "collection" && root.collectionId === "search") {
                root.backToStorefront()
            } else if (root.storeMode === "collection") {
                root.collectionSearchQuery = ""
                root.updateCollectionSlice()
            } else if (root.storeMode === "explore_all") {
                root.exploreSearchQuery = ""
                root.updateExploreSlice()
            }
        }
    }

    function setSelectedApp(app) {
        if (app !== null && root.selectedApp === null) {
            root.savedBrowseScrollY = mainScroll ? mainScroll.contentY : 0
        }
        root.selectedScreenshotIndex = 0
        root.lightboxOpen = false
        root.lightboxIndex = 0
        root.actionErrorMessage = ""
        root.selectedApp = app
        if (app !== null && typeof app !== "undefined") {
            root.selectedProviderId = app.installed_provider || app.provider_id || "pacman"
            root.selectedTargetId = app.target_id || app.id || ""
            if (typeof rightDetailFlickable !== "undefined" && rightDetailFlickable) {
                rightDetailFlickable.contentY = 0
            }
            if (typeof galFlick !== "undefined" && galFlick) {
                galFlick.contentX = 0
            }
            if (backend) {
                var txn = backend.getTransactionForApp(root.currentActionAppId())
                if (txn && txn.op_id && !txn.is_finished) {
                    root.pendingAppId = root.currentActionAppId()
                    root.pendingAction = txn.operation || "install"
                    root.installStage = txn.stage || ""
                    root.installPercentage = (txn.percentage !== undefined && txn.percentage !== null) ? txn.percentage : -1
                    root.installIsDeterminate = Boolean(txn.is_determinate)
                    root.installMessage = txn.message || ""
                    root.installBytesDownloaded = txn.bytes_downloaded || ""
                    root.installBytesTotal = txn.bytes_total || ""
                    root.installDownloadSpeed = txn.download_speed || ""
                    root.installCurrentStep = txn.current_step || 0
                    root.installTotalSteps = txn.total_steps || 0
                    root.installCurrentItem = txn.current_item || ""
                    root.installEtaStr = txn.eta_str || ""
                    root.installDiagnosticDetails = txn.diagnostic_details || ""
                } else if (!backend.isAppOperating(root.pendingAppId)) {
                    root.pendingAppId = ""
                    root.pendingAction = ""
                }
            }
        } else {
            root.selectedProviderId = ""
            root.selectedTargetId = ""
            if (backend && !backend.isAppOperating(root.pendingAppId)) {
                root.pendingAppId = ""
                root.pendingAction = ""
            }
            Qt.callLater(function() {
                if (mainScroll) {
                    mainScroll.contentY = root.savedBrowseScrollY
                }
            })
        }
    }

    function currentSelectedProviderSource() {
        if (!root.selectedApp) return null
        var pId = root.selectedProviderId || root.selectedApp.installed_provider || root.selectedApp.provider_id || "pacman"
        var tId = root.selectedTargetId || root.selectedApp.target_id || root.selectedApp.id || ""
        var provs = root.selectedApp.available_providers || []
        for (var i = 0; i < provs.length; i++) {
            if (provs[i] && provs[i].provider_id === pId) {
                return provs[i]
            }
        }
        return {
            provider_id: pId,
            target_id: tId,
            name: pId.toUpperCase(),
            repository: root.selectedApp.repository || pId,
            version: root.selectedApp.version || ""
        }
    }

    function isCurrentProviderInstalled() {
        if (!root.selectedApp) return false
        var src = currentSelectedProviderSource()
        if (src && typeof src.is_installed === "boolean") {
            return src.is_installed
        }
        return Boolean(root.selectedApp.is_installed)
    }

    function currentInstalledVersion() {
        if (!root.selectedApp) return ""
        var src = currentSelectedProviderSource()
        if (src && src.installed_version) return src.installed_version
        return (root.selectedApp && root.selectedApp.is_installed) ? (root.selectedApp.version || "") : ""
    }

    function currentAvailableVersion() {
        if (!root.selectedApp) return ""
        var src = currentSelectedProviderSource()
        if (src && src.version) return src.version
        return (root.selectedApp && root.selectedApp.version) ? root.selectedApp.version : "Latest"
    }

    function formatAppBytes(bytes) {
        if (!bytes || bytes <= 0 || isNaN(bytes)) return ""
        var b = Number(bytes)
        var units = ["B", "KB", "MB", "GB", "TB"]
        var u = 0
        while (b >= 1024 && u < units.length - 1) {
            b /= 1024
            u++
        }
        return (u === 0 ? b : b.toFixed(1)) + " " + units[u]
    }

    function currentAppDisplaySize() {
        if (!root.selectedApp) return "Unknown"
        var src = currentSelectedProviderSource()
        var isInst = isCurrentProviderInstalled()
        var inst = src ? src.installed_size : root.selectedApp.installed_size
        var dl = src ? src.download_size : root.selectedApp.download_size
        var pId = (src ? src.provider_id : (root.selectedApp.provider_id || "")).toLowerCase()

        if (isInst && inst && inst > 0) {
            return formatAppBytes(inst)
        }
        if (isInst) {
            return "Installed"
        }
        if (pId === "aur") {
            return "Source build"
        }
        if (dl && dl > 0) {
            return formatAppBytes(dl)
        }
        if (inst && inst > 0) {
            return formatAppBytes(inst)
        }
        var lbl = (src && src.size_label) ? src.size_label : (root.selectedApp.size_label || "")
        if (lbl && (lbl.indexOf("MB") !== -1 || lbl.indexOf("GB") !== -1 || lbl.indexOf("KB") !== -1 || lbl.indexOf("B") !== -1) && lbl.indexOf("Source") === -1) {
            return lbl
        }
        return "Unknown"
    }

    function currentAppSizeSubtext() {
        if (!root.selectedApp) return "Size"
        var isInst = isCurrentProviderInstalled()
        var src = currentSelectedProviderSource()
        var inst = src ? src.installed_size : root.selectedApp.installed_size
        var dl = src ? src.download_size : root.selectedApp.download_size
        var pId = (src ? src.provider_id : (root.selectedApp.provider_id || "")).toLowerCase()

        if (isInst) return "Disk footprint"
        if (pId === "aur") return "Source (AUR)"
        if (dl && dl > 0) return "Download size"
        if (inst && inst > 0) return "Installed footprint"
        return "Size"
    }

    function currentActionAppId() {
        if (!root.selectedApp) return ""
        if (root.selectedProviderId && root.selectedTargetId) {
            return root.selectedProviderId + ":" + root.selectedTargetId
        }
        var defProv = root.selectedApp.installed_provider || root.selectedApp.provider_id || "pacman"
        var defTarget = root.selectedApp.target_id || root.selectedApp.id || ""
        return defProv + ":" + defTarget
    }

    function isSelectedAppPending() {
        if (!root.selectedApp) return false
        var current = currentActionAppId()
        var backendOperating = Boolean(backend && (backend.isAppOperating(root.selectedApp.id) || backend.isAppOperating(current)))
        if (backendOperating) return true
        if (root.pendingAppId === "") return false
        return root.pendingAppId === root.selectedApp.id
            || root.pendingAppId === current
            || root.pendingAppId.endsWith(":" + root.selectedApp.id)
            || (root.selectedTargetId && root.pendingAppId.endsWith(":" + root.selectedTargetId))
    }

    function setSelectedPackage(pkg) {
        root.setSelectedApp(pkg)
    }

    function requestInstall(appId) {
        if (backend && root.pendingAppId !== "" && !backend.isAppOperating(root.pendingAppId)) {
            root.pendingAppId = ""
            root.pendingAction = ""
        }
        if ((backend && backend.isAppOperating(appId)) || (root.pendingAppId !== "" && root.pendingAppId !== appId)) {
            root.actionErrorMessage = "Another package operation is currently in progress. Please wait for it to finish."
            return
        }
        root.actionErrorMessage = ""
        root.pendingAppId = appId
        root.pendingAction = "install"
        root.installRequested(appId)
    }

    function requestUninstall(appId) {
        if (backend && root.pendingAppId !== "" && !backend.isAppOperating(root.pendingAppId)) {
            root.pendingAppId = ""
            root.pendingAction = ""
        }
        if ((backend && backend.isAppOperating(appId)) || (root.pendingAppId !== "" && root.pendingAppId !== appId)) {
            root.actionErrorMessage = "Another package operation is currently in progress. Please wait for it to finish."
            return
        }
        if (root.selectedApp) {
            uninstallConfirmationDialog.openForApp(root.selectedApp)
        } else {
            root.actionErrorMessage = ""
            root.pendingAppId = appId
            root.pendingAction = "uninstall"
            root.uninstallRequested(appId)
        }
    }

    Component.onCompleted: {
        var list = backend.appsList || []
        if (list.length > 0) {
            root.masterAppsList = list
        }
        root.refreshCuratedCollections()
        root.updateCollectionSlice()
        root.updateExploreSlice()
    }

    Connections {
        target: backend
        function onAppsChanged() {
            var list = backend.appsList || []
            root.refreshCuratedCollections()
            if (root.storeMode === "collection") {
                root.updateCollectionSlice()
            } else if (root.storeMode === "explore_all") {
                root.updateExploreSlice()
            }
            if (root.selectedCategory === "All" && root.searchQuery === "") {
                root.masterAppsList = list
            } else if (root.masterAppsList.length === 0) {
                root.masterAppsList = list
            } else {
                var updated = root.masterAppsList.slice()
                for (var j = 0; j < list.length; j++) {
                    var itm = list[j]
                    for (var k = 0; k < updated.length; k++) {
                        if (updated[k] && itm && updated[k].id === itm.id) {
                            updated[k] = itm
                            break
                        }
                    }
                }
                root.masterAppsList = updated
            }

            if (root.selectedApp) {
                for (var i = 0; i < list.length; i++) {
                    if (list[i] && list[i].id === root.selectedApp.id) {
                        root.selectedApp = list[i]
                        if (list[i].installed_provider) {
                            root.selectedProviderId = list[i].installed_provider
                            root.selectedTargetId = list[i].target_id || list[i].id
                        }
                        break
                    }
                }
            }
        }
        function onAppInstallProgress(appId, stage, percentage, isDeterminate, message, bytesDownloaded, bytesTotal, downloadSpeed, currentStep, totalSteps, currentItem, etaStr, diagnosticDetails) {
            if (root.pendingAppId === appId || root.isSelectedAppPending() || (root.selectedApp && appId.indexOf(root.selectedApp.id) !== -1)) {
                root.installStage = stage
                root.installPercentage = percentage
                root.installIsDeterminate = isDeterminate
                root.installMessage = message
                root.installBytesDownloaded = bytesDownloaded
                root.installBytesTotal = bytesTotal
                root.installDownloadSpeed = downloadSpeed
                root.installCurrentStep = (currentStep !== undefined && currentStep !== null) ? currentStep : 0
                root.installTotalSteps = (totalSteps !== undefined && totalSteps !== null) ? totalSteps : 0
                root.installCurrentItem = (currentItem !== undefined && currentItem !== null) ? currentItem : ""
                root.installEtaStr = (etaStr !== undefined && etaStr !== null) ? etaStr : ""
                root.installDiagnosticDetails = (diagnosticDetails !== undefined && diagnosticDetails !== null) ? diagnosticDetails : ""
            }
        }
        function onAppInstalled(appId, success, message) {
            var curAppId = root.selectedApp ? root.selectedApp.id : ""
            var cleanId = appId ? appId.replace(/^(pacman|aur|flatpak):/, "") : ""
            var isCur = (root.pendingAppId === appId || root.isSelectedAppPending() || curAppId === appId || curAppId === cleanId)
            if (isCur) {
                root.pendingAppId = ""
                root.pendingAction = ""
                root.installStage = ""
                root.installPercentage = -1
                root.installIsDeterminate = false
                root.installMessage = ""
                root.installBytesDownloaded = ""
                root.installBytesTotal = ""
                root.installDownloadSpeed = ""
                root.installCurrentStep = 0
                root.installTotalSteps = 0
                root.installCurrentItem = ""
                root.installEtaStr = ""
                root.installDiagnosticDetails = ""
                root.showInstallDetails = false
                if (!success) {
                    root.actionErrorMessage = message || "Installation failed. Please try again."
                } else {
                    root.actionErrorMessage = ""
                    if (root.selectedApp) {
                        var updated = Object.assign({}, root.selectedApp)
                        updated.is_installed = true
                        root.selectedApp = updated
                    }
                }
            }
            if (backend) {
                backend.refreshCatalog()
            }
        }
        function onAppUninstalled(appId, success, message) {
            var curAppId = root.selectedApp ? root.selectedApp.id : ""
            var cleanId = appId ? appId.replace(/^(pacman|aur|flatpak):/, "") : ""
            var isCur = (root.pendingAppId === appId || root.isSelectedAppPending() || curAppId === appId || curAppId === cleanId)
            if (isCur) {
                root.pendingAppId = ""
                root.pendingAction = ""
                if (!success) {
                    root.actionErrorMessage = message || "Removal failed. Please try again."
                } else {
                    root.actionErrorMessage = ""
                    if (root.selectedApp) {
                        var updatedUninst = Object.assign({}, root.selectedApp)
                        updatedUninst.is_installed = false
                        updatedUninst.installed_provider = ""
                        updatedUninst.installed_version = ""
                        root.selectedApp = updatedUninst
                    }
                }
            }
            if (backend) {
                backend.refreshCatalog()
            }
        }
        function onAppLaunched(appId, success, message) {
            if (!success) {
                root.actionErrorMessage = message || "Could not launch application."
            }
        }
    }

    // Helper: Map application category to vector icon
    function getCategoryIcon(cat) {
        var c = (cat || "").toLowerCase()
        if (c === "development") return "terminal"
        if (c === "internet") return "compass"
        if (c === "productivity") return "check"
        if (c === "multimedia") return "play"
        if (c === "graphics") return "palette"
        if (c === "utilities") return "settings"
        if (c === "system") return "activity"
        if (c === "games") return "zap"
        if (c === "installed") return "check"
        return "layout-grid"
    }

    // Helper: Determine if an application item matches the selected provider
    function appMatchesProvider(item, pTarget) {
        if (!item) return false
        if (!pTarget || pTarget === "all") return true
        var target = pTarget.toLowerCase()
        var provId = String(item.provider_id || item.installed_provider || "").toLowerCase()
        var rName = String(item.repository || "").toLowerCase()

        if (target === "pacman") {
            if (provId === "pacman" || (provId !== "aur" && provId !== "flatpak" && rName !== "aur" && rName !== "flatpak" && rName !== "flathub")) {
                return true
            }
        } else if (target === "aur") {
            if (provId === "aur" || rName === "aur") return true
        } else if (target === "flatpak") {
            if (provId === "flatpak" || rName === "flatpak" || rName === "flathub") return true
        }

        // Also inspect available_providers so apps offered across multiple providers (e.g. Spotify, Chrome) match
        var provs = item.available_providers || []
        for (var i = 0; i < provs.length; i++) {
            var ap = provs[i]
            if (!ap) continue
            var apId = String(ap.provider_id || "").toLowerCase()
            if (target === "pacman" && (apId === "pacman" || apId === "arch" || apId === "core" || apId === "extra")) {
                return true
            }
            if (target === "aur" && apId === "aur") {
                return true
            }
            if (target === "flatpak" && (apId === "flatpak" || apId === "flathub")) {
                return true
            }
        }
        return false
    }

    // Helper: Count distinct applications for category chips
    function getCategoryCount(catName) {
        var list = (root.masterAppsList && root.masterAppsList.length > 0) ? root.masterAppsList : (backend.appsList || [])
        var c = catName.toLowerCase()
        var count = 0
        for (var i = 0; i < list.length; i++) {
            var item = list[i]
            if (!item) continue
            if (!root.appMatchesProvider(item, root.selectedProvider)) continue

            if (c === "all") {
                count++
            } else if (c === "installed") {
                if (item.is_installed) count++
            } else if (String(item.category || "").toLowerCase() === c) {
                count++
            }
        }
        return count
    }

    // Filtered application list based on search, category, and sort
    readonly property var filteredApps: {
        var list = (root.masterAppsList && root.masterAppsList.length > 0) ? root.masterAppsList : (backend.appsList || [])
        var res = []
        var q = root.searchQuery.trim().toLowerCase()
        var cat = root.selectedCategory.toLowerCase()

        for (var i = 0; i < list.length; i++) {
            var item = list[i]
            if (!item) continue

            // 1. Search Query Filter
            if (q.length > 0) {
                var nameMatch = item.name && item.name.toLowerCase().indexOf(q) !== -1
                var dispMatch = item.display_name && item.display_name.toLowerCase().indexOf(q) !== -1
                var descMatch = item.description && item.description.toLowerCase().indexOf(q) !== -1
                var summMatch = item.summary && item.summary.toLowerCase().indexOf(q) !== -1
                var pubMatch = item.publisher && item.publisher.toLowerCase().indexOf(q) !== -1
                var idMatch = item.id && item.id.toLowerCase().indexOf(q) !== -1
                if (!nameMatch && !dispMatch && !descMatch && !summMatch && !pubMatch && !idMatch) {
                    continue
                }
            }

            // 2. Category Filter
            if (cat === "all") {
                // pass
            } else if (cat === "installed") {
                if (!item.is_installed) continue
            } else {
                if (String(item.category || "").toLowerCase() !== cat) {
                    continue
                }
            }

            // 3. Provider / Source Filter (all | pacman | aur | flatpak)
            if (!root.appMatchesProvider(item, root.selectedProvider)) {
                continue
            }

            res.push(item)
        }

        // 3. Sorting
        res.sort(function(a, b) {
            if (root.sortBy === "status") {
                var sa = a.is_installed ? 1 : 0
                var sb = b.is_installed ? 1 : 0
                if (sa !== sb) return sb - sa
            } else if (root.sortBy === "category") {
                var ca = String(a.category || "").toLowerCase()
                var cb = String(b.category || "").toLowerCase()
                if (ca !== cb) return ca.localeCompare(cb)
            }
            var nameA = String(a.display_name || a.name || "").toLowerCase()
            var nameB = String(b.display_name || b.name || "").toLowerCase()
            return nameA.localeCompare(nameB)
        })

        return res
    }

    // Curated Featured Applications with authentic artwork
    readonly property var featuredItems: {
        var list = (root.masterAppsList && root.masterAppsList.length > 0) ? root.masterAppsList : (backend.appsList || [])
        var res = []
        var preferredIds = ["firefox", "code", "blender", "gimp", "chromium"]
        var seen = {}

        for (var p = 0; p < preferredIds.length; p++) {
            var tid = preferredIds[p]
            for (var i = 0; i < list.length; i++) {
                var item = list[i]
                if (item && (item.id === tid || item.id.endsWith(":" + tid)) && !seen[item.id]) {
                    res.push(item)
                    seen[item.id] = true
                    break
                }
            }
        }

        if (res.length === 0) {
            for (var j = 0; j < list.length && res.length < 5; j++) {
                var cand = list[j]
                if (cand && !seen[cand.id]) {
                    res.push(cand)
                    seen[cand.id] = true
                }
            }
        }
        return res
    }

    // Active Featured App (only when on All and no search)
    readonly property var activeFeaturedApp: {
        if (root.searchQuery !== "" || root.selectedCategory !== "All" || root.featuredItems.length === 0) {
            return null
        }
        var idx = root.featuredIndex
        if (idx < 0 || idx >= root.featuredItems.length) idx = 0
        return root.featuredItems[idx]
    }

    // Stable ID of featured app to exclude from adjacent sections
    readonly property string featuredAppId: activeFeaturedApp ? activeFeaturedApp.id : ""

    // Curated Popular Apps matching exactly one responsive row across the screen
    // Note: Stable curated list so cycling the hero carousel NEVER mutates or shuffles below sections
    readonly property var popularApps: {
        if (root.searchQuery !== "" || root.selectedCategory !== "All") {
            return []
        }
        var list = (root.masterAppsList && root.masterAppsList.length > 0) ? root.masterAppsList : (backend.appsList || [])
        var preferredPopular = ["code", "vlc", "discord", "blender", "obs-studio", "spotify", "chromium", "steam", "gimp", "inkscape", "alacritty"]
        var res = []
        var seen = {}
        var maxItems = Math.min(6, root.responsiveColumns)
        for (var p = 0; p < preferredPopular.length && res.length < maxItems; p++) {
            var pid = preferredPopular[p]
            if (seen[pid]) continue
            for (var i = 0; i < list.length; i++) {
                var item = list[i]
                if (item && item.id === pid && !seen[item.id]) {
                    res.push(item)
                    seen[item.id] = true
                    break
                }
            }
        }
        return res
    }

    // Visible items in the main catalog grid (de-duplicated against popular apps)
    // Note: Stable catalog grid so cycling the hero carousel NEVER mutates or shuffles below sections
    readonly property var visibleCatalogApps: {
        var all = root.filteredApps
        if (root.searchQuery !== "" || root.selectedCategory !== "All") {
            return all
        }
        var excluded = {}
        var pop = root.popularApps
        for (var p = 0; p < pop.length; p++) {
            if (pop[p]) excluded[pop[p].id] = true
        }
        var res = []
        for (var i = 0; i < all.length; i++) {
            var it = all[i]
            if (it && !excluded[it.id]) {
                res.push(it)
            }
        }
        return res
    }

    // ── REUSABLE MICROSOFT STORE-INSPIRED APP CARD (48px ICON + PROPORTIONAL ACTION BUTTON) ──
    component AppCard: Rectangle {
        id: cardRoot
        property var appData: null
        property real cardWidth: 0
        readonly property bool isInstalled: Boolean(cardRoot.appData && cardRoot.appData.is_installed)

        width: cardWidth > 0 ? cardWidth : undefined
        Layout.fillWidth: cardWidth <= 0
        Layout.preferredWidth: cardWidth > 0 ? cardWidth : 220
        Layout.minimumWidth: 200
        implicitWidth: cardWidth > 0 ? cardWidth : 220
        implicitHeight: 216
        Layout.preferredHeight: 216
        height: 216
        radius: theme.cardRadius
        color: cardMouse.containsMouse ? theme.surfaceHover : theme.surface
        border.color: cardRoot.activeFocus ? theme.accent : (cardMouse.containsMouse ? theme.cardBorderHover : theme.cardBorder)
        border.width: cardRoot.activeFocus ? 2 : 1
        activeFocusOnTab: true
        clip: false

        Behavior on color { ColorAnimation { duration: 120 } }
        Behavior on border.color { ColorAnimation { duration: 120 } }

        Keys.onReturnPressed: {
            if (cardRoot.appData) {
                root.setSelectedApp(cardRoot.appData)
                root.appSelected(cardRoot.appData)
            }
        }
        Keys.onSpacePressed: {
            if (cardRoot.appData) {
                root.setSelectedApp(cardRoot.appData)
                root.appSelected(cardRoot.appData)
            }
        }

        MouseArea {
            id: cardMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                if (cardRoot.appData) {
                    root.setSelectedApp(cardRoot.appData)
                    root.appSelected(cardRoot.appData)
                }
            }
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 6

            // Top Header: App Icon + Action Pill Button
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                // 48x48 Icon Container
                Rectangle {
                    width: 48
                    height: 48
                    Layout.preferredWidth: 48
                    Layout.preferredHeight: 48
                    Layout.fillWidth: false
                    Layout.fillHeight: false
                    radius: 12
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    Image {
                        anchors.centerIn: parent
                        width: 34
                        height: 34
                        source: (cardRoot.appData && cardRoot.appData.icon_url) ? cardRoot.appData.icon_url : ""
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                        visible: status === Image.Ready
                    }

                    RyzoraIcon {
                        anchors.centerIn: parent
                        visible: !cardRoot.appData || !cardRoot.appData.icon_url || cardRoot.appData.icon_url === ""
                        name: cardRoot.appData ? root.getCategoryIcon(cardRoot.appData.category) : "layout-grid"
                        size: 24
                        color: theme.textMuted
                    }
                }

                Item { Layout.fillWidth: true }

                // Action Button using RyzoraButton system
                RyzoraButton {
                    id: cardActionBtn
                    readonly property bool isPending: root.pendingAppId === (cardRoot.appData ? cardRoot.appData.id : "")
                    Layout.alignment: Qt.AlignTop | Qt.AlignRight
                    Layout.preferredWidth: cardActionBtn.implicitWidth
                    Layout.minimumWidth: cardActionBtn.implicitWidth
                    Layout.preferredHeight: cardActionBtn.implicitHeight
                    size: "sm"
                    variant: cardRoot.isInstalled ? "secondary" : "primary"
                    text: isPending ? "Wait..." : (cardRoot.isInstalled ? "Open" : "Get")
                    iconName: isPending ? "arrow-clockwise" : (cardRoot.isInstalled ? "play" : "arrow-download")
                    iconFilled: cardRoot.isInstalled
                    loading: isPending
                    onClicked: {
                        if (!cardRoot.appData) return
                        if (cardRoot.isInstalled) {
                            root.launchRequested(cardRoot.appData.id)
                        } else {
                            root.requestInstall(cardRoot.appData.id)
                        }
                    }
                }
            }

            // Application Name (Clear, bold title)
            Text {
                text: cardRoot.appData ? (cardRoot.appData.display_name || cardRoot.appData.name || "") : ""
                font.family: theme.fontFamily
                font.pixelSize: 14
                font.weight: theme.weightSemibold
                color: theme.text
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            // Category & Publisher
            Text {
                text: cardRoot.appData ? ((cardRoot.appData.publisher ? cardRoot.appData.publisher + " · " : "") + (cardRoot.appData.category || "App")) : ""
                font.family: theme.fontFamily
                font.pixelSize: 11
                font.weight: theme.weightRegular
                color: theme.textMuted
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            // Description: Clean 2-line summary with elision
            Text {
                text: cardRoot.appData ? (cardRoot.appData.summary || cardRoot.appData.description || "") : ""
                font.family: theme.fontFamily
                font.pixelSize: 12
                font.weight: theme.weightRegular
                lineHeight: 1.2
                color: theme.textSecondary
                maximumLineCount: 2
                wrapMode: Text.WordWrap
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            Item { Layout.fillHeight: true }

            // Bottom Metadata Row: Clean status indicator on left + Size on right
            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                // Status Indicator: Left side
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 5

                    Rectangle {
                        width: 6
                        height: 6
                        radius: 3
                        color: "#10b981"
                        visible: cardRoot.isInstalled
                        Layout.alignment: Qt.AlignVCenter
                        Layout.fillWidth: false
                    }

                    Text {
                        text: {
                            if (!cardRoot.appData) return ""
                            if (cardRoot.isInstalled) {
                                return "Installed"
                            }
                            var pId = cardRoot.appData.provider_id || "pacman"
                            if (pId === "flatpak") return "Flathub"
                            if (pId === "aur") return "AUR"
                            var repo = cardRoot.appData.repository || "extra"
                            return repo.charAt(0).toUpperCase() + repo.slice(1)
                        }
                        font.family: theme.fontFamily
                        font.pixelSize: 11
                        font.weight: cardRoot.isInstalled ? theme.weightMedium : theme.weightRegular
                        color: cardRoot.isInstalled ? "#10b981" : theme.textMuted
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                    }
                }

                // Size Badge: Right side
                Text {
                    id: cardSizeText
                    readonly property var szBytes: (cardRoot.appData ? (cardRoot.appData.installed_size || cardRoot.appData.download_size) : 0)
                    visible: Boolean(szBytes && szBytes > 0)
                    text: root.formatAppBytes(szBytes)
                    font.family: theme.fontMono
                    font.pixelSize: 10
                    font.weight: theme.weightMedium
                    color: theme.textMuted
                    Layout.fillWidth: false
                    Layout.preferredWidth: implicitWidth
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 1. MAIN BROWSE VIEW
    // ──────────────────────────────────────────────────────────────────────────
    Flickable {
        id: mainScroll
        objectName: "appsMainScroll"
        anchors.fill: parent
        visible: root.selectedApp === null && root.storeMode === "storefront"
        contentWidth: width
        contentHeight: pageCol.implicitHeight + 40
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ScrollBar.vertical: ScrollBar {
            policy: ScrollBar.AsNeeded
            width: 6
            contentItem: Rectangle {
                implicitWidth: 6
                radius: 3
                color: theme.borderStrong
            }
        }

        ColumnLayout {
            id: pageCol
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.margins: root.width < 600 ? 12 : 20
            spacing: 16

            // ── 1. COMPACT PAGE HEADER ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: "Apps"
                        font.pixelSize: 22
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Discover, install, and launch Linux desktop software applications."
                        font.pixelSize: 12
                        color: theme.textSecondary
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                // Contextual search feedback pill when search query is active
                Rectangle {
                    visible: root.searchQuery !== ""
                    height: 28
                    implicitWidth: searchTagRow.implicitWidth + 16
                    radius: theme.radiusSm
                    color: theme.surfaceElevated
                    border.color: theme.accent
                    border.width: 1

                    RowLayout {
                        id: searchTagRow
                        anchors.centerIn: parent
                        spacing: 6

                        RyzoraIcon {
                            name: "search"
                            size: 11
                            color: theme.accent
                        }

                        Text {
                            text: 'Query: "' + root.searchQuery + '"'
                            font.pixelSize: 11
                            color: theme.text
                        }

                        Rectangle {
                            width: 14
                            height: 14
                            radius: 7
                            color: clearQueryMouse.containsMouse ? theme.surfaceHover : "transparent"

                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "x"
                                size: 9
                                color: theme.textSecondary
                            }

                            MouseArea {
                                id: clearQueryMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.searchQuery = ""
                                    root.clearSearchRequested()
                                    backend.filterApps(root.selectedCategory, "")
                                }
                            }
                        }
                    }
                }
            }

            // ── STOREFRONT DISCOVERY ACTION BAR ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                Text {
                    text: "Curated Storefront"
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                    color: theme.textSecondary
                }

                Rectangle {
                    height: 18
                    implicitWidth: 80
                    radius: 9
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 4
                        Rectangle { width: 6; height: 6; radius: 3; color: "#10b981" }
                        Text { text: "Verified"; font.pixelSize: 10; color: theme.textMuted }
                    }
                }

                Item { Layout.fillWidth: true }

                // Quick jump to Explore All Catalogue
                Rectangle {
                    height: 32
                    implicitWidth: expHdrRow.implicitWidth + 18
                    radius: theme.radiusSm
                    color: expHdrMouse.containsMouse ? theme.surfaceHover : theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: expHdrRow
                        anchors.centerIn: parent
                        spacing: 6
                        RyzoraIcon { name: "layout-grid"; size: 12; color: theme.accent }
                        Text {
                            text: "Explore Complete Catalogue →"
                            font.pixelSize: 11
                            font.weight: Font.Medium
                            color: theme.accent
                        }
                    }

                    MouseArea {
                        id: expHdrMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.openExploreAll("apps")
                    }
                }
            }

            // Dismissable Action Error Notification Banner (Visible on install/uninstall failure)
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: mainErrRow.implicitHeight + 18
                radius: 8
                visible: root.actionErrorMessage !== "" && root.selectedApp === null
                color: theme.resolvedTheme === "light" ? "#FEF2F2" : "#450A0A"
                border.color: "#EF4444"
                border.width: 1

                RowLayout {
                    id: mainErrRow
                    anchors.fill: parent
                    anchors.leftMargin: 14
                    anchors.rightMargin: 14
                    spacing: 10

                    RyzoraIcon { name: "alert"; size: 16; color: "#EF4444" }

                    Text {
                        Layout.fillWidth: true
                        text: root.actionErrorMessage
                        font.pixelSize: 13
                        font.weight: Font.Medium
                        color: theme.resolvedTheme === "light" ? "#B91C1C" : "#FCA5A5"
                        wrapMode: Text.WordWrap
                    }

                    Rectangle {
                        width: 24
                        height: 24
                        radius: 12
                        color: mainErrCloseMouse.containsMouse ? Qt.rgba(239, 68, 68, 0.18) : "transparent"

                        RyzoraIcon { anchors.centerIn: parent; name: "x"; size: 10; color: "#EF4444" }

                        MouseArea {
                            id: mainErrCloseMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.actionErrorMessage = ""
                        }
                    }
                }
            }

            // ── 2. FEATURED APPLICATION SUPER HERO CAROUSEL (Authentic Windows 11 Microsoft Store Layout) ──
            Rectangle {
                id: heroPanel
                objectName: "appsHeroBanner"
                visible: root.activeFeaturedApp !== null
                Layout.fillWidth: true
                Layout.preferredHeight: visible ? root.heroResponsiveHeight : 0
                height: Layout.preferredHeight
                radius: theme.radiusLg
                border.color: Qt.rgba(255, 255, 255, 0.12)
                border.width: 1
                clip: true

                readonly property var currentItem: root.activeFeaturedApp

                // 1. Full-Bleed 16:9 Promotional Artwork
                Image {
                    id: heroImg
                    anchors.fill: parent
                    source: heroPanel.currentItem ? (heroPanel.currentItem.banner_url || "") : ""
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    cache: true
                    sourceSize.width: 1920
                    sourceSize.height: 1080
                    smooth: true
                    visible: status === Image.Ready
                    opacity: status === Image.Ready ? 1.0 : 0.0
                    Behavior on opacity { NumberAnimation { duration: 250 } }
                }

                // Fallback rich dark gradient if image is loading
                Rectangle {
                    anchors.fill: parent
                    z: 0
                    visible: heroImg.status !== Image.Ready
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0.0; color: "#0B0F19" }
                        GradientStop { position: 1.0; color: "#161F33" }
                    }
                }

                // 2. Microsoft Store Left Gradient Scrim (Guarantees 100% Text Readability)
                Rectangle {
                    anchors.fill: parent
                    z: 1
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0.0; color: "#070A12F0" }
                        GradientStop { position: 0.38; color: "#070A12D0" }
                        GradientStop { position: 0.60; color: "#070A1270" }
                        GradientStop { position: 0.85; color: "#070A1215" }
                        GradientStop { position: 1.0; color: "transparent" }
                    }
                }

                // 3. Microsoft Store Bottom Vignette (Behind Floating Thumbnail Bar)
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 120
                    z: 1
                    gradient: Gradient {
                        orientation: Gradient.Vertical
                        GradientStop { position: 0.0; color: "transparent" }
                        GradientStop { position: 0.50; color: "#05081060" }
                        GradientStop { position: 1.0; color: "#04060DE6" }
                    }
                }

                // Auto-rotation timer (pauses when user hovers or interacts)
                Timer {
                    interval: 8000
                    running: root.visible && root.featuredItems.length > 1 && !heroHoverDetector.containsMouse
                    repeat: true
                    onTriggered: {
                        if (root.featuredItems.length > 1) {
                            root.featuredIndex = (root.featuredIndex + 1) % root.featuredItems.length
                        }
                    }
                }

                MouseArea {
                    id: heroHoverDetector
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                    z: 0
                }

                // 4. Hero Content: Typography, Branding & CTAs
                Item {
                    anchors.top: parent.top
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: bottomStrip.top
                    anchors.bottomMargin: 8
                    z: 2

                    // Clicking main hero body navigates to app details
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (heroPanel.currentItem) {
                                root.setSelectedApp(heroPanel.currentItem)
                                root.appSelected(heroPanel.currentItem)
                            }
                        }
                    }

                    ColumnLayout {
                        anchors.top: parent.top
                        anchors.left: parent.left
                        anchors.bottom: parent.bottom
                        width: Math.min(parent.width * (root.width < 760 ? 0.92 : 0.56), 620)
                        anchors.leftMargin: root.width < 760 ? 20 : 32
                        anchors.topMargin: root.width < 760 ? 18 : 26
                        anchors.bottomMargin: 8
                        spacing: root.width < 760 ? 8 : 12

                        // Category & Spotlight Tag Row
                        RowLayout {
                            spacing: 10

                            Rectangle {
                                height: 22
                                radius: 4
                                implicitWidth: featBadgeTxt.implicitWidth + 14
                                color: Qt.rgba(255, 255, 255, 0.12)
                                border.color: theme.accent
                                border.width: 1

                                Text {
                                    id: featBadgeTxt
                                    anchors.centerIn: parent
                                    text: "FEATURED SPOTLIGHT"
                                    font.pixelSize: 9
                                    font.bold: true
                                    font.letterSpacing: 0.8
                                    color: theme.accent
                                }
                            }

                            Text {
                                text: heroPanel.currentItem ? (String(heroPanel.currentItem.category || "Application").toUpperCase() + " · " + (heroPanel.currentItem.publisher || "Arch Linux Native")) : ""
                                font.pixelSize: 11
                                font.letterSpacing: 0.5
                                color: Qt.rgba(255, 255, 255, 0.75)
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                        }

                        // App Display Title
                        Text {
                            Layout.fillWidth: true
                            text: heroPanel.currentItem ? (heroPanel.currentItem.display_name || heroPanel.currentItem.name || "") : ""
                            font.pixelSize: root.width < 760 ? 24 : 30
                            font.bold: true
                            color: "#FFFFFF"
                            elide: Text.ElideRight
                        }

                        // Editorial Summary Description: 14px, readable white text
                        Text {
                            Layout.fillWidth: true
                            text: heroPanel.currentItem ? (heroPanel.currentItem.summary || heroPanel.currentItem.description || "") : ""
                            font.pixelSize: 14
                            lineHeight: 1.35
                            color: "#E2E8F0"
                            maximumLineCount: root.width < 760 ? 2 : 3
                            wrapMode: Text.WordWrap
                            elide: Text.ElideRight
                        }

                        Item { Layout.fillHeight: true }

                        // Action Buttons Row
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            // Primary Action Button: Get / Open / Installing
                            Rectangle {
                                id: heroActionBtn
                                readonly property bool isPending: heroPanel.currentItem ? (root.pendingAppId === heroPanel.currentItem.id) : false
                                height: 38
                                implicitWidth: heroBtnRow.implicitWidth + 28
                                radius: theme.radiusSm
                                color: heroActionBtn.isPending ? theme.borderStrong : (heroBtnMouse.containsMouse ? theme.accentHover : theme.accent)

                                RowLayout {
                                    id: heroBtnRow
                                    anchors.centerIn: parent
                                    spacing: 8
                                    RyzoraIcon {
                                        id: heroSpinIcon
                                        name: heroActionBtn.isPending ? "refresh" : (heroPanel.currentItem && heroPanel.currentItem.is_installed ? "play" : "download")
                                        size: 13
                                        color: "#ffffff"
                                        RotationAnimator {
                                            target: heroSpinIcon
                                            from: 0
                                            to: 360
                                            duration: 1000
                                            loops: Animation.Infinite
                                            running: heroActionBtn.isPending
                                        }
                                    }
                                    Text {
                                        text: heroActionBtn.isPending ? "Installing..." : (heroPanel.currentItem && heroPanel.currentItem.is_installed ? "Open Application" : "Get Application")
                                        font.pixelSize: 13
                                        font.bold: true
                                        color: "#ffffff"
                                    }
                                }

                                MouseArea {
                                    id: heroBtnMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    enabled: !heroActionBtn.isPending
                                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                                    onClicked: {
                                        if (heroPanel.currentItem) {
                                            if (heroPanel.currentItem.is_installed) {
                                                root.launchRequested(heroPanel.currentItem.id)
                                            } else {
                                                root.requestInstall(heroPanel.currentItem.id)
                                            }
                                        }
                                    }
                                }
                            }

                            // Secondary Action Button: View Details
                            Rectangle {
                                height: 38
                                implicitWidth: heroDetailsRow.implicitWidth + 24
                                radius: theme.radiusSm
                                color: heroDetailsMouse.containsMouse ? Qt.rgba(255, 255, 255, 0.22) : Qt.rgba(255, 255, 255, 0.12)
                                border.color: heroDetailsMouse.containsMouse ? theme.accent : Qt.rgba(255, 255, 255, 0.28)
                                border.width: 1

                                RowLayout {
                                    id: heroDetailsRow
                                    anchors.centerIn: parent
                                    spacing: 6
                                    RyzoraIcon {
                                        name: "info"
                                        size: 13
                                        color: "#FFFFFF"
                                    }
                                    Text {
                                        text: "View Details"
                                        font.pixelSize: 13
                                        font.bold: false
                                        color: "#FFFFFF"
                                    }
                                }

                                MouseArea {
                                    id: heroDetailsMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (heroPanel.currentItem) {
                                            root.setSelectedApp(heroPanel.currentItem)
                                            root.appSelected(heroPanel.currentItem)
                                        }
                                    }
                                }
                            }

                            Item { Layout.fillWidth: true }
                        }
                    }
                }

                // 5. Microsoft Store Floating Thumbnail Carousel Strip (52px high)
                Rectangle {
                    id: bottomStrip
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: 16
                    anchors.rightMargin: 16
                    anchors.bottomMargin: 12
                    height: 52
                    radius: 10
                    color: Qt.rgba(15, 23, 42, 0.65)
                    border.color: Qt.rgba(255, 255, 255, 0.14)
                    border.width: 1
                    z: 2
                    clip: true

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 8

                        // Previous Slide Arrow
                        Rectangle {
                            width: 32
                            height: 32
                            radius: 16
                            color: prevThumbMouse.containsMouse ? Qt.rgba(255, 255, 255, 0.25) : Qt.rgba(255, 255, 255, 0.12)
                            border.color: prevThumbMouse.containsMouse ? theme.accent : Qt.rgba(255, 255, 255, 0.20)
                            border.width: 1

                            Text {
                                anchors.centerIn: parent
                                text: "‹"
                                font.pixelSize: 20
                                font.bold: true
                                color: "#FFFFFF"
                            }

                            MouseArea {
                                id: prevThumbMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    var n = root.featuredItems.length
                                    if (n > 0) {
                                        root.featuredIndex = (root.featuredIndex - 1 + n) % n
                                    }
                                }
                            }
                        }

                        // Thumbnail Preview Cards Row
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            spacing: 6

                            Repeater {
                                model: root.featuredItems

                                Rectangle {
                                    id: thumbCard
                                    readonly property bool isSelected: index === root.featuredIndex
                                    readonly property var thumbItem: modelData

                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    Layout.topMargin: 4
                                    Layout.bottomMargin: 4
                                    radius: 7
                                    color: thumbCard.isSelected
                                        ? Qt.rgba(255, 255, 255, 0.22)
                                        : (thumbCardMouse.containsMouse ? Qt.rgba(255, 255, 255, 0.12) : "transparent")
                                    border.color: thumbCard.isSelected
                                        ? Qt.rgba(255, 255, 255, 0.35)
                                        : (thumbCardMouse.containsMouse ? Qt.rgba(255, 255, 255, 0.18) : "transparent")
                                    border.width: 1
                                    clip: true

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 10
                                        anchors.rightMargin: 10
                                        anchors.topMargin: 3
                                        anchors.bottomMargin: 6
                                        spacing: 8

                                        Rectangle {
                                            width: 24
                                            height: 24
                                            radius: 6
                                            color: Qt.rgba(255, 255, 255, 0.15)
                                            border.color: Qt.rgba(255, 255, 255, 0.20)
                                            border.width: 1

                                            Image {
                                                anchors.centerIn: parent
                                                width: 18
                                                height: 18
                                                source: thumbCard.thumbItem ? (thumbCard.thumbItem.icon_url || "") : ""
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                visible: status === Image.Ready
                                            }

                                            RyzoraIcon {
                                                anchors.centerIn: parent
                                                visible: !thumbCard.thumbItem || !thumbCard.thumbItem.icon_url
                                                name: thumbCard.thumbItem ? root.getCategoryIcon(thumbCard.thumbItem.category) : "layout-grid"
                                                size: 14
                                                color: "#FFFFFF"
                                            }
                                        }

                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 0

                                            Text {
                                                Layout.fillWidth: true
                                                text: thumbCard.thumbItem ? (thumbCard.thumbItem.display_name || thumbCard.thumbItem.name || "") : ""
                                                font.pixelSize: 11
                                                font.bold: thumbCard.isSelected
                                                color: "#FFFFFF"
                                                elide: Text.ElideRight
                                            }

                                            Text {
                                                Layout.fillWidth: true
                                                text: thumbCard.thumbItem ? (thumbCard.thumbItem.category || "App") : ""
                                                font.pixelSize: 9
                                                color: Qt.rgba(255, 255, 255, 0.70)
                                                elide: Text.ElideRight
                                            }
                                        }
                                    }

                                    // Microsoft Store Active Underline Indicator Bar
                                    Rectangle {
                                        anchors.bottom: parent.bottom
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        anchors.leftMargin: 8
                                        anchors.rightMargin: 8
                                        height: 3
                                        radius: 1.5
                                        color: theme.accent
                                        visible: thumbCard.isSelected
                                    }

                                    MouseArea {
                                        id: thumbCardMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: root.featuredIndex = index
                                    }
                                }
                            }
                        }

                        // Next Slide Arrow
                        Rectangle {
                            width: 32
                            height: 32
                            radius: 16
                            color: nextThumbMouse.containsMouse ? Qt.rgba(255, 255, 255, 0.25) : Qt.rgba(255, 255, 255, 0.12)
                            border.color: nextThumbMouse.containsMouse ? theme.accent : Qt.rgba(255, 255, 255, 0.20)
                            border.width: 1

                            Text {
                                anchors.centerIn: parent
                                text: "›"
                                font.pixelSize: 20
                                font.bold: true
                                color: "#FFFFFF"
                            }

                            MouseArea {
                                id: nextThumbMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    var n = root.featuredItems.length
                                    if (n > 0) {
                                        root.featuredIndex = (root.featuredIndex + 1) % n
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── 3. REAL SOFTWARE CATEGORIES FILTER BAR ──
            Flickable {
                Layout.fillWidth: true
                height: 32
                contentWidth: categoryRow.implicitWidth
                contentHeight: 32
                clip: true
                boundsBehavior: Flickable.StopAtBounds

                RowLayout {
                    id: categoryRow
                    height: 32
                    spacing: 6

                    Repeater {
                        model: [
                            { label: "All apps", id: "All", icon: "layout-grid" },
                            { label: "Development", id: "Development", icon: "terminal" },
                            { label: "Internet", id: "Internet", icon: "compass" },
                            { label: "Productivity", id: "Productivity", icon: "check" },
                            { label: "Multimedia", id: "Multimedia", icon: "play" },
                            { label: "Graphics", id: "Graphics", icon: "palette" },
                            { label: "Utilities", id: "Utilities", icon: "settings" },
                            { label: "System", id: "System", icon: "activity" },
                            { label: "Games", id: "Games", icon: "zap" },
                            { label: "Installed", id: "Installed", icon: "check" }
                        ]

                        delegate: Rectangle {
                            readonly property bool isSelected: root.selectedCategory === modelData.id
                            readonly property int count: root.getCategoryCount(modelData.id)
                            visible: count > 0 || modelData.id === "All" || modelData.id === "Installed"
                            height: 30
                            implicitWidth: catInnerRow.implicitWidth + 18
                            radius: theme.radiusSm
                            color: isSelected 
                                ? (theme.resolvedTheme === "light" ? "#EBF2FE" : "#1B283D")
                                : (catMouse.containsMouse ? theme.surfaceHover : theme.surface)
                            border.color: isSelected ? theme.accent : (catMouse.containsMouse ? theme.borderStrong : theme.borderSubtle)
                            border.width: 1

                            RowLayout {
                                id: catInnerRow
                                anchors.centerIn: parent
                                spacing: 6

                                RyzoraIcon {
                                    name: modelData.icon
                                    size: 12
                                    color: isSelected ? theme.accent : theme.textSecondary
                                }

                                Text {
                                    text: modelData.label
                                    font.pixelSize: 11
                                    font.weight: isSelected ? Font.DemiBold : Font.Medium
                                    color: isSelected ? (theme.resolvedTheme === "light" ? theme.accent : "#ffffff") : theme.textSecondary
                                }

                                Text {
                                    text: String(count)
                                    font.pixelSize: 10
                                    font.family: "monospace"
                                    color: isSelected ? theme.accent : theme.textMuted
                                }
                            }

                            MouseArea {
                                id: catMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    if (modelData.id === "All") {
                                        root.openExploreAll("apps")
                                    } else {
                                        root.openCollection(modelData.id.toLowerCase(), modelData.label, "Curated and verified " + modelData.label + " applications", modelData.id.toLowerCase())
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── 4. CURATED HORIZONTAL COLLECTIONS (Store Experience 3.0) ──
            Repeater {
                model: root.curatedCollections

                delegate: ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    visible: modelData.apps && modelData.apps.length > 0

                    // Collection Header: Title + Subtitle + Right-aligned "See all →"
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        ColumnLayout {
                            spacing: 2
                            Text {
                                text: modelData.title || ""
                                font.pixelSize: 16
                                font.bold: true
                                color: theme.text
                            }
                            Text {
                                text: modelData.subtitle || ""
                                font.pixelSize: 11
                                color: theme.textMuted
                            }
                        }

                        Item { Layout.fillWidth: true }

                        // See all action button
                        Rectangle {
                            height: 26
                            implicitWidth: seeAllRow.implicitWidth + 16
                            radius: theme.radiusSm
                            color: seeAllMouse.containsMouse ? theme.surfaceHover : "transparent"
                            border.color: seeAllMouse.containsMouse ? theme.borderStrong : "transparent"
                            border.width: 1

                            RowLayout {
                                id: seeAllRow
                                anchors.centerIn: parent
                                spacing: 4

                                Text {
                                    text: "See all"
                                    font.pixelSize: 12
                                    font.weight: Font.Medium
                                    color: theme.accent
                                }

                                RyzoraIcon {
                                    name: "chevron-right"
                                    size: 11
                                    color: theme.accent
                                }
                            }

                            MouseArea {
                                id: seeAllMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.openCollection(modelData.id, modelData.title, modelData.subtitle, modelData.category)
                                }
                            }
                        }
                    }

                    // Responsive Curated Cards Grid (fills row edge-to-edge with 0 half-cut cards)
                    GridLayout {
                        Layout.fillWidth: true
                        columns: root.responsiveColumns
                        rowSpacing: 12
                        columnSpacing: 12

                        Repeater {
                            model: root.getCuratedRowSlice(modelData.apps, root.responsiveColumns)
                            delegate: AppCard {
                                Layout.fillWidth: true
                                appData: modelData
                            }
                        }
                    }
                }
            }

            // ── 5. EXPLORE ALL APPLICATIONS ENTRY BANNER ──
            Rectangle {
                Layout.fillWidth: true
                height: 140
                radius: theme.radiusLg
                color: theme.surfaceElevated
                border.color: theme.borderStrong
                border.width: 1
                clip: true

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 24
                    spacing: 20

                    Rectangle {
                        width: 56
                        height: 56
                        radius: 14
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        RyzoraIcon {
                            anchors.centerIn: parent
                            name: "layout-grid"
                            size: 26
                            color: theme.accent
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Text {
                            text: "Explore Complete Application Catalogue"
                            font.pixelSize: 17
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            text: "Browse over 1,200 native Arch Linux, Flatpak, and AUR applications with advanced filters, category sorting, and full pagination."
                            font.pixelSize: 12
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }

                    RowLayout {
                        spacing: 10

                        Rectangle {
                            height: 38
                            implicitWidth: expAllBtnRow.implicitWidth + 24
                            radius: theme.radiusMd
                            color: expAllBtnMouse.containsMouse ? theme.accentHover : theme.accent

                            RowLayout {
                                id: expAllBtnRow
                                anchors.centerIn: parent
                                spacing: 6

                                Text {
                                    text: "Explore All Applications →"
                                    font.pixelSize: 12
                                    font.bold: true
                                    color: "#ffffff"
                                }
                            }

                            MouseArea {
                                id: expAllBtnMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.openExploreAll("apps")
                            }
                        }

                        Rectangle {
                            height: 38
                            implicitWidth: expPkgBtnRow.implicitWidth + 20
                            radius: theme.radiusSm
                            color: expPkgBtnMouse.containsMouse ? theme.surfaceHover : theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                id: expPkgBtnRow
                                anchors.centerIn: parent
                                spacing: 6

                                RyzoraIcon {
                                    name: "package"
                                    size: 13
                                    color: theme.textSecondary
                                }

                                Text {
                                    text: "All Packages (15k+)"
                                    font.pixelSize: 12
                                    font.weight: Font.Medium
                                    color: theme.text
                                }
                            }

                            MouseArea {
                                id: expPkgBtnMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.openExploreAll("all_packages")
                            }
                        }
                    }
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 1B. DEDICATED "SEE ALL" COLLECTION CATALOGUE VIEW (Store Experience 3.0)
    // ──────────────────────────────────────────────────────────────────────────
    Flickable {
        id: collectionScroll
        objectName: "appsCollectionScroll"
        anchors.fill: parent
        visible: root.selectedApp === null && root.storeMode === "collection"
        contentWidth: width
        contentHeight: collectionCol.implicitHeight + 40
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ScrollBar.vertical: ScrollBar {
            policy: ScrollBar.AsNeeded
            width: 6
            contentItem: Rectangle {
                implicitWidth: 6
                radius: 3
                color: theme.borderStrong
            }
        }

        ColumnLayout {
            id: collectionCol
            width: Math.max(300, parent.width - (root.width < 600 ? 24 : 40))
            anchors.horizontalCenter: parent.horizontalCenter
            y: 16
            spacing: 20

            // Header: Back button + Title + Subtitle + Count Badge
            RowLayout {
                Layout.fillWidth: true
                spacing: 14

                Rectangle {
                    height: 36
                    implicitWidth: backStoreRow.implicitWidth + 20
                    radius: theme.radiusMd
                    color: backStoreMouse.containsMouse ? theme.surfaceHover : theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: backStoreRow
                        anchors.centerIn: parent
                        spacing: 8

                        RyzoraIcon {
                            name: "chevron-left"
                            size: 14
                            color: theme.text
                        }

                        Text {
                            text: "Back to Store"
                            font.pixelSize: 12
                            font.weight: Font.Medium
                            color: theme.text
                        }
                    }

                    MouseArea {
                        id: backStoreMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.backToStorefront()
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: root.collectionTitle
                        font.pixelSize: 22
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: root.collectionSubtitle
                        font.pixelSize: 12
                        color: theme.textSecondary
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                // Count Badge
                Rectangle {
                    height: 28
                    implicitWidth: countBadgeRow.implicitWidth + 16
                    radius: theme.radiusSm
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: countBadgeRow
                        anchors.centerIn: parent
                        spacing: 6

                        RyzoraIcon {
                            name: "package"
                            size: 11
                            color: theme.accent
                        }

                        Text {
                            text: root.collectionTotalCount + " Applications"
                            font.pixelSize: 11
                            font.bold: true
                            color: theme.text
                        }
                    }
                }
            }

            // Filter & Search Toolbar
            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                // Search Input within collection
                Rectangle {
                    Layout.fillWidth: true
                    Layout.maximumWidth: 400
                    height: 34
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: collSearchInput.activeFocus ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 8

                        RyzoraIcon {
                            name: "search"
                            size: 13
                            color: collSearchInput.activeFocus ? theme.accent : theme.textMuted
                        }

                        TextInput {
                            id: collSearchInput
                            Layout.fillWidth: true
                            text: root.collectionSearchQuery
                            font.pixelSize: 12
                            color: theme.text
                            clip: true
                            onTextChanged: {
                                if (root.collectionSearchQuery !== text) {
                                    root.collectionSearchQuery = text
                                    root.collectionPage = 0
                                    root.updateCollectionSlice()
                                }
                            }

                            Text {
                                text: "Filter in this collection…"
                                font.pixelSize: 12
                                color: theme.textMuted
                                visible: !collSearchInput.text && !collSearchInput.activeFocus
                            }
                        }

                        Rectangle {
                            visible: collSearchInput.text.length > 0
                            width: 16
                            height: 16
                            radius: 8
                            color: collClearMouse.containsMouse ? theme.surfaceHover : "transparent"

                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "x"
                                size: 10
                                color: theme.textSecondary
                            }

                            MouseArea {
                                id: collClearMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    collSearchInput.text = ""
                                }
                            }
                        }
                    }
                }

                // Source Provider Filter Pills (All, Pacman, AUR, Flatpak)
                RowLayout {
                    spacing: 6

                    Repeater {
                        model: [
                            { id: "all", label: "All Sources" },
                            { id: "pacman", label: "Pacman (Official)" },
                            { id: "aur", label: "AUR" },
                            { id: "flatpak", label: "Flatpak" }
                        ]

                        Rectangle {
                            readonly property bool isSel: root.collectionProvider === modelData.id
                            height: 30
                            implicitWidth: provText.implicitWidth + 20
                            radius: theme.radiusSm
                            color: isSel ? theme.accent : (provMouse.containsMouse ? theme.surfaceHover : theme.surface)
                            border.color: isSel ? theme.accent : theme.borderSubtle
                            border.width: 1

                            Text {
                                id: provText
                                anchors.centerIn: parent
                                text: modelData.label
                                font.pixelSize: 11
                                font.weight: isSel ? Font.DemiBold : Font.Medium
                                color: isSel ? "#ffffff" : theme.textSecondary
                            }

                            MouseArea {
                                id: provMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    if (root.collectionProvider !== modelData.id) {
                                        root.collectionProvider = modelData.id
                                        root.collectionPage = 0
                                        root.updateCollectionSlice()
                                    }
                                }
                            }
                        }
                    }
                }

                Item { Layout.fillWidth: true }

                // Sort By Button
                Rectangle {
                    height: 30
                    implicitWidth: collSortRow.implicitWidth + 16
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: collSortMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: collSortRow
                        anchors.centerIn: parent
                        spacing: 6
                        RyzoraIcon { name: "filter"; size: 11; color: theme.textMuted }
                        Text {
                            text: "Sort: " + (root.collectionSortBy === "status" ? "Status" : "Name")
                            font.pixelSize: 11
                            color: theme.textSecondary
                        }
                    }

                    MouseArea {
                        id: collSortMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.collectionSortBy = (root.collectionSortBy === "name") ? "status" : "name"
                            root.collectionPage = 0
                            root.updateCollectionSlice()
                        }
                    }
                }
            }

            // Grid of Paginated Cards (up to 24 items)
            GridLayout {
                id: collGrid
                Layout.fillWidth: true
                columns: root.responsiveColumns
                rowSpacing: 12
                columnSpacing: 12

                Repeater {
                    model: root.collectionItems
                    delegate: AppCard {
                        Layout.fillWidth: true
                        appData: modelData
                    }
                }
            }

            // Empty State
            EmptyState {
                Layout.fillWidth: true
                Layout.preferredHeight: 240
                visible: root.collectionItems.length === 0
                iconName: "layout-grid"
                title: "No Applications in Collection"
                message: "No applications matched the current filters in " + root.collectionTitle + "."
                actionLabel: "Reset Filters"
                onActionClicked: {
                    root.collectionSearchQuery = ""
                    collSearchInput.text = ""
                    root.collectionProvider = "all"
                    root.collectionSortBy = "name"
                    root.collectionPage = 0
                    root.updateCollectionSlice()
                }
            }

            // Pagination Controls Bar
            Rectangle {
                Layout.fillWidth: true
                height: 52
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1
                visible: root.collectionTotalCount > 0

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 16
                    spacing: 12

                    Text {
                        text: "Showing " + ((root.collectionPage * root.pageSize) + 1) + "–" + Math.min((root.collectionPage + 1) * root.pageSize, root.collectionTotalCount) + " of " + root.collectionTotalCount + " applications"
                        font.pixelSize: 12
                        color: theme.textSecondary
                    }

                    Item { Layout.fillWidth: true }

                    // Previous Page Button
                    Rectangle {
                        readonly property bool canPrev: root.collectionPage > 0
                        height: 32
                        implicitWidth: prevCollRow.implicitWidth + 18
                        radius: theme.radiusSm
                        color: canPrev ? (prevCollMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated) : theme.surface
                        border.color: canPrev ? theme.borderSubtle : "transparent"
                        opacity: canPrev ? 1.0 : 0.4

                        RowLayout {
                            id: prevCollRow
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon { name: "chevron-left"; size: 12; color: theme.text }
                            Text { text: "Previous"; font.pixelSize: 11; font.weight: Font.Medium; color: theme.text }
                        }

                        MouseArea {
                            id: prevCollMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            enabled: parent.canPrev
                            cursorShape: parent.canPrev ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: {
                                if (root.collectionPage > 0) {
                                    root.collectionPage--
                                    root.updateCollectionSlice()
                                    collectionScroll.contentY = 0
                                }
                            }
                        }
                    }

                    // Page Indicator Chip
                    Rectangle {
                        height: 32
                        implicitWidth: pageCollText.implicitWidth + 20
                        radius: theme.radiusSm
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle

                        Text {
                            id: pageCollText
                            anchors.centerIn: parent
                            text: "Page " + (root.collectionPage + 1) + " of " + Math.max(1, Math.ceil(root.collectionTotalCount / root.pageSize))
                            font.pixelSize: 11
                            font.bold: true
                            color: theme.text
                        }
                    }

                    // Next Page Button
                    Rectangle {
                        readonly property bool canNext: (root.collectionPage + 1) * root.pageSize < root.collectionTotalCount
                        height: 32
                        implicitWidth: nextCollRow.implicitWidth + 18
                        radius: theme.radiusSm
                        color: canNext ? (nextCollMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated) : theme.surface
                        border.color: canNext ? theme.borderSubtle : "transparent"
                        opacity: canNext ? 1.0 : 0.4

                        RowLayout {
                            id: nextCollRow
                            anchors.centerIn: parent
                            spacing: 6
                            Text { text: "Next"; font.pixelSize: 11; font.weight: Font.Medium; color: theme.text }
                            RyzoraIcon { name: "chevron-right"; size: 12; color: theme.text }
                        }

                        MouseArea {
                            id: nextCollMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            enabled: parent.canNext
                            cursorShape: parent.canNext ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: {
                                if ((root.collectionPage + 1) * root.pageSize < root.collectionTotalCount) {
                                    root.collectionPage++
                                    root.updateCollectionSlice()
                                    collectionScroll.contentY = 0
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 1C. COMPLETE APPLICATION & PACKAGE CATALOGUE VIEW (Store Experience 3.0)
    // ──────────────────────────────────────────────────────────────────────────
    Flickable {
        id: exploreScroll
        objectName: "appsExploreScroll"
        anchors.fill: parent
        visible: root.selectedApp === null && root.storeMode === "explore_all"
        contentWidth: width
        contentHeight: exploreCol.implicitHeight + 40
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ScrollBar.vertical: ScrollBar {
            policy: ScrollBar.AsNeeded
            width: 6
            contentItem: Rectangle {
                implicitWidth: 6
                radius: 3
                color: theme.borderStrong
            }
        }

        ColumnLayout {
            id: exploreCol
            width: Math.max(300, parent.width - (root.width < 600 ? 24 : 40))
            anchors.horizontalCenter: parent.horizontalCenter
            y: 16
            spacing: 16

            // Header: Back button + Title + Scope Switcher + Count Badge
            RowLayout {
                Layout.fillWidth: true
                spacing: 14

                Rectangle {
                    height: 36
                    implicitWidth: backExpRow.implicitWidth + 20
                    radius: theme.radiusMd
                    color: backExpMouse.containsMouse ? theme.surfaceHover : theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: backExpRow
                        anchors.centerIn: parent
                        spacing: 8

                        RyzoraIcon {
                            name: "chevron-left"
                            size: 14
                            color: theme.text
                        }

                        Text {
                            text: "Back to Store"
                            font.pixelSize: 12
                            font.weight: Font.Medium
                            color: theme.text
                        }
                    }

                    MouseArea {
                        id: backExpMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.backToStorefront()
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: root.catalogMode === "all_packages" ? "System Package Repository" : "All Applications Catalogue"
                        font.pixelSize: 22
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: root.catalogMode === "all_packages"
                            ? "Complete repository of system libraries, command-line tools, and uncurated packages."
                            : "Browse the full catalogue of native Arch Linux, Flatpak, and AUR desktop applications."
                        font.pixelSize: 12
                        color: theme.textSecondary
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                // Scope Switcher (Applications vs All Packages)
                Rectangle {
                    height: 32
                    implicitWidth: expScopeRow.implicitWidth + 8
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: expScopeRow
                        anchors.centerIn: parent
                        spacing: 2

                        Rectangle {
                            height: 26
                            implicitWidth: expAppsBtn.implicitWidth + 14
                            radius: theme.radiusSm - 1
                            color: root.catalogMode === "apps" ? theme.accent : "transparent"

                            RowLayout {
                                id: expAppsBtn
                                anchors.centerIn: parent
                                spacing: 5
                                RyzoraIcon {
                                    name: "layout-grid"
                                    size: 11
                                    color: root.catalogMode === "apps" ? "#ffffff" : theme.textSecondary
                                }
                                Text {
                                    text: "Applications"
                                    font.pixelSize: 11
                                    font.weight: root.catalogMode === "apps" ? Font.DemiBold : Font.Medium
                                    color: root.catalogMode === "apps" ? "#ffffff" : theme.textSecondary
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    if (root.catalogMode !== "apps") {
                                        root.catalogMode = "apps"
                                        root.explorePage = 0
                                        backend.filterApps("all", root.exploreSearchQuery, root.exploreProvider)
                                        root.updateExploreSlice()
                                    }
                                }
                            }
                        }

                        Rectangle {
                            height: 26
                            implicitWidth: expPkgsBtn.implicitWidth + 14
                            radius: theme.radiusSm - 1
                            color: root.catalogMode === "all_packages" ? theme.accent : "transparent"

                            RowLayout {
                                id: expPkgsBtn
                                anchors.centerIn: parent
                                spacing: 5
                                RyzoraIcon {
                                    name: "package"
                                    size: 11
                                    color: root.catalogMode === "all_packages" ? "#ffffff" : theme.textSecondary
                                }
                                Text {
                                    text: "All Packages (15k+)"
                                    font.pixelSize: 11
                                    font.weight: root.catalogMode === "all_packages" ? Font.DemiBold : Font.Medium
                                    color: root.catalogMode === "all_packages" ? "#ffffff" : theme.textSecondary
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    if (root.catalogMode !== "all_packages") {
                                        root.catalogMode = "all_packages"
                                        root.explorePage = 0
                                        backend.filterApps("all_packages", root.exploreSearchQuery, root.exploreProvider)
                                        root.updateExploreSlice()
                                    }
                                }
                            }
                        }
                    }
                }

                // Total Count Badge
                Rectangle {
                    height: 28
                    implicitWidth: expCountBadgeRow.implicitWidth + 16
                    radius: theme.radiusSm
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: expCountBadgeRow
                        anchors.centerIn: parent
                        spacing: 6

                        RyzoraIcon {
                            name: "layout-grid"
                            size: 11
                            color: theme.accent
                        }

                        Text {
                            text: root.exploreTotalCount + " Items"
                            font.pixelSize: 11
                            font.bold: true
                            color: theme.text
                        }
                    }
                }
            }

            // Category Filter Pills Row (Horizontal Flickable)
            Flickable {
                Layout.fillWidth: true
                height: 32
                contentWidth: expCatRow.implicitWidth
                contentHeight: 32
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                flickableDirection: Flickable.HorizontalFlick

                RowLayout {
                    id: expCatRow
                    height: 32
                    spacing: 6

                    Repeater {
                        model: [
                            { label: "All items", id: "all", icon: "layout-grid" },
                            { label: "Development", id: "development", icon: "terminal" },
                            { label: "Internet", id: "internet", icon: "compass" },
                            { label: "Productivity", id: "productivity", icon: "check" },
                            { label: "Multimedia", id: "multimedia", icon: "play" },
                            { label: "Graphics", id: "graphics", icon: "palette" },
                            { label: "Utilities", id: "utilities", icon: "settings" },
                            { label: "System", id: "system", icon: "activity" },
                            { label: "Games", id: "games", icon: "zap" },
                            { label: "Installed", id: "installed", icon: "check" }
                        ]

                        Rectangle {
                            readonly property bool isSelected: root.exploreCategory === modelData.id
                            height: 30
                            implicitWidth: expCatItemRow.implicitWidth + 18
                            radius: theme.radiusSm
                            color: isSelected ? theme.accent : (expCatMouse.containsMouse ? theme.surfaceHover : theme.surface)
                            border.color: isSelected ? theme.accent : theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                id: expCatItemRow
                                anchors.centerIn: parent
                                spacing: 6

                                RyzoraIcon {
                                    name: modelData.icon
                                    size: 11
                                    color: isSelected ? "#ffffff" : theme.textSecondary
                                }

                                Text {
                                    text: modelData.label
                                    font.pixelSize: 11
                                    font.weight: isSelected ? Font.DemiBold : Font.Medium
                                    color: isSelected ? "#ffffff" : theme.textSecondary
                                }
                            }

                            MouseArea {
                                id: expCatMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    if (root.exploreCategory !== modelData.id) {
                                        root.exploreCategory = modelData.id
                                        root.explorePage = 0
                                        root.updateExploreSlice()
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Search & Provider Source Toolbar
            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                // Search Input Box
                Rectangle {
                    Layout.fillWidth: true
                    Layout.maximumWidth: 420
                    height: 34
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: expSearchInput.activeFocus ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 8

                        RyzoraIcon {
                            name: "search"
                            size: 13
                            color: expSearchInput.activeFocus ? theme.accent : theme.textMuted
                        }

                        TextInput {
                            id: expSearchInput
                            Layout.fillWidth: true
                            text: root.exploreSearchQuery
                            font.pixelSize: 12
                            color: theme.text
                            clip: true
                            onTextChanged: {
                                if (root.exploreSearchQuery !== text) {
                                    root.exploreSearchQuery = text
                                    root.explorePage = 0
                                    root.updateExploreSlice()
                                }
                            }

                            Text {
                                text: "Search catalogue by name, package ID, keyword…"
                                font.pixelSize: 12
                                color: theme.textMuted
                                visible: !expSearchInput.text && !expSearchInput.activeFocus
                            }
                        }

                        Rectangle {
                            visible: expSearchInput.text.length > 0
                            width: 16
                            height: 16
                            radius: 8
                            color: expClearMouse.containsMouse ? theme.surfaceHover : "transparent"

                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "x"
                                size: 10
                                color: theme.textSecondary
                            }

                            MouseArea {
                                id: expClearMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    expSearchInput.text = ""
                                }
                            }
                        }
                    }
                }

                // Source Provider Filter Pills
                RowLayout {
                    spacing: 6

                    Repeater {
                        model: [
                            { id: "all", label: "All Sources" },
                            { id: "pacman", label: "Pacman" },
                            { id: "aur", label: "AUR" },
                            { id: "flatpak", label: "Flatpak" }
                        ]

                        Rectangle {
                            readonly property bool isSel: root.exploreProvider === modelData.id
                            height: 30
                            implicitWidth: expProvText.implicitWidth + 18
                            radius: theme.radiusSm
                            color: isSel ? theme.accent : (expProvMouse.containsMouse ? theme.surfaceHover : theme.surface)
                            border.color: isSel ? theme.accent : theme.borderSubtle
                            border.width: 1

                            Text {
                                id: expProvText
                                anchors.centerIn: parent
                                text: modelData.label
                                font.pixelSize: 11
                                font.weight: isSel ? Font.DemiBold : Font.Medium
                                color: isSel ? "#ffffff" : theme.textSecondary
                            }

                            MouseArea {
                                id: expProvMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    if (root.exploreProvider !== modelData.id) {
                                        root.exploreProvider = modelData.id
                                        root.explorePage = 0
                                        root.updateExploreSlice()
                                    }
                                }
                            }
                        }
                    }
                }

                Item { Layout.fillWidth: true }

                // Sort By Button
                Rectangle {
                    height: 30
                    implicitWidth: expSortRow.implicitWidth + 16
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: expSortMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: expSortRow
                        anchors.centerIn: parent
                        spacing: 6
                        RyzoraIcon { name: "filter"; size: 11; color: theme.textMuted }
                        Text {
                            text: "Sort: " + (root.exploreSortBy === "status" ? "Status" : (root.exploreSortBy === "category" ? "Category" : "Name"))
                            font.pixelSize: 11
                            color: theme.textSecondary
                        }
                    }

                    MouseArea {
                        id: expSortMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (root.exploreSortBy === "name") root.exploreSortBy = "category"
                            else if (root.exploreSortBy === "category") root.exploreSortBy = "status"
                            else root.exploreSortBy = "name"
                            root.explorePage = 0
                            root.updateExploreSlice()
                        }
                    }
                }
            }

            // Grid of Paginated Cards (up to 24 items)
            GridLayout {
                id: expGrid
                Layout.fillWidth: true
                columns: root.responsiveColumns
                rowSpacing: 12
                columnSpacing: 12

                Repeater {
                    model: root.exploreItems
                    delegate: AppCard {
                        Layout.fillWidth: true
                        appData: modelData
                    }
                }
            }

            // Empty State
            EmptyState {
                Layout.fillWidth: true
                Layout.preferredHeight: 240
                visible: root.exploreItems.length === 0
                iconName: "layout-grid"
                title: "No Applications Found"
                message: "No software matched category '" + root.exploreCategory + "'" + (root.exploreSearchQuery ? " and query '" + root.exploreSearchQuery + "'" : "") + "."
                actionLabel: "Clear Search & Filters"
                onActionClicked: {
                    root.exploreCategory = "all"
                    root.exploreSearchQuery = ""
                    expSearchInput.text = ""
                    root.exploreProvider = "all"
                    root.exploreSortBy = "name"
                    root.explorePage = 0
                    root.updateExploreSlice()
                }
            }

            // Pagination Controls Bar
            Rectangle {
                Layout.fillWidth: true
                height: 52
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1
                visible: root.exploreTotalCount > 0

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 16
                    spacing: 12

                    Text {
                        text: "Showing " + ((root.explorePage * root.pageSize) + 1) + "–" + Math.min((root.explorePage + 1) * root.pageSize, root.exploreTotalCount) + " of " + root.exploreTotalCount + " items"
                        font.pixelSize: 12
                        color: theme.textSecondary
                    }

                    Item { Layout.fillWidth: true }

                    // Previous Page Button
                    Rectangle {
                        readonly property bool canPrev: root.explorePage > 0
                        height: 32
                        implicitWidth: prevExpRow.implicitWidth + 18
                        radius: theme.radiusSm
                        color: canPrev ? (prevExpMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated) : theme.surface
                        border.color: canPrev ? theme.borderSubtle : "transparent"
                        opacity: canPrev ? 1.0 : 0.4

                        RowLayout {
                            id: prevExpRow
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon { name: "chevron-left"; size: 12; color: theme.text }
                            Text { text: "Previous"; font.pixelSize: 11; font.weight: Font.Medium; color: theme.text }
                        }

                        MouseArea {
                            id: prevExpMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            enabled: parent.canPrev
                            cursorShape: parent.canPrev ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: {
                                if (root.explorePage > 0) {
                                    root.explorePage--
                                    root.updateExploreSlice()
                                    exploreScroll.contentY = 0
                                }
                            }
                        }
                    }

                    // Page Indicator Chip
                    Rectangle {
                        height: 32
                        implicitWidth: pageExpText.implicitWidth + 20
                        radius: theme.radiusSm
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle

                        Text {
                            id: pageExpText
                            anchors.centerIn: parent
                            text: "Page " + (root.explorePage + 1) + " of " + Math.max(1, Math.ceil(root.exploreTotalCount / root.pageSize))
                            font.pixelSize: 11
                            font.bold: true
                            color: theme.text
                        }
                    }

                    // Next Page Button
                    Rectangle {
                        readonly property bool canNext: (root.explorePage + 1) * root.pageSize < root.exploreTotalCount
                        height: 32
                        implicitWidth: nextExpRow.implicitWidth + 18
                        radius: theme.radiusSm
                        color: canNext ? (nextExpMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated) : theme.surface
                        border.color: canNext ? theme.borderSubtle : "transparent"
                        opacity: canNext ? 1.0 : 0.4

                        RowLayout {
                            id: nextExpRow
                            anchors.centerIn: parent
                            spacing: 6
                            Text { text: "Next"; font.pixelSize: 11; font.weight: Font.Medium; color: theme.text }
                            RyzoraIcon { name: "chevron-right"; size: 12; color: theme.text }
                        }

                        MouseArea {
                            id: nextExpMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            enabled: parent.canNext
                            cursorShape: parent.canNext ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: {
                                if ((root.explorePage + 1) * root.pageSize < root.exploreTotalCount) {
                                    root.explorePage++
                                    root.updateExploreSlice()
                                    exploreScroll.contentY = 0
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 2. DEDICATED APPLICATION DETAILS PRODUCT PAGE (Classic Windows 11 Store)
    // ──────────────────────────────────────────────────────────────────────────
    Rectangle {
        id: appDetailOverlay
        anchors.fill: parent
        visible: root.selectedApp !== null
        color: theme.bg
        z: 10
        clip: true

        readonly property string uiFont: "Segoe UI Variable, Segoe UI, -apple-system, BlinkMacSystemFont, 'Inter', sans-serif"
        readonly property bool isWideDetail: root.width >= 760
        readonly property int outerPad: root.width < 760 ? 12 : 16
        readonly property int paneGap: 14
        readonly property int leftPaneWidth: root.width < 960 ? 300 : (root.width < 1400 ? 330 : 350)

        // ══════════════════════════════════════════════════════════════════════
        // A. LEFT PRODUCT PANE (Fixed-Width Fluent Product Identity & Primary Action)
        // ══════════════════════════════════════════════════════════════════════
        Rectangle {
            id: leftProductPane
            x: appDetailOverlay.outerPad
            y: appDetailOverlay.isWideDetail
                ? appDetailOverlay.outerPad
                : (appDetailOverlay.outerPad - rightDetailFlickable.contentY)
            width: appDetailOverlay.isWideDetail
                ? appDetailOverlay.leftPaneWidth
                : (appDetailOverlay.width - appDetailOverlay.outerPad * 2)
            height: appDetailOverlay.isWideDetail
                ? (appDetailOverlay.height - appDetailOverlay.outerPad * 2)
                : (leftPaneCol.implicitHeight + 48)
            radius: 10
            color: theme.surface
            border.color: theme.borderSubtle
            border.width: 1
            clip: true
            z: 2

            Flickable {
                id: leftPaneFlickable
                anchors.fill: parent
                contentWidth: width
                contentHeight: leftPaneCol.height + 44
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                interactive: contentHeight > height

                ColumnLayout {
                    id: leftPaneCol
                    x: 22
                    y: 22
                    width: parent.width - 44
                    height: Math.max(leftPaneFlickable.height - 44, implicitHeight)
                    spacing: 0

                // ── 1. Top Back Navigation ──
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        height: 30
                        implicitWidth: backBtnRow.implicitWidth + 18
                        radius: 6
                        color: backMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            id: backBtnRow
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon { name: "chevron-left"; size: 12; color: theme.textSecondary }
                            Text {
                                text: "Back to Apps"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 12
                                font.weight: Font.Medium
                                color: theme.text
                            }
                        }

                        MouseArea {
                            id: backMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.setSelectedApp(null)
                        }
                    }

                    Item { Layout.fillWidth: true }
                }

                Item { Layout.preferredHeight: appDetailOverlay.isWideDetail ? 26 : 16 }

                // ── 2. Prominent Centered Application Icon (120 × 120 px) ──
                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    width: 120
                    height: 120
                    radius: 24
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    Image {
                        anchors.centerIn: parent
                        width: 90
                        height: 90
                        source: root.selectedApp ? (root.selectedApp.icon_url || "") : ""
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                        visible: status === Image.Ready
                    }

                    RyzoraIcon {
                        anchors.centerIn: parent
                        visible: !root.selectedApp || !root.selectedApp.icon_url || root.selectedApp.icon_url === ""
                        name: root.selectedApp ? root.getCategoryIcon(root.selectedApp.category) : "layout-grid"
                        size: 54
                        color: theme.textMuted
                    }
                }

                Item { Layout.preferredHeight: 18 }

                // ── 3. Application Title (32 px, Semibold) ──
                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: root.selectedApp ? (root.selectedApp.display_name || root.selectedApp.name || "") : ""
                    font.family: appDetailOverlay.uiFont
                    font.pixelSize: 32
                    font.weight: Font.DemiBold
                    font.bold: true
                    color: theme.text
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                }

                Item { Layout.preferredHeight: 6 }

                // ── 4. Publisher Name (Subtle Blue Clickable Link Style) ──
                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: root.selectedApp ? (root.selectedApp.publisher || "Arch Linux Community") : ""
                    font.family: appDetailOverlay.uiFont
                    font.pixelSize: 14
                    font.weight: Font.Medium
                    color: theme.accent
                    elide: Text.ElideRight

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: (root.selectedApp && root.selectedApp.website) ? Qt.PointingHandCursor : Qt.ArrowCursor
                        onClicked: {
                            if (root.selectedApp && root.selectedApp.website) {
                                Qt.openUrlExternally(root.selectedApp.website)
                            }
                        }
                    }
                }

                Item { Layout.preferredHeight: 22 }

                // ── 5. Primary Action Button (46 px high, Microsoft Blue Accent) ──
                ColumnLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 8

                    // Primary Action: Open Application (when installed)
                    Rectangle {
                        visible: Boolean(root.selectedApp && root.isCurrentProviderInstalled())
                        width: 228
                        height: 46
                        radius: 6
                        color: openBtnMouse.containsMouse ? theme.accentHover : theme.accent

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 8
                            RyzoraIcon { name: "play"; size: 13; color: "#ffffff" }
                            Text {
                                text: "Open"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 14
                                font.weight: Font.DemiBold
                                font.bold: true
                                color: "#ffffff"
                            }
                        }

                        MouseArea {
                            id: openBtnMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.selectedApp) {
                                    root.launchRequested(root.currentActionAppId())
                                }
                            }
                        }
                    }

                    // Primary Action: Install / Get (when not installed and not pending)
                    Rectangle {
                        visible: Boolean(root.selectedApp && !root.isCurrentProviderInstalled() && !root.isSelectedAppPending())
                        width: 228
                        height: 46
                        radius: 6
                        color: installBtnMouse.containsMouse ? theme.accentHover : theme.accent

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 8
                            RyzoraIcon {
                                name: "download"
                                size: 14
                                color: "#ffffff"
                            }
                            Text {
                                text: "Install"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 14
                                font.weight: Font.DemiBold
                                font.bold: true
                                color: "#ffffff"
                            }
                        }

                        MouseArea {
                            id: installBtnMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.selectedApp) {
                                    root.requestInstall(root.currentActionAppId())
                                }
                            }
                        }
                    }

                    // ── Active Installation Placeholder (Directs attention to Wide Install Section) ──
                    Rectangle {
                        id: activeInstallPlaceholder
                        visible: Boolean(root.selectedApp && !root.isCurrentProviderInstalled() && root.isSelectedAppPending() && root.pendingAction === "install")
                        width: 228
                        height: 40
                        radius: 8
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 8
                            RyzoraIcon {
                                name: "spinner"
                                size: 15
                                color: theme.accent
                                RotationAnimation on rotation {
                                    from: 0
                                    to: 360
                                    duration: 1000
                                    loops: Animation.Infinite
                                    running: activeInstallPlaceholder.visible
                                }
                            }
                            Text {
                                text: root.installStage === "cancelling" ? "Cancelling..." : "Installing..."
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 13
                                font.weight: Font.DemiBold
                                color: theme.textSecondary
                            }
                        }
                    }

                    // Secondary Action: Uninstall (when installed)
                    Rectangle {
                        visible: Boolean(root.selectedApp && root.isCurrentProviderInstalled())
                        width: 228
                        height: 34
                        radius: 6
                        color: uninstBtnMouse.containsMouse
                            ? (theme.resolvedTheme === "light" ? "#FEE2E2" : "#451A1A")
                            : theme.surfaceElevated
                        border.color: uninstBtnMouse.containsMouse ? "#EF4444" : theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon {
                                name: "trash"
                                size: 12
                                color: uninstBtnMouse.containsMouse ? "#DC2626" : theme.textSecondary
                            }
                            Text {
                                text: root.isSelectedAppPending() && root.pendingAction === "uninstall"
                                    ? "Removing..."
                                    : "Uninstall"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 12
                                font.weight: Font.Medium
                                color: uninstBtnMouse.containsMouse ? "#DC2626" : theme.textSecondary
                            }
                        }

                        MouseArea {
                            id: uninstBtnMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            enabled: !root.isSelectedAppPending()
                            cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: {
                                if (root.selectedApp) {
                                    root.requestUninstall(root.currentActionAppId())
                                }
                            }
                        }
                    }

                    // Provider Source Selector (Pacman / AUR / Flatpak)
                    RowLayout {
                        Layout.alignment: Qt.AlignHCenter
                        Layout.topMargin: 4
                        spacing: 6
                        visible: Boolean(root.selectedApp && root.selectedApp.available_providers && root.selectedApp.available_providers.length > 0)

                        Repeater {
                            model: root.selectedApp && root.selectedApp.available_providers ? root.selectedApp.available_providers : []
                            delegate: Rectangle {
                                readonly property bool isCurrentProvider: root.selectedProviderId === modelData.provider_id
                                height: 26
                                implicitWidth: Math.max(68, provRow.implicitWidth + 18)
                                radius: 13
                                color: isCurrentProvider
                                    ? Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.14)
                                    : (provMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                                border.color: isCurrentProvider ? theme.accent : theme.borderSubtle
                                border.width: 1

                                RowLayout {
                                    id: provRow
                                    anchors.centerIn: parent
                                    spacing: 5
                                    Rectangle {
                                        width: 6
                                        height: 6
                                        radius: 3
                                        visible: Boolean(modelData.is_installed)
                                        color: "#10B981"
                                    }
                                    Text {
                                        text: modelData.name || modelData.provider_id
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 11
                                        font.weight: isCurrentProvider ? Font.DemiBold : Font.Medium
                                        font.bold: isCurrentProvider
                                        color: isCurrentProvider ? theme.accent : theme.textSecondary
                                    }
                                }

                                MouseArea {
                                    id: provMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    enabled: !root.isSelectedAppPending()
                                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                                    onClicked: {
                                        root.selectedProviderId = modelData.provider_id
                                        root.selectedTargetId = modelData.target_id
                                    }
                                }
                            }
                        }
                    }
                }

                Item { Layout.preferredHeight: 22 }

                // ── 6. Three-Column Legitimate Metadata Stat Row (with Vertical Dividers) ──
                RowLayout {
                    Layout.fillWidth: true
                    Layout.preferredWidth: leftPaneCol.width
                    spacing: 4

                    // 1. Version Stat
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.preferredWidth: Math.floor((leftPaneCol.width - 16) / 3)
                        Layout.maximumWidth: Math.floor((leftPaneCol.width - 16) / 3)
                        Layout.alignment: Qt.AlignVCenter
                        clip: true
                        spacing: 2
                        Text {
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                            text: root.currentInstalledVersion() || root.currentAvailableVersion() || "Latest"
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 13
                            font.weight: Font.DemiBold
                            font.bold: true
                            color: theme.text
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                            text: "Version"
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 11
                            color: theme.textMuted
                            elide: Text.ElideRight
                        }
                    }

                    // Vertical Divider 1
                    Rectangle {
                        width: 1
                        height: 28
                        color: theme.borderSubtle
                    }

                    // 2. Size Stat
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.preferredWidth: Math.floor((leftPaneCol.width - 16) / 3)
                        Layout.maximumWidth: Math.floor((leftPaneCol.width - 16) / 3)
                        Layout.alignment: Qt.AlignVCenter
                        clip: true
                        spacing: 2
                        Text {
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                            text: root.currentAppDisplaySize()
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 13
                            font.weight: Font.DemiBold
                            font.bold: true
                            color: theme.text
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                            text: root.currentAppSizeSubtext()
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 11
                            color: theme.textMuted
                            elide: Text.ElideRight
                        }
                    }

                    // Vertical Divider 2
                    Rectangle {
                        width: 1
                        height: 28
                        color: theme.borderSubtle
                    }

                    // 3. Status / State Stat
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.preferredWidth: Math.floor((leftPaneCol.width - 16) / 3)
                        Layout.maximumWidth: Math.floor((leftPaneCol.width - 16) / 3)
                        Layout.alignment: Qt.AlignVCenter
                        clip: true
                        spacing: 2
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignHCenter
                            spacing: 4
                            Rectangle {
                                width: 6
                                height: 6
                                radius: 3
                                color: (root.selectedApp && root.isCurrentProviderInstalled()) ? "#10B981" : theme.accent
                            }
                            Text {
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignLeft
                                text: {
                                    if (!root.selectedApp) return ""
                                    if (root.isSelectedAppPending() && root.pendingAction === "install") {
                                        return root.installStage === "cancelling" ? "Cancelling" : "Installing"
                                    }
                                    if (root.isCurrentProviderInstalled()) return "Installed"
                                    var src = root.currentSelectedProviderSource()
                                    if (src) {
                                        if (src.provider_id === "flatpak") return "Flathub"
                                        if (src.provider_id === "aur") return "AUR"
                                        return src.repository || "Official"
                                    }
                                    return root.selectedApp.repository || "Official"
                                }
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 13
                                font.weight: Font.DemiBold
                                font.bold: true
                                color: {
                                    if (root.selectedApp && root.isCurrentProviderInstalled()) return (theme.resolvedTheme === "light" ? "#059669" : "#34D399")
                                    if (root.isSelectedAppPending() && root.pendingAction === "install") return theme.accent
                                    return theme.text
                                }
                                elide: Text.ElideRight
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                            text: {
                                if (root.isSelectedAppPending() && root.pendingAction === "install") return "In progress"
                                if (root.selectedApp && root.isCurrentProviderInstalled()) return "System state"
                                var src = root.currentSelectedProviderSource()
                                if (src && src.provider_id === "flatpak") return "Sandboxed"
                                if (src && src.provider_id === "aur") return "AUR User Repo"
                                return "Official Repo"
                            }
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 11
                            color: theme.textMuted
                            elide: Text.ElideRight
                        }
                    }
                }

                Item { Layout.preferredHeight: 20 }

                // ── 7. Centered Category Pill Badge ──
                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    height: 30
                    implicitWidth: Math.max(120, catPillText.implicitWidth + 30)
                    radius: 15
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    Text {
                        id: catPillText
                        anchors.centerIn: parent
                        text: root.selectedApp ? (root.selectedApp.category || "Application") : ""
                        font.family: appDetailOverlay.uiFont
                        font.pixelSize: 12
                        font.weight: Font.Medium
                        color: theme.textSecondary
                    }
                }

                Item { Layout.preferredHeight: 16 }

                // ── 8. Concise Summary Description (Comfortable Line Height) ──
                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: root.selectedApp ? (root.selectedApp.summary || "") : ""
                    font.family: appDetailOverlay.uiFont
                    font.pixelSize: 13
                    lineHeight: 1.45
                    color: theme.textSecondary
                    wrapMode: Text.WordWrap
                    maximumLineCount: 3
                    elide: Text.ElideRight
                }

                // Flexible spacer pushing bottom details down cleanly
                Item {
                    Layout.fillHeight: true
                    Layout.minimumHeight: 20
                }

                // ── 9. Bottom Footer Row (Architecture / License / Website) ──
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    Rectangle {
                        width: 36
                        height: 36
                        radius: 6
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 0
                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: "LINUX"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 8
                                font.weight: Font.Bold
                                font.bold: true
                                color: theme.textMuted
                            }
                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: "x64"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 11
                                font.weight: Font.DemiBold
                                font.bold: true
                                color: theme.text
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Text {
                            Layout.fillWidth: true
                            text: root.selectedApp && root.selectedApp.license
                                ? root.selectedApp.license.toUpperCase()
                                : "OPEN SOURCE"
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 11
                            font.weight: Font.Medium
                            color: theme.text
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: {
                                var src = root.currentSelectedProviderSource()
                                if (src) {
                                    if (src.provider_id === "flatpak") return "Flathub · Flatpak Container"
                                    if (src.provider_id === "aur") return "Arch User Repository (AUR)"
                                    return "Arch Linux · " + (src.repository || "extra")
                                }
                                return root.selectedApp && root.selectedApp.repository
                                    ? ("Arch Linux · " + root.selectedApp.repository)
                                    : "Arch Linux Native"
                            }
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 11
                            color: theme.textMuted
                            elide: Text.ElideRight
                        }
                    }

                    // Official Website External Action Button
                    Rectangle {
                        visible: Boolean(root.selectedApp && root.selectedApp.website)
                        width: 32
                        height: 32
                        radius: 6
                        color: footerWebMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        RyzoraIcon {
                            anchors.centerIn: parent
                            name: "compass"
                            size: 15
                            color: footerWebMouse.containsMouse ? theme.accent : theme.textSecondary
                        }

                        MouseArea {
                            id: footerWebMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.selectedApp && root.selectedApp.website) {
                                    Qt.openUrlExternally(root.selectedApp.website)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

        // ══════════════════════════════════════════════════════════════════════
        // B. RIGHT CONTENT PANE (Screenshots, Description, Info & Requirements)
        // ══════════════════════════════════════════════════════════════════════
        Flickable {
            id: rightDetailFlickable
            x: appDetailOverlay.isWideDetail
                ? (appDetailOverlay.outerPad + appDetailOverlay.leftPaneWidth + appDetailOverlay.paneGap)
                : appDetailOverlay.outerPad
            y: appDetailOverlay.outerPad
            width: appDetailOverlay.isWideDetail
                ? (appDetailOverlay.width - appDetailOverlay.leftPaneWidth - appDetailOverlay.outerPad * 2 - appDetailOverlay.paneGap)
                : (appDetailOverlay.width - appDetailOverlay.outerPad * 2)
            height: appDetailOverlay.height - appDetailOverlay.outerPad * 2
            contentWidth: width
            contentHeight: rightPaneCol.implicitHeight + 24
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
                width: 6
                contentItem: Rectangle {
                    implicitWidth: 6
                    radius: 3
                    color: theme.borderStrong
                }
            }

            ColumnLayout {
                id: rightPaneCol
                width: rightDetailFlickable.width
                spacing: appDetailOverlay.paneGap

                // Responsive top spacer when stacked vertically on narrow screens
                Item {
                    Layout.fillWidth: true
                    Layout.preferredHeight: leftProductPane.height + 6
                    visible: !appDetailOverlay.isWideDetail
                }

                // ── Action Error Notification Banner (Visible on failure) ──
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: errRow.implicitHeight + 20
                    radius: 8
                    visible: root.actionErrorMessage !== ""
                    color: theme.resolvedTheme === "light" ? "#FEF2F2" : "#450A0A"
                    border.color: "#EF4444"
                    border.width: 1

                    RowLayout {
                        id: errRow
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12

                        RyzoraIcon { name: "alert"; size: 16; color: "#EF4444" }

                        Text {
                            Layout.fillWidth: true
                            text: root.actionErrorMessage
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 13
                            font.weight: Font.Medium
                            color: theme.resolvedTheme === "light" ? "#B91C1C" : "#FCA5A5"
                            wrapMode: Text.WordWrap
                        }

                        Rectangle {
                            width: 24; height: 24; radius: 12
                            color: errCloseMouse.containsMouse ? (theme.resolvedTheme === "light" ? "#FEE2E2" : "#7F1D1D") : "transparent"
                            Text { anchors.centerIn: parent; text: "✕"; font.pixelSize: 11; font.bold: true; color: theme.textSecondary }
                            MouseArea {
                                id: errCloseMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.actionErrorMessage = ""
                            }
                        }
                    }
                }

                // ── REUSABLE INSTALLATION WORKSPACE ──
                InstallProgressView {
                    id: installWorkspace
                    Layout.fillWidth: true
                    visible: Boolean(root.selectedApp && !root.selectedApp.is_installed && root.isSelectedAppPending() && root.pendingAction === "install")
                    appName: root.selectedApp ? root.selectedApp.name : "Application"
                    iconUrl: (root.selectedApp && root.selectedApp.icon_url) ? root.selectedApp.icon_url : ""
                    stage: root.installStage
                    percentage: root.installPercentage
                    isDeterminate: root.installIsDeterminate
                    message: root.installMessage
                    bytesDownloaded: root.installBytesDownloaded
                    bytesTotal: root.installBytesTotal
                    downloadSpeed: root.installDownloadSpeed
                    currentStep: root.installCurrentStep
                    totalSteps: root.installTotalSteps
                    currentItem: root.installCurrentItem
                    etaStr: root.installEtaStr
                    diagnosticDetails: root.installDiagnosticDetails
                    showActivity: root.showInstallDetails
                    onCancelRequested: {
                        if (root.installStage !== "cancelling") {
                            root.installStage = "cancelling"
                            root.installMessage = "Cancelling installation..."
                            backend.cancelAppOperation(root.currentActionAppId())
                        }
                    }
                }

                // ── CARD 1: Screenshots Gallery Card (Main Attraction, 20 px Heading) ──
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: scrCardCol.implicitHeight
                    radius: 10
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1
                    visible: Boolean(root.selectedApp && root.selectedApp.screenshots && root.selectedApp.screenshots.length > 0)

                    ColumnLayout {
                        id: scrCardCol
                        anchors.top: parent.top
                        anchors.left: parent.left
                        anchors.right: parent.right
                        spacing: 0

                        // Card Header Row: 20 px semibold heading + chevrons
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 16
                            Layout.bottomMargin: 14
                            Layout.leftMargin: 24
                            Layout.rightMargin: 20
                            spacing: 10

                            Text {
                                text: "Screenshots"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 20
                                font.weight: Font.DemiBold
                                font.bold: true
                                color: theme.text
                            }

                            Item { Layout.fillWidth: true }

                            // Previous Screenshot Button
                            Rectangle {
                                width: 28
                                height: 28
                                radius: 14
                                color: prevGalMouse.containsMouse ? theme.surfaceHover : "transparent"
                                visible: galFlick.contentX > 4

                                Text {
                                    anchors.centerIn: parent
                                    text: "‹"
                                    font.pixelSize: 20
                                    font.bold: true
                                    color: theme.textSecondary
                                }

                                MouseArea {
                                    id: prevGalMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: galFlick.contentX = Math.max(0, galFlick.contentX - 510)
                                }
                            }

                            // Next Screenshot Button
                            Rectangle {
                                width: 28
                                height: 28
                                radius: 14
                                color: nextGalMouse.containsMouse ? theme.surfaceHover : "transparent"
                                visible: galFlick.contentWidth > galFlick.width

                                Text {
                                    anchors.centerIn: parent
                                    text: "›"
                                    font.pixelSize: 20
                                    font.bold: true
                                    color: theme.textSecondary
                                }

                                MouseArea {
                                    id: nextGalMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: galFlick.contentX = Math.min(galFlick.contentWidth - galFlick.width, galFlick.contentX + 510)
                                }
                            }
                        }

                        // Subtle Header Divider
                        Rectangle {
                            Layout.fillWidth: true
                            height: 1
                            color: theme.borderSubtle
                        }

                        // Horizontal Gallery Strip (Primary Large Screenshot + Right Peek)
                        Item {
                            Layout.fillWidth: true
                            Layout.preferredHeight: root.width < 760 ? 220 : 280
                            Layout.topMargin: 18
                            Layout.bottomMargin: 20
                            Layout.leftMargin: 24
                            Layout.rightMargin: 24

                            Flickable {
                                id: galFlick
                                anchors.fill: parent
                                contentWidth: galRow.implicitWidth
                                contentHeight: height
                                clip: true
                                boundsBehavior: Flickable.StopAtBounds

                                Behavior on contentX {
                                    NumberAnimation { duration: 220; easing.type: Easing.OutCubic }
                                }

                                RowLayout {
                                    id: galRow
                                    height: galFlick.height
                                    spacing: 16

                                    Repeater {
                                        model: (root.selectedApp && root.selectedApp.screenshots) ? root.selectedApp.screenshots : []

                                        Rectangle {
                                            readonly property int count: (root.selectedApp && root.selectedApp.screenshots) ? root.selectedApp.screenshots.length : 1
                                            width: root.width < 760
                                                ? 350
                                                : (count > 1 ? 496 : Math.min(560, galFlick.width))
                                            height: galFlick.height
                                            radius: 10
                                            color: theme.surfaceElevated
                                            border.color: scrMouse.containsMouse ? theme.accent : theme.borderSubtle
                                            border.width: scrMouse.containsMouse ? 2 : 1
                                            clip: true

                                            Image {
                                                anchors.fill: parent
                                                source: modelData
                                                fillMode: Image.PreserveAspectCrop
                                                asynchronous: true
                                                smooth: true
                                            }

                                            // Hover expand icon badge
                                            Rectangle {
                                                anchors.top: parent.top
                                                anchors.right: parent.right
                                                anchors.margins: 12
                                                width: 32
                                                height: 32
                                                radius: 16
                                                color: Qt.rgba(0, 0, 0, 0.65)
                                                visible: scrMouse.containsMouse

                                                RyzoraIcon {
                                                    anchors.centerIn: parent
                                                    name: "eye"
                                                    size: 14
                                                    color: "#FFFFFF"
                                                }
                                            }

                                            MouseArea {
                                                id: scrMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: {
                                                    root.lightboxIndex = index
                                                    root.lightboxOpen = true
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── CARD 2: Description & Features Card (20 px Heading, 15 px Body) ──
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: descCardCol.implicitHeight
                    radius: 10
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    ColumnLayout {
                        id: descCardCol
                        anchors.top: parent.top
                        anchors.left: parent.left
                        anchors.right: parent.right
                        spacing: 0

                        // Section Heading
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 16
                            Layout.bottomMargin: 14
                            Layout.leftMargin: 24
                            Layout.rightMargin: 24

                            Text {
                                text: "Description"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 20
                                font.weight: Font.DemiBold
                                font.bold: true
                                color: theme.text
                            }
                        }

                        // Subtle Header Divider
                        Rectangle {
                            Layout.fillWidth: true
                            height: 1
                            color: theme.borderSubtle
                        }

                        // Description Body & Features
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 18
                            Layout.bottomMargin: 20
                            Layout.leftMargin: 24
                            Layout.rightMargin: 24
                            spacing: 18

                            Text {
                                Layout.fillWidth: true
                                text: root.selectedApp ? (root.selectedApp.description || root.selectedApp.summary || "") : ""
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 15
                                lineHeight: 1.55
                                color: theme.text
                                wrapMode: Text.WordWrap
                            }

                            // Genuine Features Grid
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                visible: Boolean(root.selectedApp && root.selectedApp.features && root.selectedApp.features.length > 0)

                                Text {
                                    text: "Features"
                                    font.family: appDetailOverlay.uiFont
                                    font.pixelSize: 16
                                    font.weight: Font.DemiBold
                                    font.bold: true
                                    color: theme.text
                                }

                                GridLayout {
                                    Layout.fillWidth: true
                                    columns: root.width < 1080 ? 1 : 2
                                    columnSpacing: 24
                                    rowSpacing: 8

                                    Repeater {
                                        model: (root.selectedApp && root.selectedApp.features) ? root.selectedApp.features : []

                                        RowLayout {
                                            Layout.fillWidth: true
                                            Layout.alignment: Qt.AlignTop
                                            spacing: 10

                                            Text {
                                                text: "•"
                                                font.pixelSize: 16
                                                font.bold: true
                                                color: theme.accent
                                                Layout.alignment: Qt.AlignTop
                                            }

                                            Text {
                                                Layout.fillWidth: true
                                                text: modelData
                                                font.family: appDetailOverlay.uiFont
                                                font.pixelSize: 14
                                                lineHeight: 1.45
                                                color: theme.textSecondary
                                                wrapMode: Text.WordWrap
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── CARD 3: Additional Information Card (20 px Heading, 3-Column Grid) ──
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: infoCardCol.implicitHeight
                    radius: 10
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    ColumnLayout {
                        id: infoCardCol
                        anchors.top: parent.top
                        anchors.left: parent.left
                        anchors.right: parent.right
                        spacing: 0

                        // Section Heading
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 16
                            Layout.bottomMargin: 14
                            Layout.leftMargin: 24
                            Layout.rightMargin: 24

                            Text {
                                text: "Additional information"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 20
                                font.weight: Font.DemiBold
                                font.bold: true
                                color: theme.text
                            }
                        }

                        // Subtle Header Divider
                        Rectangle {
                            Layout.fillWidth: true
                            height: 1
                            color: theme.borderSubtle
                        }

                        // Balanced Technical Metadata Grid
                        GridLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 18
                            Layout.bottomMargin: 20
                            Layout.leftMargin: 24
                            Layout.rightMargin: 24
                            columns: root.width < 1080 ? 2 : 3
                            columnSpacing: 28
                            rowSpacing: 18

                            // 1. Published by
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "store"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Published by"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text { text: root.selectedApp ? (root.selectedApp.publisher || "Community") : ""; font.family: appDetailOverlay.uiFont; font.pixelSize: 14; font.weight: Font.Medium; color: theme.accent; elide: Text.ElideRight; Layout.fillWidth: true }
                                }
                            }

                            // 2. Version
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "info"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Version"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text { text: root.selectedApp ? (root.selectedApp.version || "Latest") : ""; font.family: appDetailOverlay.uiFont; font.pixelSize: 14; font.weight: Font.Medium; color: theme.text; elide: Text.ElideRight; Layout.fillWidth: true }
                                }
                            }

                            // 3. Category
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "layout-grid"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Category"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text { text: root.selectedApp ? (root.selectedApp.category || "General") : ""; font.family: appDetailOverlay.uiFont; font.pixelSize: 14; font.weight: Font.Medium; color: theme.text; elide: Text.ElideRight; Layout.fillWidth: true }
                                }
                            }

                            // 4. Software source
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "package"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Software source"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text {
                                        text: {
                                            if (!root.selectedApp) return ""
                                            var src = root.currentSelectedProviderSource()
                                            if (src) {
                                                if (src.provider_id === "flatpak") return "Flathub · Flatpak Universal Container"
                                                if (src.provider_id === "aur") return "AUR · Arch User Repository (Build from source)"
                                                return "Pacman · " + (src.repository ? src.repository + " Repository" : "Official Repository")
                                            }
                                            return "Pacman · " + (root.selectedApp.repository || "extra") + " Repository"
                                        }
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 14
                                        color: theme.textSecondary
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }
                                }
                            }

                            // 5. License
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "shield"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "License"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text { text: root.selectedApp && root.selectedApp.license ? root.selectedApp.license : "Open Source"; font.family: appDetailOverlay.uiFont; font.pixelSize: 14; color: theme.textSecondary; elide: Text.ElideRight; Layout.fillWidth: true }
                                }
                            }

                            // 6. Architecture
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "desktop"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Architecture"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text { text: "x86_64 (Linux 64-bit)"; font.family: appDetailOverlay.uiFont; font.pixelSize: 14; color: theme.textSecondary; elide: Text.ElideRight; Layout.fillWidth: true }
                                }
                            }

                            // 7. Download size
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "download"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Download size"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text {
                                        readonly property var dlBytes: {
                                            var src = root.currentSelectedProviderSource()
                                            return src ? (src.download_size || root.selectedApp.download_size) : (root.selectedApp ? root.selectedApp.download_size : 0)
                                        }
                                        text: {
                                            var src = root.currentSelectedProviderSource()
                                            var pId = (src ? src.provider_id : (root.selectedApp ? root.selectedApp.provider_id : "")).toLowerCase()
                                            if (pId === "aur" && !root.isCurrentProviderInstalled()) {
                                                return "Source build (No binary download)"
                                            }
                                            if (dlBytes && dlBytes > 0) {
                                                return root.formatAppBytes(dlBytes) + " (" + Number(dlBytes).toLocaleString() + " B)"
                                            }
                                            return "Unknown"
                                        }
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 14
                                        color: theme.textSecondary
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }
                                }
                            }

                            // 8. Installed size / Disk footprint
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "hard-drive"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Installed size (Disk footprint)"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text {
                                        readonly property var instBytes: {
                                            var src = root.currentSelectedProviderSource()
                                            return src ? (src.installed_size || root.selectedApp.installed_size) : (root.selectedApp ? root.selectedApp.installed_size : 0)
                                        }
                                        text: {
                                            var src = root.currentSelectedProviderSource()
                                            var pId = (src ? src.provider_id : (root.selectedApp ? root.selectedApp.provider_id : "")).toLowerCase()
                                            if (root.isCurrentProviderInstalled() && instBytes && instBytes > 0) {
                                                return root.formatAppBytes(instBytes) + " (" + Number(instBytes).toLocaleString() + " B)"
                                            }
                                            if (pId === "aur" && !root.isCurrentProviderInstalled()) {
                                                return "Unknown until built"
                                            }
                                            if (instBytes && instBytes > 0) {
                                                return root.formatAppBytes(instBytes) + " (" + Number(instBytes).toLocaleString() + " B)"
                                            }
                                            return "Unknown"
                                        }
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 14
                                        color: theme.textSecondary
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }
                                }
                            }

                            // 9. Source package / Build strategy
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "file-text"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Source package"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text {
                                        text: {
                                            var src = root.currentSelectedProviderSource()
                                            var pId = (src ? src.provider_id : (root.selectedApp ? root.selectedApp.provider_id : "")).toLowerCase()
                                            if (pId === "aur") return "AUR Git repository / PKGBUILD"
                                            if (pId === "flatpak") return "Flathub OCI container"
                                            return "Official Arch Linux package archive"
                                        }
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 14
                                        color: theme.textSecondary
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }
                                }
                            }

                            // 10. Build environment
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "terminal"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Build environment"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text {
                                        text: {
                                            var src = root.currentSelectedProviderSource()
                                            var pId = (src ? src.provider_id : (root.selectedApp ? root.selectedApp.provider_id : "")).toLowerCase()
                                            if (pId === "aur") return "Unprivileged makepkg sandbox"
                                            return "Pre-compiled distribution binary"
                                        }
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 14
                                        color: theme.textSecondary
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }
                                }
                            }

                            // 7. Installation status
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon {
                                    name: (root.isSelectedAppPending() ? (root.pendingAction === "uninstall" ? "trash" : "download") : (root.isCurrentProviderInstalled() ? "check" : "download"))
                                    size: 15
                                    color: (root.isSelectedAppPending() ? theme.accent : (root.isCurrentProviderInstalled() ? "#10B981" : theme.textMuted))
                                    Layout.alignment: Qt.AlignTop
                                    Layout.topMargin: 2
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Installation status"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text {
                                        text: {
                                            if (!root.selectedApp) return ""
                                            if (root.isCurrentProviderInstalled()) {
                                                var v = root.currentInstalledVersion() || root.currentAvailableVersion()
                                                return v ? "Installed on this system (" + v + ")" : "Installed on this system"
                                            }
                                            if (root.isSelectedAppPending() && root.pendingAction === "install") {
                                                if (root.installStage === "cancelling") return "Cancelling installation..."
                                                if (root.installStage === "downloading") return "Downloading (" + (root.installPercentage >= 0 ? root.installPercentage + "%" : "in progress") + ")"
                                                if (root.installStage === "installing") return "Deploying package..."
                                                if (root.installStage === "resolving") return "Resolving dependencies..."
                                                if (root.installStage === "finalizing") return "Finalizing installation..."
                                                return "Installing..."
                                            }
                                            if (root.isSelectedAppPending() && root.pendingAction === "uninstall") return "Removing application..."
                                            return "Ready to install"
                                        }
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 14
                                        font.weight: Font.Medium
                                        color: {
                                            if (root.isCurrentProviderInstalled()) return (theme.resolvedTheme === "light" ? "#059669" : "#34D399")
                                            if (root.isSelectedAppPending()) return theme.accent
                                            return theme.textSecondary
                                        }
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }
                                }
                            }

                            // 8. Package ID
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "terminal"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Package identifier"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text {
                                        text: root.selectedTargetId !== ""
                                            ? root.selectedTargetId
                                            : (root.selectedApp ? (root.selectedApp.target_id || root.selectedApp.package_name || root.selectedApp.id || "") : "")
                                        font.pixelSize: 13
                                        font.family: "monospace"
                                        color: theme.textSecondary
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }
                                }
                            }

                            // 9. Official website
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                visible: Boolean(root.selectedApp && root.selectedApp.website)
                                RyzoraIcon { name: "compass"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Official website"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text {
                                        text: root.selectedApp && root.selectedApp.website ? root.selectedApp.website : ""
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 14
                                        color: theme.accent
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                        MouseArea {
                                            anchors.fill: parent
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                if (root.selectedApp && root.selectedApp.website) {
                                                    Qt.openUrlExternally(root.selectedApp.website)
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── CARD 4: System Requirements Card (Visually Lighter Section) ──
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: reqCardCol.implicitHeight
                    radius: 10
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    ColumnLayout {
                        id: reqCardCol
                        anchors.top: parent.top
                        anchors.left: parent.left
                        anchors.right: parent.right
                        spacing: 0

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 16
                            Layout.bottomMargin: 14
                            Layout.leftMargin: 24
                            Layout.rightMargin: 24

                            Text {
                                text: "System requirements"
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 18
                                font.weight: Font.DemiBold
                                font.bold: true
                                color: theme.text
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 1
                            color: theme.borderSubtle
                        }

                        GridLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 16
                            Layout.bottomMargin: 18
                            Layout.leftMargin: 24
                            Layout.rightMargin: 24
                            columns: root.width < 960 ? 1 : 2
                            columnSpacing: 32
                            rowSpacing: 12

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                RyzoraIcon { name: "check"; size: 14; color: "#10B981" }
                                Text { text: "OS:"; font.family: appDetailOverlay.uiFont; font.pixelSize: 13; font.weight: Font.Medium; color: theme.text }
                                Text { text: "Arch Linux (rolling release)"; font.family: appDetailOverlay.uiFont; font.pixelSize: 13; color: theme.textSecondary; Layout.fillWidth: true; elide: Text.ElideRight }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                RyzoraIcon { name: "check"; size: 14; color: "#10B981" }
                                Text { text: "Architecture:"; font.family: appDetailOverlay.uiFont; font.pixelSize: 13; font.weight: Font.Medium; color: theme.text }
                                Text { text: "x86_64 (64-bit native)"; font.family: appDetailOverlay.uiFont; font.pixelSize: 13; color: theme.textSecondary; Layout.fillWidth: true; elide: Text.ElideRight }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                RyzoraIcon { name: "check"; size: 14; color: "#10B981" }
                                Text { text: "Package manager:"; font.family: appDetailOverlay.uiFont; font.pixelSize: 13; font.weight: Font.Medium; color: theme.text }
                                Text { text: "Pacman / ALPM transaction service"; font.family: appDetailOverlay.uiFont; font.pixelSize: 13; color: theme.textSecondary; Layout.fillWidth: true; elide: Text.ElideRight }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                RyzoraIcon { name: "check"; size: 14; color: "#10B981" }
                                Text { text: "Display session:"; font.family: appDetailOverlay.uiFont; font.pixelSize: 13; font.weight: Font.Medium; color: theme.text }
                                Text { text: "Wayland / X11 desktop session"; font.family: appDetailOverlay.uiFont; font.pixelSize: 13; color: theme.textSecondary; Layout.fillWidth: true; elide: Text.ElideRight }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Full-Screen High-Resolution Lightbox Modal ──
    Rectangle {
        id: lightboxModal
        anchors.fill: parent
        visible: root.lightboxOpen
        color: Qt.rgba(0, 0, 0, 0.88)
        z: 100
        focus: visible

        Keys.onEscapePressed: root.lightboxOpen = false

        MouseArea {
            anchors.fill: parent
            onClicked: root.lightboxOpen = false
        }

        // Close button (top right)
        Rectangle {
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 24
            width: 36
            height: 36
            radius: 18
            color: closeMouse.containsMouse ? Qt.rgba(255, 255, 255, 0.25) : Qt.rgba(255, 255, 255, 0.12)
            border.color: Qt.rgba(255, 255, 255, 0.20)
            border.width: 1

            Text {
                anchors.centerIn: parent
                text: "✕"
                font.pixelSize: 14
                font.bold: true
                color: "#FFFFFF"
            }

            MouseArea {
                id: closeMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.lightboxOpen = false
            }
        }

        // Previous arrow (left)
        Rectangle {
            anchors.left: parent.left
            anchors.leftMargin: 24
            anchors.verticalCenter: parent.verticalCenter
            width: 44
            height: 44
            radius: 22
            visible: Boolean(root.selectedApp && root.selectedApp.screenshots && root.selectedApp.screenshots.length > 1)
            color: prevLbMouse.containsMouse ? Qt.rgba(255, 255, 255, 0.30) : Qt.rgba(255, 255, 255, 0.15)
            border.color: Qt.rgba(255, 255, 255, 0.25)
            border.width: 1

            Text {
                anchors.centerIn: parent
                text: "‹"
                font.pixelSize: 26
                font.bold: true
                color: "#FFFFFF"
            }

            MouseArea {
                id: prevLbMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    var total = root.selectedApp.screenshots.length
                    root.lightboxIndex = (root.lightboxIndex - 1 + total) % total
                }
            }
        }

        // High-res Centered Screenshot
        Image {
            anchors.centerIn: parent
            width: Math.min(parent.width - 160, 1280)
            height: Math.min(parent.height - 120, 720)
            source: (root.selectedApp && root.selectedApp.screenshots && root.selectedApp.screenshots.length > root.lightboxIndex)
                ? root.selectedApp.screenshots[root.lightboxIndex]
                : ""
            fillMode: Image.PreserveAspectFit
            smooth: true
            asynchronous: true
        }

        // Next arrow (right)
        Rectangle {
            anchors.right: parent.right
            anchors.rightMargin: 24
            anchors.verticalCenter: parent.verticalCenter
            width: 44
            height: 44
            radius: 22
            visible: Boolean(root.selectedApp && root.selectedApp.screenshots && root.selectedApp.screenshots.length > 1)
            color: nextLbMouse.containsMouse ? Qt.rgba(255, 255, 255, 0.30) : Qt.rgba(255, 255, 255, 0.15)
            border.color: Qt.rgba(255, 255, 255, 0.25)
            border.width: 1

            Text {
                anchors.centerIn: parent
                text: "›"
                font.pixelSize: 26
                font.bold: true
                color: "#FFFFFF"
            }

            MouseArea {
                id: nextLbMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    var total = root.selectedApp.screenshots.length
                    root.lightboxIndex = (root.lightboxIndex + 1) % total
                }
            }
        }

        // Counter pill (bottom center)
        Rectangle {
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 24
            anchors.horizontalCenter: parent.horizontalCenter
            height: 28
            radius: 14
            color: Qt.rgba(0, 0, 0, 0.60)
            border.color: Qt.rgba(255, 255, 255, 0.15)
            border.width: 1
            implicitWidth: counterTxt.implicitWidth + 24
            visible: Boolean(root.selectedApp && root.selectedApp.screenshots && root.selectedApp.screenshots.length > 1)

            Text {
                id: counterTxt
                anchors.centerIn: parent
                text: (root.lightboxIndex + 1) + " / " + (root.selectedApp && root.selectedApp.screenshots ? root.selectedApp.screenshots.length : 1)
                font.pixelSize: 12
                font.bold: true
                color: "#FFFFFF"
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 6. PROVIDER-AWARE UNINSTALL CONFIRMATION MODAL
    // ──────────────────────────────────────────────────────────────────────────
    UninstallConfirmationDialog {
        id: uninstallConfirmationDialog
        onConfirmed: function(req) {
            root.actionErrorMessage = ""
            root.pendingAppId = root.currentActionAppId()
            root.pendingAction = "uninstall"
        }
    }
}
