import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property int currentSubTab: 0 // 0: Diagnostics, 1: Compatibility Lab
    property string selectedProfileId: "live"
    property bool isProbing: false

    Timer {
        id: probeTimer
        interval: 1200
        repeat: false
        onTriggered: root.isProbing = false
    }

    // Helper to find selected profile object
    readonly property var activeProfile: {
        var profiles = backend.compatibilityProfiles || []
        for (var i = 0; i < profiles.length; i++) {
            if (profiles[i].id === selectedProfileId) {
                return profiles[i]
            }
        }
        return profiles.length > 0 ? profiles[0] : null
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        // ── Navigation Header & Sub-Tab Switcher ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                spacing: 2
                Layout.fillWidth: true
                Text {
                    text: root.currentSubTab === 0 ? "Host System & Diagnostics" : "Universal Linux Compatibility Lab"
                    font.pixelSize: theme.fontLg
                    font.bold: true
                    color: theme.text
                }
                Text {
                    text: root.currentSubTab === 0 
                        ? "Real-time inspection of Garuda Linux, Hyprland compositor, SDDM, and Polkit boundaries"
                        : "Capability matrix debugger for host environments against runtime display adapters"
                    font.pixelSize: theme.fontSm
                    color: theme.textMuted
                }
            }

            // Sub-Tab Switcher Pills
            Rectangle {
                height: 36
                width: tabRow.implicitWidth + 8
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.border
                border.width: 1

                RowLayout {
                    id: tabRow
                    anchors.centerIn: parent
                    spacing: 4

                    Rectangle {
                        height: 28
                        width: diagTabTxt.implicitWidth + 24
                        radius: theme.radiusSm
                        color: root.currentSubTab === 0 ? theme.surfaceElevated : "transparent"
                        border.color: root.currentSubTab === 0 ? theme.borderStrong : "transparent"
                        border.width: 1

                        Text {
                            id: diagTabTxt
                            anchors.centerIn: parent
                            text: "Host Diagnostics"
                            font.pixelSize: 11
                            font.bold: root.currentSubTab === 0
                            color: root.currentSubTab === 0 ? theme.text : theme.textMuted
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.currentSubTab = 0
                        }
                    }

                    Rectangle {
                        height: 28
                        width: labTabTxt.implicitWidth + 24
                        radius: theme.radiusSm
                        color: root.currentSubTab === 1 ? Qt.rgba(139/255, 92/255, 246/255, 0.20) : "transparent"
                        border.color: root.currentSubTab === 1 ? Qt.rgba(139/255, 92/255, 246/255, 0.50) : "transparent"
                        border.width: 1

                        Text {
                            id: labTabTxt
                            anchors.centerIn: parent
                            text: "⚗ Compatibility Lab"
                            font.pixelSize: 11
                            font.bold: root.currentSubTab === 1
                            color: root.currentSubTab === 1 ? "#c4b5fd" : theme.textMuted
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.currentSubTab = 1
                        }
                    }
                }
            }
        }

        // ══════════════════════════════════════════════════════════════
        // SUB-TAB 0: HOST DIAGNOSTICS VIEW
        // ══════════════════════════════════════════════════════════════
        ColumnLayout {
            visible: root.currentSubTab === 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 16

            RowLayout {
                Layout.fillWidth: true
                spacing: 16

                // Host OS Card
                Rectangle {
                    Layout.fillWidth: true
                    height: 140
                    radius: theme.radiusMd
                    color: theme.surface
                    border.color: theme.border

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 8

                        RowLayout {
                            spacing: 8
                            Text { text: "🐧"; font.pixelSize: 18 }
                            Text {
                                text: "Operating System"
                                font.pixelSize: theme.fontBase
                                font.bold: true
                                color: theme.text
                            }
                        }

                        Text {
                            text: backend.systemInfo.os_name || "Garuda Linux"
                            font.pixelSize: theme.fontLg
                            font.bold: true
                            color: theme.accent
                        }

                        Text {
                            text: "Kernel: " + (backend.systemInfo.kernel || "Linux 6.x")
                            font.pixelSize: theme.fontSm
                            color: theme.textSecondary
                        }
                    }
                }

                // Compositor Card
                Rectangle {
                    Layout.fillWidth: true
                    height: 140
                    radius: theme.radiusMd
                    color: theme.surface
                    border.color: theme.border

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 8

                        RowLayout {
                            spacing: 8
                            Text { text: "🪟"; font.pixelSize: 18 }
                            Text {
                                text: "Compositor & Session"
                                font.pixelSize: theme.fontBase
                                font.bold: true
                                color: theme.text
                            }
                        }

                        Text {
                            text: backend.systemInfo.desktop || "Hyprland"
                            font.pixelSize: theme.fontLg
                            font.bold: true
                            color: theme.accent
                        }

                        Text {
                            text: "Wayland Display: " + (backend.systemInfo.wayland_display || "wayland-1")
                            font.pixelSize: theme.fontSm
                            color: theme.textSecondary
                        }
                    }
                }

                // Integration Boundaries Card
                Rectangle {
                    Layout.fillWidth: true
                    height: 140
                    radius: theme.radiusMd
                    color: theme.surface
                    border.color: theme.border

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 8

                        RowLayout {
                            spacing: 8
                            Text { text: "🛡️"; font.pixelSize: 18 }
                            Text {
                                text: "Security & Privileges"
                                font.pixelSize: theme.fontBase
                                font.bold: true
                                color: theme.text
                            }
                        }

                        RowLayout {
                            spacing: 6
                            Rectangle {
                                width: 8
                                height: 8
                                radius: 4
                                color: (backend.systemInfo.polkit_helper_installed) ? theme.success : theme.warning
                            }
                            Text {
                                text: "SDDM Helper: " + (backend.systemInfo.polkit_status || "Probing...")
                                font.pixelSize: theme.fontSm
                                color: theme.text
                            }
                        }

                        RowLayout {
                            spacing: 6
                            Rectangle {
                                width: 8
                                height: 8
                                radius: 4
                                color: (backend.systemInfo.process_isolation && backend.systemInfo.process_isolation.indexOf("Not probed") === -1 && backend.systemInfo.process_isolation.indexOf("Unknown") === -1) ? theme.success : theme.textMuted
                            }
                            Text {
                                text: "Process Tracking: " + (backend.systemInfo.process_isolation || "Not probed")
                                font.pixelSize: theme.fontSm
                                color: theme.text
                            }
                        }
                    }
                }
            }

            // Detailed Metrics Table Card
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.border

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 12

                    Text {
                        text: "Active Subsystem Diagnostics"
                        font.pixelSize: theme.fontBase
                        font.bold: true
                        color: theme.text
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    DiagRow { label: "SDDM Login Manager"; value: backend.systemInfo.sddm_status || "Detected" }
                    DiagRow { label: "Privileged Polkit Helper"; value: backend.systemInfo.polkit_status || "Probing..." }
                    DiagRow { label: "Configuration Drift Probe"; value: backend.systemInfo.config_drift || "In sync" }
                    DiagRow { label: "Catalog Manifests Loaded"; value: backend.catalogCount + " community packages" }
                    DiagRow { label: "Local Installed Packages"; value: backend.libraryCount + " packages in ~/.local/share/ryzora/installed" }
                    DiagRow { label: "Snapshots In Store"; value: backend.snapshotsCount + " verified restore points" }
                    DiagRow { label: "Active Subprocess PID"; value: backend.isProcessActive() ? backend.activePid().toString() : "None (Idle)" }
                    DiagRow { label: "UI Surface Engine"; value: "Qt 6 Quick SceneGraph • Wayland EGL Native" }

                    Item { Layout.fillHeight: true }
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
                            height: 26
                            width: probeTxt.implicitWidth + 24
                            radius: theme.radiusSm
                            color: probeArea.containsMouse ? theme.surfaceElevated : theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 6
                                Text { text: root.isProbing ? "⌛" : "↻"; font.pixelSize: 11; color: "#c4b5fd" }
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
                                    backend.loadCompatibilityProfiles()
                                    backend.loadSystemInfo()
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
                            model: backend.compatibilityProfiles || []
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
                            Text { text: "🛡️"; font.pixelSize: 14 }
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

                                        Text {
                                            text: modelData.category === "session_lock" ? "🛡️" : "🔒"
                                            font.pixelSize: 14
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

                        Text { text: "ℹ️"; font.pixelSize: 16 }

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

                Item { height: 16 }
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
            Layout.minimumWidth: 200
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
