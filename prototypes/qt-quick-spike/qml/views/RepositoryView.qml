import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property string selectedChannel: "stable"

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 18

        // ── 1. Page Header ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "Package Repositories"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Manage upstream package sources, release channels, and cryptographic index caches"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            Button {
                text: "⟳ Refresh All Repositories"
                onClicked: {
                    backend.filterCatalog("all", "")
                }
            }
        }

        // ── 2. Official Community Repository Card ──
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: repoCol.implicitHeight + 32
            radius: theme.radiusLg
            color: theme.surfaceElevated
            border.color: theme.border
            border.width: 1

            ColumnLayout {
                id: repoCol
                anchors.fill: parent
                anchors.margins: 18
                spacing: 14

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    Rectangle {
                        width: 36
                        height: 36
                        radius: theme.radiusSm
                        color: theme.accentFaint
                        border.color: theme.accent
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: "🗄️"
                            font.pixelSize: 18
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            spacing: 8
                            Text {
                                text: "Official Ryzora Community Repository"
                                font.pixelSize: 15
                                font.bold: true
                                color: theme.text
                            }
                            Rectangle {
                                height: 18
                                width: statusTxt.implicitWidth + 10
                                radius: 4
                                color: "#1e3a2f"
                                border.color: "#10b981"
                                border.width: 1

                                Text {
                                    id: statusTxt
                                    anchors.centerIn: parent
                                    text: "ONLINE · VERIFIED"
                                    font.pixelSize: 9
                                    font.bold: true
                                    font.family: "monospace"
                                    color: "#6ee7b7"
                                }
                            }
                        }

                        Text {
                            text: "https://raw.githubusercontent.com/Gayensubhajit/Ryzora-Packages/main"
                            font.pixelSize: 11
                            font.family: "monospace"
                            color: theme.textMuted
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: theme.borderSubtle
                }

                // Channel Selector Row
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 16

                    Text {
                        text: "Release Channel:"
                        font.pixelSize: 12
                        font.bold: true
                        color: theme.text
                    }

                    RowLayout {
                        spacing: 8

                        Repeater {
                            model: ["stable", "beta", "edge"]
                            delegate: Rectangle {
                                property bool isSelected: root.selectedChannel === modelData
                                implicitWidth: chLabel.implicitWidth + 20
                                implicitHeight: 28
                                radius: theme.radiusSm
                                color: isSelected ? theme.accent : theme.surface
                                border.color: isSelected ? theme.accent : theme.borderSubtle
                                border.width: 1

                                Text {
                                    id: chLabel
                                    anchors.centerIn: parent
                                    text: modelData.toUpperCase()
                                    font.pixelSize: 10
                                    font.bold: true
                                    font.family: "monospace"
                                    color: isSelected ? "#ffffff" : theme.textSecondary
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.selectedChannel = modelData
                                }
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: "Index: 87 packages · Cache: 142 KB"
                        font.pixelSize: 11
                        font.family: "monospace"
                        color: theme.textMuted
                    }
                }
            }
        }

        // ── 3. Local Override & Development Repositories ──
        Text {
            text: "LOCAL OVERRIDES"
            font.pixelSize: 11
            font.bold: true
            font.letterSpacing: 1.0
            color: theme.navSectionLabel
            Layout.topMargin: 8
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 80
            radius: theme.radiusLg
            color: theme.surface
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 14

                Text { text: "📁"; font.pixelSize: 20 }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: "Development Workspace Repository (Local Override)"
                        font.pixelSize: 13
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Path: repositories/community/ (Linked to active workspace catalog)"
                        font.pixelSize: 11
                        font.family: "monospace"
                        color: theme.textMuted
                    }
                }

                Rectangle {
                    height: 22
                    width: 64
                    radius: 4
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    Text {
                        anchors.centerIn: parent
                        text: "ACTIVE"
                        font.pixelSize: 9
                        font.bold: true
                        font.family: "monospace"
                        color: theme.textSecondary
                    }
                }
            }
        }

        Item { Layout.fillHeight: true }
    }
}
