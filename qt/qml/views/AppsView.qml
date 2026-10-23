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

    function setSelectedApp(app) {
        root.selectedApp = app
    }

    function setSelectedPackage(pkg) {
        root.selectedApp = pkg
    }

    Connections {
        target: backend
        function onAppsChanged() {
            if (root.selectedApp) {
                var list = backend.appsList || []
                for (var i = 0; i < list.length; i++) {
                    if (list[i] && list[i].id === root.selectedApp.id) {
                        root.selectedApp = list[i]
                        break
                    }
                }
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
        var list = backend.appsList || []
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
        var list = backend.appsList || []
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
        var list = backend.appsList || []
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
    readonly property var popularApps: {
        if (root.searchQuery !== "" || root.selectedCategory !== "All") {
            return []
        }
        var list = backend.appsList || []
        var preferredPopular = ["code", "vlc", "discord", "blender", "obs-studio", "spotify", "chromium", "steam", "gimp", "inkscape", "alacritty"]
        var res = []
        var seen = {}
        if (root.featuredAppId) {
            seen[root.featuredAppId] = true
        }
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

    // Visible items in the main catalog grid (de-duplicated against featured & popular)
    readonly property var visibleCatalogApps: {
        var all = root.filteredApps
        if (root.searchQuery !== "" || root.selectedCategory !== "All") {
            return all
        }
        var excluded = {}
        if (root.featuredAppId) excluded[root.featuredAppId] = true
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

                // Action Pill Button (Get / Open)
                Rectangle {
                    Layout.alignment: Qt.AlignTop
                    height: 28
                    implicitWidth: 68
                    radius: theme.radiusSm
                    color: cardRoot.isInstalled
                        ? (actionBtnMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                        : (actionBtnMouse.containsMouse ? theme.accentHover : theme.accent)
                    border.color: cardRoot.isInstalled ? theme.borderStrong : theme.accent
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 4

                        RyzoraIcon {
                            name: cardRoot.isInstalled ? "play" : "download"
                            size: 10
                            color: cardRoot.isInstalled ? "#10b981" : "#ffffff"
                        }

                        Text {
                            text: cardRoot.isInstalled ? "Open" : "Get"
                            font.pixelSize: 11
                            font.bold: true
                            color: cardRoot.isInstalled ? theme.text : "#ffffff"
                        }
                    }

                    MouseArea {
                        id: actionBtnMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (!cardRoot.appData) return
                            if (cardRoot.isInstalled) {
                                root.launchRequested(cardRoot.appData.id)
                            } else {
                                root.installRequested(cardRoot.appData.id)
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
                    text: cardRoot.appData && cardRoot.appData.is_installed
                        ? ("● Installed · v" + (cardRoot.appData.installed_version || cardRoot.appData.version || "1.0"))
                        : ((cardRoot.appData && cardRoot.appData.repository ? cardRoot.appData.repository : "extra") + " repository")
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

            // ── 2. FEATURED APPLICATION CAROUSEL (Microsoft Store-inspired, 240px high) ──
            Rectangle {
                id: heroPanel
                visible: root.activeFeaturedApp !== null
                Layout.fillWidth: true
                height: root.width < 700 ? 270 : 240
                radius: theme.radiusLg
                border.color: theme.borderSubtle
                border.width: 1
                clip: true

                // Rich theme-aware background gradient
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0.0; color: theme.resolvedTheme === "light" ? "#E8EEF8" : "#101624" }
                        GradientStop { position: 0.55; color: theme.resolvedTheme === "light" ? "#DFE7F3" : "#151C2C" }
                        GradientStop { position: 1.0; color: theme.resolvedTheme === "light" ? "#D5DFEE" : "#0C121B" }
                    }
                }

                // Subtle Accent ambient glow
                Rectangle {
                    anchors.fill: parent
                    color: theme.accent
                    opacity: theme.resolvedTheme === "light" ? 0.04 : 0.07
                }

                readonly property var currentItem: root.activeFeaturedApp

                RowLayout {
                    anchors.fill: parent
                    spacing: 0

                    // Left Column: Application Information & Actions (~48-55% width)
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.margins: root.width < 700 ? 16 : 22
                        spacing: 8

                        // Badge / Tag Row
                        RowLayout {
                            spacing: 8

                            Rectangle {
                                height: 20
                                radius: 4
                                implicitWidth: featBadgeTxt.implicitWidth + 12
                                color: theme.surfaceElevated
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
                                color: theme.textMuted
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                        }

                        // App Icon + Name Row
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Rectangle {
                                width: 44
                                height: 44
                                radius: 10
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1

                                Image {
                                    anchors.centerIn: parent
                                    width: 34
                                    height: 34
                                    source: heroPanel.currentItem ? (heroPanel.currentItem.icon_url || "") : ""
                                    fillMode: Image.PreserveAspectFit
                                    smooth: true
                                    visible: status === Image.Ready
                                }

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    visible: !heroPanel.currentItem || !heroPanel.currentItem.icon_url || heroPanel.currentItem.icon_url === ""
                                    name: heroPanel.currentItem ? root.getCategoryIcon(heroPanel.currentItem.category) : "layout-grid"
                                    size: 24
                                    color: theme.textMuted
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                text: heroPanel.currentItem ? (heroPanel.currentItem.display_name || heroPanel.currentItem.name || "") : ""
                                font.pixelSize: root.width < 700 ? 19 : 22
                                font.bold: true
                                color: theme.text
                                elide: Text.ElideRight
                            }
                        }

                        // Summary description: Noticeably bigger font (13.5px) for clear reading
                        Text {
                            Layout.fillWidth: true
                            text: heroPanel.currentItem ? (heroPanel.currentItem.summary || heroPanel.currentItem.description || "") : ""
                            font.pixelSize: 14
                            lineHeight: 1.3
                            color: theme.textSecondary
                            maximumLineCount: 2
                            wrapMode: Text.WordWrap
                            elide: Text.ElideRight
                        }

                        Item { Layout.fillHeight: true }

                        // Action Row + Carousel Controls
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10

                            // Primary Button: Get / Open Application
                            Rectangle {
                                height: 32
                                implicitWidth: heroBtnRow.implicitWidth + 22
                                radius: theme.radiusSm
                                color: heroBtnMouse.containsMouse ? theme.accentHover : theme.accent

                                RowLayout {
                                    id: heroBtnRow
                                    anchors.centerIn: parent
                                    spacing: 6
                                    RyzoraIcon {
                                        name: heroPanel.currentItem && heroPanel.currentItem.is_installed ? "play" : "download"
                                        size: 11
                                        color: "#ffffff"
                                    }
                                    Text {
                                        text: heroPanel.currentItem && heroPanel.currentItem.is_installed ? "Open Application" : "Get Application"
                                        font.pixelSize: 11
                                        font.bold: true
                                        color: "#ffffff"
                                    }
                                }

                                MouseArea {
                                    id: heroBtnMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (heroPanel.currentItem) {
                                            if (heroPanel.currentItem.is_installed) {
                                                root.launchRequested(heroPanel.currentItem.id)
                                            } else {
                                                root.installRequested(heroPanel.currentItem.id)
                                            }
                                        }
                                    }
                                }
                            }

                            // Secondary Button: View Details
                            Rectangle {
                                height: 32
                                implicitWidth: detailBtnRow.implicitWidth + 18
                                radius: theme.radiusSm
                                color: detailBtnMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                border.color: detailBtnMouse.containsMouse ? theme.accent : theme.borderStrong
                                border.width: 1

                                RowLayout {
                                    id: detailBtnRow
                                    anchors.centerIn: parent
                                    spacing: 4
                                    Text {
                                        text: "See Details"
                                        font.pixelSize: 11
                                        color: theme.text
                                    }
                                }

                                MouseArea {
                                    id: detailBtnMouse
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

                            // Carousel Navigation Controls (‹ Prev, Dots, Next ›)
                            RowLayout {
                                spacing: 8
                                visible: root.featuredItems.length > 1

                                // Prev Arrow
                                Rectangle {
                                    width: 26
                                    height: 26
                                    radius: 13
                                    color: prevMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                    border.color: prevMouse.containsMouse ? theme.accent : theme.borderStrong
                                    border.width: 1

                                    Text {
                                        anchors.centerIn: parent
                                        text: "‹"
                                        font.pixelSize: 16
                                        font.bold: true
                                        color: theme.text
                                    }

                                    MouseArea {
                                        id: prevMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            var n = root.featuredItems.length
                                            root.featuredIndex = (root.featuredIndex - 1 + n) % n
                                        }
                                    }
                                }

                                // Pagination Pills
                                RowLayout {
                                    spacing: 5
                                    Repeater {
                                        model: root.featuredItems.length
                                        Rectangle {
                                            width: modelData === root.featuredIndex ? 20 : 6
                                            height: 6
                                            radius: 3
                                            color: modelData === root.featuredIndex ? theme.accent : theme.borderStrong
                                            Behavior on width { NumberAnimation { duration: 150 } }
                                            MouseArea {
                                                anchors.fill: parent
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: root.featuredIndex = modelData
                                            }
                                        }
                                    }
                                }

                                // Next Arrow
                                Rectangle {
                                    width: 26
                                    height: 26
                                    radius: 13
                                    color: nextMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                    border.color: nextMouse.containsMouse ? theme.accent : theme.borderStrong
                                    border.width: 1

                                    Text {
                                        anchors.centerIn: parent
                                        text: "›"
                                        font.pixelSize: 16
                                        font.bold: true
                                        color: theme.text
                                    }

                                    MouseArea {
                                        id: nextMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            var n = root.featuredItems.length
                                            root.featuredIndex = (root.featuredIndex + 1) % n
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Right Column: Authentic Application Artwork (~50% width)
                    Rectangle {
                        Layout.preferredWidth: Math.max(260, Math.round(heroPanel.width * (root.width < 700 ? 0.40 : 0.50)))
                        Layout.fillHeight: true
                        color: "transparent"
                        clip: true

                        Image {
                            id: heroImg
                            anchors.fill: parent
                            source: heroPanel.currentItem ? (heroPanel.currentItem.banner_url || (heroPanel.currentItem.screenshots && heroPanel.currentItem.screenshots.length > 0 ? heroPanel.currentItem.screenshots[0] : "")) : ""
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                            cache: true
                            sourceSize.width: 1080
                            sourceSize.height: 600
                            smooth: true
                            visible: status === Image.Ready
                            opacity: status === Image.Ready ? 1.0 : 0.0
                            Behavior on opacity { NumberAnimation { duration: 180 } }
                        }

                        // Left-edge gradient overlay for seamless blending into left card background
                        Rectangle {
                            anchors.left: parent.left
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            width: root.width < 700 ? 90 : 160
                            z: 2
                            gradient: Gradient {
                                orientation: Gradient.Horizontal
                                GradientStop { position: 0.0; color: theme.resolvedTheme === "light" ? "#DFE7F3" : "#151C2C" }
                                GradientStop { position: 1.0; color: "transparent" }
                            }
                        }

                        // Clean fallback if screenshot is not ready
                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 8
                            visible: heroImg.status !== Image.Ready

                            Rectangle {
                                Layout.alignment: Qt.AlignHCenter
                                width: 64
                                height: 64
                                radius: 14
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1

                                Image {
                                    anchors.centerIn: parent
                                    width: 48
                                    height: 48
                                    source: heroPanel.currentItem ? (heroPanel.currentItem.icon_url || "") : ""
                                    fillMode: Image.PreserveAspectFit
                                    smooth: true
                                    visible: status === Image.Ready
                                }

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    visible: !heroPanel.currentItem || !heroPanel.currentItem.icon_url
                                    name: heroPanel.currentItem ? root.getCategoryIcon(heroPanel.currentItem.category) : "layout-grid"
                                    size: 32
                                    color: theme.textMuted
                                }
                            }

                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: heroPanel.currentItem ? (heroPanel.currentItem.display_name || heroPanel.currentItem.name) : ""
                                font.pixelSize: 13
                                font.bold: true
                                color: theme.textSecondary
                            }
                        }

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
                        text: (root.featuredAppId !== "") ? "More Applications" : (root.selectedCategory === "All" ? "All Applications" : root.selectedCategory)
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

                // Empty State
                EmptyState {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 240
                    visible: root.visibleCatalogApps.length === 0
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
    // 2. DEDICATED APPLICATION DETAIL VIEW (Overlay for full software details)
    // ──────────────────────────────────────────────────────────────────────────
    Rectangle {
        id: appDetailOverlay
        anchors.fill: parent
        visible: root.selectedApp !== null
        color: theme.background
        z: 10

        Flickable {
            anchors.fill: parent
            contentWidth: width
            contentHeight: detailCol.implicitHeight + 60
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
                id: detailCol
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.margins: 28
                spacing: 20

                // Back Navigation Button
                Rectangle {
                    height: 32
                    implicitWidth: backRow.implicitWidth + 18
                    radius: theme.radiusSm
                    color: backMouse.containsMouse ? theme.surfaceHover : theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: backRow
                        anchors.centerIn: parent
                        spacing: 6
                        RyzoraIcon { name: "arrow-right"; size: 12; color: theme.textSecondary; rotation: 180 }
                        Text {
                            text: "Back to Applications"
                            font.pixelSize: 12
                            font.bold: true
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

                // App Hero Header: Large Icon + Name + Publisher + Install/Launch Actions
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: headerRow.implicitHeight + 36
                    radius: theme.radiusLg
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: headerRow
                        anchors.fill: parent
                        anchors.margins: 18
                        spacing: 20

                        // Big App Icon
                        Rectangle {
                            width: 72
                            height: 72
                            radius: theme.radiusMd
                            color: theme.surfaceElevated
                            border.color: theme.borderSubtle
                            border.width: 1

                            Image {
                                anchors.centerIn: parent
                                width: 48
                                height: 48
                                source: root.selectedApp ? (root.selectedApp.icon_url || "") : ""
                                fillMode: Image.PreserveAspectFit
                                visible: status === Image.Ready
                            }

                            RyzoraIcon {
                                anchors.centerIn: parent
                                visible: !root.selectedApp || !root.selectedApp.icon_url || root.selectedApp.icon_url === ""
                                name: root.selectedApp ? root.getCategoryIcon(root.selectedApp.category) : "layout-grid"
                                size: 36
                                color: theme.textMuted
                            }
                        }

                        // App Info
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: root.selectedApp ? (root.selectedApp.display_name || root.selectedApp.name) : ""
                                    font.pixelSize: 22
                                    font.bold: true
                                    color: theme.text
                                }
                                Rectangle {
                                    visible: Boolean(root.selectedApp && root.selectedApp.is_installed)
                                    height: 18
                                    implicitWidth: instBadgeTxt.implicitWidth + 10
                                    radius: 9
                                    color: Qt.rgba(16/255, 185/255, 129/255, 0.15)
                                    border.color: "#10b981"
                                    border.width: 1
                                    Text {
                                        id: instBadgeTxt
                                        anchors.centerIn: parent
                                        text: "INSTALLED"
                                        font.pixelSize: 9
                                        font.bold: true
                                        color: "#10b981"
                                    }
                                }
                            }

                            Text {
                                text: root.selectedApp ? (root.selectedApp.publisher + " · " + root.selectedApp.category + " · " + (root.selectedApp.repository || "Arch Native")) : ""
                                font.pixelSize: 12
                                color: theme.textSecondary
                            }

                            Text {
                                text: root.selectedApp ? (root.selectedApp.summary || root.selectedApp.description || "") : ""
                                font.pixelSize: 12
                                color: theme.textMuted
                                Layout.fillWidth: true
                                wrapMode: Text.WordWrap
                            }
                        }

                        // Action Buttons: Open, Install, Uninstall
                        RowLayout {
                            spacing: 10

                            Rectangle {
                                visible: Boolean(root.selectedApp && root.selectedApp.is_installed)
                                height: 36
                                implicitWidth: launchBtnRow.implicitWidth + 24
                                radius: theme.radiusSm
                                color: theme.surfaceElevated
                                border.color: theme.accent
                                border.width: 1

                                RowLayout {
                                    id: launchBtnRow
                                    anchors.centerIn: parent
                                    spacing: 6
                                    RyzoraIcon { name: "play"; size: 12; color: theme.accent }
                                    Text {
                                        text: "Open"
                                        font.pixelSize: 12
                                        font.bold: true
                                        color: theme.text
                                    }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (root.selectedApp) {
                                            root.launchRequested(root.selectedApp.id)
                                        }
                                    }
                                }
                            }

                            Rectangle {
                                visible: Boolean(root.selectedApp && !root.selectedApp.is_installed)
                                height: 36
                                implicitWidth: instBtnRow.implicitWidth + 28
                                radius: theme.radiusSm
                                color: theme.accent

                                RowLayout {
                                    id: instBtnRow
                                    anchors.centerIn: parent
                                    spacing: 6
                                    RyzoraIcon { name: "download"; size: 12; color: "#ffffff" }
                                    Text {
                                        text: "Install"
                                        font.pixelSize: 12
                                        font.bold: true
                                        color: "#ffffff"
                                    }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (root.selectedApp) {
                                            root.installRequested(root.selectedApp.id)
                                        }
                                    }
                                }
                            }

                            Rectangle {
                                visible: Boolean(root.selectedApp && root.selectedApp.is_installed)
                                height: 36
                                implicitWidth: uninstBtnRow.implicitWidth + 20
                                radius: theme.radiusSm
                                color: theme.surfaceElevated
                                border.color: theme.borderStrong
                                border.width: 1

                                RowLayout {
                                    id: uninstBtnRow
                                    anchors.centerIn: parent
                                    spacing: 6
                                    RyzoraIcon { name: "trash"; size: 12; color: "#ef4444" }
                                    Text {
                                        text: "Uninstall"
                                        font.pixelSize: 12
                                        color: "#ef4444"
                                    }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (root.selectedApp) {
                                            root.uninstallRequested(root.selectedApp.id)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Screenshots Carousel (if screenshots exist)
                ColumnLayout {
                    visible: Boolean(root.selectedApp && root.selectedApp.screenshots && root.selectedApp.screenshots.length > 0)
                    Layout.fillWidth: true
                    spacing: 10

                    Text {
                        text: "Screenshots"
                        font.pixelSize: 16
                        font.bold: true
                        color: theme.text
                    }

                    Flickable {
                        Layout.fillWidth: true
                        height: 220
                        contentWidth: screenRow.implicitWidth
                        contentHeight: 220
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds

                        RowLayout {
                            id: screenRow
                            spacing: 14

                            Repeater {
                                model: root.selectedApp && root.selectedApp.screenshots ? root.selectedApp.screenshots : []
                                Rectangle {
                                    width: 390
                                    height: 220
                                    radius: theme.radiusMd
                                    color: theme.surface
                                    border.color: theme.borderSubtle
                                    border.width: 1
                                    clip: true

                                    Image {
                                        anchors.fill: parent
                                        source: modelData
                                        fillMode: Image.PreserveAspectCrop
                                        asynchronous: true
                                        cache: true
                                        smooth: true
                                    }
                                }
                            }
                        }
                    }
                }

                // Full Application Description & Metadata
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    Text {
                        text: "About this application"
                        font.pixelSize: 16
                        font.bold: true
                        color: theme.text
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: descCol.implicitHeight + 28
                        radius: theme.radiusMd
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            id: descCol
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 12

                            Text {
                                Layout.fillWidth: true
                                text: root.selectedApp ? (root.selectedApp.description || root.selectedApp.summary || "") : ""
                                font.pixelSize: 13
                                color: theme.text
                                wrapMode: Text.WordWrap
                                lineHeight: 1.4
                            }

                            Rectangle {
                                Layout.fillWidth: true
                                height: 1
                                color: theme.borderSubtle
                            }

                            GridLayout {
                                Layout.fillWidth: true
                                columns: 2
                                rowSpacing: 8
                                columnSpacing: 20

                                Text { text: "Version:"; font.pixelSize: 11; font.bold: true; color: theme.textSecondary }
                                Text { text: root.selectedApp ? root.selectedApp.version : ""; font.pixelSize: 11; font.family: "monospace"; color: theme.text }

                                Text { text: "Publisher:"; font.pixelSize: 11; font.bold: true; color: theme.textSecondary }
                                Text { text: root.selectedApp ? root.selectedApp.publisher : ""; font.pixelSize: 11; color: theme.text }

                                Text { text: "Repository:"; font.pixelSize: 11; font.bold: true; color: theme.textSecondary }
                                Text { text: root.selectedApp ? (root.selectedApp.repository + " (Official Arch Native)") : ""; font.pixelSize: 11; color: theme.text }

                                Text { text: "License:"; font.pixelSize: 11; font.bold: true; color: theme.textSecondary }
                                Text { text: root.selectedApp && root.selectedApp.license ? root.selectedApp.license : "Open Source"; font.pixelSize: 11; color: theme.text }

                                Text { text: "Website:"; font.pixelSize: 11; font.bold: true; color: theme.textSecondary }
                                Text {
                                    text: root.selectedApp && root.selectedApp.website ? root.selectedApp.website : ""
                                    font.pixelSize: 11
                                    color: theme.accent
                                    elide: Text.ElideRight
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
        }
    }
}
