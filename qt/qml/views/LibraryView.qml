import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    // ── PUBLIC SIGNALS & API ──
    signal appSelected(var appData)
    signal packageSelected(var packageData)
    signal customizeRequested(string subpage, var packageData)
    signal navigateRequested(string tabName)
    signal launchRequested(string appId)
    signal uninstallRequested(string appId)

    // ── PERSISTENT PAGE STATE ──
    property string searchQuery: ""
    property string selectedProvider: "all"       // "all" | "pacman" | "aur" | "flatpak"
    property string selectedTypeFilter: "all"     // "all" | "recent" | "updates" | "packages"
    property string selectedSort: "name"          // "name" | "size" | "recent"
    property string viewMode: "grid"              // "grid" | "list"
    property var selectedApp: null
    property var pendingUninstallApp: null
    property string pendingUninstallProvider: ""
    property bool isRefreshing: false

    // State revision counter for live updates
    property int libraryRev: 0

    // Dynamic responsive column count: guarantees zero horizontal overflow
    readonly property int responsiveColumns: {
        var w = root.width - (root.width < 600 ? 24 : 40)
        var spacing = 12
        var minCardWidth = 240
        var cols = Math.floor((w + spacing) / (minCardWidth + spacing))
        return Math.max(1, Math.min(w >= 1600 ? 5 : 4, cols))
    }

    Connections {
        target: backend
        function onAppsChanged() { root.libraryRev++ }
        function onLibraryChanged() { root.libraryRev++ }
        function onActiveAppOperationsChanged() { root.libraryRev++ }
        function onAppInstalled(appId, success, message) {
            root.libraryRev++
            root.isRefreshing = false
        }
        function onAppUninstalled(appId, success, message) {
            root.libraryRev++
            root.isRefreshing = false
            if (success && root.selectedApp && (root.selectedApp.id === appId || (appId && appId.indexOf(root.selectedApp.id) !== -1))) {
                root.selectedApp = null
            }
        }
    }

    // Cached library summary metrics
    readonly property var librarySummary: {
        var _rev = root.libraryRev
        if (backend && typeof backend.getLibrarySummary === "function") {
            return backend.getLibrarySummary()
        }
        return {
            installed_count: 0,
            packages_count: 0,
            updates_count: 0,
            total_installed_bytes: 0,
            total_installed_size_label: "—",
            recent_count: 0
        }
    }

    // Installed applications list matching current filters
    readonly property var installedList: {
        var _rev = root.libraryRev
        if (!backend || typeof backend.getInstalledApplications !== "function") return []
        return backend.getInstalledApplications(
            root.selectedProvider,
            root.selectedTypeFilter,
            root.searchQuery,
            root.selectedSort
        )
    }

    function formatBytes(bytes) {
        if (!bytes || bytes <= 0) return "—"
        if (bytes < 1024) return bytes + " B"
        if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB"
        if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + " MB"
        return (bytes / (1024 * 1024 * 1024)).toFixed(2) + " GB"
    }

    function getCategoryIcon(cat) {
        var c = (cat || "").toLowerCase()
        if (c.indexOf("internet") !== -1 || c.indexOf("network") !== -1 || c.indexOf("browser") !== -1) return "compass"
        if (c.indexOf("dev") !== -1 || c.indexOf("code") !== -1 || c.indexOf("terminal") !== -1) return "terminal"
        if (c.indexOf("multimedia") !== -1 || c.indexOf("audio") !== -1 || c.indexOf("video") !== -1) return "play"
        if (c.indexOf("graphic") !== -1 || c.indexOf("photo") !== -1 || c.indexOf("draw") !== -1) return "palette"
        if (c.indexOf("game") !== -1) return "zap"
        if (c.indexOf("system") !== -1 || c.indexOf("util") !== -1 || c.indexOf("tool") !== -1) return "settings"
        return "layout-grid"
    }

    function getProviderBadge(app) {
        if (!app) return "Official"
        var p = (app.provider_id || "").toLowerCase()
        if (p === "flatpak") return "Flatpak"
        if (p === "aur") return "AUR"
        var repo = (app.repository || "extra").toLowerCase()
        if (repo.indexOf("core") !== -1 || repo.indexOf("extra") !== -1 || repo.indexOf("multilib") !== -1) return "Official"
        return repo.charAt(0).toUpperCase() + repo.slice(1)
    }

    function openAppDetails(app) {
        root.selectedApp = app
        if (typeof detailScroll !== "undefined" && detailScroll) detailScroll.contentY = 0
    }

    function promptUninstall(app) {
        root.pendingUninstallApp = app
        root.pendingUninstallProvider = app ? (app.provider_id || "pacman") : "pacman"
        uninstallDialog.openForApp(app)
    }

    function confirmUninstall() {
        if (!root.pendingUninstallApp) return
        var id = root.pendingUninstallApp.id
        root.uninstallRequested(id)
        if (backend) {
            backend.requestUninstall(id)
        }
        root.pendingUninstallApp = null
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 1. MAIN LIBRARY BROWSE VIEW
    // ──────────────────────────────────────────────────────────────────────────
    Flickable {
        id: libraryScroll
        objectName: "libraryScroll"
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

            // ── A. HEADER ROW (Title + Subtitle + Refresh + View Mode) ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Text {
                        text: "My Library"
                        font.pixelSize: 22
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Your installed applications and software"
                        font.pixelSize: 13
                        color: theme.textSecondary
                    }
                }

                Item { Layout.fillWidth: true }

                // Refresh / Sync Library Button
                Rectangle {
                    height: 32
                    implicitWidth: refreshRow.implicitWidth + 20
                    radius: theme.radiusSm
                    color: refreshMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: refreshRow
                        anchors.centerIn: parent
                        spacing: 6

                        RyzoraIcon {
                            id: refreshSpinIcon
                            name: "refresh"
                            size: 13
                            color: theme.textSecondary
                            RotationAnimator {
                                target: refreshSpinIcon
                                from: 0
                                to: 360
                                duration: 1000
                                loops: Animation.Infinite
                                running: root.isRefreshing
                            }
                        }

                        Text {
                            text: "Sync"
                            font.pixelSize: 12
                            font.weight: Font.Medium
                            color: theme.text
                        }
                    }

                    MouseArea {
                        id: refreshMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.isRefreshing = true
                            if (backend) {
                                backend.refreshLibraryState()
                            }
                        }
                    }
                }

                // Grid / List View Mode Toggle
                Rectangle {
                    height: 32
                    width: 72
                    radius: theme.radiusSm
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        spacing: 0

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            radius: theme.radiusSm
                            color: root.viewMode === "grid" ? theme.surfaceHover : "transparent"
                            border.color: root.viewMode === "grid" ? theme.borderStrong : "transparent"
                            border.width: root.viewMode === "grid" ? 1 : 0

                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "layout-grid"
                                size: 14
                                color: root.viewMode === "grid" ? theme.accent : theme.textMuted
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.viewMode = "grid"
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            radius: theme.radiusSm
                            color: root.viewMode === "list" ? theme.surfaceHover : "transparent"
                            border.color: root.viewMode === "list" ? theme.borderStrong : "transparent"
                            border.width: root.viewMode === "list" ? 1 : 0

                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "list"
                                size: 14
                                color: root.viewMode === "list" ? theme.accent : theme.textMuted
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.viewMode = "list"
                            }
                        }
                    }
                }
            }

            // ── B. SUMMARY METRICS ROW (Installed / Updates / Disk Usage / Recent) ──
            GridLayout {
                Layout.fillWidth: true
                columns: root.width < 700 ? 2 : 4
                rowSpacing: 10
                columnSpacing: 10

                // Card 1: Installed Applications
                Rectangle {
                    Layout.fillWidth: true
                    height: 82
                    radius: theme.radiusMd
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12

                        Rectangle {
                            width: 44
                            height: 44
                            radius: 10
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1
                            RyzoraIcon { anchors.centerIn: parent; name: "package"; size: 20; color: theme.accent }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Text {
                                text: String(root.librarySummary.installed_count || 0)
                                font.pixelSize: 20
                                font.bold: true
                                color: theme.text
                            }
                            Text {
                                text: "Installed applications"
                                font.pixelSize: 11
                                color: theme.textMuted
                            }
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.selectedTypeFilter = "all"
                        }
                    }
                }

                // Card 2: Available Updates
                Rectangle {
                    Layout.fillWidth: true
                    height: 82
                    radius: theme.radiusMd
                    color: (root.librarySummary.updates_count > 0 && root.selectedTypeFilter === "updates")
                        ? theme.surfaceHover
                        : theme.surfaceElevated
                    border.color: (root.librarySummary.updates_count > 0) ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12

                        Rectangle {
                            width: 44
                            height: 44
                            radius: 10
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1
                            RyzoraIcon { anchors.centerIn: parent; name: "arrow-up"; size: 20; color: (root.librarySummary.updates_count > 0) ? "#f59e0b" : theme.textMuted }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Text {
                                text: String(root.librarySummary.updates_count || 0)
                                font.pixelSize: 20
                                font.bold: true
                                color: (root.librarySummary.updates_count > 0) ? "#f59e0b" : theme.text
                            }
                            Text {
                                text: "Available updates"
                                font.pixelSize: 11
                                color: theme.textMuted
                            }
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.selectedTypeFilter = (root.selectedTypeFilter === "updates") ? "all" : "updates"
                        }
                    }
                }

                // Card 3: Total Disk Footprint
                Rectangle {
                    Layout.fillWidth: true
                    height: 82
                    radius: theme.radiusMd
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12

                        Rectangle {
                            width: 44
                            height: 44
                            radius: 10
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1
                            RyzoraIcon { anchors.centerIn: parent; name: "hard-drive"; size: 20; color: "#10b981" }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Text {
                                text: root.librarySummary.total_installed_size_label || "—"
                                font.pixelSize: 18
                                font.bold: true
                                color: theme.text
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                            Text {
                                text: "Known disk usage"
                                font.pixelSize: 11
                                color: theme.textMuted
                            }
                        }
                    }
                }

                // Card 4: Recently Added
                Rectangle {
                    Layout.fillWidth: true
                    height: 82
                    radius: theme.radiusMd
                    color: root.selectedTypeFilter === "recent" ? theme.surfaceHover : theme.surfaceElevated
                    border.color: root.selectedTypeFilter === "recent" ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12

                        Rectangle {
                            width: 44
                            height: 44
                            radius: 10
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1
                            RyzoraIcon { anchors.centerIn: parent; name: "clock"; size: 20; color: "#8b5cf6" }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Text {
                                text: String(root.librarySummary.recent_count || 0)
                                font.pixelSize: 20
                                font.bold: true
                                color: theme.text
                            }
                            Text {
                                text: "Recently installed"
                                font.pixelSize: 11
                                color: theme.textMuted
                            }
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.selectedTypeFilter = (root.selectedTypeFilter === "recent") ? "all" : "recent"
                        }
                    }
                }
            }

            // ── C. SEARCH & SORT TOOLBAR ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                // Search Input Box
                Rectangle {
                    Layout.fillWidth: true
                    height: 38
                    radius: theme.radiusSm
                    color: theme.surfaceElevated
                    border.color: searchInp.activeFocus ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 8

                        RyzoraIcon {
                            name: "search"
                            size: 14
                            color: searchInp.activeFocus ? theme.accent : theme.textMuted
                        }

                        TextInput {
                            id: searchInp
                            Layout.fillWidth: true
                            text: root.searchQuery
                            color: theme.text
                            font.pixelSize: 13
                            selectByMouse: true
                            clip: true

                            Text {
                                text: "Search installed applications…"
                                color: theme.textMuted
                                font.pixelSize: 13
                                visible: searchInp.text.length === 0 && !searchInp.activeFocus
                                anchors.fill: parent
                                verticalAlignment: Text.AlignVCenter
                            }

                            onTextChanged: {
                                if (root.searchQuery !== text) {
                                    root.searchQuery = text
                                }
                            }
                        }

                        Rectangle {
                            width: 18
                            height: 18
                            radius: 9
                            color: theme.surfaceHover
                            visible: searchInp.text.length > 0
                            RyzoraIcon { anchors.centerIn: parent; name: "x"; size: 10; color: theme.textMuted }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: searchInp.text = ""
                            }
                        }
                    }
                }

                // Sort Dropdown Button
                Rectangle {
                    height: 38
                    implicitWidth: sortRow.implicitWidth + 24
                    radius: theme.radiusSm
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: sortRow
                        anchors.centerIn: parent
                        spacing: 6

                        RyzoraIcon { name: "filter"; size: 12; color: theme.textMuted }

                        Text {
                            text: "Sort: " + (root.selectedSort === "name" ? "Name" : (root.selectedSort === "size" ? "Disk Size" : "Recently Added"))
                            font.pixelSize: 12
                            font.weight: Font.Medium
                            color: theme.textSecondary
                        }

                        RyzoraIcon { name: "chevron-down"; size: 10; color: theme.textMuted }
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (root.selectedSort === "name") root.selectedSort = "size"
                            else if (root.selectedSort === "size") root.selectedSort = "recent"
                            else root.selectedSort = "name"
                        }
                    }
                }
            }

            // ── D. FILTER CHIPS (Providers & Types) ──
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8

                // Provider Filters
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: "Provider:"
                        font.pixelSize: 11
                        font.bold: true
                        color: theme.textMuted
                        Layout.alignment: Qt.AlignVCenter
                    }

                    Repeater {
                        model: [
                            { id: "all", label: "All Providers" },
                            { id: "pacman", label: "Official Repositories" },
                            { id: "aur", label: "AUR" },
                            { id: "flatpak", label: "Flatpak" }
                        ]

                        delegate: Rectangle {
                            readonly property bool isSel: root.selectedProvider === modelData.id
                            height: 28
                            implicitWidth: provText.implicitWidth + 20
                            radius: 14
                            color: isSel ? theme.accent : theme.surfaceElevated
                            border.color: isSel ? theme.accent : theme.borderSubtle
                            border.width: 1

                            Text {
                                id: provText
                                anchors.centerIn: parent
                                text: modelData.label
                                font.pixelSize: 11
                                font.weight: isSel ? Font.DemiBold : Font.Normal
                                color: isSel ? "#ffffff" : theme.textSecondary
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.selectedProvider = modelData.id
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }
                }

                // Type / Category Filters
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: "Type:"
                        font.pixelSize: 11
                        font.bold: true
                        color: theme.textMuted
                        Layout.alignment: Qt.AlignVCenter
                    }

                    Repeater {
                        model: [
                            { id: "all", label: "All Installed" },
                            { id: "recent", label: "Recently Added" },
                            { id: "updates", label: "Updates Available" },
                            { id: "packages", label: "Other Installed Packages" }
                        ]

                        delegate: Rectangle {
                            readonly property bool isSel: root.selectedTypeFilter === modelData.id
                            height: 28
                            implicitWidth: typeText.implicitWidth + 20
                            radius: 14
                            color: isSel ? theme.surfaceHover : "transparent"
                            border.color: isSel ? theme.borderStrong : theme.borderSubtle
                            border.width: 1

                            Text {
                                id: typeText
                                anchors.centerIn: parent
                                text: modelData.label
                                font.pixelSize: 11
                                font.weight: isSel ? Font.DemiBold : Font.Normal
                                color: isSel ? theme.text : theme.textMuted
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.selectedTypeFilter = modelData.id
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: root.installedList.length + " matching"
                        font.pixelSize: 11
                        color: theme.textMuted
                        Layout.alignment: Qt.AlignVCenter
                    }
                }
            }

            // ── E1. GRID VIEW PRESENTATION ──
            GridLayout {
                id: appsGrid
                Layout.fillWidth: true
                visible: root.viewMode === "grid" && root.installedList.length > 0
                columns: root.responsiveColumns
                rowSpacing: 12
                columnSpacing: 12

                Repeater {
                    model: root.installedList

                    delegate: Rectangle {
                        id: cardItem
                        Layout.fillWidth: true
                        Layout.preferredWidth: 220
                        Layout.minimumWidth: 200
                        implicitWidth: 220
                        Layout.preferredHeight: 216
                        implicitHeight: 216
                        height: 216
                        radius: theme.cardRadius
                        color: cardMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: cardItem.activeFocus ? theme.accent : (cardMouse.containsMouse ? theme.cardBorderHover : theme.cardBorder)
                        border.width: cardItem.activeFocus ? 2 : 1
                        activeFocusOnTab: true
                        clip: true

                        Behavior on color { ColorAnimation { duration: 120 } }
                        Behavior on border.color { ColorAnimation { duration: 120 } }

                        Keys.onReturnPressed: root.openAppDetails(modelData)
                        Keys.onSpacePressed: root.openAppDetails(modelData)

                        MouseArea {
                            id: cardMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.openAppDetails(modelData)
                        }

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 6

                            // Top Header: App Icon + Action Pill + More Menu
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
                                        source: modelData.icon_url || ""
                                        fillMode: Image.PreserveAspectFit
                                        smooth: true
                                        visible: status === Image.Ready
                                    }

                                    RyzoraIcon {
                                        anchors.centerIn: parent
                                        visible: !modelData.icon_url || modelData.icon_url === ""
                                        name: root.getCategoryIcon(modelData.category)
                                        size: 24
                                        color: theme.textMuted
                                    }
                                }

                                Item { Layout.fillWidth: true }

                                // Open / Launch Action Button
                                RyzoraButton {
                                    size: "sm"
                                    variant: "secondary"
                                    text: "Open"
                                    iconName: "play"
                                    iconFilled: true
                                    onClicked: {
                                        root.launchRequested(modelData.id)
                                        if (backend) backend.launchApp(modelData.id)
                                    }
                                }

                                // More Actions Button (···)
                                RyzoraButton {
                                    size: "sm"
                                    variant: "icon-only"
                                    iconName: "more-vertical"
                                    iconSize: 14
                                    tooltip: "More actions"
                                    onClicked: cardMenu.open()

                                    Menu {
                                        id: cardMenu
                                        MenuItem {
                                            text: "View details"
                                            onTriggered: root.openAppDetails(modelData)
                                        }
                                        MenuItem {
                                            text: "Uninstall"
                                            onTriggered: root.promptUninstall(modelData)
                                        }
                                    }
                                }
                            }

                            // Application Name
                            Text {
                                text: modelData.display_name || modelData.name || ""
                                font.family: theme.fontFamily
                                font.pixelSize: 14
                                font.weight: theme.weightSemibold
                                color: theme.text
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            // Category & Provenance Badge
                            Text {
                                text: (modelData.category || "Application") + " · " + root.getProviderBadge(modelData)
                                font.family: theme.fontFamily
                                font.pixelSize: 11
                                font.weight: theme.weightRegular
                                color: theme.textMuted
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            // Description (2 lines max)
                            Text {
                                text: modelData.summary || modelData.description || ""
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

                            // Bottom Status & Size Metadata Row
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 6

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 5

                                    Rectangle {
                                        width: 6
                                        height: 6
                                        radius: 3
                                        color: modelData.has_update ? "#f59e0b" : "#10b981"
                                        Layout.alignment: Qt.AlignVCenter
                                        Layout.fillWidth: false
                                    }

                                    Text {
                                        text: modelData.has_update
                                            ? ("Update: v" + (modelData.update_version || "new"))
                                            : "Installed"
                                        font.pixelSize: 11
                                        font.weight: Font.DemiBold
                                        color: modelData.has_update ? "#f59e0b" : "#10b981"
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                        Layout.alignment: Qt.AlignVCenter
                                    }
                                }

                                Text {
                                    readonly property var sz: (modelData.installed_size || modelData.download_size || 0)
                                    visible: sz > 0
                                    text: root.formatBytes(sz)
                                    font.pixelSize: 10
                                    font.family: "monospace"
                                    font.weight: Font.Medium
                                    color: theme.textMuted
                                    Layout.fillWidth: false
                                    Layout.preferredWidth: implicitWidth
                                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                                }
                            }
                        }
                    }
                }
            }

            // ── E2. LIST VIEW PRESENTATION (Dense table rows) ──
            ColumnLayout {
                Layout.fillWidth: true
                visible: root.viewMode === "list" && root.installedList.length > 0
                spacing: 4

                // Table Header
                Rectangle {
                    Layout.fillWidth: true
                    height: 32
                    color: "transparent"

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 12

                        Text { text: "Application"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; Layout.preferredWidth: 260 }
                        Text { text: "Category"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; Layout.preferredWidth: 140 }
                        Text { text: "Origin"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; Layout.preferredWidth: 100 }
                        Text { text: "Version"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; Layout.preferredWidth: 120 }
                        Text { text: "Size"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; Layout.preferredWidth: 90 }
                        Item { Layout.fillWidth: true }
                        Text { text: "Actions"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; Layout.preferredWidth: 110; horizontalAlignment: Text.AlignRight }
                    }
                }

                Repeater {
                    model: root.installedList

                    delegate: Rectangle {
                        Layout.fillWidth: true
                        height: 52
                        radius: theme.radiusSm
                        color: rowMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: rowMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
                        border.width: 1

                        MouseArea {
                            id: rowMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.openAppDetails(modelData)
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 12
                            anchors.rightMargin: 12
                            spacing: 12

                            // Icon (40px)
                            Rectangle {
                                width: 36
                                height: 36
                                radius: 8
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1
                                Layout.fillWidth: false

                                Image {
                                    anchors.centerIn: parent
                                    width: 26
                                    height: 26
                                    source: modelData.icon_url || ""
                                    fillMode: Image.PreserveAspectFit
                                    smooth: true
                                    visible: status === Image.Ready
                                }

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    visible: !modelData.icon_url || modelData.icon_url === ""
                                    name: root.getCategoryIcon(modelData.category)
                                    size: 18
                                    color: theme.textMuted
                                }
                            }

                            // Application Name
                            ColumnLayout {
                                Layout.preferredWidth: 210
                                spacing: 1
                                Text {
                                    text: modelData.display_name || modelData.name || ""
                                    font.pixelSize: 13
                                    font.bold: true
                                    color: theme.text
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                                Text {
                                    text: modelData.summary || modelData.description || ""
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }

                            // Category
                            Text {
                                text: modelData.category || "Application"
                                font.pixelSize: 12
                                color: theme.textSecondary
                                Layout.preferredWidth: 140
                                elide: Text.ElideRight
                            }

                            // Origin / Provider Badge
                            Rectangle {
                                height: 22
                                implicitWidth: provPill.implicitWidth + 14
                                radius: 11
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1
                                Layout.preferredWidth: implicitWidth

                                Text {
                                    id: provPill
                                    anchors.centerIn: parent
                                    text: root.getProviderBadge(modelData)
                                    font.pixelSize: 10
                                    font.weight: Font.Medium
                                    color: theme.textSecondary
                                }
                            }

                            // Version
                            Text {
                                text: "v" + (modelData.version || "1.0")
                                font.pixelSize: 12
                                font.family: "monospace"
                                color: theme.textSecondary
                                Layout.preferredWidth: 120
                                elide: Text.ElideRight
                            }

                            // Size
                            Text {
                                readonly property var sz: (modelData.installed_size || modelData.download_size || 0)
                                text: root.formatBytes(sz)
                                font.pixelSize: 11
                                font.family: "monospace"
                                color: theme.textMuted
                                Layout.preferredWidth: 90
                            }

                            Item { Layout.fillWidth: true }

                            // Action: Open + Menu
                            RowLayout {
                                spacing: 6
                                Layout.preferredWidth: 110
                                Layout.alignment: Qt.AlignRight

                                Rectangle {
                                    height: 28
                                    implicitWidth: 68
                                    radius: theme.radiusSm
                                    color: listOpenMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                    border.color: theme.borderStrong
                                    border.width: 1

                                    Row {
                                        anchors.centerIn: parent
                                        spacing: 5
                                        RyzoraIcon { name: "play"; size: 10; color: "#10b981" }
                                        Text { text: "Open"; font.pixelSize: 11; font.bold: true; color: theme.text }
                                    }

                                    MouseArea {
                                        id: listOpenMouse
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            root.launchRequested(modelData.id)
                                            if (backend) backend.launchApp(modelData.id)
                                        }
                                    }
                                }

                                Rectangle {
                                    width: 28
                                    height: 28
                                    radius: theme.radiusSm
                                    color: listMoreMouse.containsMouse ? theme.surfaceHover : "transparent"

                                    RyzoraIcon { anchors.centerIn: parent; name: "more-vertical"; size: 13; color: theme.textMuted }

                                    MouseArea {
                                        id: listMoreMouse
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: listCardMenu.open()
                                    }

                                    Menu {
                                        id: listCardMenu
                                        MenuItem {
                                            text: "View details"
                                            onTriggered: root.openAppDetails(modelData)
                                        }
                                        MenuItem {
                                            text: "Uninstall"
                                            onTriggered: root.promptUninstall(modelData)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── EMPTY STATE ──
            EmptyState {
                Layout.fillWidth: true
                Layout.preferredHeight: 240
                visible: root.installedList.length === 0
                iconName: "layout-grid"
                title: "No Installed Applications Found"
                message: "No applications match the current search query or provider filter."
                actionLabel: "Clear Filters"
                onActionClicked: {
                    root.searchQuery = ""
                    searchInp.text = ""
                    root.selectedProvider = "all"
                    root.selectedTypeFilter = "all"
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 2. CANONICAL APPLICATION DETAIL VIEW OVERLAY
    // ──────────────────────────────────────────────────────────────────────────
    Rectangle {
        id: detailOverlay
        anchors.fill: parent
        visible: root.selectedApp !== null
        color: theme.bg
        z: 20

        Flickable {
            id: detailScroll
            anchors.fill: parent
            contentWidth: width
            contentHeight: detailCol.implicitHeight + 60
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
                width: 6
            }

            ColumnLayout {
                id: detailCol
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.margins: root.width < 600 ? 16 : 24
                spacing: 18

                // Back Button Toolbar
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    Rectangle {
                        width: 36
                        height: 36
                        radius: 18
                        color: backMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        RyzoraIcon { anchors.centerIn: parent; name: "chevron-left"; size: 16; color: theme.text }

                        MouseArea {
                            id: backMouse
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.selectedApp = null
                        }
                    }

                    Text {
                        text: "Back to Library"
                        font.pixelSize: 14
                        font.weight: Font.Medium
                        color: theme.textSecondary
                    }

                    Item { Layout.fillWidth: true }
                }

                // Hero Details Card
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: heroCol.implicitHeight + 36
                    radius: theme.radiusLg
                    color: theme.surfaceElevated
                    border.color: theme.borderStrong
                    border.width: 1

                    ColumnLayout {
                        id: heroCol
                        anchors.fill: parent
                        anchors.margins: 18
                        spacing: 14

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16

                            // Big App Icon (64x64)
                            Rectangle {
                                width: 64
                                height: 64
                                radius: 14
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1
                                Layout.fillWidth: false

                                Image {
                                    anchors.centerIn: parent
                                    width: 48
                                    height: 48
                                    source: root.selectedApp ? (root.selectedApp.icon_url || "") : ""
                                    fillMode: Image.PreserveAspectFit
                                    smooth: true
                                    visible: status === Image.Ready
                                }

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    visible: !root.selectedApp || !root.selectedApp.icon_url || root.selectedApp.icon_url === ""
                                    name: root.selectedApp ? root.getCategoryIcon(root.selectedApp.category) : "layout-grid"
                                    size: 32
                                    color: theme.textMuted
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 4

                                Text {
                                    text: root.selectedApp ? (root.selectedApp.display_name || root.selectedApp.name || "") : ""
                                    font.pixelSize: 20
                                    font.bold: true
                                    color: theme.text
                                }

                                Text {
                                    text: root.selectedApp ? ((root.selectedApp.publisher ? root.selectedApp.publisher + " · " : "") + (root.selectedApp.category || "Application")) : ""
                                    font.pixelSize: 12
                                    color: theme.textMuted
                                }

                                RowLayout {
                                    spacing: 8
                                    Rectangle {
                                        height: 22
                                        implicitWidth: detailProvBadge.implicitWidth + 14
                                        radius: 11
                                        color: theme.surface
                                        border.color: theme.borderSubtle
                                        border.width: 1

                                        Text {
                                            id: detailProvBadge
                                            anchors.centerIn: parent
                                            text: root.selectedApp ? root.getProviderBadge(root.selectedApp) : "Official"
                                            font.pixelSize: 10
                                            font.bold: true
                                            color: theme.accent
                                        }
                                    }

                                    Text {
                                        text: "v" + (root.selectedApp ? (root.selectedApp.version || "1.0") : "1.0")
                                        font.pixelSize: 11
                                        font.family: "monospace"
                                        color: theme.textMuted
                                    }
                                }
                            }

                            Item { Layout.fillWidth: true }

                            // Action Buttons: Open + Uninstall
                            RowLayout {
                                spacing: 10

                                Rectangle {
                                    height: 36
                                    implicitWidth: 90
                                    radius: theme.radiusSm
                                    color: theme.accent
                                    border.color: theme.accentHover
                                    border.width: 1

                                    Row {
                                        anchors.centerIn: parent
                                        spacing: 6
                                        RyzoraIcon { name: "play"; size: 12; color: "#ffffff" }
                                        Text { text: "Open"; font.pixelSize: 12; font.bold: true; color: "#ffffff" }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            if (root.selectedApp) {
                                                root.launchRequested(root.selectedApp.id)
                                                if (backend) backend.launchApp(root.selectedApp.id)
                                            }
                                        }
                                    }
                                }

                                Rectangle {
                                    height: 36
                                    implicitWidth: 100
                                    radius: theme.radiusSm
                                    color: theme.surfaceHover
                                    border.color: theme.borderStrong
                                    border.width: 1

                                    Row {
                                        anchors.centerIn: parent
                                        spacing: 6
                                        RyzoraIcon { name: "trash"; size: 12; color: "#ef4444" }
                                        Text { text: "Uninstall"; font.pixelSize: 12; font.bold: true; color: "#ef4444" }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            if (root.selectedApp) {
                                                root.promptUninstall(root.selectedApp)
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Summary description
                        Text {
                            text: root.selectedApp ? (root.selectedApp.description || root.selectedApp.summary || "") : ""
                            font.pixelSize: 13
                            lineHeight: 1.3
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }
                }

                // Technical Metadata Card
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: metaCol.implicitHeight + 36
                    radius: theme.radiusLg
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    ColumnLayout {
                        id: metaCol
                        anchors.fill: parent
                        anchors.margins: 18
                        spacing: 12

                        Text {
                            text: "Package Details & Provenance"
                            font.pixelSize: 15
                            font.bold: true
                            color: theme.text
                        }

                        GridLayout {
                            Layout.fillWidth: true
                            columns: root.width < 700 ? 1 : 3
                            rowSpacing: 10
                            columnSpacing: 16

                            ColumnLayout {
                                spacing: 2
                                Text { text: "Package Identifier"; font.pixelSize: 11; color: theme.textMuted }
                                Text { text: root.selectedApp ? root.selectedApp.id : "—"; font.pixelSize: 13; font.family: "monospace"; color: theme.text }
                            }

                            ColumnLayout {
                                spacing: 2
                                Text { text: "Installed Footprint"; font.pixelSize: 11; color: theme.textMuted }
                                Text {
                                    readonly property var sz: (root.selectedApp ? (root.selectedApp.installed_size || root.selectedApp.download_size) : 0)
                                    text: root.formatBytes(sz)
                                    font.pixelSize: 13
                                    font.family: "monospace"
                                    color: theme.text
                                }
                            }

                            ColumnLayout {
                                spacing: 2
                                Text { text: "License"; font.pixelSize: 11; color: theme.textMuted }
                                Text { text: root.selectedApp && root.selectedApp.license ? root.selectedApp.license : "Open Source"; font.pixelSize: 13; color: theme.text }
                            }
                        }
                    }
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 3. PROVIDER-AWARE UNINSTALL CONFIRMATION MODAL
    // ──────────────────────────────────────────────────────────────────────────
    UninstallConfirmationDialog {
        id: uninstallDialog
        onConfirmed: function(req) {
            if (root.pendingUninstallApp) {
                root.uninstallRequested(root.pendingUninstallApp.id)
            }
            root.pendingUninstallApp = null
        }
        onCancelled: {
            root.pendingUninstallApp = null
        }
    }
}
