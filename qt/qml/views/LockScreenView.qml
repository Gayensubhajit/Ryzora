import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    property string selectedSubFilter: "All" // "All" | "Qylock" | "SilentSDDM" | "SDDM" | "My Media"
    property string selectedPackageId: ""
    property var selectedPackage: null

    signal packageSelected(var packageData)
    signal actionRequested(string action, var data, var targetOverride)

    function setSelectedPackage(pkg) {
        if (!pkg) {
            root.selectedPackageId = ""
            root.selectedPackage = null
            if (lockscreenDetailView) {
                lockscreenDetailView.packageData = null
                lockscreenDetailView.errorMessage = ""
                lockscreenDetailView.pendingAction = ""
            }
            return
        }
        var pkgId = typeof pkg === "string" ? pkg : (pkg && pkg.id ? String(pkg.id) : "")
        if (!pkgId) {
            root.selectedPackageId = ""
            root.selectedPackage = null
            if (lockscreenDetailView) {
                lockscreenDetailView.packageData = null
            }
            return
        }
        root.selectedPackageId = pkgId
        var fresh = root.findPackage(pkgId)
        if (fresh) {
            root.selectedPackage = fresh
        } else if (typeof pkg === "object" && pkg !== null) {
            root.selectedPackage = pkg
        } else {
            root.selectedPackage = null
        }
        if (lockscreenDetailView) {
            lockscreenDetailView.packageData = root.selectedPackage
            lockscreenDetailView.errorMessage = ""
            lockscreenDetailView.pendingAction = ""
        }
    }

    onSelectedPackageChanged: {
        if (lockscreenDetailView) {
            lockscreenDetailView.packageData = root.selectedPackage
        }
    }

    function findPackage(pkgId) {
        if (!pkgId) return null
        var list = backend.catalogList || []
        for (var i = 0; i < list.length; ++i) {
            if (list[i] && String(list[i].id) === String(pkgId)) return list[i]
        }
        var lib = backend.libraryList || []
        for (var j = 0; j < lib.length; ++j) {
            if (lib[j] && String(lib[j].id) === String(pkgId)) return lib[j]
        }
        return null
    }

    function syncSelectedPackage() {
        if (!root.selectedPackageId) return
        var fresh = root.findPackage(root.selectedPackageId)
        if (fresh) {
            root.selectedPackage = fresh
            if (lockscreenDetailView) {
                lockscreenDetailView.packageData = fresh
            }
        }
    }

    readonly property var allLockscreenPackages: {
        var list = backend.catalogList || []
        return list.filter(function(item) {
            return String(item && item.category ? item.category : "").toLowerCase() === "lockscreens"
        })
    }

    readonly property var filteredPackages: {
        var list = root.allLockscreenPackages
        var sub = root.selectedSubFilter
        if (sub === "Qylock") {
            return list.filter(function(item) {
                var isCustom = (item.tags && item.tags.indexOf("custom") !== -1) || String(item.id).indexOf("custom:") === 0
                var isSilent = !isCustom && (String(item.subfilter) === "silentsddm" || String(item.id).indexOf("silentsddm-") === 0)
                return !isCustom && !isSilent && ((item.tags && item.tags.indexOf("qylock") !== -1) || String(item.id).indexOf("lockscreen-qylock") === 0 || String(item.subfilter) === "qylock")
            })
        } else if (sub === "SilentSDDM") {
            return list.filter(function(item) {
                var isCustom = (item.tags && item.tags.indexOf("custom") !== -1) || String(item.id).indexOf("custom:") === 0
                return !isCustom && (String(item.subfilter) === "silentsddm" || String(item.id).indexOf("silentsddm-") === 0)
            })
        } else if (sub === "SDDM") {
            return list.filter(function(item) {
                return Boolean(item.supports_sddm) || (item.tags && item.tags.indexOf("sddm") !== -1)
            })
        } else if (sub === "My Media") {
            return list.filter(function(item) {
                return (item.tags && item.tags.indexOf("custom") !== -1) || String(item.id).indexOf("custom:") === 0 || String(item.subfilter) === "custom_media"
            })
        }
        return list
    }

    Connections {
        target: backend
        function onCatalogChanged() { root.syncSelectedPackage() }
        function onLibraryChanged() { root.syncSelectedPackage() }
        function onActiveConfigurationChanged() { root.syncSelectedPackage() }
        function onTargetDeactivated(target, success, message) { root.syncSelectedPackage() }
        function onTargetActivated(packageId, target, success, message) { root.syncSelectedPackage() }
        function onPackageInstalled(packageId, success, message) { root.syncSelectedPackage() }
        function onPackageUninstalled(packageId, success, message) { root.syncSelectedPackage() }
        function onPackageTested(packageId, success, message) { root.syncSelectedPackage() }
    }

    ScrollView {
        id: mainScroll
        anchors.fill: parent
        visible: root.selectedPackage === null
        clip: true
        contentWidth: availableWidth
        ScrollBar.vertical.policy: ScrollBar.AsNeeded

        WheelHandler {
            target: null
            acceptedDevices: PointerDevice.Mouse
            blocking: true
            onWheel: (event) => {
                var fl = mainScroll.contentItem
                if (fl && fl.contentHeight > fl.height) {
                    var delta = event.angleDelta.y
                    var step = -delta * (80.0 / 120.0) * 1.18
                    fl.contentY = Math.max(0, Math.min(fl.contentHeight - fl.height, fl.contentY + step))
                }
            }
        }



        ColumnLayout {
            width: mainScroll.availableWidth
            spacing: 20

            // Inner margins
            Item { height: 16 }

            // ── Row 1: Header (Title, Subtitle & Active SDDM Greeter Badge) ──
            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24
                spacing: 16

                ColumnLayout {
                    spacing: 3
                    Layout.fillWidth: true

                    Text {
                        text: "Lock Screens"
                        font.pixelSize: 24
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Explore Wayland session locks, SDDM greeters, and video backgrounds."
                        font.pixelSize: 13
                        color: theme.textSecondary
                    }
                }

                // Active SDDM Status Pill
                Rectangle {
                    height: 28
                    width: sddmStatusRow.implicitWidth + 18
                    radius: 6
                    color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                        ? Qt.rgba(16/255, 185/255, 129/255, 0.15)
                        : theme.surfaceActive
                    border.color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                        ? Qt.rgba(16/255, 185/255, 129/255, 0.35)
                        : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: sddmStatusRow
                        anchors.centerIn: parent
                        spacing: 6
                        Rectangle {
                            width: 6
                            height: 6
                            radius: 3
                            color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                ? "#10b981"
                                : theme.textMuted
                        }
                        Text {
                            text: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                ? "SDDM Active"
                                : "SDDM Default"
                            font.pixelSize: 11
                            font.bold: true
                            color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                ? "#10b981"
                                : theme.text
                        }
                    }
                }
            }

            // ── Row 2: Subfilter Pills Control ──
            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24
                spacing: 8

                ScrollView {
                    Layout.fillWidth: true
                    height: 32
                    clip: true
                    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                    ScrollBar.vertical.policy: ScrollBar.AlwaysOff

                    RowLayout {
                        spacing: 6

                        LockSubFilterChip {
                            label: "All"
                            subId: "All"
                            isSelected: root.selectedSubFilter === "All"
                            count: root.allLockscreenPackages.length
                            onClicked: root.selectedSubFilter = "All"
                        }
                        LockSubFilterChip {
                            label: "Qylock"
                            subId: "Qylock"
                            isSelected: root.selectedSubFilter === "Qylock"
                            onClicked: root.selectedSubFilter = "Qylock"
                        }
                        LockSubFilterChip {
                            label: "SilentSDDM"
                            subId: "SilentSDDM"
                            isSelected: root.selectedSubFilter === "SilentSDDM"
                            onClicked: root.selectedSubFilter = "SilentSDDM"
                        }
                        LockSubFilterChip {
                            label: "SDDM"
                            subId: "SDDM"
                            isSelected: root.selectedSubFilter === "SDDM"
                            onClicked: root.selectedSubFilter = "SDDM"
                        }
                        LockSubFilterChip {
                            label: "My Media"
                            subId: "My Media"
                            isSelected: root.selectedSubFilter === "My Media"
                            onClicked: root.selectedSubFilter = "My Media"
                        }
                    }
                }

                // Count Pill
                Text {
                    text: root.filteredPackages.length + " items"
                    font.pixelSize: 12
                    font.bold: true
                    color: theme.textSecondary
                }
            }

            // ── Responsive Virtualized Package Gallery Grid ──
            Item {
                Layout.fillWidth: true
                Layout.leftMargin: 16
                Layout.rightMargin: 16
                implicitHeight: Math.max(280, Math.ceil(root.filteredPackages.length / Math.max(1, lockGrid.columnCount)) * lockGrid.cellHeight)

                EmptyState {
                    anchors.fill: parent
                    visible: root.filteredPackages.length === 0
                    iconName: "shield"
                    title: "No lockscreens found"
                    message: "No lockscreen packages match subfilter '" + root.selectedSubFilter + "'."
                    actionLabel: "Show All Locks"
                    onActionClicked: root.selectedSubFilter = "All"
                }

                GridView {
                    id: lockGrid
                    anchors.fill: parent
                    clip: true
                    interactive: false // scrolling handled by main outer ScrollView
                    visible: root.filteredPackages.length > 0

                    property int columnCount: width >= 1150 ? 4 : (width >= 820 ? 3 : (width >= 520 ? 2 : 1))
                    cellWidth: Math.floor(width / columnCount)
                    cellHeight: 260

                    model: root.filteredPackages
                    delegate: Item {
                        width: lockGrid.cellWidth
                        height: lockGrid.cellHeight

                        StoreCard {
                            anchors.fill: parent
                            anchors.margins: 8
                            itemData: modelData
                            isPageVisible: root.visible && root.selectedPackage === null
                            allowAutoplay: true
                            onCardClicked: function(data) {
                                root.setSelectedPackage(data)
                                root.packageSelected(root.selectedPackage || data)
                            }
                            onActionClicked: function(action, data) {
                                root.actionRequested(action, data, null)
                                window.handlePackageAction(action, data, null)
                            }
                        }
                    }
                }
            }

            // ── System Diagnostics & Deactivation Controls ──
            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24
                spacing: 12

                Text {
                    text: "System Greeter & Lock Integration"
                    font.pixelSize: 15
                    font.bold: true
                    color: theme.text
                }

                // 1. SDDM Login Screen Integration Card
                Rectangle {
                    Layout.fillWidth: true
                    height: 140
                    radius: theme.radiusMd
                    color: theme.surface
                    border.color: theme.border
                    border.width: 1

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 8

                        RowLayout {
                            spacing: 8
                            RyzoraIcon { name: "display"; size: 16; color: theme.accent }
                            Text {
                                text: "SDDM Login Greeter (Pre-Session)"
                                font.pixelSize: theme.fontSection
                                font.bold: true
                                color: theme.text
                            }
                            Item { Layout.fillWidth: true }
                            Rectangle {
                                height: 20
                                width: sddmActiveTxt.implicitWidth + 16
                                radius: 4
                                color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                    ? Qt.rgba(16/255, 185/255, 129/255, 0.20)
                                    : Qt.rgba(100/255, 116/255, 139/255, 0.20)
                                border.color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                    ? Qt.rgba(16/255, 185/255, 129/255, 0.35)
                                    : Qt.rgba(100/255, 116/255, 139/255, 0.30)
                                border.width: 1

                                Text {
                                    id: sddmActiveTxt
                                    anchors.centerIn: parent
                                    text: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                        ? "ACTIVE SDDM GREETER"
                                        : "SYSTEM DEFAULT GREETER"
                                    font.pixelSize: 9
                                    font.bold: true
                                    color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active) ? "#10b981" : "#94a3b8"
                                }
                            }
                        }

                        Text {
                            text: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                ? ("Managed Theme: " + (backend.activeConfiguration ? backend.activeConfiguration.active_title : "None"))
                                : "No custom SDDM theme is currently active. The display manager runs using system default greeter."
                            font.pixelSize: theme.fontSm
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        Item { Layout.fillHeight: true }

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: "Config drift: " + (backend.systemInfo ? backend.systemInfo.config_drift : "In sync")
                                font.pixelSize: 11
                                color: theme.textMuted
                            }
                            Item { Layout.fillWidth: true }

                            Rectangle {
                                visible: Boolean(backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                height: 28
                                width: deactSddmTxt.implicitWidth + 20
                                radius: theme.radiusSm
                                color: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                    ? (deactSddmMouse.containsMouse ? "#3b1219" : theme.surfaceElevated)
                                    : theme.surfaceElevated
                                border.color: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                    ? (deactSddmMouse.containsMouse ? "#ef4444" : theme.borderStrong)
                                    : theme.border
                                border.width: 1
                                opacity: (backend.systemInfo && backend.systemInfo.polkit_helper_installed) ? 1.0 : 0.65

                                Text {
                                    id: deactSddmTxt
                                    anchors.centerIn: parent
                                    text: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                        ? "Deactivate SDDM"
                                        : "Deactivate SDDM (Elevation Required)"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                        ? (deactSddmMouse.containsMouse ? "#f87171" : theme.textMuted)
                                        : theme.textFaint
                                }
                                MouseArea {
                                    id: deactSddmMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                        ? Qt.PointingHandCursor
                                        : Qt.ForbiddenCursor
                                    onClicked: {
                                        if (backend.systemInfo && !backend.systemInfo.polkit_helper_installed) {
                                            backend.postNotification("warning", "Elevation Helper Required", "SDDM deactivation requires privileged Polkit helper. Please set up the helper in System Integration.");
                                            return;
                                        }
                                        backend.deactivateTarget("login_screen")
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Desktop Lock Status Card (Native Dusky / Hyprlock)
                Rectangle {
                    Layout.fillWidth: true
                    height: 120
                    radius: theme.radiusMd
                    color: theme.surface
                    border.color: theme.border
                    border.width: 1

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 8

                        RowLayout {
                            spacing: 8
                            RyzoraIcon { name: "shield"; size: 16; color: "#10b981" }
                            Text {
                                text: "Desktop Lock (Dusky / Hyprlock)"
                                font.pixelSize: theme.fontSection
                                font.bold: true
                                color: theme.text
                            }
                            Item { Layout.fillWidth: true }
                            Rectangle {
                                height: 20
                                width: sessionActiveTxt.implicitWidth + 16
                                radius: 4
                                color: Qt.rgba(100/255, 116/255, 139/255, 0.20)
                                border.color: Qt.rgba(100/255, 116/255, 139/255, 0.30)
                                border.width: 1

                                Text {
                                    id: sessionActiveTxt
                                    anchors.centerIn: parent
                                    text: "UNMANAGED · NATIVE DUSKY"
                                    font.pixelSize: 9
                                    font.bold: true
                                    color: "#94a3b8"
                                }
                            }
                        }

                        Text {
                            text: "Desktop session locking is controlled independently by native Dusky and Hyprlock. Ryzora does not wrap, hook, or mutate host lock scripts or hypridle bindings."
                            font.pixelSize: 11
                            color: theme.textMuted
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }
                }
            }

            Item { height: 24 }
        }
    }

    // ── Full-Page Package Detail View Overlay (Preserves Lockscreen Route & Subfilter) ──
    PackageDetailView {
        id: lockscreenDetailView
        objectName: "lockscreenPackageDetailView"
        anchors.fill: parent
        visible: root.visible && root.selectedPackage !== null
        packageData: root.selectedPackage
        fromRoute: "lockscreens"
        onBackRequested: root.setSelectedPackage(null)
        onPackageSelected: function(pkg) {
            root.setSelectedPackage(pkg)
        }
        onActionRequested: function(action, data, targetOverride) {
            window.handlePackageAction(action, data, targetOverride)
        }
    }

    // LockSubFilterChip Component
    component LockSubFilterChip : Rectangle {
        property string label: ""
        property string subId: ""
        property bool isSelected: false
        property int count: -1
        signal clicked()

        height: 30
        width: chipLabel.implicitWidth + (count >= 0 ? countBadge.implicitWidth + 10 : 0) + 24
        radius: 8
        color: isSelected ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#282C35") : (chipArea.containsMouse ? theme.surfaceHover : theme.surface)
        border.color: isSelected ? theme.accent : theme.borderSubtle
        border.width: 1

        Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }
        Behavior on border.color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }

        RowLayout {
            anchors.centerIn: parent
            spacing: 6

            Text {
                id: chipLabel
                text: label
                font.pixelSize: 12
                font.bold: isSelected
                color: isSelected ? theme.text : (chipArea.containsMouse ? theme.text : theme.textSecondary)
            }

            Text {
                id: countBadge
                visible: count >= 0
                text: "(" + count + ")"
                font.pixelSize: 10
                color: isSelected ? theme.accent : theme.textMuted
            }
        }

        MouseArea {
            id: chipArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }
}
