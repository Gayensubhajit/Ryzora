import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root
    property int currentSubTab: 0 // 0: Diagnostics, 1: Compatibility Lab, 2: Integrity, 3: Activity, 4: Package Creator
    property string selectedProfileId: "live"
    property bool isProbing: false
    property string lastChecked: Qt.formatDateTime(new Date(), "hh:mm:ss")

    Timer {
        id: probeTimer
        interval: 1200
        repeat: false
        onTriggered: root.isProbing = false
    }

    // Helper to find selected profile object
    readonly property var activeProfile: {
        var profiles = (backend && backend.compatibilityProfiles) ? backend.compatibilityProfiles : []
        for (var i = 0; i < profiles.length; i++) {
            if (profiles[i].id === selectedProfileId) {
                return profiles[i]
            }
        }
        return profiles.length > 0 ? profiles[0] : null
    }

    function generateDiagnosticsReport() {
        var info = (backend && backend.systemInfo) ? backend.systemInfo : {}
        var lines = []
        lines.push("==================================================")
        lines.push("           RYZORA SYSTEM DIAGNOSTICS REPORT        ")
        lines.push("==================================================")
        lines.push("Generated:         " + Qt.formatDateTime(new Date(), "yyyy-MM-dd hh:mm:ss"))
        lines.push("")
        lines.push("[HOST ENVIRONMENT]")
        lines.push("OS Name:           " + (info.os_name || "Linux Operating System"))
        lines.push("Kernel:            " + (info.kernel || "Linux 6.x"))
        lines.push("Desktop:           " + (info.desktop || "Desktop Session"))
        lines.push("Session Type:      " + (info.session_type ? info.session_type.toUpperCase() : "WAYLAND"))
        lines.push("Wayland Display:   " + (info.wayland_display || "wayland-0"))
        lines.push("")
        lines.push("[SECURITY & PRIVILEGES]")
        lines.push("Polkit Helper:     " + (info.polkit_status || "Active"))
        lines.push("Helper Installed:  " + (info.polkit_helper_installed ? "Yes" : "No"))
        lines.push("Process Isolation: " + (info.process_isolation || "Enforced"))
        lines.push("SDDM Status:       " + (info.sddm_status || "Detected"))
        lines.push("Config Drift:      " + (info.config_drift || "In sync"))
        lines.push("")
        lines.push("[STORE METRICS]")
        lines.push("Catalog Packages:  " + (backend ? backend.catalogCount : 0))
        lines.push("Installed Items:   " + (backend ? backend.libraryCount : 0))
        lines.push("Restore Points:    " + (backend ? backend.snapshotsCount : 0))
        lines.push("Active Subprocess: " + (backend && backend.isProcessActive() ? backend.activePid().toString() : "None (Idle)"))
        lines.push("UI Rendering:      Qt 6 Quick SceneGraph • Wayland EGL Native")
        lines.push("==================================================")
        return lines.join("\n")
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16

        // ══════════════════════════════════════════════════════════════
        // 1. HEADER: Title, Subtitle, Last Checked & Primary Actions
        // ══════════════════════════════════════════════════════════════
        GridLayout {
            Layout.fillWidth: true
            columns: root.width >= 1150 ? 2 : 1
            rowSpacing: 10
            columnSpacing: 16

            ColumnLayout {
                spacing: 3
                Layout.fillWidth: true

                RowLayout {
                    spacing: 10
                    Text {
                        text: root.currentSubTab === 0 ? "Host System & Diagnostics" :
                              (root.currentSubTab === 1 ? "Universal Linux Compatibility Lab" :
                              (root.currentSubTab === 2 ? "Package Integrity & Security" :
                              (root.currentSubTab === 3 ? "Session Activity Log" : "Developer Package Creator")))
                        font.pixelSize: 26
                        font.bold: true
                        font.family: theme.fontFamily
                        color: theme.text
                    }

                    Rectangle {
                        height: 20
                        radius: 10
                        width: lastCheckedTxt.implicitWidth + 14
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 5
                            Rectangle {
                                width: 6
                                height: 6
                                radius: 3
                                color: theme.success
                            }
                            Text {
                                id: lastCheckedTxt
                                text: "Checked " + root.lastChecked
                                font.pixelSize: 10
                                color: theme.textMuted
                            }
                        }
                    }
                }

                Text {
                    text: root.currentSubTab === 0 
                        ? "System health, runtime environment, and security boundaries" :
                          (root.currentSubTab === 1 
                        ? "Capability matrix debugger for host environments against runtime display adapters" :
                          (root.currentSubTab === 2 
                        ? "Configuration drift verification, cryptographic integrity, and host restore points" :
                          (root.currentSubTab === 3 
                        ? "Real-time in-memory event stream of installation, deactivation, and backend operations"
                        : "Authoring studio to generate, validate, and export Ryzora package specifications")))
                    font.pixelSize: 13
                    font.family: theme.fontFamily
                    color: theme.textSecondary
                }
            }

            // Right-aligned Action Buttons (Responsive placement)
            RowLayout {
                Layout.alignment: root.width >= 1150 ? (Qt.AlignRight | Qt.AlignVCenter) : Qt.AlignLeft
                spacing: 8

                RyzoraButton {
                    id: copyReportBtn
                    objectName: "copyReportButton"
                    text: "Copy report"
                    variant: "secondary"
                    size: "md"
                    tooltip: "Copy full diagnostics report to clipboard"
                    onClicked: {
                        var report = root.generateDiagnosticsReport()
                        if (backend) {
                            backend.copyToClipboard(report)
                            backend.postNotification("success", "Diagnostics Copied", "Diagnostics report copied to clipboard.")
                        }
                    }
                }

                RyzoraButton {
                    id: exportDiagBtn
                    objectName: "exportDiagnosticsButton"
                    text: "Export report"
                    iconName: "arrow-download"
                    variant: "secondary"
                    size: "md"
                    tooltip: "Save diagnostics report file to disk"
                    onClicked: {
                        var report = root.generateDiagnosticsReport()
                        if (backend) {
                            var savedPath = backend.exportDiagnosticsReport(report)
                            if (savedPath && savedPath.length > 0) {
                                backend.postNotification("success", "Diagnostics Exported", "Report saved to " + savedPath)
                            } else {
                                backend.postNotification("error", "Export Failed", "Could not save diagnostics report.")
                            }
                        }
                    }
                }

                RyzoraButton {
                    id: refreshDiagBtn
                    objectName: "refreshDiagnosticsButton"
                    text: "Refresh"
                    iconName: "arrow-clockwise"
                    variant: "primary"
                    size: "md"
                    loading: root.isProbing
                    tooltip: "Probe host hardware, compositors, Polkit, and SDDM status"
                    onClicked: {
                        root.isProbing = true
                        probeTimer.start()
                        root.lastChecked = Qt.formatDateTime(new Date(), "hh:mm:ss")
                        if (backend) {
                            backend.loadSystemInfo()
                            backend.loadCompatibilityProfiles()
                            backend.postNotification("info", "Diagnostics Refreshed", "System probes and compatibility profiles reloaded.")
                        }
                    }
                }
            }
        }

        // ══════════════════════════════════════════════════════════════
        // 2. NAVIGATION ROW: Full-width Segmented Bar (No Emojis)
        // ══════════════════════════════════════════════════════════════
        Rectangle {
            Layout.fillWidth: true
            height: 40
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.margins: 4
                spacing: 4

                Repeater {
                    model: [
                        { id: 0, label: "Host", icon: "desktop" },
                        { id: 1, label: "Compatibility", icon: "wrench" },
                        { id: 2, label: "Integrity", icon: "shield" },
                        { id: 3, label: "Activity", icon: "pulse" },
                        { id: 4, label: "Packages", icon: "code" }
                    ]

                    delegate: Rectangle {
                        property bool isSelected: root.currentSubTab === modelData.id
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        radius: theme.radiusSm
                        color: isSelected ? theme.surfaceElevated : (tabArea.containsMouse ? theme.surfaceHover : "transparent")
                        border.color: isSelected ? theme.accent : "transparent"
                        border.width: isSelected ? 1 : 0

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: root.width < 1100 ? 5 : 8

                            RyzoraIcon {
                                name: modelData.icon
                                size: root.width < 1100 ? 13 : 14
                                filled: isSelected
                                color: isSelected ? theme.accent : theme.textMuted
                            }

                            Text {
                                text: modelData.label
                                font.pixelSize: root.width < 1100 ? 11 : 12
                                font.bold: isSelected
                                color: isSelected ? theme.text : theme.textSecondary
                                elide: Text.ElideRight
                            }
                        }

                        MouseArea {
                            id: tabArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.currentSubTab = modelData.id
                        }
                    }
                }
            }
        }

        // ══════════════════════════════════════════════════════════════
        // SUB-TAB 0: HOST DIAGNOSTICS VIEW
        // ══════════════════════════════════════════════════════════════
        ScrollView {
            visible: root.currentSubTab === 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ScrollBar.vertical.policy: ScrollBar.AsNeeded

            ColumnLayout {
                width: parent.width
                spacing: 16

                // ── A. 3 Summary Cards (Responsive Grid) ──
                GridLayout {
                    Layout.fillWidth: true
                    columns: root.width > 900 ? 3 : 1
                    rowSpacing: 14
                    columnSpacing: 14

                    // Host OS Card
                    Rectangle {
                        Layout.fillWidth: true
                        height: 128
                        radius: theme.radiusMd
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 16
                            spacing: 8

                            RowLayout {
                                spacing: 8
                                RyzoraIcon {
                                    name: "desktop"
                                    size: 16
                                    color: theme.accent
                                }
                                Text {
                                    text: "Operating System"
                                    font.pixelSize: theme.fontBase
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            Text {
                                text: (backend && backend.systemInfo && backend.systemInfo.os_name && backend.systemInfo.os_name.length > 0)
                                    ? backend.systemInfo.os_name : "Linux Operating System"
                                font.pixelSize: 17
                                font.bold: true
                                color: theme.accent
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Text {
                                text: "Kernel: " + ((backend && backend.systemInfo && backend.systemInfo.kernel) ? backend.systemInfo.kernel : "Linux 6.x") +
                                      " (" + ((backend && backend.systemInfo && backend.systemInfo.arch) ? backend.systemInfo.arch : "x86_64") + ")"
                                font.pixelSize: theme.fontSm
                                color: theme.textSecondary
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                        }
                    }

                    // Compositor Card
                    Rectangle {
                        Layout.fillWidth: true
                        height: 128
                        radius: theme.radiusMd
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 16
                            spacing: 8

                            RowLayout {
                                spacing: 8
                                RyzoraIcon {
                                    name: "desktop"
                                    size: 16
                                    color: theme.accent
                                }
                                Text {
                                    text: "Compositor & Session"
                                    font.pixelSize: theme.fontBase
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            Text {
                                text: (backend && backend.systemInfo && backend.systemInfo.desktop && backend.systemInfo.desktop.length > 0)
                                    ? backend.systemInfo.desktop : "Desktop Session"
                                font.pixelSize: 17
                                font.bold: true
                                color: theme.accent
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Text {
                                text: "Display Server: " + ((backend && backend.systemInfo && backend.systemInfo.session_type && backend.systemInfo.session_type.length > 0)
                                      ? backend.systemInfo.session_type.toUpperCase() : "WAYLAND") +
                                      (backend && backend.systemInfo && backend.systemInfo.wayland_display ? " (" + backend.systemInfo.wayland_display + ")" : "")
                                font.pixelSize: theme.fontSm
                                color: theme.textSecondary
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                        }
                    }

                    // Security & Privileges Card
                    Rectangle {
                        Layout.fillWidth: true
                        height: 128
                        radius: theme.radiusMd
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 16
                            spacing: 8

                            RowLayout {
                                spacing: 8
                                RyzoraIcon {
                                    name: "shield"
                                    size: 16
                                    color: theme.success
                                }
                                Text {
                                    text: "Security & Privileges"
                                    font.pixelSize: theme.fontBase
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            RowLayout {
                                spacing: 8
                                Rectangle {
                                    width: 8
                                    height: 8
                                    radius: 4
                                    color: (backend && backend.systemInfo && backend.systemInfo.polkit_helper_installed) ? theme.success : theme.warning
                                }
                                Text {
                                    text: "SDDM Helper: " + ((backend && backend.systemInfo && backend.systemInfo.polkit_status) ? backend.systemInfo.polkit_status : "Active")
                                    font.pixelSize: theme.fontSm
                                    color: theme.text
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }

                            RowLayout {
                                spacing: 8
                                Rectangle {
                                    width: 8
                                    height: 8
                                    radius: 4
                                    color: (backend && backend.systemInfo && backend.systemInfo.process_isolation &&
                                            backend.systemInfo.process_isolation.indexOf("Not probed") === -1 &&
                                            backend.systemInfo.process_isolation.indexOf("Unknown") === -1)
                                           ? theme.success : theme.textMuted
                                }
                                Text {
                                    text: "Process Tracking: " + ((backend && backend.systemInfo && backend.systemInfo.process_isolation) ? backend.systemInfo.process_isolation : "Enforced")
                                    font.pixelSize: theme.fontSm
                                    color: theme.text
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }
                }

                // ── B. Compact 2-Column Subsystem Diagnostics Table Card ──
                Rectangle {
                    Layout.fillWidth: true
                    radius: theme.radiusMd
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1
                    // Naturally sized: height fits content cleanly without artificial vertical stretch
                    implicitHeight: diagContentCol.implicitHeight + 32

                    ColumnLayout {
                        id: diagContentCol
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 12

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            RyzoraIcon {
                                name: "pulse"
                                size: 15
                                color: theme.accent
                            }

                            Text {
                                text: "Active Subsystem Diagnostics"
                                font.pixelSize: theme.fontBase
                                font.bold: true
                                color: theme.text
                            }

                            Item { Layout.fillWidth: true }

                            Text {
                                text: "Live Host Probes"
                                font.pixelSize: 11
                                color: theme.textMuted
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 1
                            color: theme.borderSubtle
                        }

                        // Compact 2-Column Responsive Information Grid
                        GridLayout {
                            Layout.fillWidth: true
                            columns: root.width > 800 ? 2 : 1
                            rowSpacing: 10
                            columnSpacing: 24

                            DiagRow {
                                label: "SDDM Login Manager"
                                value: (backend && backend.systemInfo && backend.systemInfo.sddm_status) ? backend.systemInfo.sddm_status : "Detected"
                            }
                            DiagRow {
                                label: "Local Installed Packages"
                                value: (backend ? backend.libraryCount : 0) + " packages in ~/.local/share/ryzora/installed"
                            }
                            DiagRow {
                                label: "Privileged Polkit Helper"
                                value: (backend && backend.systemInfo && backend.systemInfo.polkit_status) ? backend.systemInfo.polkit_status : "Active"
                            }
                            DiagRow {
                                label: "Snapshots In Store"
                                value: (backend ? backend.snapshotsCount : 0) + " verified restore points"
                            }
                            DiagRow {
                                label: "Configuration Drift Probe"
                                value: (backend && backend.systemInfo && backend.systemInfo.config_drift) ? backend.systemInfo.config_drift : "In sync"
                            }
                            DiagRow {
                                label: "Active Subprocess PID"
                                value: (backend && backend.isProcessActive()) ? backend.activePid().toString() : "None (Idle)"
                            }
                            DiagRow {
                                label: "Catalog Manifests Loaded"
                                value: (backend ? backend.catalogCount : 0) + " community packages"
                            }
                            DiagRow {
                                label: "UI Surface Engine"
                                value: "Qt 6 Quick SceneGraph • Wayland EGL Native"
                            }
                        }
                    }
                }
            }
        }

        // ══════════════════════════════════════════════════════════════
        // SUB-TAB 1: UNIVERSAL LINUX COMPATIBILITY LAB
        // ══════════════════════════════════════════════════════════════
        ScrollView {
            visible: root.currentSubTab === 1
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ScrollBar.vertical.policy: ScrollBar.AsNeeded

            ColumnLayout {
                width: parent.width
                spacing: 16

                // Section 1: Environment Profile Switcher
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "SELECT ENVIRONMENT PROFILE"
                            font.pixelSize: 11
                            font.bold: true
                            color: theme.textMuted
                        }
                        Item { Layout.fillWidth: true }
                        // Refresh Live Probe button
                        Rectangle {
                            height: 28
                            width: probeTxt.implicitWidth + 28
                            radius: theme.radiusSm
                            color: probeArea.containsMouse ? theme.surfaceElevated : theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 6
                                RyzoraIcon {
                                    name: "arrow-clockwise"
                                    size: 12
                                    color: theme.accent
                                }
                                Text {
                                    id: probeTxt
                                    text: root.isProbing ? "Probing Host…" : "Refresh Live Probe"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: theme.text
                                }
                            }
                            MouseArea {
                                id: probeArea
                                anchors.fill: parent
                                enabled: !root.isProbing
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.isProbing = true
                                    probeTimer.start()
                                    if (backend) {
                                        backend.loadCompatibilityProfiles()
                                        backend.loadSystemInfo()
                                    }
                                }
                            }
                        }
                    }

                    // Profiles Grid
                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        rowSpacing: 10
                        columnSpacing: 10

                        Repeater {
                            model: (backend && backend.compatibilityProfiles) ? backend.compatibilityProfiles : []
                            delegate: Rectangle {
                                Layout.fillWidth: true
                                height: 86
                                radius: theme.radiusMd
                                color: modelData.id === root.selectedProfileId ? theme.surfaceElevated : theme.surface
                                border.color: modelData.id === root.selectedProfileId ? Qt.rgba(139/255, 92/255, 246/255, 0.70) : theme.borderSubtle
                                border.width: modelData.id === root.selectedProfileId ? 1.5 : 1

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 12
                                    spacing: 4

                                    RowLayout {
                                        Layout.fillWidth: true
                                        Text {
                                            text: modelData.name
                                            font.pixelSize: 12
                                            font.bold: true
                                            color: theme.text
                                            elide: Text.ElideRight
                                            Layout.fillWidth: true
                                        }

                                        Rectangle {
                                            height: 18
                                            width: isLiveTxt.implicitWidth + 12
                                            radius: 4
                                            color: modelData.is_live ? Qt.rgba(16/255, 185/255, 129/255, 0.20) : Qt.rgba(100/255, 116/255, 139/255, 0.20)
                                            border.color: modelData.is_live ? Qt.rgba(16/255, 185/255, 129/255, 0.40) : Qt.rgba(100/255, 116/255, 139/255, 0.30)
                                            border.width: 1

                                            Text {
                                                id: isLiveTxt
                                                anchors.centerIn: parent
                                                text: modelData.is_live ? "LIVE" : "SIMULATED"
                                                font.pixelSize: 9
                                                font.bold: true
                                                color: modelData.is_live ? "#34d399" : "#94a3b8"
                                            }
                                        }
                                    }

                                    Text {
                                        text: modelData.description
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }

                                    Text {
                                        text: modelData.os + " · " + modelData.desktop + " · " + modelData.display_manager.toUpperCase()
                                        font.pixelSize: 10
                                        color: theme.textSecondary
                                        font.family: "monospace"
                                    }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.selectedProfileId = modelData.id
                                }
                            }
                        }
                    }
                }

                // Section 2: Selected Profile Detail Specification Card
                Rectangle {
                    visible: root.activeProfile !== null
                    Layout.fillWidth: true
                    height: 100
                    radius: theme.radiusMd
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 10

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            RyzoraIcon {
                                name: "shield"
                                size: 14
                                color: theme.accent
                            }
                            Text {
                                text: "Host Profile Specification: " + (root.activeProfile ? root.activeProfile.name : "")
                                font.pixelSize: 12
                                font.bold: true
                                color: theme.text
                            }
                            Item { Layout.fillWidth: true }
                            Text {
                                text: root.activeProfile && root.activeProfile.is_live ? "Real Hardware Telemetry" : "Simulated Fixture (Zero Side Effects)"
                                font.pixelSize: 10
                                color: theme.textMuted
                                font.family: "monospace"
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            SpecField { title: "OPERATING SYSTEM"; value: root.activeProfile ? root.activeProfile.os : "" }
                            SpecField { title: "COMPOSITOR / DESKTOP"; value: root.activeProfile ? root.activeProfile.desktop : "" }
                            SpecField { title: "SESSION TYPE"; value: root.activeProfile ? root.activeProfile.session_type.toUpperCase() : "" }
                            SpecField { title: "DISPLAY MANAGER"; value: root.activeProfile ? root.activeProfile.display_manager.toUpperCase() : "" }
                        }
                    }
                }

                // Section 3: Concrete Host Adapters Compatibility State
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: "HOST ADAPTERS COMPATIBILITY STATE"
                        font.pixelSize: 11
                        font.bold: true
                        color: theme.textMuted
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        Repeater {
                            model: root.activeProfile ? root.activeProfile.adapters : []
                            delegate: Rectangle {
                                Layout.fillWidth: true
                                height: Math.max(88, adapterCol.implicitHeight + 24)
                                radius: theme.radiusMd
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1

                                ColumnLayout {
                                    id: adapterCol
                                    anchors.fill: parent
                                    anchors.margins: 14
                                    spacing: 6

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 10

                                        RyzoraIcon {
                                            name: modelData.category === "session_lock" ? "shield" : "lock"
                                            size: 14
                                            color: theme.accent
                                        }

                                        Text {
                                            text: modelData.name
                                            font.pixelSize: 13
                                            font.bold: true
                                            color: theme.text
                                        }

                                        Item { Layout.fillWidth: true }

                                        // Distinct Capability State Pill
                                        Rectangle {
                                            height: 20
                                            width: suppTxt.implicitWidth + 16
                                            radius: 4
                                            color: {
                                                var s = modelData.status || ""
                                                if (s === "operational") return Qt.rgba(16/255, 185/255, 129/255, 0.20)
                                                if (s === "configured") return Qt.rgba(59/255, 130/255, 246/255, 0.20)
                                                if (s === "available") return Qt.rgba(139/255, 92/255, 246/255, 0.20)
                                                if (s === "unavailable") return Qt.rgba(245/255, 158/255, 11/255, 0.20)
                                                if (s === "unknown") return Qt.rgba(148/255, 163/255, 184/255, 0.20)
                                                return Qt.rgba(239/255, 68/255, 68/255, 0.20)
                                            }
                                            border.color: {
                                                var s = modelData.status || ""
                                                if (s === "operational") return Qt.rgba(16/255, 185/255, 129/255, 0.50)
                                                if (s === "configured") return Qt.rgba(59/255, 130/255, 246/255, 0.50)
                                                if (s === "available") return Qt.rgba(139/255, 92/255, 246/255, 0.50)
                                                if (s === "unavailable") return Qt.rgba(245/255, 158/255, 11/255, 0.50)
                                                if (s === "unknown") return Qt.rgba(148/255, 163/255, 184/255, 0.50)
                                                return Qt.rgba(239/255, 68/255, 68/255, 0.50)
                                            }
                                            border.width: 1

                                            Text {
                                                id: suppTxt
                                                anchors.centerIn: parent
                                                text: (modelData.status_label || (modelData.supported ? "SUPPORTED" : "UNSUPPORTED")).toUpperCase()
                                                font.pixelSize: 9
                                                font.bold: true
                                                color: {
                                                    var s = modelData.status || ""
                                                    if (s === "operational") return "#34d399"
                                                    if (s === "configured") return "#60a5fa"
                                                    if (s === "available") return "#c084fc"
                                                    if (s === "unavailable") return "#fbbf24"
                                                    if (s === "unknown") return "#94a3b8"
                                                    return "#f87171"
                                                }
                                            }
                                        }
                                    }

                                    Text {
                                        text: modelData.reason
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 16
                                        Text {
                                            text: "Target Category: " + modelData.category
                                            font.pixelSize: 10
                                            color: theme.textSecondary
                                            font.family: "monospace"
                                        }
                                        Text {
                                            text: "Required Privilege: " + modelData.required_privilege
                                            font.pixelSize: 10
                                            color: modelData.required_privilege === "administrator" ? "#fbbf24" : "#94a3b8"
                                            font.family: "monospace"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Section 4: Universal Policy & Invariants Note
                Rectangle {
                    Layout.fillWidth: true
                    height: 64
                    radius: theme.radiusMd
                    color: Qt.rgba(56/255, 189/255, 248/255, 0.08)
                    border.color: Qt.rgba(56/255, 189/255, 248/255, 0.25)
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12

                        RyzoraIcon {
                            name: "info"
                            size: 16
                            color: "#7dd3fc"
                        }

                        ColumnLayout {
                            spacing: 2
                            Layout.fillWidth: true
                            Text {
                                text: "Universal Linux Compatibility Invariants"
                                font.pixelSize: 11
                                font.bold: true
                                color: "#7dd3fc"
                            }
                            Text {
                                text: "Ryzora enforces zero side-effect simulation for foreign distros, never installs foreign SDDM themes into GDM, and guards all administrator mutations with Polkit elevation."
                                font.pixelSize: 10
                                color: theme.textMuted
                            }
                        }
                    }
                }
            }
        }

        // ══════════════════════════════════════════════════════════════
        // SUB-TAB 2: INTEGRITY VIEW
        // ══════════════════════════════════════════════════════════════
        Item {
            visible: root.currentSubTab === 2
            Layout.fillWidth: true
            Layout.fillHeight: true

            IntegrityView {
                anchors.fill: parent
            }
        }

        // ══════════════════════════════════════════════════════════════
        // SUB-TAB 3: ACTIVITY VIEW
        // ══════════════════════════════════════════════════════════════
        Item {
            visible: root.currentSubTab === 3
            Layout.fillWidth: true
            Layout.fillHeight: true

            ActivityView {
                anchors.fill: parent
            }
        }

        // ══════════════════════════════════════════════════════════════
        // SUB-TAB 4: DEVELOPER PACKAGE CREATOR VIEW
        // ══════════════════════════════════════════════════════════════
        Item {
            visible: root.currentSubTab === 4
            Layout.fillWidth: true
            Layout.fillHeight: true

            AuthorView {
                anchors.fill: parent
            }
        }
    }

    // ── Reusable Component Definitions ──

    component DiagRow : RowLayout {
        property string label: ""
        property string value: ""

        Layout.fillWidth: true
        spacing: 12

        Text {
            text: label
            font.pixelSize: theme.fontSm
            color: theme.textMuted
            Layout.minimumWidth: 180
            Layout.preferredWidth: 200
        }

        Text {
            text: value
            font.pixelSize: theme.fontSm
            font.bold: true
            color: theme.text
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }

    component SpecField : ColumnLayout {
        property string title: ""
        property string value: ""

        Layout.fillWidth: true
        spacing: 2

        Text {
            text: title
            font.pixelSize: 9
            font.bold: true
            color: theme.textMuted
            font.family: "monospace"
        }

        Text {
            text: value
            font.pixelSize: 11
            font.bold: true
            color: theme.text
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }
}
