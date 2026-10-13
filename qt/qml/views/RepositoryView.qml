import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property string selectedChannel: "stable"
    property string refreshNotice: ""

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
                    text: "Package Repositories"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Upstream repository sources, release channels, and local catalog cache"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            Button {
                text: "⟳ Reload Local Catalog Index"
                onClicked: {
                    backend.filterCatalog("all", "")
                    root.refreshNotice = "Reloaded local catalog index (" + backend.catalogCount + " packages in local cache). Note: Remote HTTPS sync is not connected for local repositories."
                }
            }
        }

        // Status Notice Banner if refreshed
        Rectangle {
            visible: root.refreshNotice.length > 0
            Layout.fillWidth: true
            implicitHeight: 36
            radius: theme.radiusSm
            color: theme.surface
            border.color: theme.accent
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 8
                Text { text: "ℹ️"; font.pixelSize: 12 }
                Text {
                    text: root.refreshNotice
                    font.pixelSize: 11
                    color: theme.text
                    Layout.fillWidth: true
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
                                    text: "LOCAL CACHE · LOADED"
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
                                    onClicked: {
                                        root.selectedChannel = modelData
                                        if (modelData !== "stable") {
                                            root.refreshNotice = "Channel '" + modelData + "' preview: upstream remote sync is inactive; loaded packages remain from local repository."
                                        } else {
                                            root.refreshNotice = ""
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: "Index: " + backend.catalogCount + " packages loaded"
                        font.pixelSize: 11
                        font.family: "monospace"
                        color: theme.textSecondary
                    }
                }
            }
        }

        // ── 3. Local Workspace Override ──
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
            implicitHeight: 74
            radius: theme.radiusLg
            color: theme.surface
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.margins: 14
                spacing: 14

                Text { text: "📁"; font.pixelSize: 20 }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: "Workspace Repository Catalog"
                        font.pixelSize: 13
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Path: repositories/community/ (Ingested via ryzora-core crate)"
                        font.pixelSize: 11
                        font.family: "monospace"
                        color: theme.textMuted
                    }
                }

                Rectangle {
                    height: 22
                    width: 72
                    radius: 4
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    Text {
                        anchors.centerIn: parent
                        text: "INGESTED"
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
