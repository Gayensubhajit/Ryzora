import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root
    objectName: "updatesView"

    signal appSelected(var appData)

    property string selectedFilter: "all"
    property string searchQuery: ""
    property var expandedDetails: ({})
    property int updateRev: 0
    property bool systemPackagesExpanded: false
    property string lastDiagnosticError: ""

    Connections {
        target: typeof backend !== "undefined" ? backend : null
        function onAppInstallProgress(appId, stage, pct, isDet, msg, dl, tot, spd, curStep, totSteps, curItem, eta, diag) {
            root.updateRev++
            if (stage === "failed" && diag) {
                root.lastDiagnosticError = String(diag)
            }
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

    function systemUpdatesList() {
        if (!backend || !backend.updatesList) return []
        var res = []
        for (var i = 0; i < backend.updatesList.length; ++i) {
            var it = backend.updatesList[i]
            var prov = (it.provider || "").toLowerCase()
            if (prov === "pacman") {
                if (searchQuery.trim().length > 0) {
                    var q = searchQuery.trim().toLowerCase()
                    if ((it.name || "").toLowerCase().indexOf(q) === -1 && (it.package_id || "").toLowerCase().indexOf(q) === -1) {
                        continue
                    }
                }
                res.push(it)
            }
        }
        return res
    }

    function appUpdatesList() {
        if (!backend || !backend.updatesList) return []
        var res = []
        for (var i = 0; i < backend.updatesList.length; ++i) {
            var it = backend.updatesList[i]
            var prov = (it.provider || "").toLowerCase()
            var pType = (it.package_type || "").toLowerCase()
            if (prov === "flatpak" || prov === "aur" || (prov !== "pacman" && pType === "app")) {
                if (selectedFilter === "flatpak" && prov !== "flatpak") continue
                if (selectedFilter === "aur" && prov !== "aur") continue
                if (searchQuery.trim().length > 0) {
                    var q = searchQuery.trim().toLowerCase()
                    if ((it.name || "").toLowerCase().indexOf(q) === -1 && (it.package_id || "").toLowerCase().indexOf(q) === -1) {
                        continue
                    }
                }
                res.push(it)
            }
        }
        return res
    }

    function communityUpdatesList() {
        if (!backend || !backend.updatesList) return []
        var res = []
        for (var i = 0; i < backend.updatesList.length; ++i) {
            var it = backend.updatesList[i]
            var prov = (it.provider || "").toLowerCase()
            var pType = (it.package_type || "").toLowerCase()
            if (prov !== "pacman" && prov !== "flatpak" && prov !== "aur" && pType !== "app") {
                if (searchQuery.trim().length > 0) {
                    var q = searchQuery.trim().toLowerCase()
                    if ((it.name || "").toLowerCase().indexOf(q) === -1 && (it.package_id || "").toLowerCase().indexOf(q) === -1) {
                        continue
                    }
                }
                res.push(it)
            }
        }
        return res
    }

    readonly property bool hasAvailableUpdates: Boolean(backend && (backend.updatesCount > 0 || backend.isAppOperating("all")))
    readonly property bool isNarrowHeader: root.width > 0 && root.width < 800

    readonly property bool isSystemOperating: Boolean(backend && (backend.isAppOperating("system") || backend.isAppOperating("all")))
    readonly property var systemTxn: {
        var dummy = root.updateRev
        if (!backend) return null
        return backend.getTransactionForApp("system") || backend.getTransactionForApp("all")
    }

    // ── Reusable Right Actions Group ──
    Component {
        id: rightActionsComponent

        RowLayout {
            spacing: 12
            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter

            Text {
                text: backend && backend.lastUpdatesCheckedTime
                    ? "Last checked: " + backend.lastUpdatesCheckedTime
                    : "Last checked: Today"
                font.family: theme.fontFamily
                font.pixelSize: 12
                color: theme.textMuted
                Layout.alignment: Qt.AlignVCenter
            }

            RyzoraButton {
                size: "md"
                variant: "secondary"
                iconName: "arrow-sync"
                text: "Check for updates"
                loading: Boolean(backend && backend.isUpdatesLoading)
                loadingText: "Checking…"
                enabled: Boolean(!backend || !backend.isUpdatesLoading)
                Layout.alignment: Qt.AlignVCenter
                onClicked: {
                    if (backend) backend.checkForUpdates()
                }
            }

            // Primary Update All button when updates are pending
            RyzoraButton {
                visible: root.hasAvailableUpdates
                size: "md"
                variant: "primary"
                iconName: "arrow-download"
                iconFilled: true
                text: "Update all (" + (backend ? backend.updatesCount : 0) + ")"
                enabled: Boolean(backend && backend.updatesCount > 0 && backend.activeAppOperationsCount === 0)
                loading: Boolean(backend && backend.isAppOperating("all"))
                loadingText: "Updating all…"
                Layout.alignment: Qt.AlignVCenter
                onClicked: {
                    if (backend) backend.applyAllUpdates()
                }
            }

            // Subtle Up to date status chip when system and apps are up to date
            Rectangle {
                visible: !root.hasAvailableUpdates
                height: 36
                implicitWidth: upToDateLayout.implicitWidth + 24
                radius: 8
                color: theme.resolvedTheme === "light" ? "#F0FDF4" : Qt.rgba(16/255, 185/255, 129/255, 0.15)
                border.color: theme.resolvedTheme === "light" ? "#BBF7D0" : Qt.rgba(16/255, 185/255, 129/255, 0.35)
                border.width: 1
                Layout.alignment: Qt.AlignVCenter

                RowLayout {
                    id: upToDateLayout
                    anchors.centerIn: parent
                    spacing: 6

                    RyzoraIcon {
                        name: "check"
                        size: 14
                        color: theme.resolvedTheme === "light" ? "#16A34A" : "#34D399"
                    }

                    Text {
                        text: "Up to date"
                        font.family: theme.fontFamily
                        font.pixelSize: 13
                        font.weight: theme.weightMedium
                        color: theme.resolvedTheme === "light" ? "#15803D" : "#34D399"
                    }
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16

        // ── 1. Top Header Row ──
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 12

            RowLayout {
                Layout.fillWidth: true
                spacing: 16

                // Left side: Page Identity
                ColumnLayout {
                    id: leftHeaderIdentity
                    spacing: 4
                    Layout.alignment: Qt.AlignLeft | Qt.AlignVCenter

                    RowLayout {
                        spacing: 10
                        Text {
                            text: "Updates"
                            font.family: theme.fontFamily
                            font.pixelSize: 24
                            font.weight: theme.weightBold
                            color: theme.text
                        }

                        Rectangle {
                            visible: Boolean(backend && backend.updatesCount > 0)
                            height: 22
                            implicitWidth: badgeTxt.implicitWidth + 14
                            radius: 11
                            color: theme.accent
                            Text {
                                id: badgeTxt
                                anchors.centerIn: parent
                                text: (backend ? backend.updatesCount : 0).toString() + " pending"
                                font.family: theme.fontFamily
                                font.pixelSize: 11
                                font.weight: theme.weightSemibold
                                color: "#FFFFFF"
                            }
                        }
                    }

                    Text {
                        text: "Manage application and system updates"
                        font.family: theme.fontFamily
                        font.pixelSize: theme.fontCardDesc
                        color: theme.textMuted
                    }
                }

                // Flexible spacer pushing actions to the far right on wide viewports
                Item {
                    visible: !root.isNarrowHeader
                    Layout.fillWidth: true
                }

                // Right side action group on wide viewports
                Loader {
                    visible: !root.isNarrowHeader
                    active: !root.isNarrowHeader
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    sourceComponent: rightActionsComponent
                }
            }

            // Second row for right action group when viewport is narrow
            RowLayout {
                visible: root.isNarrowHeader
                Layout.fillWidth: true

                Item {
                    Layout.fillWidth: true
                }

                Loader {
                    visible: root.isNarrowHeader
                    active: root.isNarrowHeader
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    sourceComponent: rightActionsComponent
                }
            }
        }

        // ── 2. Filter Bar & Search ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            RowLayout {
                spacing: 6

                Repeater {
                    model: [
                        { id: "all", label: "All (" + (backend ? backend.updatesCount : 0) + ")" },
                        { id: "system", label: "System (" + (backend ? backend.systemUpdatesCount : 0) + ")" },
                        { id: "apps", label: "Applications (" + (backend ? backend.applicationUpdatesCount : 0) + ")" },
                        { id: "flatpak", label: "Flatpak" },
                        { id: "aur", label: "AUR" },
                        { id: "themes", label: "Presets (" + (backend ? backend.communityUpdatesCount : 0) + ")" }
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

        // ── 3. Diagnostic Error Banner (When update encounters failure) ──
        Rectangle {
            visible: Boolean((backend && backend.updatesError && backend.updatesError.length > 0) || root.lastDiagnosticError.length > 0)
            Layout.fillWidth: true
            implicitHeight: errorLayout.implicitHeight + 20
            radius: theme.radiusSm
            color: theme.resolvedTheme === "light" ? "#FEF2F2" : "#451A1A"
            border.color: "#EF4444"
            border.width: 1

            ColumnLayout {
                id: errorLayout
                anchors.fill: parent
                anchors.margins: 12
                spacing: 6

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    RyzoraIcon {
                        name: "alert"
                        size: 16
                        color: "#EF4444"
                    }

                    Text {
                        text: "Update Operation Encountered an Issue"
                        font.pixelSize: 13
                        font.bold: true
                        color: theme.resolvedTheme === "light" ? "#991B1B" : "#FCA5A5"
                        Layout.fillWidth: true
                    }

                    RyzoraButton {
                        size: "sm"
                        variant: "secondary"
                        text: "Copy Diagnostics"
                        iconName: "copy"
                        onClicked: {
                            var diag = root.lastDiagnosticError || (backend ? backend.updatesError : "")
                            if (backend) {
                                backend.copyToClipboard(diag)
                                backend.postNotification("info", "Copied", "Diagnostic failure log copied to clipboard.")
                            }
                        }
                    }

                    RyzoraIcon {
                        name: "x"
                        size: 12
                        color: theme.textMuted
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.lastDiagnosticError = ""
                                if (backend) backend.updatesError = ""
                            }
                        }
                    }
                }

                Text {
                    text: root.lastDiagnosticError || (backend ? backend.updatesError : "")
                    font.family: "monospace"
                    font.pixelSize: 11
                    color: theme.resolvedTheme === "light" ? "#7F1D1D" : "#FECACA"
                    wrapMode: Text.WrapAnywhere
                    Layout.fillWidth: true
                }
            }
        }

        // ── 4. Main Scrollable Content Area ──
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.border
            border.width: 1
            clip: true

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

            // State C: Grouped Updates Sections
            Flickable {
                id: updatesFlickable
                anchors.fill: parent
                anchors.margins: 16
                contentWidth: width
                contentHeight: mainSectionsCol.implicitHeight + 24
                visible: Boolean(backend && backend.updatesCount > 0)
                boundsBehavior: Flickable.StopAtBounds
                clip: true

                ColumnLayout {
                    id: mainSectionsCol
                    width: updatesFlickable.width - 12
                    spacing: 20

                    // ══════════════════════════════════════════════════════════
                    // GROUP 1: Official Pacman Coordinated System Upgrade
                    // ══════════════════════════════════════════════════════════
                    Rectangle {
                        id: systemUpdatesCard
                        visible: root.systemUpdatesList().length > 0 && (root.selectedFilter === "all" || root.selectedFilter === "system")
                        Layout.fillWidth: true
                        implicitHeight: sysCardCol.implicitHeight + 24
                        radius: theme.cardRadius
                        color: theme.surfaceElevated
                        border.color: root.isSystemOperating ? theme.accent : theme.cardBorder
                        border.width: root.isSystemOperating ? 2 : 1

                        ColumnLayout {
                            id: sysCardCol
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 16
                            spacing: 12

                            // Header Row
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 14

                                Rectangle {
                                    width: 44
                                    height: 44
                                    radius: 10
                                    color: Qt.rgba(16/255, 185/255, 129/255, 0.15)
                                    border.color: Qt.rgba(16/255, 185/255, 129/255, 0.35)
                                    border.width: 1

                                    RyzoraIcon {
                                        anchors.centerIn: parent
                                        name: "shield-check"
                                        size: 22
                                        color: "#10B981"
                                    }
                                }

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 3

                                    RowLayout {
                                        spacing: 8
                                        Text {
                                            text: "System Updates"
                                            font.pixelSize: 15
                                            font.bold: true
                                            color: theme.text
                                        }

                                        Rectangle {
                                            height: 20
                                            implicitWidth: sysBadgeTxt.implicitWidth + 10
                                            radius: 4
                                            color: Qt.rgba(16/255, 185/255, 129/255, 0.15)
                                            border.color: Qt.rgba(16/255, 185/255, 129/255, 0.35)
                                            border.width: 1

                                            Text {
                                                id: sysBadgeTxt
                                                anchors.centerIn: parent
                                                text: root.systemUpdatesList().length + " packages · Pacman · Coordinated system upgrade"
                                                font.pixelSize: 10
                                                font.bold: true
                                                color: "#10B981"
                                            }
                                        }
                                    }

                                    Text {
                                        text: "Official repository updates. Applied together in one coordinated transaction to prevent partial upgrade library mismatches."
                                        font.pixelSize: 12
                                        color: theme.textMuted
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }
                                }

                                // Single Coordinated System Upgrade Action Button
                                RyzoraButton {
                                    size: "md"
                                    variant: "primary"
                                    text: "Update system"
                                    iconName: "arrow-download"
                                    iconFilled: true
                                    loading: root.isSystemOperating
                                    loadingText: "Upgrading system…"
                                    enabled: !root.isSystemOperating
                                    onClicked: {
                                        if (backend) backend.applySystemUpgrade()
                                    }
                                }
                            }

                            // Live Progress Bar (During System Upgrade)
                            ColumnLayout {
                                visible: root.isSystemOperating
                                Layout.fillWidth: true
                                spacing: 4

                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        text: root.systemTxn && root.systemTxn.message ? root.systemTxn.message : "Synchronizing system repositories…"
                                        font.pixelSize: 11
                                        color: theme.accent
                                        Layout.fillWidth: true
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        visible: Boolean(root.systemTxn && root.systemTxn.percentage >= 0)
                                        text: (root.systemTxn ? root.systemTxn.percentage : 0) + "%"
                                        font.pixelSize: 11
                                        font.bold: true
                                        color: theme.accent
                                    }
                                }

                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 4
                                    radius: 2
                                    color: theme.surfaceHover
                                    clip: true

                                    Rectangle {
                                        visible: Boolean(root.systemTxn && root.systemTxn.is_determinate && root.systemTxn.percentage >= 0)
                                        height: parent.height
                                        radius: 2
                                        color: theme.accent
                                        width: root.systemTxn ? parent.width * Math.max(0, Math.min(100, root.systemTxn.percentage)) / 100 : 0
                                    }

                                    Rectangle {
                                        id: sysIndetBar
                                        visible: !Boolean(root.systemTxn && root.systemTxn.is_determinate && root.systemTxn.percentage >= 0)
                                        height: parent.height
                                        width: Math.max(40, parent.width * 0.3)
                                        radius: 2
                                        color: theme.accent
                                        SequentialAnimation on x {
                                            loops: Animation.Infinite
                                            running: sysIndetBar.visible && root.isSystemOperating
                                            PropertyAnimation {
                                                from: -sysIndetBar.width
                                                to: sysIndetBar.parent ? sysIndetBar.parent.width : 400
                                                duration: 1000
                                                easing.type: Easing.InOutQuad
                                            }
                                        }
                                    }
                                }
                            }

                            // Expandable Package List Toggle
                            Rectangle {
                                Layout.fillWidth: true
                                height: 32
                                radius: 6
                                color: sysToggleMouse.containsMouse ? theme.surfaceHover : "transparent"

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 8
                                    anchors.rightMargin: 8
                                    spacing: 8

                                    RyzoraIcon {
                                        name: root.systemPackagesExpanded ? "chevron-up" : "chevron-down"
                                        size: 13
                                        color: theme.accent
                                    }

                                    Text {
                                        text: (root.systemPackagesExpanded ? "Hide" : "View") + " " + root.systemUpdatesList().length + " affected system packages"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        color: theme.accent
                                        Layout.fillWidth: true
                                    }
                                }

                                MouseArea {
                                    id: sysToggleMouse
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    hoverEnabled: true
                                    onClicked: root.systemPackagesExpanded = !root.systemPackagesExpanded
                                }
                            }

                            // Expandable Affected Packages Content (No individual buttons!)
                            ColumnLayout {
                                visible: root.systemPackagesExpanded
                                Layout.fillWidth: true
                                spacing: 4

                                Repeater {
                                    model: root.systemUpdatesList()
                                    delegate: Rectangle {
                                        Layout.fillWidth: true
                                        height: 38
                                        radius: 4
                                        color: sysRowMouse.containsMouse ? theme.surfaceHover : theme.surface
                                        border.color: theme.borderSubtle
                                        border.width: 1

                                        RowLayout {
                                            anchors.fill: parent
                                            anchors.leftMargin: 12
                                            anchors.rightMargin: 12
                                            spacing: 12

                                            RyzoraIcon {
                                                name: "package"
                                                size: 14
                                                color: theme.textMuted
                                            }

                                            Text {
                                                text: modelData.name || modelData.package_id
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: theme.text
                                                Layout.minimumWidth: 140
                                            }

                                            Text {
                                                text: (modelData.current_version || "—") + " → " + (modelData.available_version || "latest")
                                                font.pixelSize: 12
                                                color: theme.accent
                                                font.family: "monospace"
                                            }

                                            Item { Layout.fillWidth: true }

                                            Text {
                                                text: modelData.download_size_str || modelData.installed_size_str || "Official repo"
                                                font.pixelSize: 11
                                                color: theme.textMuted
                                            }
                                        }

                                        MouseArea {
                                            id: sysRowMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ══════════════════════════════════════════════════════════
                    // GROUP 2: Application Updates (Flatpak & AUR Apps)
                    // ══════════════════════════════════════════════════════════
                    ColumnLayout {
                        visible: root.appUpdatesList().length > 0 && (root.selectedFilter === "all" || root.selectedFilter === "apps" || root.selectedFilter === "flatpak" || root.selectedFilter === "aur")
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: "Application Updates (" + root.appUpdatesList().length + ")"
                            font.pixelSize: 15
                            font.bold: true
                            color: theme.text
                        }

                        Repeater {
                            model: root.appUpdatesList()
                            delegate: Rectangle {
                                id: appRowDelegate
                                Layout.fillWidth: true
                                implicitHeight: appRowCol.implicitHeight + 16
                                radius: theme.cardRadius
                                color: appRowMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                border.color: appRowMouse.containsMouse ? theme.cardBorderHover : theme.cardBorder
                                border.width: 1

                                readonly property var activeTxn: {
                                    var dummy = root.updateRev
                                    if (!backend) return null
                                    return backend.getTransactionForApp(modelData.package_id)
                                }
                                readonly property bool isOperating: Boolean(backend && backend.isAppOperating(modelData.package_id))

                                ColumnLayout {
                                    id: appRowCol
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.margins: 12
                                    spacing: 6

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 12

                                        Rectangle {
                                            width: 36
                                            height: 36
                                            radius: 8
                                            color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.10)
                                            border.color: theme.borderSubtle
                                            border.width: 1

                                            Image {
                                                anchors.fill: parent
                                                anchors.margins: 4
                                                source: modelData.icon_url || ""
                                                fillMode: Image.PreserveAspectFit
                                                visible: status === Image.Ready
                                            }

                                            RyzoraIcon {
                                                anchors.centerIn: parent
                                                visible: parent.children[0].status !== Image.Ready
                                                name: (modelData.provider === "flatpak") ? "layers" : "box"
                                                size: 18
                                                color: theme.accent
                                            }
                                        }

                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 2

                                            RowLayout {
                                                spacing: 8
                                                Text {
                                                    text: modelData.name || modelData.package_id
                                                    font.pixelSize: 14
                                                    font.bold: true
                                                    color: theme.text
                                                }

                                                Rectangle {
                                                    height: 18
                                                    implicitWidth: appProvTxt.implicitWidth + 8
                                                    radius: 4
                                                    color: (modelData.provider === "flatpak") ? Qt.rgba(59/255, 130/255, 246/255, 0.15) : Qt.rgba(245/255, 158/255, 11/255, 0.15)
                                                    border.color: (modelData.provider === "flatpak") ? "#3B82F6" : "#F59E0B"
                                                    border.width: 1

                                                    Text {
                                                        id: appProvTxt
                                                        anchors.centerIn: parent
                                                        text: (modelData.provider || "App").toUpperCase()
                                                        font.pixelSize: 9
                                                        font.bold: true
                                                        color: (modelData.provider === "flatpak") ? "#3B82F6" : "#F59E0B"
                                                    }
                                                }
                                            }

                                            Text {
                                                text: (modelData.current_version || "—") + " → " + (modelData.available_version || "latest")
                                                font.pixelSize: 12
                                                color: theme.textMuted
                                                font.family: "monospace"
                                            }
                                        }

                                        // Individual App Update Action Button
                                        RyzoraButton {
                                            size: "sm"
                                            variant: "primary"
                                            loading: appRowDelegate.isOperating
                                            loadingText: "Updating…"
                                            text: "Update"
                                            iconName: "arrow-download"
                                            iconFilled: true
                                            onClicked: {
                                                if (backend) backend.applyUpdate(modelData.package_id)
                                            }
                                        }
                                    }

                                    // Live progress for app update
                                    RowLayout {
                                        visible: appRowDelegate.isOperating
                                        Layout.fillWidth: true
                                        spacing: 8

                                        Text {
                                            text: appRowDelegate.activeTxn && appRowDelegate.activeTxn.message ? appRowDelegate.activeTxn.message : "Updating application…"
                                            font.pixelSize: 11
                                            color: theme.accent
                                            elide: Text.ElideRight
                                            Layout.fillWidth: true
                                        }
                                    }
                                }

                                MouseArea {
                                    id: appRowMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    z: -1
                                }
                            }
                        }
                    }

                    // ══════════════════════════════════════════════════════════
                    // GROUP 3: Community Customizations & Presets
                    // ══════════════════════════════════════════════════════════
                    ColumnLayout {
                        visible: root.communityUpdatesList().length > 0 && (root.selectedFilter === "all" || root.selectedFilter === "themes")
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: "Community Customizations (" + root.communityUpdatesList().length + ")"
                            font.pixelSize: 15
                            font.bold: true
                            color: theme.text
                        }

                        Repeater {
                            model: root.communityUpdatesList()
                            delegate: Rectangle {
                                id: commRowDelegate
                                Layout.fillWidth: true
                                implicitHeight: commRowCol.implicitHeight + 16
                                radius: theme.cardRadius
                                color: commRowMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                border.color: commRowMouse.containsMouse ? theme.cardBorderHover : theme.cardBorder
                                border.width: 1

                                readonly property bool isOperating: Boolean(backend && backend.isAppOperating(modelData.package_id))

                                ColumnLayout {
                                    id: commRowCol
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.margins: 12
                                    spacing: 6

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 12

                                        Rectangle {
                                            width: 36
                                            height: 36
                                            radius: 8
                                            color: Qt.rgba(139/255, 92/255, 246/255, 0.15)
                                            border.color: theme.borderSubtle
                                            border.width: 1

                                            RyzoraIcon {
                                                anchors.centerIn: parent
                                                name: "palette"
                                                size: 18
                                                color: "#8B5CF6"
                                            }
                                        }

                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 2

                                            Text {
                                                text: modelData.name || modelData.package_id
                                                font.pixelSize: 14
                                                font.bold: true
                                                color: theme.text
                                            }

                                            Text {
                                                text: (modelData.current_version || "—") + " → " + (modelData.available_version || "latest")
                                                font.pixelSize: 12
                                                color: theme.textMuted
                                                font.family: "monospace"
                                            }
                                        }

                                        RyzoraButton {
                                            size: "sm"
                                            variant: "primary"
                                            loading: commRowDelegate.isOperating
                                            loadingText: "Updating…"
                                            text: "Update"
                                            iconName: "arrow-download"
                                            iconFilled: true
                                            onClicked: {
                                                if (backend) backend.applyUpdate(modelData.package_id)
                                            }
                                        }
                                    }
                                }

                                MouseArea {
                                    id: commRowMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    z: -1
                                }
                            }
                        }
                    }
                }

                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            }
        }
    }
}
