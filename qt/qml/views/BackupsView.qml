import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    property string pendingAction: ""
    property string pendingSnapshotId: ""
    property string pendingSnapshotLabel: ""

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

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
                    text: "Real filesystem restore points stored in ~/.local/share/ryzora/snapshots/ capturing configuration state, integration drop-ins, and installed package records."
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
                        Text { text: "+"; font.pixelSize: 14; font.bold: true; color: "#0a0a0c" }
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
                        Text {
                            text: backend.isSnapshotsLoading ? "Scanning…" : "↻ Refresh"
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
                iconName: "package"
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
                    width: snapshotList.width
                    height: 78
                    radius: theme.radiusMd
                    color: itemMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 14

                        Rectangle {
                            width: 36
                            height: 36
                            radius: theme.radiusMd
                            color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.12)
                            border.color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.3)

                            Text {
                                anchors.centerIn: parent
                                text: "📦"
                                font.pixelSize: 16
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
                                    Layout.maximumWidth: 260
                                }
                                Text { text: "•"; font.pixelSize: 10; color: theme.textFaint }
                                Text {
                                    text: "Date: " + modelData.formatted_date
                                    font.pixelSize: 11
                                    color: theme.textSecondary
                                }
                                Text { text: "•"; font.pixelSize: 10; color: theme.textFaint }
                                Text {
                                    text: modelData.entries_count + " files backed up"
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

                            Text {
                                anchors.centerIn: parent
                                text: "↻ Restore"
                                font.pixelSize: 11
                                font.bold: true
                                color: restoreMouse.containsMouse ? theme.success : theme.text
                            }

                            MouseArea {
                                id: restoreMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    pendingAction = "restore"
                                    pendingSnapshotId = modelData.id
                                    pendingSnapshotLabel = modelData.label
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

                            Text {
                                anchors.centerIn: parent
                                text: "🗑"
                                font.pixelSize: 12
                                color: deleteMouse.containsMouse ? theme.error : theme.textMuted
                            }

                            MouseArea {
                                id: deleteMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    pendingAction = "delete"
                                    pendingSnapshotId = modelData.id
                                    pendingSnapshotLabel = modelData.label
                                    confirmModal.visible = true
                                }
                            }
                        }
                    }

                    MouseArea {
                        id: itemMouse
                        anchors.fill: parent
                        hoverEnabled: true
                    }
                }

                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            }
        }
    }

    // ── Confirmation Modal Overlay ──
    Rectangle {
        id: confirmModal
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, 0.70)
        visible: false
        z: 100

        MouseArea {
            anchors.fill: parent
            // block clicks to underlay
        }

        Rectangle {
            anchors.centerIn: parent
            width: 460
            height: 200
            radius: theme.radiusMd
            color: theme.surfaceElevated
            border.color: theme.borderStrong
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 12

                RowLayout {
                    spacing: 8
                    Text {
                        text: pendingAction === "restore" ? "↺" : "🗑"
                        font.pixelSize: 18
                    }
                    Text {
                        text: pendingAction === "restore" ? "Confirm Snapshot Restoration" : "Confirm Snapshot Deletion"
                        font.pixelSize: theme.fontBase
                        font.bold: true
                        color: theme.text
                    }
                }

                Text {
                    text: pendingAction === "restore"
                        ? "Are you sure you want to restore snapshot '" + pendingSnapshotLabel + "'?\n\nAll stored files will be cryptographically verified with SHA-256 before restoration."
                        : "Are you sure you want to permanently delete snapshot '" + pendingSnapshotLabel + "'?\n\nThis backup data cannot be recovered once deleted."
                    font.pixelSize: 11
                    color: theme.textSecondary
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
                        width: 90
                        height: 32
                        radius: theme.radiusSm
                        color: pendingAction === "restore" ? theme.accent : theme.error
                        Text {
                            anchors.centerIn: parent
                            text: pendingAction === "restore" ? "Restore" : "Delete"
                            font.pixelSize: 11
                            font.bold: true
                            color: pendingAction === "restore" ? "#0a0a0c" : "#ffffff"
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                confirmModal.visible = false;
                                if (pendingAction === "restore") {
                                    backend.restoreSnapshot(pendingSnapshotId);
                                } else if (pendingAction === "delete") {
                                    backend.deleteSnapshot(pendingSnapshotId);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
