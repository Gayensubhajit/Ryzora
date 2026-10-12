import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16

        // ── 1. Page Header ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "Activity & Audit Log"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Chronological audit timeline of package operations, configuration updates, and restore points"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            Button {
                text: "Clear Log"
                flat: true
            }
        }

        // ── 2. Timeline List ──
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 10

            model: [
                {
                    time: "Today, 12:05 PM",
                    type: "snapshot",
                    icon: "🛡️",
                    title: "Automatic Snapshot Created",
                    detail: "Created backup restore point (30 snapshots total) before transactional package state mutation."
                },
                {
                    time: "Today, 11:42 AM",
                    type: "diagnostics",
                    icon: "💻",
                    title: "Host Capabilities Probed",
                    detail: "Verified Hyprland compositor environment, SDDM privileged helper presence, and ext-session-lock-v1 protocol."
                },
                {
                    time: "Yesterday, 06:15 PM",
                    type: "repo",
                    icon: "🗄️",
                    title: "Repository Index Synced",
                    detail: "Successfully updated Official Ryzora Community repository cache (87 catalog packages verified)."
                },
                {
                    time: "Oct 11, 09:30 PM",
                    type: "settings",
                    icon: "⚙️",
                    title: "Appearance Settings Updated",
                    detail: "Theme preferences saved to appearance.json (Ocean Blue accent, Dark Mode, High Contrast intact)."
                }
            ]

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
    }
}
