import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    signal appSelected(var appData)

    property string selectedFilter: "all"
    property string searchQuery: ""
    property var expandedDetails: ({})
    property int updateRev: 0

    Connections {
        target: typeof backend !== "undefined" ? backend : null
        function onAppInstallProgress(appId, stage, pct, isDet, msg, dl, tot, spd, curStep, totSteps, curItem, eta, diag) {
            root.updateRev++
        }
        function onActiveAppOperationsChanged() {
            root.updateRev++
        }
        function onUpdatesChanged() {
            root.updateRev++
        }
    }

    function isExpanded(pkgId) {
        return Boolean(expandedDetails[pkgId])
    }

    function toggleExpanded(pkgId) {
        var copy = Object.assign({}, expandedDetails)
        copy[pkgId] = !copy[pkgId]
        expandedDetails = copy
    }

    function filterMatches(item) {
        if (!item) return false
        var pType = (item.package_type || "").toLowerCase()
        var prov = (item.provider || "").toLowerCase()

        if (selectedFilter === "apps" && pType !== "app") return false
        if (selectedFilter === "flatpak" && prov !== "flatpak") return false
        if (selectedFilter === "system" && pType !== "system" && pType !== "runtime") return false
        if (selectedFilter === "themes" && pType !== "theme") return false

        if (searchQuery.trim().length > 0) {
            var q = searchQuery.trim().toLowerCase()
            var name = (item.name || "").toLowerCase()
            var pkgId = (item.package_id || "").toLowerCase()
            return name.indexOf(q) !== -1 || pkgId.indexOf(q) !== -1
        }
        return true
    }

    function filteredUpdates() {
        if (!backend || !backend.updatesList) return []
        var res = []
        for (var i = 0; i < backend.updatesList.length; ++i) {
            var it = backend.updatesList[i]
            if (filterMatches(it)) {
                res.push(it)
            }
        }
        return res
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16

        // ── 1. Header Row ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                spacing: 4
                Layout.fillWidth: true

                RowLayout {
                    spacing: 10
                    Text {
                        text: "Updates"
                        font.pixelSize: 24
                        font.bold: true
                        color: theme.text
                    }

                    Rectangle {
                        visible: backend ? backend.updatesCount > 0 : false
                        height: 22
                        width: badgeTxt.implicitWidth + 14
                        radius: 11
                        color: theme.accent
                        Text {
                            id: badgeTxt
                            anchors.centerIn: parent
                            text: (backend ? backend.updatesCount : 0).toString()
                            font.pixelSize: 11
                            font.bold: true
                            color: "#FFFFFF"
                        }
                    }
                }

                Text {
                    text: "Synchronized with Flatpak, official repositories, AUR, and personalization presets"
                    font.pixelSize: theme.fontCardDesc
                    color: theme.textMuted
                }
            }

            // Header Actions
            RowLayout {
                spacing: 10

                // Last Checked Label
                Text {
                    text: backend && backend.lastUpdatesCheckedTime
                        ? "Last checked: " + backend.lastUpdatesCheckedTime
                        : "Last checked: Today"
                    font.pixelSize: 12
                    color: theme.textMuted
                    Layout.alignment: Qt.AlignVCenter
                }

                // Check for updates
                RyzoraButton {
                    variant: "secondary"
                    iconName: "arrow-sync"
                    text: "Check for updates"
                    loading: Boolean(backend && backend.isUpdatesLoading)
                    loadingText: "Checking…"
                    onClicked: {
                        if (backend) backend.checkForUpdates()
                    }
                }

                // Update All Button
                RyzoraButton {
                    variant: "primary"
                    iconName: "arrow-download"
                    iconFilled: true
                    text: (!backend || backend.updatesCount === 0) ? "Up to date" : "Update all (" + backend.updatesCount + ")"
                    enabled: Boolean(backend && backend.updatesCount > 0 && backend.activeAppOperationsCount === 0)
                    onClicked: {
                        if (backend) backend.applyAllUpdates()
                    }
                }
            }
        }

        // ── 2. Filter Bar & Search ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            // Filter Chips
            RowLayout {
                spacing: 6

                Repeater {
                    model: [
                        { id: "all", label: "All" },
                        { id: "apps", label: "Apps" },
                        { id: "flatpak", label: "Flatpak" },
                        { id: "system", label: "System" },
                        { id: "themes", label: "Presets" }
                    ]

                    Rectangle {
                        implicitWidth: chipTxt.implicitWidth + 20
                        height: 30
                        radius: 15
                        color: root.selectedFilter === modelData.id
                            ? (theme.resolvedTheme === "light" ? "#DBEAFE" : "#1E3A8A")
                            : (chipMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                        border.color: root.selectedFilter === modelData.id ? theme.accent : theme.borderSubtle
                        border.width: 1

                        Text {
                            id: chipTxt
                            anchors.centerIn: parent
                            text: modelData.label
                            font.pixelSize: 12
                            font.weight: root.selectedFilter === modelData.id ? Font.DemiBold : Font.Normal
                            color: root.selectedFilter === modelData.id ? theme.accent : theme.textSecondary
                        }

                        MouseArea {
                            id: chipMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.selectedFilter = modelData.id
                        }
                    }
                }
            }

            Item { Layout.fillWidth: true }

            // Search Filter Input
            Rectangle {
                width: 220
                height: 32
                radius: 16
                color: theme.surfaceElevated
                border.color: searchInput.activeFocus ? theme.accent : theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    anchors.rightMargin: 10
                    spacing: 6

                    RyzoraIcon {
                        name: "search"
                        size: 13
                        color: theme.textMuted
                    }

                    TextInput {
                        id: searchInput
                        Layout.fillWidth: true
                        font.pixelSize: 12
                        color: theme.text
                        selectByMouse: true
                        Text {
                            text: "Filter updates…"
                            font.pixelSize: 12
                            color: theme.textMuted
                            visible: !searchInput.text && !searchInput.activeFocus
                        }
                        onTextChanged: root.searchQuery = text
                    }

                    RyzoraIcon {
                        name: "x"
                        size: 12
                        color: theme.textMuted
                        visible: Boolean(searchInput.text.length > 0)
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                searchInput.text = ""
                                root.searchQuery = ""
                            }
                        }
                    }
                }
            }
        }

        // ── 3. Content Area ──
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.border
            border.width: 1

            // State A: Checking / Loading
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 12
                visible: Boolean(backend && backend.isUpdatesLoading && backend.updatesCount === 0)

                RyzoraIcon {
                    name: "spinner"
                    size: 32
                    color: theme.accent
                    Layout.alignment: Qt.AlignHCenter
                    RotationAnimation on rotation {
                        from: 0
                        to: 360
                        duration: 1000
                        loops: Animation.Infinite
                        running: backend ? backend.isUpdatesLoading : false
                    }
                }

                Text {
                    text: "Checking for updates across Flatpak, Arch repositories, and AUR…"
                    font.pixelSize: 14
                    color: theme.textMuted
                    Layout.alignment: Qt.AlignHCenter
                }
            }

            // State B: Up to Date / Empty
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 12
                visible: Boolean(backend && !backend.isUpdatesLoading && backend.updatesCount === 0)

                Rectangle {
                    width: 56
                    height: 56
                    radius: 28
                    color: Qt.rgba(16/255, 185/255, 129/255, 0.12)
                    border.color: Qt.rgba(16/255, 185/255, 129/255, 0.3)
                    Layout.alignment: Qt.AlignHCenter

                    RyzoraIcon {
                        anchors.centerIn: parent
                        name: "check"
                        size: 26
                        color: "#10B981"
                    }
                }

                Text {
                    text: "You're up to date"
                    font.family: theme.fontFamily
                    font.pixelSize: 18
                    font.weight: theme.weightSemibold
                    color: theme.text
                    Layout.alignment: Qt.AlignHCenter
                }

                Text {
                    text: "No updates detected for installed packages or presets."
                    font.family: theme.fontFamily
                    font.pixelSize: 13
                    font.weight: theme.weightRegular
                    color: theme.textMuted
                    Layout.alignment: Qt.AlignHCenter
                }

                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    implicitWidth: checkEmptyTxt.implicitWidth + 24
                    height: 32
                    radius: theme.radiusSm
                    color: checkEmptyMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                    border.color: theme.border
                    border.width: 1

                    Text {
                        id: checkEmptyTxt
                        anchors.centerIn: parent
                        text: "Check again"
                        font.pixelSize: 12
                        font.weight: Font.Medium
                        color: theme.text
                    }

                    MouseArea {
                        id: checkEmptyMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (backend) backend.checkForUpdates()
                        }
                    }
                }
            }

            // State C: Available Updates List
            ListView {
                id: updateListView
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8
                clip: true
                visible: Boolean(backend && backend.updatesCount > 0)
                model: root.filteredUpdates()

                delegate: Rectangle {
                    id: updateRowDelegate
                    width: updateListView.width
                    implicitHeight: rowCol.implicitHeight + 20
                    radius: theme.cardRadius
                    color: itemMouseArea.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                    border.color: itemMouseArea.containsMouse ? theme.cardBorderHover : theme.cardBorder
                    border.width: 1

                    Behavior on color { ColorAnimation { duration: 120 } }
                    Behavior on border.color { ColorAnimation { duration: 120 } }

                    readonly property var activeTxn: {
                        var dummy = root.updateRev
                        if (!backend) return null
                        var t = backend.getTransactionForApp(modelData.package_id)
                        if ((!t || !t.op_id) && modelData.package_id.indexOf(':') !== -1) {
                            t = backend.getTransactionForApp(modelData.package_id.split(':')[1])
                        }
                        return t
                    }
                    readonly property bool isOperating: Boolean(backend && (backend.isAppOperating(modelData.package_id) || (modelData.package_id.indexOf(':') !== -1 && backend.isAppOperating(modelData.package_id.split(':')[1]))))

                    ColumnLayout {
                        id: rowCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 12
                        spacing: 8

                        // Main Row
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            // App Icon
                            Rectangle {
                                width: 40
                                height: 40
                                radius: 8
                                color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.10)
                                border.color: theme.borderSubtle
                                border.width: 1

                                Image {
                                    id: updIconImg
                                    anchors.centerIn: parent
                                    width: 28
                                    height: 28
                                    source: {
                                        var u = (modelData.icon_url || "").trim()
                                        if (!u) return ""
                                        if (u.indexOf(":/") !== -1 || u.indexOf("qrc:/") === 0 || u.indexOf("data:") === 0) return u
                                        if (u.indexOf("/") === 0) return "file://" + u
                                        return u
                                    }
                                    fillMode: Image.PreserveAspectFit
                                    visible: status === Image.Ready
                                }

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: {
                                        if (modelData.package_type === "runtime") return "cpu"
                                        if (modelData.package_type === "system") return "terminal"
                                        if (modelData.package_type === "theme") return "sliders-horizontal"
                                        return "package"
                                    }
                                    size: 20
                                    color: theme.accent
                                    visible: updIconImg.status !== Image.Ready
                                }
                            }

                            // Identity & Versions
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 3

                                RowLayout {
                                    spacing: 8

                                    Text {
                                        text: modelData.name || modelData.package_id
                                        font.pixelSize: 14
                                        font.bold: true
                                        color: nameMouse.containsMouse ? theme.accent : theme.text
                                        MouseArea {
                                            id: nameMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                if (backend) {
                                                    var app = backend.getAppById(modelData.package_id || modelData.name)
                                                    if (app && app.id) {
                                                        root.appSelected(app)
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Provider Pill
                                    Rectangle {
                                        height: 18
                                        implicitWidth: provTxt.implicitWidth + 10
                                        radius: 4
                                        color: {
                                            var p = (modelData.provider || "").toLowerCase()
                                            if (p === "flatpak") return Qt.rgba(59/255, 130/255, 246/255, 0.15)
                                            if (p === "aur") return Qt.rgba(245/255, 158/255, 11/255, 0.15)
                                            if (p === "pacman") return Qt.rgba(16/255, 185/255, 129/255, 0.15)
                                            return Qt.rgba(139/255, 92/255, 246/255, 0.15)
                                        }
                                        border.color: {
                                            var p = (modelData.provider || "").toLowerCase()
                                            if (p === "flatpak") return Qt.rgba(59/255, 130/255, 246/255, 0.3)
                                            if (p === "aur") return Qt.rgba(245/255, 158/255, 11/255, 0.3)
                                            if (p === "pacman") return Qt.rgba(16/255, 185/255, 129/255, 0.3)
                                            return Qt.rgba(139/255, 92/255, 246/255, 0.3)
                                        }
                                        Text {
                                            id: provTxt
                                            anchors.centerIn: parent
                                            text: (modelData.provider || "Package").toUpperCase()
                                            font.pixelSize: 9
                                            font.bold: true
                                            color: {
                                                var p = (modelData.provider || "").toLowerCase()
                                                if (p === "flatpak") return "#3B82F6"
                                                if (p === "aur") return "#F59E0B"
                                                if (p === "pacman") return "#10B981"
                                                return "#8B5CF6"
                                            }
                                        }
                                    }

                                    // Package Type Pill
                                    Rectangle {
                                        height: 18
                                        implicitWidth: typeTxt.implicitWidth + 10
                                        radius: 4
                                        color: Qt.rgba(100/255, 116/255, 139/255, 0.15)
                                        Text {
                                            id: typeTxt
                                            anchors.centerIn: parent
                                            text: (modelData.package_type || "Package").toUpperCase()
                                            font.pixelSize: 9
                                            font.bold: true
                                            color: theme.textSecondary
                                        }
                                    }
                                }

                                RowLayout {
                                    spacing: 8

                                    Text {
                                        text: "Version: " + modelData.installed_version + " → " + modelData.available_version
                                        font.pixelSize: 12
                                        font.weight: Font.Medium
                                        color: theme.accent
                                    }

                                    Text {
                                        visible: Boolean(modelData.download_size)
                                        text: "• " + (modelData.download_size || "")
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                    }

                                    Text {
                                        visible: Boolean(modelData.scope)
                                        text: "• " + (modelData.scope === "user" ? "User installation" : "System installation")
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                    }
                                }
                            }

                            // Expand Info Toggle
                            Rectangle {
                                width: 28
                                height: 28
                                radius: 14
                                color: expandMouse.containsMouse ? theme.surfaceHover : "transparent"
                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: root.isExpanded(modelData.package_id) ? "chevron-up" : "chevron-down"
                                    size: 13
                                    color: theme.textMuted
                                }
                                MouseArea {
                                    id: expandMouse
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.toggleExpanded(modelData.package_id)
                                }
                            }

                            // Update Action Button
                            RyzoraButton {
                                size: "sm"
                                variant: modelData.can_update_individually ? "primary" : "secondary"
                                loading: updateRowDelegate.isOperating
                                loadingText: "Updating…"
                                text: !modelData.can_update_individually ? "System Sync" : "Update"
                                iconName: "arrow-download"
                                iconFilled: true
                                onClicked: {
                                    if (modelData.can_update_individually) {
                                        if (backend) backend.applyUpdate(modelData.package_id)
                                    } else {
                                        if (backend) {
                                            backend.postNotification(
                                                "info",
                                                "System Upgrade Recommended",
                                                "Arch Linux packages share system libraries and should be updated together. Use 'Update All' for a coordinated system upgrade."
                                            )
                                        }
                                    }
                                }
                            }
                        }

                        // Live Progress Detail Text (Visible while actively updating)
                        RowLayout {
                            Layout.fillWidth: true
                            visible: updateRowDelegate.isOperating
                            spacing: 8

                            Text {
                                text: updateRowDelegate.activeTxn && updateRowDelegate.activeTxn.message ? updateRowDelegate.activeTxn.message : "Updating package..."
                                font.family: theme.fontFamily
                                font.pixelSize: 11
                                color: theme.accent
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Text {
                                visible: Boolean(updateRowDelegate.activeTxn && updateRowDelegate.activeTxn.bytes_downloaded && updateRowDelegate.activeTxn.bytes_total)
                                text: updateRowDelegate.activeTxn ? (updateRowDelegate.activeTxn.bytes_downloaded + " / " + updateRowDelegate.activeTxn.bytes_total) : ""
                                font.family: theme.fontMono
                                font.pixelSize: 10
                                color: theme.textMuted
                            }

                            Text {
                                visible: Boolean(updateRowDelegate.activeTxn && updateRowDelegate.activeTxn.download_speed)
                                text: (updateRowDelegate.activeTxn && updateRowDelegate.activeTxn.download_speed) ? String(updateRowDelegate.activeTxn.download_speed) : ""
                                font.family: theme.fontMono
                                font.pixelSize: 10
                                color: theme.textMuted
                            }

                            Text {
                                visible: Boolean(updateRowDelegate.activeTxn && updateRowDelegate.activeTxn.percentage >= 0)
                                text: updateRowDelegate.activeTxn ? (updateRowDelegate.activeTxn.percentage + "%") : ""
                                font.family: theme.fontFamily
                                font.pixelSize: 11
                                font.weight: theme.weightSemibold
                                color: theme.accent
                            }
                        }

                        // Progress Bar (Visible while this item is actively operating)
                        Rectangle {
                            Layout.fillWidth: true
                            height: 6
                            radius: 3
                            color: theme.borderSubtle
                            clip: true
                            visible: updateRowDelegate.isOperating

                            // Determinate Progress
                            Rectangle {
                                visible: Boolean(updateRowDelegate.activeTxn && updateRowDelegate.activeTxn.is_determinate && updateRowDelegate.activeTxn.percentage >= 0)
                                height: parent.height
                                radius: 3
                                color: theme.accent
                                width: {
                                    if (!parent.visible || !updateRowDelegate.activeTxn) return 0
                                    return parent.width * Math.max(0, Math.min(100, updateRowDelegate.activeTxn.percentage)) / 100
                                }
                                Behavior on width { NumberAnimation { duration: 150 } }
                            }

                            // Indeterminate Slide
                            Rectangle {
                                id: indetBar
                                visible: !Boolean(updateRowDelegate.activeTxn && updateRowDelegate.activeTxn.is_determinate && updateRowDelegate.activeTxn.percentage >= 0)
                                height: parent.height
                                width: Math.max(40, parent.width * 0.28)
                                radius: 3
                                color: theme.accent

                                SequentialAnimation on x {
                                    loops: Animation.Infinite
                                    running: indetBar.visible && updateRowDelegate.isOperating
                                    PropertyAnimation {
                                        from: -indetBar.width
                                        to: parent.width
                                        duration: 1000
                                        easing.type: Easing.InOutQuad
                                    }
                                }
                            }
                        }

                        // Expandable Details (Accordion)
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: detailsCol.implicitHeight + 12
                            radius: 6
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1
                            visible: root.isExpanded(modelData.package_id)

                            ColumnLayout {
                                id: detailsCol
                                anchors.fill: parent
                                anchors.margins: 10
                                spacing: 4

                                Text {
                                    text: modelData.description || "Package update available from repository."
                                    font.pixelSize: 12
                                    color: theme.textSecondary
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }

                                RowLayout {
                                    spacing: 12
                                    Text {
                                        text: "Source: " + (modelData.source || "upstream")
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                    }
                                    Text {
                                        text: "Scope: " + (modelData.scope || "system")
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                    }
                                    Text {
                                        text: "Package ID: " + modelData.package_id
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                    }
                                }

                                Text {
                                    visible: Boolean(modelData.safety_warning)
                                    text: "⚠️ " + (modelData.safety_warning || "")
                                    font.pixelSize: 11
                                    color: "#F59E0B"
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }

                    MouseArea {
                        id: itemMouseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        z: -1
                    }
                }

                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            }
        }
    }
}
