import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    property var appData: null
    property string appId: ""
    property string appName: ""
    property string packageId: ""
    property string providerId: "pacman"
    property string version: ""
    property string iconUrl: ""
    property real installedBytes: 0

    // Options state
    property bool cleanFlatpakData: false
    property bool removeUnusedRuntimes: false
    property bool cleanAppConfig: false
    property bool cleanAppCache: false
    property bool cleanOrphans: false
    property bool cleanAurBuild: false
    property bool cleanRyzoraCache: false

    // Inspection preview from backend
    property var previewData: null
    property bool isLoadingPreview: false

    signal confirmed(var request)
    signal cancelled()

    visible: false
    enabled: visible
    anchors.fill: parent
    color: "#99000000"
    z: 200

    function formatBytes(bytes) {
        if (!bytes || bytes <= 0) return "0 B"
        if (bytes < 1024) return bytes + " B"
        var kb = bytes / 1024
        if (kb < 1024) return kb.toFixed(1) + " KB"
        var mb = kb / 1024
        if (mb < 1024) return mb.toFixed(1) + " MB"
        var gb = mb / 1024
        return gb.toFixed(2) + " GB"
    }

    readonly property real totalReclaimableBytes: {
        var sum = root.installedBytes
        if (root.previewData) {
            if (root.previewData.installed_package_bytes) {
                sum = root.previewData.installed_package_bytes
            }
            if (root.providerId === "flatpak") {
                if (root.cleanFlatpakData && root.previewData.flatpak_data_bytes) {
                    sum += root.previewData.flatpak_data_bytes
                }
            } else {
                if (root.cleanAppConfig && root.previewData.app_config_bytes) {
                    sum += root.previewData.app_config_bytes
                }
                if (root.cleanAppCache && root.previewData.app_cache_bytes) {
                    sum += root.previewData.app_cache_bytes
                }
                if (root.cleanOrphans && root.previewData.total_unused_dependencies_bytes) {
                    sum += root.previewData.total_unused_dependencies_bytes
                }
                if (root.cleanAurBuild) {
                    if (root.previewData.aur_build_dir_bytes) sum += root.previewData.aur_build_dir_bytes
                    if (root.previewData.aur_cache_bytes) sum += root.previewData.aur_cache_bytes
                }
            }
        }
        return sum
    }

    function openForApp(app) {
        root.appData = app
        if (!app) return
        root.appId = app.id || ""
        root.appName = app.display_name || app.name || "Application"
        root.packageId = (app.target_id || app.id || "").replace(/^(pacman|aur|flatpak):/, "")
        root.providerId = (app.provider_id || (app.id && app.id.indexOf("flatpak:") === 0 ? "flatpak" : "pacman")).toLowerCase()
        root.version = app.installed_version || app.version || ""
        root.iconUrl = app.icon_url || ""
        root.installedBytes = app.installed_size || app.download_size || 0

        // Reset toggles to safe defaults (only application removed by default)
        root.cleanFlatpakData = false
        root.removeUnusedRuntimes = false
        root.cleanAppConfig = false
        root.cleanAppCache = false
        root.cleanOrphans = false
        root.cleanAurBuild = false
        root.cleanRyzoraCache = false
        root.previewData = null
        root.isLoadingPreview = true
        root.visible = true

        if (typeof backend !== "undefined" && backend) {
            backend.inspectAppCleanup(root.packageId, root.providerId)
        } else {
            root.isLoadingPreview = false
        }
    }

    function closeDialog() {
        root.visible = false
        root.cancelled()
    }

    function confirmRemoval() {
        var req = {
            "package_id": root.packageId,
            "provider_id": root.providerId,
            "remove_package": true,
            "remove_unused_dependencies": root.providerId === "flatpak" ? root.removeUnusedRuntimes : root.cleanOrphans,
            "clean_package_cache": false,
            "clean_app_cache": root.cleanAppCache,
            "clean_app_config": root.cleanAppConfig,
            "clean_aur_build": root.cleanAurBuild,
            "clean_flatpak_data": root.cleanFlatpakData
        }
        root.visible = false
        root.confirmed(req)
        if (typeof backend !== "undefined" && backend) {
            backend.executeAppCleanup(JSON.stringify(req))
        }
    }

    Connections {
        target: typeof backend !== "undefined" ? backend : null
        function onAppCleanupInspected(appId, success, previewJson) {
            if (root.visible && (appId === root.packageId || appId === root.appId)) {
                root.isLoadingPreview = false
                if (success && previewJson) {
                    try {
                        root.previewData = JSON.parse(previewJson)
                    } catch (e) {
                        root.previewData = null
                    }
                }
            }
        }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: root.closeDialog()
    }

    // Modal Card Window
    Rectangle {
        id: dialogCard
        width: Math.min(500, parent.width - 32)
        implicitHeight: dialogContent.implicitHeight + 40
        anchors.centerIn: parent
        radius: theme.radiusLg
        color: theme.surfaceElevated
        border.color: theme.borderStrong
        border.width: 1
        clip: true

        MouseArea {
            anchors.fill: parent
            // Prevent click-through to overlay backdrop
        }

        ColumnLayout {
            id: dialogContent
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.margins: 20
            spacing: 16

            // 1. Header: Icon + Titles + Close Button
            RowLayout {
                Layout.fillWidth: true
                spacing: 14

                Rectangle {
                    width: 44
                    height: 44
                    radius: 10
                    color: theme.resolvedTheme === "light" ? "#FEE2E2" : "#451A1A"
                    border.color: theme.resolvedTheme === "light" ? "#FCA5A5" : "#7F1D1D"
                    border.width: 1
                    Layout.alignment: Qt.AlignVCenter

                    RyzoraIcon {
                        anchors.centerIn: parent
                        name: "trash"
                        size: 22
                        color: "#EF4444"
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: "Uninstall " + root.appName
                        font.pixelSize: 17
                        font.bold: true
                        color: theme.text
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Text {
                        text: root.packageId + " · " + (root.providerId === "flatpak" ? "Flatpak" : (root.providerId === "aur" ? "AUR" : "Official Repository")) + (root.version ? (" · v" + root.version) : "")
                        font.pixelSize: 12
                        color: theme.textSecondary
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                // Close Button
                Rectangle {
                    width: 28
                    height: 28
                    radius: 14
                    color: closeMouse.containsMouse ? theme.surfaceHover : "transparent"
                    Layout.alignment: Qt.AlignTop | Qt.AlignRight

                    RyzoraIcon {
                        anchors.centerIn: parent
                        name: "close"
                        size: 14
                        color: theme.textMuted
                    }

                    MouseArea {
                        id: closeMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.closeDialog()
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            // 2. Section Subtitle
            Text {
                text: "Choose what to remove:"
                font.pixelSize: 13
                font.weight: Font.Medium
                color: theme.text
            }

            // 3. Options List
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10

                // Option 1: Remove Application (Required, Always Checked)
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: opt1Row.implicitHeight + 16
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: opt1Row
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 10

                        // Checkbox (Checked & Disabled)
                        Rectangle {
                            width: 18
                            height: 18
                            radius: 4
                            color: theme.accent
                            border.color: theme.accent
                            border.width: 1
                            Layout.alignment: Qt.AlignTop

                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "check"
                                size: 12
                                color: "#ffffff"
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 6

                                Text {
                                    text: "Remove " + root.appName + " application"
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    color: theme.text
                                }

                                Rectangle {
                                    height: 16
                                    width: reqBadgeText.implicitWidth + 8
                                    radius: 3
                                    color: theme.surfaceElevated
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    Text {
                                        id: reqBadgeText
                                        anchors.centerIn: parent
                                        text: "Required"
                                        font.pixelSize: 9
                                        color: theme.textMuted
                                    }
                                }

                                Item { Layout.fillWidth: true }

                                Text {
                                    text: root.formatBytes(root.previewData && root.previewData.installed_package_bytes ? root.previewData.installed_package_bytes : root.installedBytes)
                                    font.pixelSize: 11
                                    font.family: "monospace"
                                    color: theme.textSecondary
                                }
                            }

                            Text {
                                text: "Removes application binaries, libraries, and desktop entry launchers."
                                font.pixelSize: 11
                                color: theme.textMuted
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }
                    }
                }

                // ── FLATPAK SPECIFIC CLEANUP OPTIONS ──
                ColumnLayout {
                    visible: root.providerId === "flatpak"
                    Layout.fillWidth: true
                    spacing: 10

                    // Option 2: Delete Flatpak Data (~/.var/app/<app-id>)
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: optFpDataRow.implicitHeight + 16
                        radius: theme.radiusSm
                        color: fpDataMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: root.cleanFlatpakData ? theme.accent : theme.borderSubtle
                        border.width: 1

                        MouseArea {
                            id: fpDataMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.cleanFlatpakData = !root.cleanFlatpakData
                        }

                        RowLayout {
                            id: optFpDataRow
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                width: 18
                                height: 18
                                radius: 4
                                color: root.cleanFlatpakData ? theme.accent : "transparent"
                                border.color: root.cleanFlatpakData ? theme.accent : theme.borderStrong
                                border.width: 1
                                Layout.alignment: Qt.AlignTop

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: "check"
                                    size: 12
                                    color: "#ffffff"
                                    visible: root.cleanFlatpakData
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Text {
                                        text: "Delete " + root.appName + " application data"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        color: theme.text
                                    }

                                    Item { Layout.fillWidth: true }

                                    Text {
                                        text: root.previewData && root.previewData.flatpak_data_bytes ? root.formatBytes(root.previewData.flatpak_data_bytes) : "0 B"
                                        font.pixelSize: 11
                                        font.family: "monospace"
                                        color: theme.textSecondary
                                        visible: root.previewData && root.previewData.flatpak_data_bytes > 0
                                    }
                                }

                                Text {
                                    text: "Data inside ~/.var/app/" + root.packageId + " will be deleted (flatpak uninstall --delete-data). Any files you saved outside this directory will not be affected."
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }

                    // Option 3: Remove Unused Flatpak Runtimes
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: optFpRtRow.implicitHeight + 16
                        radius: theme.radiusSm
                        color: fpRtMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: root.removeUnusedRuntimes ? theme.accent : theme.borderSubtle
                        border.width: 1

                        MouseArea {
                            id: fpRtMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.removeUnusedRuntimes = !root.removeUnusedRuntimes
                        }

                        RowLayout {
                            id: optFpRtRow
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                width: 18
                                height: 18
                                radius: 4
                                color: root.removeUnusedRuntimes ? theme.accent : "transparent"
                                border.color: root.removeUnusedRuntimes ? theme.accent : theme.borderStrong
                                border.width: 1
                                Layout.alignment: Qt.AlignTop

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: "check"
                                    size: 12
                                    color: "#ffffff"
                                    visible: root.removeUnusedRuntimes
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Text {
                                        text: "Remove unused Flatpak runtimes"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        color: theme.text
                                    }

                                    Item { Layout.fillWidth: true }

                                    Text {
                                        text: (root.previewData && root.previewData.flatpak_runtime_info) ? root.previewData.flatpak_runtime_info : "Safe cleanup"
                                        font.pixelSize: 10
                                        color: theme.textMuted
                                    }
                                }

                                Text {
                                    text: "Executes 'flatpak uninstall --unused'. Removes shared runtime dependencies no longer needed by any installed Flatpak application."
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }

                    // Option 4: Clear associated Ryzora cache
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: optRyzoraCacheRow.implicitHeight + 16
                        radius: theme.radiusSm
                        color: ryzoraCacheMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: root.cleanRyzoraCache ? theme.accent : theme.borderSubtle
                        border.width: 1

                        MouseArea {
                            id: ryzoraCacheMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.cleanRyzoraCache = !root.cleanRyzoraCache
                        }

                        RowLayout {
                            id: optRyzoraCacheRow
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                width: 18
                                height: 18
                                radius: 4
                                color: root.cleanRyzoraCache ? theme.accent : "transparent"
                                border.color: root.cleanRyzoraCache ? theme.accent : theme.borderStrong
                                border.width: 1
                                Layout.alignment: Qt.AlignTop

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: "check"
                                    size: 12
                                    color: "#ffffff"
                                    visible: root.cleanRyzoraCache
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Text {
                                    text: "Clear associated Ryzora cache data"
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    color: theme.text
                                }

                                Text {
                                    text: "Clears preview media, icon thumbnails, and cached store metadata for this application."
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }
                }

                // ── PACMAN / AUR SPECIFIC CLEANUP OPTIONS ──
                ColumnLayout {
                    visible: root.providerId !== "flatpak"
                    Layout.fillWidth: true
                    spacing: 10

                    // Option: User Config (~/.config/<app>)
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: optCfgRow.implicitHeight + 16
                        radius: theme.radiusSm
                        color: cfgMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: root.cleanAppConfig ? theme.accent : theme.borderSubtle
                        border.width: 1

                        MouseArea {
                            id: cfgMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.cleanAppConfig = !root.cleanAppConfig
                        }

                        RowLayout {
                            id: optCfgRow
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                width: 18
                                height: 18
                                radius: 4
                                color: root.cleanAppConfig ? theme.accent : "transparent"
                                border.color: root.cleanAppConfig ? theme.accent : theme.borderStrong
                                border.width: 1
                                Layout.alignment: Qt.AlignTop

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: "check"
                                    size: 12
                                    color: "#ffffff"
                                    visible: root.cleanAppConfig
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Text {
                                        text: "Delete application user configuration"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        color: theme.text
                                    }

                                    Item { Layout.fillWidth: true }

                                    Text {
                                        text: root.previewData && root.previewData.app_config_bytes ? root.formatBytes(root.previewData.app_config_bytes) : "0 B"
                                        font.pixelSize: 11
                                        font.family: "monospace"
                                        color: theme.textSecondary
                                        visible: root.previewData && root.previewData.app_config_bytes > 0
                                    }
                                }

                                Text {
                                    text: "Directory: " + ((root.previewData && root.previewData.app_config_path) ? root.previewData.app_config_path : ("~/.config/" + root.packageId))
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }

                    // Option: User Cache (~/.cache/<app>)
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: optCacheRow.implicitHeight + 16
                        radius: theme.radiusSm
                        color: cacheMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: root.cleanAppCache ? theme.accent : theme.borderSubtle
                        border.width: 1

                        MouseArea {
                            id: cacheMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.cleanAppCache = !root.cleanAppCache
                        }

                        RowLayout {
                            id: optCacheRow
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                width: 18
                                height: 18
                                radius: 4
                                color: root.cleanAppCache ? theme.accent : "transparent"
                                border.color: root.cleanAppCache ? theme.accent : theme.borderStrong
                                border.width: 1
                                Layout.alignment: Qt.AlignTop

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: "check"
                                    size: 12
                                    color: "#ffffff"
                                    visible: root.cleanAppCache
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Text {
                                        text: "Clear application cache directory"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        color: theme.text
                                    }

                                    Item { Layout.fillWidth: true }

                                    Text {
                                        text: root.previewData && root.previewData.app_cache_bytes ? root.formatBytes(root.previewData.app_cache_bytes) : "0 B"
                                        font.pixelSize: 11
                                        font.family: "monospace"
                                        color: theme.textSecondary
                                        visible: root.previewData && root.previewData.app_cache_bytes > 0
                                    }
                                }

                                Text {
                                    text: "Directory: " + ((root.previewData && root.previewData.app_cache_path) ? root.previewData.app_cache_path : ("~/.cache/" + root.packageId))
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }

                    // Option: Unused Dependencies (Orphans) for Pacman
                    Rectangle {
                        visible: root.providerId === "pacman"
                        Layout.fillWidth: true
                        implicitHeight: optOrphanRow.implicitHeight + 16
                        radius: theme.radiusSm
                        color: orphanMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: root.cleanOrphans ? theme.accent : theme.borderSubtle
                        border.width: 1

                        MouseArea {
                            id: orphanMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.cleanOrphans = !root.cleanOrphans
                        }

                        RowLayout {
                            id: optOrphanRow
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                width: 18
                                height: 18
                                radius: 4
                                color: root.cleanOrphans ? theme.accent : "transparent"
                                border.color: root.cleanOrphans ? theme.accent : theme.borderStrong
                                border.width: 1
                                Layout.alignment: Qt.AlignTop

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: "check"
                                    size: 12
                                    color: "#ffffff"
                                    visible: root.cleanOrphans
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Text {
                                        text: "Remove unused dependencies (orphans)"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        color: theme.text
                                    }

                                    Item { Layout.fillWidth: true }

                                    Text {
                                        text: root.previewData && root.previewData.total_unused_dependencies_bytes ? root.formatBytes(root.previewData.total_unused_dependencies_bytes) : "0 B"
                                        font.pixelSize: 11
                                        font.family: "monospace"
                                        color: theme.textSecondary
                                        visible: root.previewData && root.previewData.total_unused_dependencies_bytes > 0
                                    }
                                }

                                Text {
                                    text: "Executes 'pacman -Rs'. Removes orphaned packages no longer required by any other installed package."
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }

                    // Option: Clear AUR Build Directory (for AUR)
                    Rectangle {
                        visible: root.providerId === "aur"
                        Layout.fillWidth: true
                        implicitHeight: optAurRow.implicitHeight + 16
                        radius: theme.radiusSm
                        color: aurMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: root.cleanAurBuild ? theme.accent : theme.borderSubtle
                        border.width: 1

                        MouseArea {
                            id: aurMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.cleanAurBuild = !root.cleanAurBuild
                        }

                        RowLayout {
                            id: optAurRow
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                width: 18
                                height: 18
                                radius: 4
                                color: root.cleanAurBuild ? theme.accent : "transparent"
                                border.color: root.cleanAurBuild ? theme.accent : theme.borderStrong
                                border.width: 1
                                Layout.alignment: Qt.AlignTop

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: "check"
                                    size: 12
                                    color: "#ffffff"
                                    visible: root.cleanAurBuild
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Text {
                                        text: "Clear AUR build directory and cache"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        color: theme.text
                                    }

                                    Item { Layout.fillWidth: true }

                                    Text {
                                        text: root.previewData && (root.previewData.aur_build_dir_bytes || root.previewData.aur_cache_bytes) ? root.formatBytes((root.previewData.aur_build_dir_bytes || 0) + (root.previewData.aur_cache_bytes || 0)) : "0 B"
                                        font.pixelSize: 11
                                        font.family: "monospace"
                                        color: theme.textSecondary
                                        visible: root.previewData && ((root.previewData.aur_build_dir_bytes || 0) + (root.previewData.aur_cache_bytes || 0)) > 0
                                    }
                                }

                                Text {
                                    text: "Clears ~/.cache/ryzora/aur-build/" + root.packageId + " and helper build clones."
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }
                }
            }

            // 4. Space Reclaimed Summary Callout
            Rectangle {
                Layout.fillWidth: true
                height: 38
                radius: theme.radiusSm
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 8

                    RyzoraIcon {
                        name: "hard-drive"
                        size: 15
                        color: theme.textMuted
                    }

                    Text {
                        text: "Estimated space recovered:"
                        font.pixelSize: 12
                        color: theme.textSecondary
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: "~ " + root.formatBytes(root.totalReclaimableBytes)
                        font.pixelSize: 13
                        font.bold: true
                        color: "#10B981"
                    }
                }
            }

            // 5. Actions Footer
            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                Item { Layout.fillWidth: true }

                // Cancel Button
                RyzoraButton {
                    variant: "secondary"
                    text: "Cancel"
                    onClicked: root.closeDialog()
                }

                // Uninstall Confirmation Button (Destructive)
                RyzoraButton {
                    variant: "destructive-solid"
                    iconName: "delete"
                    iconFilled: true
                    text: "Uninstall " + root.appName
                    onClicked: root.confirmRemoval()
                }
            }
        }
    }
}

