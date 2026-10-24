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
    property string selectedProviderId: ""
    property string selectedTargetId: ""
    property real savedBrowseScrollY: 0
    property var masterAppsList: []

    // Dynamic responsive column count (cards stay between 220px and 270px wide)
    readonly property int responsiveColumns: {
        var w = root.width - (root.width < 600 ? 24 : 40)
        if (w < 440) return 1
        if (w < 680) return 2
        if (w < 960) return 3
        if (w < 1240) return 4
        if (w < 1520) return 5
        return 6
    }

    signal appSelected(var appData)
    signal packageSelected(var appData)
    signal installRequested(string appId)
    signal uninstallRequested(string appId)
    signal launchRequested(string appId)
    signal categorySelected(string category)
    signal clearSearchRequested()

    onSearchQueryChanged: {
        if (root.searchQuery.trim().length > 0 && root.selectedApp !== null) {
            root.setSelectedApp(null)
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
        } else {
            root.selectedProviderId = ""
            root.selectedTargetId = ""
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
        if (root.pendingAppId !== "" || (backend && backend.isAppOperating(appId))) {
            root.actionErrorMessage = "Another package operation is currently in progress. Please wait for it to finish."
            return
        }
        root.actionErrorMessage = ""
        root.pendingAppId = appId
        root.pendingAction = "install"
        root.installRequested(appId)
    }

    function requestUninstall(appId) {
        if (root.pendingAppId !== "" || (backend && backend.isAppOperating(appId))) {
            root.actionErrorMessage = "Another package operation is currently in progress. Please wait for it to finish."
            return
        }
        root.actionErrorMessage = ""
        root.pendingAppId = appId
        root.pendingAction = "uninstall"
        root.uninstallRequested(appId)
    }

    Component.onCompleted: {
        var list = backend.appsList || []
        if (list.length > 0) {
            root.masterAppsList = list
        }
    }

    Connections {
        target: backend
        function onAppsChanged() {
            var list = backend.appsList || []
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
        function onAppInstallProgress(appId, stage, percentage, isDeterminate, message, bytesDownloaded, bytesTotal, downloadSpeed) {
            if (root.pendingAppId === appId || root.isSelectedAppPending() || (root.selectedApp && appId.indexOf(root.selectedApp.id) !== -1)) {
                root.installStage = stage
                root.installPercentage = percentage
                root.installIsDeterminate = isDeterminate
                root.installMessage = message
                root.installBytesDownloaded = bytesDownloaded
                root.installBytesTotal = bytesTotal
                root.installDownloadSpeed = downloadSpeed
            }
        }
        function onAppInstalled(appId, success, message) {
            if (root.pendingAppId === appId || root.isSelectedAppPending()) {
                root.pendingAppId = ""
                root.pendingAction = ""
                root.installStage = ""
                root.installPercentage = -1
                root.installIsDeterminate = false
                root.installMessage = ""
                root.installBytesDownloaded = ""
                root.installBytesTotal = ""
                root.installDownloadSpeed = ""
                if (!success) {
                    root.actionErrorMessage = message || "Installation failed. Please try again."
                } else {
                    root.actionErrorMessage = ""
                }
            }
        }
        function onAppUninstalled(appId, success, message) {
            if (root.pendingAppId === appId || root.isSelectedAppPending()) {
                root.pendingAppId = ""
                root.pendingAction = ""
                if (!success) {
                    root.actionErrorMessage = message || "Removal failed. Please try again."
                } else {
                    root.actionErrorMessage = ""
                }
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

    // Helper: Count distinct applications for category chips
    function getCategoryCount(catName) {
        var list = (root.masterAppsList && root.masterAppsList.length > 0) ? root.masterAppsList : (backend.appsList || [])
        var c = catName.toLowerCase()
        if (c === "all") return list.length
        var count = 0
        for (var i = 0; i < list.length; i++) {
            var item = list[i]
            if (!item) continue
            if (c === "installed") {
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
                res.push(item)
            } else if (cat === "installed") {
                if (item.is_installed) res.push(item)
            } else {
                if (String(item.category || "").toLowerCase() === cat) {
                    res.push(item)
                }
            }
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
                if (item && item.id === tid && !seen[item.id]) {
                    res.push(item)
                    seen[item.id] = true
                    break
                }
            }
        }

        if (res.length === 0) {
            for (var j = 0; j < list.length && res.length < 5; j++) {
                var cand = list[j]
                if (cand && !seen[cand.id] && cand.banner_url) {
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

    // ── REUSABLE MICROSOFT STORE-INSPIRED APP CARD (BIG 68px ICON + LARGER DESCRIPTION) ──
    component AppCard: Rectangle {
        id: cardRoot
        property var appData: null
        readonly property bool isInstalled: Boolean(cardRoot.appData && cardRoot.appData.is_installed)

        Layout.fillWidth: true
        height: 200
        radius: theme.radiusMd
        color: cardMouse.containsMouse ? theme.surfaceHover : theme.surface
        border.color: cardMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
        border.width: 1
        clip: true

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
            spacing: 8

            // Top Header: Prominent 68x68 Square App Icon + Action Pill Button
            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                // Big 68x68 Icon Container (as in Windows 11 Microsoft Store)
                Rectangle {
                    width: 68
                    height: 68
                    radius: 14
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    Image {
                        anchors.centerIn: parent
                        width: 52
                        height: 52
                        source: (cardRoot.appData && cardRoot.appData.icon_url) ? cardRoot.appData.icon_url : ""
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                        visible: status === Image.Ready
                    }

                    RyzoraIcon {
                        anchors.centerIn: parent
                        visible: !cardRoot.appData || !cardRoot.appData.icon_url || cardRoot.appData.icon_url === ""
                        name: cardRoot.appData ? root.getCategoryIcon(cardRoot.appData.category) : "layout-grid"
                        size: 34
                        color: theme.textMuted
                    }
                }

                Item { Layout.fillWidth: true }

                // Action Pill Button (Get / Open / Installing)
                Rectangle {
                    id: cardActionBtn
                    readonly property bool isPending: root.pendingAppId === (cardRoot.appData ? cardRoot.appData.id : "")
                    Layout.alignment: Qt.AlignTop
                    height: 28
                    implicitWidth: cardActionBtn.isPending ? (actionBtnRow.implicitWidth + 18) : 68
                    radius: theme.radiusSm
                    color: cardActionBtn.isPending
                        ? theme.borderStrong
                        : (cardRoot.isInstalled
                            ? (actionBtnMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                            : (actionBtnMouse.containsMouse ? theme.accentHover : theme.accent))
                    border.color: cardActionBtn.isPending ? theme.borderStrong : (cardRoot.isInstalled ? theme.borderStrong : theme.accent)
                    border.width: 1

                    RowLayout {
                        id: actionBtnRow
                        anchors.centerIn: parent
                        spacing: 4

                        RyzoraIcon {
                            id: cardSpinIcon
                            name: cardActionBtn.isPending ? "refresh" : (cardRoot.isInstalled ? "play" : "download")
                            size: 10
                            color: cardActionBtn.isPending ? "#ffffff" : (cardRoot.isInstalled ? "#10b981" : "#ffffff")
                            RotationAnimator {
                                target: cardSpinIcon
                                from: 0
                                to: 360
                                duration: 1000
                                loops: Animation.Infinite
                                running: cardActionBtn.isPending
                            }
                        }

                        Text {
                            text: cardActionBtn.isPending ? "Wait..." : (cardRoot.isInstalled ? "Open" : "Get")
                            font.pixelSize: 11
                            font.bold: true
                            color: (cardActionBtn.isPending || !cardRoot.isInstalled) ? "#ffffff" : theme.text
                        }
                    }

                    MouseArea {
                        id: actionBtnMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        enabled: !cardActionBtn.isPending
                        cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
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
            }

            // Application Name (Clear, bold title)
            Text {
                text: cardRoot.appData ? (cardRoot.appData.display_name || cardRoot.appData.name || "") : ""
                font.pixelSize: 15
                font.bold: true
                color: theme.text
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            // Category & Publisher
            Text {
                text: cardRoot.appData ? ((cardRoot.appData.publisher ? cardRoot.appData.publisher + " · " : "") + (cardRoot.appData.category || "App")) : ""
                font.pixelSize: 11
                color: theme.textMuted
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            // Description: Noticeably bigger font (12.5px) with comfortable line height
            Text {
                text: cardRoot.appData ? (cardRoot.appData.summary || cardRoot.appData.description || "") : ""
                font.pixelSize: 13
                lineHeight: 1.25
                color: theme.textSecondary
                maximumLineCount: 2
                wrapMode: Text.WordWrap
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            Item { Layout.fillHeight: true }

            // Bottom Metadata Row: Repository / Installed state + Platform Tag
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: {
                        if (!cardRoot.appData) return ""
                        if (cardRoot.appData.is_installed) {
                            return "● Installed · v" + (cardRoot.appData.installed_version || cardRoot.appData.version || "1.0")
                        }
                        var pId = cardRoot.appData.provider_id || "pacman"
                        if (pId === "flatpak") return "Flathub · Flatpak"
                        if (pId === "aur") return "AUR · Community"
                        var repo = cardRoot.appData.repository || "extra"
                        return "Pacman · " + repo
                    }
                    font.pixelSize: 11
                    font.weight: cardRoot.appData && cardRoot.appData.is_installed ? Font.Medium : Font.Normal
                    color: cardRoot.appData && cardRoot.appData.is_installed ? "#10b981" : theme.textMuted
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: (cardRoot.appData && cardRoot.appData.license) ? cardRoot.appData.license : "Native"
                    font.pixelSize: 11
                    color: theme.textMuted
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 1. MAIN BROWSE VIEW
    // ──────────────────────────────────────────────────────────────────────────
    Flickable {
        id: mainScroll
        anchors.fill: parent
        visible: root.selectedApp === null
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
                visible: root.activeFeaturedApp !== null
                Layout.fillWidth: true
                Layout.preferredHeight: visible ? (root.width < 760 ? 330 : 375) : 0
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
                                    root.selectedCategory = modelData.id
                                    root.categorySelected(modelData.id)
                                    backend.filterApps(modelData.id, root.searchQuery)
                                }
                            }
                        }
                    }
                }
            }

            // ── 4. CURATED ROW OF POPULAR APPLICATIONS (Responsive Row of Cards) ──
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10
                visible: root.popularApps.length > 0

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: "Popular Applications"
                        font.pixelSize: 15
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Curated desktop essentials"
                        font.pixelSize: 11
                        color: theme.textMuted
                    }

                    Item { Layout.fillWidth: true }
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: root.responsiveColumns
                    rowSpacing: 12
                    columnSpacing: 12

                    Repeater {
                        model: root.popularApps
                        delegate: AppCard {
                            appData: modelData
                        }
                    }
                }
            }

            // ── 5. MAIN APPLICATIONS GRID (All Remaining Applications) ──
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 12

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: (root.searchQuery === "" && root.selectedCategory === "All") ? "More Applications" : (root.selectedCategory === "All" ? "All Applications" : root.selectedCategory)
                        font.pixelSize: 15
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "(" + root.visibleCatalogApps.length + ")"
                        font.pixelSize: 12
                        font.family: "monospace"
                        color: theme.textMuted
                    }

                    Item { Layout.fillWidth: true }

                    // Sort Control neatly aligned with grid header
                    Rectangle {
                        height: 28
                        implicitWidth: sortRow.implicitWidth + 16
                        radius: theme.radiusSm
                        color: theme.surface
                        border.color: sortMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            id: sortRow
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon { name: "filter"; size: 11; color: theme.textMuted }
                            Text {
                                text: "Sort: " + (root.sortBy === "status" ? "Status" : (root.sortBy === "category" ? "Category" : "Name"))
                                font.pixelSize: 11
                                font.weight: Font.Medium
                                color: theme.textSecondary
                            }
                        }

                        MouseArea {
                            id: sortMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.sortBy === "name") root.sortBy = "category"
                                else if (root.sortBy === "category") root.sortBy = "status"
                                else root.sortBy = "name"
                            }
                        }
                    }
                }

                GridLayout {
                    id: catalogGrid
                    Layout.fillWidth: true
                    columns: root.responsiveColumns
                    rowSpacing: 12
                    columnSpacing: 12

                    Repeater {
                        model: root.visibleCatalogApps
                        delegate: AppCard {
                            appData: modelData
                        }
                    }
                }

                // Loading State Indicator
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 200
                    visible: backend.isAppsLoading && root.visibleCatalogApps.length === 0
                    color: "transparent"

                    ColumnLayout {
                        anchors.centerIn: parent
                        spacing: 12

                        RyzoraIcon {
                            id: loadingSpinIcon
                            Layout.alignment: Qt.AlignHCenter
                            name: "refresh"
                            size: 22
                            color: theme.accent
                            RotationAnimator {
                                target: loadingSpinIcon
                                from: 0
                                to: 360
                                duration: 1000
                                loops: Animation.Infinite
                                running: backend.isAppsLoading
                            }
                        }

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: "Loading software catalogue..."
                            font.pixelSize: 13
                            color: theme.textSecondary
                        }
                    }
                }

                // Empty State
                EmptyState {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 240
                    visible: root.visibleCatalogApps.length === 0 && !backend.isAppsLoading
                    iconName: "layout-grid"
                    title: "No Applications Found"
                    message: "No software applications match category '" + root.selectedCategory + "'" + (root.searchQuery ? " and query '" + root.searchQuery + "'" : "") + "."
                    actionLabel: "Clear Search & Filter"
                    onActionClicked: {
                        root.selectedCategory = "All"
                        root.searchQuery = ""
                        root.clearSearchRequested()
                        backend.filterApps("All", "")
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
        readonly property bool isWideDetail: root.width >= 920
        readonly property int outerPad: root.width < 760 ? 12 : 16
        readonly property int paneGap: 14
        readonly property int leftPaneWidth: 320

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
            z: 2

            ColumnLayout {
                id: leftPaneCol
                anchors.fill: parent
                anchors.margins: 22
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
                        visible: Boolean(root.selectedApp && root.selectedApp.is_installed)
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
                        visible: Boolean(root.selectedApp && !root.selectedApp.is_installed && !root.isSelectedAppPending())
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

                    // ── Active Installation Progress Component (Microsoft Store Style) ──
                    Rectangle {
                        id: progressBox
                        visible: Boolean(root.selectedApp && !root.selectedApp.is_installed && root.isSelectedAppPending() && root.pendingAction === "install")
                        width: 228
                        implicitHeight: progCol.implicitHeight + 16
                        radius: 8
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            id: progCol
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 6

                            // Row 1: Phase and optional percentage
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 4

                                Text {
                                    id: phaseText
                                    Layout.fillWidth: true
                                    text: {
                                        if (root.installStage === "cancelling") return "Cancelling installation..."
                                        if (root.installMessage !== "") return root.installMessage
                                        var p = root.selectedProviderId || "pacman"
                                        var provName = p === "flatpak" ? "Flatpak" : (p === "aur" ? "AUR" : "Pacman")
                                        if (root.installStage === "resolving") return "Resolving dependencies..."
                                        if (root.installStage === "downloading") return "Downloading via " + provName + "..."
                                        if (root.installStage === "building") return "Compiling AUR package..."
                                        if (root.installStage === "installing") return "Installing package..."
                                        if (root.installStage === "finalizing") return "Running post-install hooks..."
                                        return "Installing via " + provName + "..."
                                    }
                                    font.family: appDetailOverlay.uiFont
                                    font.pixelSize: 12
                                    font.weight: Font.Medium
                                    color: theme.text
                                    elide: Text.ElideRight
                                }

                                Text {
                                    visible: root.installIsDeterminate && root.installPercentage >= 0
                                    text: root.installPercentage + "%"
                                    font.family: appDetailOverlay.uiFont
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    font.bold: true
                                    color: theme.accent
                                }
                            }

                            // Row 2: Microsoft Store Style Progress Bar (determinate or animated indeterminate)
                            Rectangle {
                                Layout.fillWidth: true
                                height: 5
                                radius: 2.5
                                color: theme.surfaceHover
                                clip: true

                                // Determinate progress fill
                                Rectangle {
                                    visible: root.installIsDeterminate && root.installPercentage >= 0
                                    height: parent.height
                                    width: parent.width * Math.max(0, Math.min(100, root.installPercentage)) / 100
                                    radius: 2.5
                                    color: theme.accent

                                    Behavior on width {
                                        NumberAnimation { duration: 180; easing.type: Easing.OutQuad }
                                    }
                                }

                                // Indeterminate glowing sliding indicator
                                Rectangle {
                                    id: indeterminateBar
                                    visible: !root.installIsDeterminate || root.installPercentage < 0
                                    height: parent.height
                                    width: Math.max(36, parent.width * 0.32)
                                    radius: 2.5
                                    color: theme.accent

                                    SequentialAnimation on x {
                                        loops: Animation.Infinite
                                        running: progressBox.visible && (!root.installIsDeterminate || root.installPercentage < 0)
                                        PropertyAnimation {
                                            from: -indeterminateBar.width
                                            to: progressBox.width
                                            duration: 1200
                                            easing.type: Easing.InOutQuad
                                        }
                                    }
                                }
                            }

                            // Row 3: Downloaded size / total size / speed & Cancel action
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 4

                                Text {
                                    Layout.fillWidth: true
                                    text: {
                                        if (root.installBytesDownloaded !== "" && root.installBytesTotal !== "") {
                                            var sz = root.installBytesDownloaded + " / " + root.installBytesTotal
                                            if (root.installDownloadSpeed !== "") {
                                                sz += " (" + root.installDownloadSpeed + ")"
                                            }
                                            return sz
                                        }
                                        if (root.installStage !== "") {
                                            return root.installStage.toUpperCase()
                                        }
                                        return "In progress"
                                    }
                                    font.family: appDetailOverlay.uiFont
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    elide: Text.ElideRight
                                }

                                Rectangle {
                                    width: 62
                                    height: 20
                                    radius: 4
                                    enabled: root.installStage !== "cancelling"
                                    opacity: enabled ? 1.0 : 0.65
                                    color: cancelBtnMouse.containsMouse && enabled ? theme.surfaceHover : "transparent"
                                    border.color: cancelBtnMouse.containsMouse && enabled ? theme.borderStrong : "transparent"
                                    border.width: 1

                                    Text {
                                        anchors.centerIn: parent
                                        text: root.installStage === "cancelling" ? "Cancelling..." : "Cancel"
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 11
                                        font.weight: Font.Medium
                                        color: cancelBtnMouse.containsMouse && enabled ? "#EF4444" : theme.textMuted
                                    }

                                    MouseArea {
                                        id: cancelBtnMouse
                                        anchors.fill: parent
                                        hoverEnabled: parent.enabled
                                        cursorShape: parent.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                                        onClicked: {
                                            if (root.installStage !== "cancelling") {
                                                root.installStage = "cancelling"
                                                root.installMessage = "Cancelling installation..."
                                                backend.cancelAppOperation(root.currentActionAppId())
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Secondary Action: Uninstall (when installed)
                    Rectangle {
                        visible: Boolean(root.selectedApp && root.selectedApp.is_installed)
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
                                    enabled: !Boolean(root.selectedApp && root.selectedApp.is_installed)
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

                // ── 6. Two-Column Legitimate Metadata Stat Row (with Vertical Divider) ──
                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 24

                    // Left Stat: Version
                    ColumnLayout {
                        Layout.alignment: Qt.AlignVCenter
                        spacing: 2
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: root.selectedApp ? (root.selectedApp.version || "Latest") : ""
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 15
                            font.weight: Font.DemiBold
                            font.bold: true
                            color: theme.text
                        }
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: "Version"
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 12
                            color: theme.textMuted
                        }
                    }

                    // Vertical Divider
                    Rectangle {
                        width: 1
                        height: 32
                        color: theme.borderSubtle
                    }

                    // Right Stat: Status / Repository
                    ColumnLayout {
                        Layout.alignment: Qt.AlignVCenter
                        spacing: 2
                        RowLayout {
                            Layout.alignment: Qt.AlignHCenter
                            spacing: 6
                            Rectangle {
                                width: 7
                                height: 7
                                radius: 4
                                color: root.selectedApp && root.selectedApp.is_installed ? "#10B981" : theme.accent
                            }
                            Text {
                                text: {
                                    if (!root.selectedApp) return ""
                                    if (root.selectedApp.is_installed) return "Installed"
                                    var src = root.currentSelectedProviderSource()
                                    if (src) {
                                        if (src.provider_id === "flatpak") return "Flathub"
                                        if (src.provider_id === "aur") return "AUR"
                                        return src.repository || "Official"
                                    }
                                    return root.selectedApp.repository || "Official"
                                }
                                font.family: appDetailOverlay.uiFont
                                font.pixelSize: 15
                                font.weight: Font.DemiBold
                                font.bold: true
                                color: root.selectedApp && root.selectedApp.is_installed
                                    ? (theme.resolvedTheme === "light" ? "#059669" : "#34D399")
                                    : theme.text
                            }
                        }
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: {
                                if (root.selectedApp && root.selectedApp.is_installed) return "System state"
                                var src = root.currentSelectedProviderSource()
                                if (src && src.provider_id === "flatpak") return "Sandboxed Flatpak"
                                if (src && src.provider_id === "aur") return "AUR User Repo"
                                return "Official Repository"
                            }
                            font.family: appDetailOverlay.uiFont
                            font.pixelSize: 12
                            color: theme.textMuted
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

                            // 7. Installation status
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                spacing: 10
                                RyzoraIcon { name: "check"; size: 15; color: theme.textMuted; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Installation status"; font.family: appDetailOverlay.uiFont; font.pixelSize: 12; color: theme.textMuted }
                                    Text {
                                        text: root.selectedApp && root.selectedApp.is_installed ? "Installed on this system" : "Ready to install"
                                        font.family: appDetailOverlay.uiFont
                                        font.pixelSize: 14
                                        font.weight: Font.Medium
                                        color: root.selectedApp && root.selectedApp.is_installed
                                            ? (theme.resolvedTheme === "light" ? "#059669" : "#34D399")
                                            : theme.textSecondary
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
}
