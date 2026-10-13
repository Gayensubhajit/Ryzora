import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    // Dynamic session events model
    property var activityLog: []

    function addEvent(icon, title, detail) {
        var now = new Date()
        var timeStr = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
        var item = {
            time: timeStr,
            icon: icon,
            title: title,
            detail: detail
        }
        var newArr = [item].concat(root.activityLog)
        root.activityLog = newArr
    }

    Component.onCompleted: {
        root.activityLog = []
    }

    Connections {
        target: backend
        function onNotificationPosted(type, title, message) {
            var icon = type === "success" ? "✓" : (type === "error" ? "✕" : "ℹ️")
            root.addEvent(icon, title, message)
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        // ── 1. Page Header ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "Session Activity"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Current session event log (in-memory) · " + root.activityLog.length + " events recorded · Not persisted to disk"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            Button {
                text: "Clear Session Log"
                enabled: root.activityLog.length > 0
                flat: true
                onClicked: {
                    root.activityLog = []
                }
            }
        }

        // ── 2. Real Timeline List ──
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 10
            visible: root.activityLog.length > 0

            model: root.activityLog

            delegate: Rectangle {
                width: ListView.view.width
                implicitHeight: itemCol.implicitHeight + 24
                radius: theme.radiusMd
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    id: itemCol
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 14

                    Rectangle {
                        width: 36
                        height: 36
                        radius: 18
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: modelData.icon
                            font.pixelSize: 16
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            spacing: 8
                            Text {
                                text: modelData.title
                                font.pixelSize: 13
                                font.bold: true
                                color: theme.text
                            }
                            Item { Layout.fillWidth: true }
                            Text {
                                text: modelData.time
                                font.pixelSize: 10
                                font.family: "monospace"
                                color: theme.textMuted
                            }
                        }

                        Text {
                            text: modelData.detail
                            font.pixelSize: 11
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }
                }
            }
        }

        // Empty State
        EmptyState {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.activityLog.length === 0
            title: "No Activity In Current Session"
            message: "The session activity log is clear. Events from snapshots, settings changes, and manifest exports will appear here as they occur in this session."
            actionLabel: "Record Test Event"
            onActionClicked: {
                root.addEvent("ℹ️", "Manual Audit Check", "User verified session log responsiveness.")
            }
        }
    }
}
