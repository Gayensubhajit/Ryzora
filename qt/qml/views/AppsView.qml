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
            for (var j = 0; j < list.length && res.length < 4; j++) {
                var cand = list[j]
                if (cand && !seen[cand.id] && cand.banner_url) {
                    res.push(cand)
                    seen[cand.id] = true
                }
            }
        }
        return res
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 1. MAIN BROWSE VIEW
    // ──────────────────────────────────────────────────────────────────────────
    Flickable {
        id: mainScroll
        anchors.fill: parent
        visible: root.selectedApp === null
        contentWidth: width
        contentHeight: pageCol.implicitHeight + 36
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
            anchors.margins: 20
            spacing: 14

            // ── 1. COMPACT PAGE HEADER ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 14

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

                // In-page Search Input
                Rectangle {
                    Layout.preferredWidth: Math.min(240, Math.max(160, root.width * 0.24))
                    height: 32
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: searchInp.activeFocus ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 8
                        spacing: 6

                        RyzoraIcon {
                            name: "search"
                            size: 13
                            color: theme.textMuted
                        }

                        TextInput {
                            id: searchInp
                            Layout.fillWidth: true
                            text: root.searchQuery
                            font.pixelSize: 12
                            color: theme.text
                            clip: true
                            selectByMouse: true
                            onTextChanged: {
                                root.searchQuery = text
                                backend.filterApps(root.selectedCategory, text)
                            }

                            Text {
                                visible: !searchInp.text && !searchInp.activeFocus
                                text: "Search applications…"
                                font.pixelSize: 12
                                color: theme.textMuted
                            }
                        }

                        Rectangle {
                            visible: searchInp.text.length > 0
                            width: 16
                            height: 16
                            radius: 8
                            color: "transparent"
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "x"
                                size: 10
                                color: theme.textMuted
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    searchInp.text = ""
                                    root.clearSearchRequested()
                                }
                            }
                        }
                    }
                }

                // Sort Control
                Rectangle {
                    height: 32
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

            // ── 2. REAL SOFTWARE CATEGORIES FILTER BAR ──
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
                            color: isSelected ? theme.surfaceElevated : (catMouse.containsMouse ? theme.surfaceHover : theme.surface)
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
                                    color: isSelected ? theme.text : theme.textSecondary
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

            // ── 3. FEATURED APPLICATION HERO PANEL (168px high) ──
            Rectangle {
                id: heroPanel
                visible: root.searchQuery === "" && root.featuredItems.length > 0 && root.selectedCategory === "All"
                Layout.fillWidth: true
                height: 168
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1
                clip: true

                readonly property var currentItem: (root.featuredItems.length > 0 && root.featuredIndex < root.featuredItems.length) ? root.featuredItems[root.featuredIndex] : null

                RowLayout {
                    anchors.fill: parent
                    spacing: 0

                    // Left Column: Application Information & Actions
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.margins: 18
                        spacing: 6

                        RowLayout {
                            spacing: 8

                            Rectangle {
                                height: 18
                                radius: 4
                                implicitWidth: featBadgeTxt.implicitWidth + 10
                                color: theme.surfaceElevated
                                border.color: theme.borderStrong
                                border.width: 1

                                Text {
                                    id: featBadgeTxt
                                    anchors.centerIn: parent
                                    text: "FEATURED"
                                    font.pixelSize: 9
                                    font.bold: true
                                    font.letterSpacing: 0.8
                                    color: theme.accent
                                }
                            }

                            Text {
                                text: heroPanel.currentItem ? (String(heroPanel.currentItem.category || "Application").toUpperCase() + " · " + (heroPanel.currentItem.publisher || "Arch Linux Native")) : ""
                                font.pixelSize: 10
                                font.letterSpacing: 0.5
                                color: theme.textMuted
                                elide: Text.ElideRight
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: heroPanel.currentItem ? (heroPanel.currentItem.display_name || heroPanel.currentItem.name || "") : ""
                            font.pixelSize: 18
                            font.bold: true
                            color: theme.text
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: heroPanel.currentItem ? (heroPanel.currentItem.summary || heroPanel.currentItem.description || "") : ""
                            font.pixelSize: 12
                            color: theme.textSecondary
                            maximumLineCount: 2
                            wrapMode: Text.WordWrap
                            elide: Text.ElideRight
                        }

                        Item { Layout.fillHeight: true }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Rectangle {
                                height: 28
                                implicitWidth: exploreRow.implicitWidth + 20
                                radius: theme.radiusSm
                                color: heroBtnMouse.containsMouse ? theme.accentHover : theme.accent

                                RowLayout {
                                    id: exploreRow
                                    anchors.centerIn: parent
                                    spacing: 6
                                    Text {
                                        text: heroPanel.currentItem && heroPanel.currentItem.is_installed ? "Open Application" : "Get Application"
                                        font.pixelSize: 11
                                        font.bold: true
                                        color: "#ffffff"
                                    }
                                    RyzoraIcon {
                                        name: heroPanel.currentItem && heroPanel.currentItem.is_installed ? "play" : "download"
                                        size: 11
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

                            // Secondary: View Details
                            Rectangle {
                                height: 28
                                implicitWidth: detailRow.implicitWidth + 16
                                radius: theme.radiusSm
                                color: theme.surfaceElevated
                                border.color: detailMouse.containsMouse ? theme.accent : theme.borderStrong
                                border.width: 1

                                RowLayout {
                                    id: detailRow
                                    anchors.centerIn: parent
                                    spacing: 4
                                    Text {
                                        text: "Details"
                                        font.pixelSize: 11
                                        color: theme.text
                                    }
                                }

                                MouseArea {
                                    id: detailMouse
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

                            // Carousel Dots
                            RowLayout {
                                spacing: 5
                                visible: root.featuredItems.length > 1
                                Repeater {
                                    model: root.featuredItems.length
                                    Rectangle {
                                        width: modelData === root.featuredIndex ? 16 : 6
                                        height: 6
                                        radius: 3
                                        color: modelData === root.featuredIndex ? theme.accent : theme.borderStrong
                                        Behavior on width { NumberAnimation { duration: 140 } }
                                        MouseArea {
                                            anchors.fill: parent
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: root.featuredIndex = modelData
                                        }
                                    }
                                }
                            }

                            Item { Layout.fillWidth: true }
                        }
                    }

                    // Subtle Vertical Divider
                    Rectangle {
                        Layout.fillHeight: true
                        width: 1
                        color: theme.borderSubtle
                    }

                    // Right Column: Authentic Application Screenshot / Banner
                    Rectangle {
                        Layout.preferredWidth: Math.min(360, Math.max(240, heroPanel.width * 0.40))
                        Layout.fillHeight: true
                        color: theme.resolvedTheme === "light" ? "#F0F2F5" : "#14171D"
                        clip: true

                        Image {
                            id: heroImg
                            anchors.fill: parent
                            source: heroPanel.currentItem ? (heroPanel.currentItem.banner_url || (heroPanel.currentItem.screenshots && heroPanel.currentItem.screenshots.length > 0 ? heroPanel.currentItem.screenshots[0] : "")) : ""
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                            cache: true
                            sourceSize.width: 720
                            sourceSize.height: 405
                            smooth: true
                            visible: status === Image.Ready
                        }

                        // Fallback when banner is loading or unavailable
                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 8
                            visible: heroImg.status !== Image.Ready

                            Image {
                                Layout.alignment: Qt.AlignHCenter
                                width: 48
                                height: 48
                                source: heroPanel.currentItem ? (heroPanel.currentItem.icon_url || "") : ""
                                fillMode: Image.PreserveAspectFit
                                visible: status === Image.Ready
                            }

                            RyzoraIcon {
                                Layout.alignment: Qt.AlignHCenter
                                visible: !heroPanel.currentItem || !heroPanel.currentItem.icon_url
                                name: heroPanel.currentItem ? root.getCategoryIcon(heroPanel.currentItem.category) : "layout-grid"
                                size: 32
                                color: theme.textMuted
                            }

                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: heroPanel.currentItem ? heroPanel.currentItem.display_name : ""
                                font.pixelSize: 12
                                font.bold: true
                                color: theme.textMuted
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

            // ── 4. MAIN APPLICATIONS GRID (4-5 Columns on Desktop) ──
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: root.selectedCategory === "All" ? "All Applications" : root.selectedCategory
                        font.pixelSize: 15
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "(" + root.filteredApps.length + ")"
                        font.pixelSize: 12
                        font.family: "monospace"
                        color: theme.textMuted
                    }

                    Item { Layout.fillWidth: true }
                }

                GridLayout {
                    id: catalogGrid
                    Layout.fillWidth: true
                    columns: root.width < 520 ? 1 : (root.width < 760 ? 2 : (root.width < 1000 ? 3 : (root.width < 1300 ? 4 : 5)))
                    rowSpacing: 14
                    columnSpacing: 14

                    Repeater {
                        model: root.filteredApps

                        delegate: Rectangle {
                            id: appCard
                            Layout.fillWidth: true
                            height: 220
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
                                    root.setSelectedApp(modelData)
                                    root.appSelected(modelData)
                                }
                            }

                            ColumnLayout {
                                anchors.fill: parent
                                spacing: 0

                                // Top Landscape Area (116px): Genuine Screenshot or Neutral Framed Icon
                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 116
                                    color: theme.resolvedTheme === "light" ? "#F1F3F6" : "#14171D"
                                    clip: true

                                    // 1. Screenshot banner if available
                                    Image {
                                        id: cardBannerImg
                                        anchors.fill: parent
                                        source: modelData.banner_url || ""
                                        fillMode: Image.PreserveAspectCrop
                                        asynchronous: true
                                        cache: true
                                        sourceSize.width: 480
                                        sourceSize.height: 270
                                        smooth: true
                                        visible: modelData.banner_url !== "" && status === Image.Ready
                                    }

                                    // 2. Centered Authentic Icon if no screenshot banner
                                    Item {
                                        anchors.fill: parent
                                        visible: !cardBannerImg.visible

                                        Rectangle {
                                            anchors.centerIn: parent
                                            width: 56
                                            height: 56
                                            radius: theme.radiusMd
                                            color: theme.surface
                                            border.color: theme.borderSubtle
                                            border.width: 1

                                            Image {
                                                id: fallbackIconImg
                                                anchors.centerIn: parent
                                                width: 36
                                                height: 36
                                                source: modelData.icon_url || ""
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                visible: status === Image.Ready
                                            }

                                            RyzoraIcon {
                                                anchors.centerIn: parent
                                                visible: fallbackIconImg.status !== Image.Ready
                                                name: root.getCategoryIcon(modelData.category)
                                                size: 24
                                                color: theme.textMuted
                                            }
                                        }
                                    }

                                    // Subtle divider
                                    Rectangle {
                                        anchors.bottom: parent.bottom
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        height: 1
                                        color: theme.borderSubtle
                                    }
                                }

                                // Compact Information Block (104px)
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    Layout.leftMargin: 12
                                    Layout.rightMargin: 12
                                    Layout.topMargin: 10
                                    Layout.bottomMargin: 10
                                    spacing: 4

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 8

                                        // Mini App Icon in Header
                                        Rectangle {
                                            width: 20
                                            height: 20
                                            radius: 4
                                            color: theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            border.width: 1

                                            Image {
                                                anchors.centerIn: parent
                                                width: 14
                                                height: 14
                                                source: modelData.icon_url || ""
                                                fillMode: Image.PreserveAspectFit
                                                visible: status === Image.Ready
                                            }

                                            RyzoraIcon {
                                                anchors.centerIn: parent
                                                visible: !modelData.icon_url || modelData.icon_url === ""
                                                name: root.getCategoryIcon(modelData.category)
                                                size: 10
                                                color: theme.textSecondary
                                            }
                                        }

                                        Text {
                                            Layout.fillWidth: true
                                            text: modelData.display_name || modelData.name || modelData.id
                                            font.pixelSize: 13
                                            font.bold: true
                                            color: theme.text
                                            elide: Text.ElideRight
                                        }
                                    }

                                    Text {
                                        Layout.fillWidth: true
                                        text: modelData.summary || modelData.description || "Linux desktop application."
                                        font.pixelSize: 11
                                        color: theme.textSecondary
                                        maximumLineCount: 2
                                        wrapMode: Text.WordWrap
                                        elide: Text.ElideRight
                                    }

                                    Item { Layout.fillHeight: true }

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 6

                                        Text {
                                            Layout.fillWidth: true
                                            text: (modelData.publisher || "Arch Native") + " · " + (modelData.category || "App")
                                            font.pixelSize: 10
                                            color: theme.textMuted
                                            elide: Text.ElideRight
                                        }

                                        // Action Pill: Open (if installed) or Get (if uninstalled)
                                        Rectangle {
                                            height: 22
                                            implicitWidth: cardBtnRow.implicitWidth + 14
                                            radius: 4
                                            color: modelData.is_installed
                                                ? (actionMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                                                : (actionMouse.containsMouse ? theme.accentHover : theme.accent)
                                            border.color: modelData.is_installed ? theme.borderStrong : theme.accent
                                            border.width: 1

                                            RowLayout {
                                                id: cardBtnRow
                                                anchors.centerIn: parent
                                                spacing: 4

                                                RyzoraIcon {
                                                    name: modelData.is_installed ? "play" : "download"
                                                    size: 9
                                                    color: modelData.is_installed ? "#10b981" : "#ffffff"
                                                }

                                                Text {
                                                    text: modelData.is_installed ? "Open" : "Get"
                                                    font.pixelSize: 10
                                                    font.bold: true
                                                    color: modelData.is_installed ? theme.text : "#ffffff"
                                                }
                                            }

                                            MouseArea {
                                                id: actionMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: {
                                                    if (modelData.is_installed) {
                                                        root.launchRequested(modelData.id)
                                                    } else {
                                                        root.installRequested(modelData.id)
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

                // Empty State
                EmptyState {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 280
                    visible: root.filteredApps.length === 0
                    iconName: "layout-grid"
                    title: "No Applications Found"
                    message: "No software applications match category '" + root.selectedCategory + "'" + (root.searchQuery ? " and query '" + root.searchQuery + "'" : "") + "."
                    actionLabel: "Clear Search & Filter"
                    onActionClicked: {
                        root.selectedCategory = "All"
                        root.searchQuery = ""
                        searchInp.text = ""
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
