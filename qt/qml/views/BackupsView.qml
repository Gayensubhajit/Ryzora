import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root
    objectName: "backupsView"

    property string pendingAction: ""
    property string pendingSnapshotId: ""
    property string pendingSnapshotLabel: ""
    property string pendingSnapshotDate: ""
    property string pendingSnapshotSize: ""
    property int pendingSnapshotEntries: 0
    property string pendingSnapshotStatus: ""

    property string statusBannerText: ""
    property string statusBannerType: "info"

    function formatBytes(bytes) {
        if (!bytes || bytes <= 0) return "0 B"
        var k = 1024
        var sizes = ["B", "KB", "MB", "GB"]
        var i = Math.floor(Math.log(bytes) / Math.log(k))
        if (i < 0) i = 0
        if (i >= sizes.length) i = sizes.length - 1
        return (bytes / Math.pow(k, i)).toFixed(1) + " " + sizes[i]
    }

    readonly property real totalStorageBytes: {
        var total = 0
        var list = backend.snapshotsList || []
        for (var i = 0; i < list.length; ++i) {
            if (list[i].total_size_bytes) total += list[i].total_size_bytes
        }
        return total
    }

    Connections {
        target: backend
        function onSnapshotDeleted(id, success, message) {
            root.statusBannerText = message
            root.statusBannerType = success ? "success" : "error"
            bannerTimer.restart()
        }
    }

    Timer {
        id: bannerTimer
        interval: 6000
        onTriggered: root.statusBannerText = ""
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 14

        // ── Header Row ──
        RowLayout {
            Layout.fillWidth: true

            ColumnLayout {
                spacing: 2
                Text {
                    text: "Configuration Snapshots (" + backend.snapshotsCount + ")"
                    font.pixelSize: theme.fontLg
                    font.bold: true
                    color: theme.text
                }
                Text {
                    text: "Filesystem restore points capturing configuration state, integration drop-ins, and installed package records."
                    font.pixelSize: theme.fontSm
                    color: theme.textMuted
                }
            }

            Item { Layout.fillWidth: true }

            RowLayout {
                spacing: 8

                // Create Snapshot Button
                Rectangle {
                    width: 140
                    height: 34
                    radius: theme.radiusMd
                    color: createMouse.containsMouse ? theme.accentHover : theme.accent
                    border.color: theme.accent
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6
                        RyzoraIcon {
                            name: "plus"
                            size: 13
                            color: "#0a0a0c"
                        }
                        Text {
                            text: "Create Snapshot"
                            font.pixelSize: theme.fontSm
                            font.bold: true
                            color: "#0a0a0c"
                        }
                    }

                    MouseArea {
                        id: createMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            backend.createSnapshot("Manual snapshot from Qt UI")
                        }
                    }
                }

                // Refresh Button
                Rectangle {
                    width: 100
                    height: 34
                    radius: theme.radiusMd
                    color: backend.isSnapshotsLoading ? theme.surfaceElevated : (refreshArea.containsMouse ? theme.surfaceHover : theme.surface)
                    border.color: theme.border
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6
                        RyzoraIcon {
                            name: "refresh"
                            size: 13
                            color: theme.textSecondary
                        }
                        Text {
                            text: backend.isSnapshotsLoading ? "Scanning…" : "Refresh"
                            font.pixelSize: theme.fontSm
                            font.bold: true
                            color: theme.text
                        }
                    }

                    MouseArea {
                        id: refreshArea
                        anchors.fill: parent
                        enabled: !backend.isSnapshotsLoading
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: backend.loadSnapshots()
                    }
                }
            }
        }

        // ── Storage Summary Bar ──
        Rectangle {
            Layout.fillWidth: true
            height: 44
            radius: theme.radiusMd
            color: theme.surfaceElevated
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 16

                RyzoraIcon {
                    name: "hard-drive"
                    size: 15
                    color: theme.accent
                }

                Text {
                    text: "Total Snapshots: " + backend.snapshotsCount + "  ·  Storage Used: " + root.formatBytes(root.totalStorageBytes)
                    font.pixelSize: 12
                    font.bold: true
                    color: theme.text
                }

                Text { text: "•"; color: theme.borderStrong }

                Text {
                    text: "Path: ~/.local/share/ryzora/snapshots/"
                    font.pixelSize: 11
                    font.family: "monospace"
                    color: theme.textMuted
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }
        }

        // ── Status Banner (if active) ──
        Rectangle {
            visible: root.statusBannerText.length > 0
            Layout.fillWidth: true
            height: 36
            radius: theme.radiusSm
            color: root.statusBannerType === "success" ? Qt.rgba(16/255, 185/255, 129/255, 0.15) : Qt.rgba(239/255, 68/255, 68/255, 0.15)
            border.color: root.statusBannerType === "success" ? theme.success : theme.error
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 8

                RyzoraIcon {
                    name: root.statusBannerType === "success" ? "check-circle" : "alert"
                    size: 14
                    color: root.statusBannerType === "success" ? theme.success : theme.error
                }

                Text {
                    text: root.statusBannerText
                    font.pixelSize: 12
                    color: theme.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }
        }

        // ── Snapshots List ──
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.border
            border.width: 1

            // Empty State
            EmptyState {
                anchors.centerIn: parent
                visible: backend.snapshotsCount === 0 && !backend.isSnapshotsLoading
                iconName: "archive"
                title: "No Snapshots Created"
                message: "Create a system snapshot to preserve your active configuration, integration drop-ins, and installed packages."
                actionLabel: "Create First Snapshot"
                onActionClicked: backend.createSnapshot("Initial manual snapshot")
            }

            ListView {
                id: snapshotList
                anchors.fill: parent
                anchors.margins: 12
                spacing: 10
                clip: true
                visible: backend.snapshotsCount > 0
                model: backend.snapshotsList

                delegate: Rectangle {
                    id: delegateRect
                    width: snapshotList.width
                    height: 78
                    radius: theme.radiusMd
                    color: itemMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    // Background hover detector with z: 0
                    MouseArea {
                        id: itemMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        z: 0
                    }

                    // Content layout on top with z: 1
                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 14
                        z: 1

                        Rectangle {
                            width: 36
                            height: 36
                            radius: theme.radiusMd
                            color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.12)
                            border.color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.3)

                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "archive"
                                size: 18
                                color: theme.accent
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: modelData.label
                                    font.pixelSize: theme.fontBase
                                    font.bold: true
                                    color: theme.text
                                    elide: Text.ElideRight
                                    Layout.maximumWidth: 400
                                }
                                Rectangle {
                                    height: 18
                                    width: statusText.implicitWidth + 12
                                    radius: theme.radiusSm
                                    color: modelData.verified ? theme.successBg : (modelData.status === "Corrupted" ? Qt.rgba(239/255, 68/255, 68/255, 0.2) : theme.surfaceElevated)
                                    border.color: modelData.verified ? theme.success : (modelData.status === "Corrupted" ? theme.error : theme.border)
                                    Text {
                                        id: statusText
                                        anchors.centerIn: parent
                                        text: (modelData.verified ? "✓ " : (modelData.status === "Corrupted" ? "✗ " : "• ")) + modelData.status
                                        font.pixelSize: 9
                                        font.bold: true
                                        color: modelData.verified ? theme.success : (modelData.status === "Corrupted" ? theme.error : theme.textSecondary)
                                    }
                                }
                            }

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: "ID: " + modelData.id
                                    font.pixelSize: 10
                                    color: theme.textFaint
                                    font.family: "monospace"
                                    elide: Text.ElideRight
                                    Layout.maximumWidth: 240
                                }
                                Text { text: "•"; font.pixelSize: 10; color: theme.textFaint }
                                Text {
                                    text: modelData.formatted_date
                                    font.pixelSize: 11
                                    color: theme.textSecondary
                                }
                                Text { text: "•"; font.pixelSize: 10; color: theme.textFaint }
                                Text {
                                    text: modelData.entries_count + " files (" + root.formatBytes(modelData.total_size_bytes || 0) + ")"
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                }
                            }
                        }

                        // Restore Action Button
                        Rectangle {
                            width: 90
                            height: 30
                            radius: theme.radiusSm
                            color: restoreMouse.containsMouse ? Qt.rgba(16/255, 185/255, 129/255, 0.25) : theme.surface
                            border.color: restoreMouse.containsMouse ? theme.success : theme.borderStrong
                            border.width: 1

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 4
                                RyzoraIcon {
                                    name: "refresh"
                                    size: 11
                                    color: restoreMouse.containsMouse ? theme.success : theme.text
                                }
                                Text {
                                    text: "Restore"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: restoreMouse.containsMouse ? theme.success : theme.text
                                }
                            }

                            MouseArea {
                                id: restoreMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.pendingAction = "restore"
                                    root.pendingSnapshotId = modelData.id
                                    root.pendingSnapshotLabel = modelData.label
                                    root.pendingSnapshotDate = modelData.formatted_date || ""
                                    root.pendingSnapshotSize = root.formatBytes(modelData.total_size_bytes || 0)
                                    root.pendingSnapshotEntries = modelData.entries_count || 0
                                    root.pendingSnapshotStatus = modelData.status || ""
                                    confirmModal.visible = true
                                }
                            }
                        }

                        // Delete Action Button
                        Rectangle {
                            width: 32
                            height: 30
                            radius: theme.radiusSm
                            color: deleteMouse.containsMouse ? Qt.rgba(239/255, 68/255, 68/255, 0.20) : theme.surface
                            border.color: deleteMouse.containsMouse ? theme.error : theme.border
                            border.width: 1

                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "trash"
                                size: 13
                                color: deleteMouse.containsMouse ? theme.error : theme.textMuted
                            }

                            MouseArea {
                                id: deleteMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.pendingAction = "delete"
                                    root.pendingSnapshotId = modelData.id
                                    root.pendingSnapshotLabel = modelData.label
                                    root.pendingSnapshotDate = modelData.formatted_date || ""
                                    root.pendingSnapshotSize = root.formatBytes(modelData.total_size_bytes || 0)
                                    root.pendingSnapshotEntries = modelData.entries_count || 0
                                    root.pendingSnapshotStatus = modelData.status || ""
                                    confirmModal.visible = true
                                }
                            }
                        }
                    }
                }

                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            }
        }
    }

    // ── Confirmation Modal Overlay ──
    Rectangle {
        id: confirmModal
        objectName: "confirmModal"
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, 0.65)
        visible: false
        z: 999

        MouseArea {
            anchors.fill: parent
            onClicked: {} // Block underlying clicks
        }

        Rectangle {
            anchors.centerIn: parent
            width: 440
            height: 290
            radius: theme.radiusLg
            color: theme.surfaceElevated
            border.color: theme.borderStrong
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 12

                RowLayout {
                    spacing: 10
                    RyzoraIcon {
                        name: root.pendingAction === "restore" ? "refresh" : "trash"
                        size: 20
                        color: root.pendingAction === "restore" ? theme.accent : theme.error
                    }
                    Text {
                        text: root.pendingAction === "restore" ? "Confirm Snapshot Restoration" : "Confirm Snapshot Deletion"
                        font.pixelSize: theme.fontBase
                        font.bold: true
                        color: theme.text
                    }
                }

                // Details Card Inside Modal
                Rectangle {
                    Layout.fillWidth: true
                    height: 90
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 4

                        Text {
                            text: "Snapshot: " + root.pendingSnapshotLabel
                            font.pixelSize: 12
                            font.bold: true
                            color: theme.text
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                        Text {
                            text: "ID: " + root.pendingSnapshotId
                            font.pixelSize: 10
                            font.family: "monospace"
                            color: theme.textMuted
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                        RowLayout {
                            spacing: 12
                            Text {
                                text: "Date: " + root.pendingSnapshotDate
                                font.pixelSize: 11
                                color: theme.textSecondary
                            }
                            Text { text: "•"; color: theme.textFaint }
                            Text {
                                text: "Size: " + root.pendingSnapshotSize + " (" + root.pendingSnapshotEntries + " files)"
                                font.pixelSize: 11
                                color: theme.textSecondary
                            }
                        }
                    }
                }

                Text {
                    text: root.pendingAction === "restore"
                        ? "Restoration will cryptographically verify all stored files with SHA-256 before applying. Existing files will be staged safely before overwriting."
                        : "Warning: Permanently deleting this snapshot removes its stored file archives and cryptographic metadata. This action cannot be undone."
                    font.pixelSize: 11
                    color: root.pendingAction === "restore" ? theme.textSecondary : theme.textMuted
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                Item { Layout.fillHeight: true }

                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 10

                    Rectangle {
                        width: 80
                        height: 32
                        radius: theme.radiusSm
                        color: theme.surface
                        border.color: theme.border
                        border.width: 1
                        Text {
                            anchors.centerIn: parent
                            text: "Cancel"
                            font.pixelSize: 11
                            color: theme.textMuted
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: confirmModal.visible = false
                        }
                    }

                    Rectangle {
                        width: 130
                        height: 32
                        radius: theme.radiusSm
                        color: root.pendingAction === "restore" ? theme.accent : theme.error

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon {
                                name: root.pendingAction === "restore" ? "refresh" : "trash"
                                size: 12
                                color: root.pendingAction === "restore" ? "#0a0a0c" : "#ffffff"
                            }
                            Text {
                                text: root.pendingAction === "restore" ? "Restore" : "Delete"
                                font.pixelSize: 11
                                font.bold: true
                                color: root.pendingAction === "restore" ? "#0a0a0c" : "#ffffff"
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                confirmModal.visible = false
                                if (root.pendingAction === "restore") {
                                    backend.restoreSnapshot(root.pendingSnapshotId)
                                } else if (root.pendingAction === "delete") {
                                    backend.deleteSnapshot(root.pendingSnapshotId)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
