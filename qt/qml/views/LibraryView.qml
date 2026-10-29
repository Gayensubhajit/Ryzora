import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    signal packageSelected(var packageData)
    signal navigateRequested(string tabName)
    signal customizeRequested(string subpage, var packageData)

    property string searchQuery: ""
    property string selectedCategory: "all" // "all" | "lockscreens" | "fastfetch" | "emblems" | "wallpapers"
    property string selectedStatusFilter: "all" // "all" | "active"
    property var pendingUninstallPackage: null

    property int libraryAppsRev: 0

    Connections {
        target: backend
        function onAppsChanged() {
            root.libraryAppsRev++
        }
        function onLibraryChanged() {
            root.libraryAppsRev++
        }
        function onActiveAppOperationsChanged() {
            root.libraryAppsRev++
        }
    }

    readonly property var combinedList: {
        var _rev = root.libraryAppsRev
        var combined = []
        var libList = backend ? (backend.libraryList || []) : []
        for (var i = 0; i < libList.length; ++i) {
            if (libList[i]) combined.push(libList[i])
        }
        var apps = backend ? (backend.getFilteredApps("Installed", "", "all", "name") || []) : []
        for (var j = 0; j < apps.length; ++j) {
            var a = apps[j]
            if (a && a.is_installed) {
                combined.push({
                    id: a.id,
                    name: a.display_name || a.name || a.id,
                    version: a.installed_version || a.version || "1.0.0",
                    package_type: "desktop_app",
                    category: "apps",
                    description: a.summary || a.description || "",
                    preview_url: a.icon_url || a.banner_url || "",
                    author: a.publisher || "Community",
                    is_installed: true,
                    is_active: false,
                    supports_sddm: false,
                    appData: a
                })
            }
        }
        return combined
    }

    function getCombinedList() {
        return root.combinedList
    }

    function getPackageCategory(item) {
        if (!item) return "other"
        var id = String(item.id || "").toLowerCase()
        var cat = String(item.category || "").toLowerCase()
        var ptype = String(item.package_type || "").toLowerCase()
        var sub = String(item.subfilter || "").toLowerCase()

        if (ptype === "desktop_app" || cat === "apps") {
            return "apps"
        }
        if (id.indexOf("fastfetch-emblem-") === 0 || sub === "emblem" || (cat === "fastfetch" && ptype === "emblem")) {
            return "emblems"
        }
        if (cat === "fastfetch" || ptype === "fastfetch" || id.indexOf("fastfetch-") === 0) {
            return "fastfetch"
        }
        if (ptype === "wallpaper" && !item.supports_sddm && sub !== "silentsddm") {
            return "wallpapers"
        }
        if (cat === "lockscreens" || cat === "lockscreen" || cat === "sddm" || ptype === "lockscreen" || item.supports_sddm || id.indexOf("lockscreen-") === 0 || id.indexOf("silentsddm-") === 0) {
            return "lockscreens"
        }
        return "other"
    }

    function getCategoryLabel(catKey) {
        if (catKey === "apps") return "Application"
        if (catKey === "lockscreens") return "Lockscreen"
        if (catKey === "fastfetch") return "Fastfetch"
        if (catKey === "emblems") return "Emblem"
        if (catKey === "wallpapers") return "Wallpaper"
        return "Package"
    }

    function getCategoryCount(catKey) {
        var list = root.getCombinedList()
        if (catKey === "all") return list.length
        var count = 0
        for (var i = 0; i < list.length; ++i) {
            if (getPackageCategory(list[i]) === catKey) count++
        }
        return count
    }

    function getActiveCount() {
        var list = backend.libraryList || []
        var count = 0
        for (var i = 0; i < list.length; ++i) {
            if (list[i] && list[i].is_active) count++
        }
        return count
    }

    function getFilteredList() {
        var list = root.getCombinedList()
        var q = root.searchQuery.trim().toLowerCase()
        var cat = root.selectedCategory
        var status = root.selectedStatusFilter

        var res = []
        for (var i = 0; i < list.length; ++i) {
            var item = list[i]
            if (!item) continue
            var itemCat = getPackageCategory(item)

            if (cat !== "all" && itemCat !== cat) continue
            if (status === "active" && !item.is_active) continue

            if (q.length > 0) {
                var name = String(item.name || "").toLowerCase()
                var id = String(item.id || "").toLowerCase()
                var desc = String(item.description || "").toLowerCase()
                if (name.indexOf(q) === -1 && id.indexOf(q) === -1 && desc.indexOf(q) === -1) {
                    continue
                }
            }
            res.push(item)
        }
        return res
    }

    function canOpenInStudio(item) {
        if (!item) return false
        var cat = getPackageCategory(item)
        return cat === "lockscreens" || cat === "fastfetch" || cat === "emblems" || cat === "wallpapers"
    }

    function openInStudio(item) {
        if (!item) return
        var cat = getPackageCategory(item)
        var subpage = (cat === "lockscreens") ? "lockscreen" :
                      (cat === "fastfetch" || cat === "emblems") ? "fastfetch" :
                      (cat === "wallpapers") ? "wallpapers" : "lockscreen"
        root.customizeRequested(subpage, item)
    }

    ScrollView {
        anchors.fill: parent
        clip: true
        contentWidth: availableWidth
        ScrollBar.vertical.policy: ScrollBar.AsNeeded

        ColumnLayout {
            width: Math.max(300, parent.width - 40)
            x: 20
            spacing: 16

            Item { width: 1; height: 4 }

            // ── 1. Header Toolbar ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 16

                ColumnLayout {
                    spacing: 2
                    Layout.fillWidth: true

                    Text {
                        text: "Library"
                        font.pixelSize: 22
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Manage your installed packages · " + root.getCategoryCount("all") + " installed · " + root.getActiveCount() + " active"
                        font.pixelSize: 12
                        color: theme.textMuted
                    }
                }

                // Compact Search Input
                Rectangle {
                    width: 210
                    height: 32
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: searchInput.activeFocus ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 6

                        RyzoraIcon {
                            name: "search"
                            size: 13
                            color: theme.textMuted
                        }

                        TextInput {
                            id: searchInput
                            Layout.fillWidth: true
                            font.pixelSize: 11
                            color: theme.text
                            text: root.searchQuery
                            selectByMouse: true
                            clip: true
                            onTextChanged: root.searchQuery = text

                            Text {
                                text: "Search installed…"
                                font.pixelSize: 11
                                color: theme.textMuted
                                visible: !searchInput.text && !searchInput.activeFocus
                            }
                        }

                        Rectangle {
                            visible: root.searchQuery.length > 0
                            width: 16; height: 16; radius: 8
                            color: clrSearchM.containsMouse ? theme.surfaceHover : "transparent"
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "x"
                                size: 9
                                color: theme.textMuted
                            }
                            MouseArea {
                                id: clrSearchM
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.searchQuery = ""
                                    searchInput.text = ""
                                }
                            }
                        }
                    }
                }

                // Check Updates Action
                Rectangle {
                    height: 32
                    width: checkRow.implicitWidth + 20
                    radius: theme.radiusSm
                    color: checkM.containsMouse ? theme.surfaceHover : theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: checkRow
                        anchors.centerIn: parent
                        spacing: 6

                        RyzoraIcon {
                            name: "refresh"
                            size: 13
                            color: backend.isUpdatesLoading ? theme.accent : theme.textSecondary
                        }

                        Text {
                            text: backend.isUpdatesLoading ? "Checking…" : "Check for Updates"
                            font.pixelSize: 11
                            font.weight: Font.Medium
                            color: theme.textSecondary
                        }
                    }

                    MouseArea {
                        id: checkM
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: backend.loadUpdates()
                    }
                }
            }

            // ── 2. Filter Pills Bar ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                // Category Filter Pills
                Repeater {
                    model: [
                        { id: "all", label: "All", count: root.getCategoryCount("all") },
                        { id: "apps", label: "Apps", count: root.getCategoryCount("apps") },
                        { id: "lockscreens", label: "Lockscreen", count: root.getCategoryCount("lockscreens") },
                        { id: "fastfetch", label: "Fastfetch", count: root.getCategoryCount("fastfetch") },
                        { id: "emblems", label: "Emblems", count: root.getCategoryCount("emblems") },
                        { id: "wallpapers", label: "Wallpapers", count: root.getCategoryCount("wallpapers") }
                    ]

                    Rectangle {
                        property bool isSel: root.selectedCategory === modelData.id
                        height: 28
                        width: catPillRow.implicitWidth + 18
                        radius: 14
                        color: isSel ? theme.accentFaint : (catM.containsMouse ? theme.surfaceHover : theme.surface)
                        border.color: isSel ? theme.accent : theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            id: catPillRow
                            anchors.centerIn: parent
                            spacing: 5

                            Text {
                                text: modelData.label
                                font.pixelSize: 11
                                font.weight: parent.parent.isSel ? Font.DemiBold : Font.Normal
                                color: parent.parent.isSel ? theme.accent : theme.textSecondary
                            }

                            Text {
                                text: String(modelData.count)
                                font.pixelSize: 10
                                font.weight: Font.DemiBold
                                color: parent.parent.isSel ? theme.accent : theme.textMuted
                            }
                        }

                        MouseArea {
                            id: catM
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.selectedCategory = modelData.id
                        }
                    }
                }

                Rectangle {
                    width: 1
                    height: 18
                    color: theme.borderSubtle
                }

                // Active-Only Status Filter Pill
                Rectangle {
                    property bool isSel: root.selectedStatusFilter === "active"
                    height: 28
                    width: actPillRow.implicitWidth + 18
                    radius: 14
                    color: isSel ? theme.accentFaint : (actM.containsMouse ? theme.surfaceHover : theme.surface)
                    border.color: isSel ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: actPillRow
                        anchors.centerIn: parent
                        spacing: 5

                        Rectangle {
                            width: 6; height: 6; radius: 3
                            color: "#10b981"
                        }

                        Text {
                            text: "Active Only"
                            font.pixelSize: 11
                            font.weight: parent.parent.isSel ? Font.DemiBold : Font.Normal
                            color: parent.parent.isSel ? theme.accent : theme.textSecondary
                        }

                        Text {
                            text: String(root.getActiveCount())
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            color: parent.parent.isSel ? theme.accent : theme.textMuted
                        }
                    }

                    MouseArea {
                        id: actM
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.selectedStatusFilter = (root.selectedStatusFilter === "active" ? "all" : "active")
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: root.getFilteredList().length + " of " + backend.libraryCount + " packages"
                    font.pixelSize: 11
                    color: theme.textMuted
                }
            }

            // ── 3. Empty State (No Installed Packages on Host) ──
            Rectangle {
                visible: backend.libraryCount === 0 && !backend.isLibraryLoading
                Layout.fillWidth: true
                height: 220
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1

                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 10

                    RyzoraIcon {
                        name: "library"
                        size: 32
                        color: theme.textMuted
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Text {
                        text: "No packages installed yet"
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                        color: theme.text
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Text {
                        text: "Explore themes, lockscreens, and fastfetch presets in Discover."
                        font.pixelSize: 11
                        color: theme.textMuted
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Rectangle {
                        height: 32
                        width: discTxt.implicitWidth + 24
                        radius: theme.radiusSm
                        color: theme.accent
                        Layout.alignment: Qt.AlignHCenter

                        Text {
                            id: discTxt
                            anchors.centerIn: parent
                            text: "Browse Discover"
                            font.pixelSize: 11
                            font.weight: Font.DemiBold
                            color: "#ffffff"
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.navigateRequested("discover")
                        }
                    }
                }
            }

            // ── 4. Empty State (Filtered / Search Result Empty) ──
            Rectangle {
                visible: backend.libraryCount > 0 && root.getFilteredList().length === 0
                Layout.fillWidth: true
                height: 180
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1

                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 8

                    RyzoraIcon {
                        name: "filter"
                        size: 24
                        color: theme.textMuted
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Text {
                        text: "No packages match your search or filter"
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                        color: theme.text
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Rectangle {
                        height: 28
                        width: rstTxt.implicitWidth + 20
                        radius: theme.radiusSm
                        color: theme.surfaceHover
                        border.color: theme.borderSubtle
                        border.width: 1
                        Layout.alignment: Qt.AlignHCenter

                        Text {
                            id: rstTxt
                            anchors.centerIn: parent
                            text: "Clear Filters"
                            font.pixelSize: 11
                            font.weight: Font.Medium
                            color: theme.textSecondary
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.searchQuery = ""
                                searchInput.text = ""
                                root.selectedCategory = "all"
                                root.selectedStatusFilter = "all"
                            }
                        }
                    }
                }
            }

            // ── 5. Installed Packages Grid ──
            Grid {
                id: packageGrid
                Layout.fillWidth: true
                visible: root.getFilteredList().length > 0
                columns: {
                    var w = width
                    if (w >= 1100) return 4
                    if (w >= 780) return 3
                    if (w >= 500) return 2
                    return 1
                }
                spacing: 14
                readonly property real itemWidth: Math.floor((width - (columns - 1) * spacing) / Math.max(1, columns))

                Repeater {
                    model: root.getFilteredList()

                    Rectangle {
                        width: packageGrid.itemWidth
                        height: 228
                        radius: theme.radiusMd
                        color: theme.surface
                        border.color: modelData.is_active ? theme.accent : theme.borderSubtle
                        border.width: modelData.is_active ? 2 : 1
                        clip: true

                        ColumnLayout {
                            anchors.fill: parent
                            spacing: 0

                            // 1. Thumbnail Header
                            Rectangle {
                                Layout.fillWidth: true
                                height: 116
                                color: "#07090D"
                                clip: true

                                Image {
                                    id: cardImg
                                    anchors.fill: parent
                                    source: (modelData && modelData.preview_url && String(modelData.preview_url).indexOf("://") !== -1 && !String(modelData.preview_url).toLowerCase().endsWith(".jxl")) ? modelData.preview_url : ""
                                    fillMode: Image.PreserveAspectFit
                                    smooth: true
                                    mipmap: true
                                    asynchronous: true
                                    visible: status === Image.Ready
                                }

                                // Fallback Icon if Image Missing
                                RyzoraIcon {
                                    visible: !cardImg.source || cardImg.status === Image.Error || cardImg.status === Image.Null
                                    anchors.centerIn: parent
                                    name: {
                                        var c = root.getPackageCategory(modelData)
                                        if (c === "lockscreens") return "lock"
                                        if (c === "fastfetch") return "terminal"
                                        if (c === "emblems" || c === "wallpapers") return "image"
                                        return "package"
                                    }
                                    size: 28
                                    color: theme.textMuted
                                }

                                // Category Pill (Top-Left)
                                Rectangle {
                                    anchors.top: parent.top
                                    anchors.left: parent.left
                                    anchors.margins: 8
                                    height: 20
                                    width: catTxt.implicitWidth + 12
                                    radius: 10
                                    color: "#B3000000"
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    Text {
                                        id: catTxt
                                        anchors.centerIn: parent
                                        text: root.getCategoryLabel(root.getPackageCategory(modelData))
                                        font.pixelSize: 9
                                        font.weight: Font.DemiBold
                                        color: "#E2E8F0"
                                    }
                                }

                                // Active Badge (Top-Right)
                                Rectangle {
                                    visible: Boolean(modelData.is_active)
                                    anchors.top: parent.top
                                    anchors.right: parent.right
                                    anchors.margins: 8
                                    height: 20
                                    width: actBadgeRow.implicitWidth + 12
                                    radius: 10
                                    color: "#064E3B"
                                    border.color: "#10B981"
                                    border.width: 1

                                    RowLayout {
                                        id: actBadgeRow
                                        anchors.centerIn: parent
                                        spacing: 4
                                        Rectangle {
                                            width: 5; height: 5; radius: 2.5
                                            color: "#34D399"
                                        }
                                        Text {
                                            text: "ACTIVE"
                                            font.pixelSize: 9
                                            font.weight: Font.Bold
                                            color: "#A7F3D0"
                                        }
                                    }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.packageSelected(modelData)
                                }
                            }

                            // 2. Info & Metadata Area
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                Layout.margins: 10
                                spacing: 4

                                // Package Title
                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.name || modelData.id
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    color: theme.text
                                    elide: Text.ElideRight
                                }

                                // Version and Details Meta
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Text {
                                        text: modelData.version ? ("v" + modelData.version) : "v1.0.0"
                                        font.pixelSize: 10
                                        font.family: "JetBrains Mono, monospace"
                                        color: theme.accent
                                    }

                                    Text {
                                        text: "·"
                                        font.pixelSize: 10
                                        color: theme.textMuted
                                    }

                                    Text {
                                        text: modelData.author || "Community"
                                        font.pixelSize: 10
                                        color: theme.textMuted
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }
                                }

                                Item { Layout.fillHeight: true }

                                // 3. Action Buttons Row
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    // View Details Button
                                    Rectangle {
                                        Layout.fillWidth: !root.canOpenInStudio(modelData) && modelData.package_type !== "desktop_app"
                                        width: (root.canOpenInStudio(modelData) || modelData.package_type === "desktop_app") ? (detTxt.implicitWidth + 14) : undefined
                                        height: 26
                                        radius: theme.radiusSm
                                        color: detM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                        border.color: theme.borderSubtle
                                        border.width: 1

                                        Text {
                                            id: detTxt
                                            anchors.centerIn: parent
                                            text: "Details"
                                            font.pixelSize: 11
                                            font.weight: Font.Medium
                                            color: theme.textSecondary
                                        }

                                        MouseArea {
                                            id: detM
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: root.packageSelected(modelData)
                                        }
                                    }

                                    // Open Application Button (for desktop applications)
                                    Rectangle {
                                        visible: modelData && modelData.package_type === "desktop_app"
                                        Layout.fillWidth: true
                                        height: 26
                                        radius: theme.radiusSm
                                        color: openAppM.containsMouse ? theme.accent : theme.accentFaint
                                        border.color: theme.accent
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 4

                                            RyzoraIcon {
                                                name: "play"
                                                size: 10
                                                color: openAppM.containsMouse ? "#FFFFFF" : theme.accent
                                            }

                                            Text {
                                                text: "Open"
                                                font.pixelSize: 11
                                                font.weight: Font.DemiBold
                                                color: openAppM.containsMouse ? "#FFFFFF" : theme.accent
                                            }
                                        }

                                        MouseArea {
                                            id: openAppM
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: backend.launchApp(modelData.id)
                                        }
                                    }

                                    // Open in Studio Button (If compatible editor exists)
                                    Rectangle {
                                        visible: root.canOpenInStudio(modelData)
                                        Layout.fillWidth: true
                                        height: 26
                                        radius: theme.radiusSm
                                        color: stdM.containsMouse ? theme.accent : theme.accentFaint
                                        border.color: theme.accent
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 4

                                            RyzoraIcon {
                                                name: "sliders-horizontal"
                                                size: 11
                                                color: stdM.containsMouse ? "#FFFFFF" : theme.accent
                                            }

                                            Text {
                                                text: "Studio"
                                                font.pixelSize: 11
                                                font.weight: Font.DemiBold
                                                color: stdM.containsMouse ? "#FFFFFF" : theme.accent
                                            }
                                        }

                                        MouseArea {
                                            id: stdM
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: root.openInStudio(modelData)
                                        }
                                    }

                                    // Uninstall Button (Secondary with confirmation)
                                    Rectangle {
                                        width: 26; height: 26
                                        radius: theme.radiusSm
                                        color: unM.containsMouse ? Qt.rgba(239/255, 68/255, 68/255, 0.15) : theme.surfaceElevated
                                        border.color: unM.containsMouse ? "#EF4444" : theme.borderSubtle
                                        border.width: 1

                                        RyzoraIcon {
                                            anchors.centerIn: parent
                                            name: "trash"
                                            size: 11
                                            color: unM.containsMouse ? "#EF4444" : theme.textMuted
                                        }

                                        MouseArea {
                                            id: unM
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: root.pendingUninstallPackage = modelData
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Item { width: 1; height: 20 }
        }
    }

    // ── 6. Uninstall Confirmation Dialog ──
    Rectangle {
        visible: root.pendingUninstallPackage !== null
        anchors.fill: parent
        color: "#B3000000"
        z: 1000

        MouseArea {
            anchors.fill: parent
            onClicked: root.pendingUninstallPackage = null
        }

        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width - 40, 420)
            height: confirmCol.implicitHeight + 36
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.borderStrong
            border.width: 1

            MouseArea { anchors.fill: parent }

            ColumnLayout {
                id: confirmCol
                anchors.fill: parent
                anchors.margins: 18
                spacing: 14

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    Rectangle {
                        width: 32; height: 32; radius: 16
                        color: Qt.rgba(239/255, 68/255, 68/255, 0.15)
                        RyzoraIcon {
                            anchors.centerIn: parent
                            name: "trash"
                            size: 14
                            color: "#EF4444"
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Text {
                            text: "Uninstall Package"
                            font.pixelSize: 14
                            font.bold: true
                            color: theme.text
                        }
                        Text {
                            text: "Confirm package removal"
                            font.pixelSize: 11
                            color: theme.textMuted
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: root.pendingUninstallPackage
                          ? ("Are you sure you want to uninstall \"" + (root.pendingUninstallPackage.name || root.pendingUninstallPackage.id) + "\"? This will remove all package files and revert any associated configurations.")
                          : ""
                    font.pixelSize: 12
                    color: theme.textSecondary
                    wrapMode: Text.Wrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Item { Layout.fillWidth: true }

                    Rectangle {
                        width: cnlTxt.implicitWidth + 24
                        height: 32
                        radius: theme.radiusSm
                        color: cnlM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        Text {
                            id: cnlTxt
                            anchors.centerIn: parent
                            text: "Cancel"
                            font.pixelSize: 12
                            font.weight: Font.Medium
                            color: theme.text
                        }

                        MouseArea {
                            id: cnlM
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.pendingUninstallPackage = null
                        }
                    }

                    Rectangle {
                        width: unTxt.implicitWidth + 24
                        height: 32
                        radius: theme.radiusSm
                        color: "#DC2626"

                        Text {
                            id: unTxt
                            anchors.centerIn: parent
                            text: "Uninstall"
                            font.pixelSize: 12
                            font.weight: Font.DemiBold
                            color: "#FFFFFF"
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.pendingUninstallPackage && root.pendingUninstallPackage.id) {
                                    if (root.pendingUninstallPackage.package_type === "desktop_app") {
                                        backend.uninstallApp(root.pendingUninstallPackage.id)
                                    } else {
                                        backend.uninstallPackage(root.pendingUninstallPackage.id)
                                    }
                                }
                                root.pendingUninstallPackage = null
                            }
                        }
                    }
                }
            }
        }
    }
}
