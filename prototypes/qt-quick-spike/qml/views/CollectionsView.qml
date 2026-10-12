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
        ColumnLayout {
            spacing: 2
            Layout.fillWidth: true

            Text {
                text: "Collections"
                font.pixelSize: 22
                font.bold: true
                color: theme.text
            }

            Text {
                text: "Curated package suites, desktop rices, and multi-component theme bundles"
                font.pixelSize: 13
                color: theme.textSecondary
            }
        }

        // ── 2. Collections Grid ──
        GridView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            cellWidth: Math.floor(width / (width >= 900 ? 3 : (width >= 600 ? 2 : 1)))
            cellHeight: 220

            model: [
                {
                    title: "Garuda Dr460nized Suite",
                    tag: "OFFICIAL BUNDLE",
                    itemsCount: "4 Components",
                    desc: "The complete iconic Garuda dragonized gaming look: Sweet-Dark theme, BeautyLine icons, and Dragonized SDDM login.",
                    color: "#f43f5e",
                    author: "Garuda Linux Team"
                },
                {
                    title: "Nord Minimalist Workspace",
                    tag: "FEATURED",
                    itemsCount: "5 Components",
                    desc: "An arctic, north-bluish clean aesthetic for productivity. Includes Nord SilentSDDM, Waybar CSS, and Fastfetch spec.",
                    color: "#38bdf8",
                    author: "Arctic Ice"
                },
                {
                    title: "Cyberpunk 2077 Night City",
                    tag: "COMMUNITY",
                    itemsCount: "3 Components",
                    desc: "Neon yellow and crimson holographic lock screens and Fastfetch system readout emblems.",
                    color: "#eab308",
                    author: "Subhajit Gayen"
                },
                {
                    title: "Tokyo Night Storm",
                    tag: "POPULAR",
                    itemsCount: "4 Components",
                    desc: "A clean dark theme celebrating the lights of downtown Tokyo. High-contrast ext-session-lock-v1 Quickshell adapter.",
                    color: "#818cf8",
                    author: "Enki Design"
                }
            ]

            delegate: Item {
                width: GridView.view.cellWidth
                height: GridView.view.cellHeight

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 6
                    radius: theme.radiusLg
                    color: colMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                    border.color: colMouse.containsMouse ? theme.accent : theme.borderSubtle
                    border.width: 1

                    MouseArea {
                        id: colMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 8

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Rectangle {
                                width: 10
                                height: 10
                                radius: 5
                                color: modelData.color
                            }

                            Rectangle {
                                height: 18
                                width: tagTxt.implicitWidth + 10
                                radius: 4
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1

                                Text {
                                    id: tagTxt
                                    anchors.centerIn: parent
                                    text: modelData.tag
                                    font.pixelSize: 9
                                    font.bold: true
                                    font.family: "monospace"
                                    color: theme.textMuted
                                }
                            }

                            Item { Layout.fillWidth: true }

                            Text {
                                text: modelData.itemsCount
                                font.pixelSize: 11
                                font.family: "monospace"
                                color: theme.textSecondary
                            }
                        }

                        Text {
                            text: modelData.title
                            font.pixelSize: 15
                            font.bold: true
                            color: theme.text
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }

                        Text {
                            text: modelData.desc
                            font.pixelSize: 11
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            maximumLineCount: 3
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                        }

                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                text: "By " + modelData.author
                                font.pixelSize: 10
                                color: theme.textMuted
                            }

                            Item { Layout.fillWidth: true }

                            Text {
                                text: "View Bundle →"
                                font.pixelSize: 11
                                font.bold: true
                                color: theme.accent
                            }
                        }
                    }
                }
            }
        }
    }
}
