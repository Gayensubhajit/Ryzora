import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    signal packageSelected(var packageData)

    // Curated suites mapped to actual packages in backend.catalogList
    readonly property var suites: [
        {
            title: "Cyberpunk Neon 2077 Desktop Suite",
            targetPackageId: "rice-cyberpunk-neon",
            tag: "COMMUNITY RICE",
            desc: "Complete desktop rice featuring translucent Kitty terminal palette, custom Waybar modules, and Hyprlock lock screen.",
            color: "#eab308",
            author: "NeoDrifter"
        },
        {
            title: "Catppuccin Mocha Dusk Workspace",
            targetPackageId: "rice-catppuccin-mocha",
            tag: "FEATURED RICE",
            desc: "Soothing pastel dark palette for Hyprland with matching terminal accents and subtle borders.",
            color: "#f43f5e",
            author: "PasticheDream"
        },
        {
            title: "Nordic Frost Minimalist Rice",
            targetPackageId: "rice-nordic-frost",
            tag: "POPULAR RICE",
            desc: "Clean Arctic minimalist aesthetic with frosty blues, clean typography, and Wayland integration.",
            color: "#38bdf8",
            author: "SvenViking"
        },
        {
            title: "Clockwork Orbital Session Lock",
            targetPackageId: "lockscreen-qylock-clockwork-orbital",
            tag: "SDDM THEME",
            desc: "Astronomical analog clock and dual-engine SilentSDDM and ext-session-lock-v1 Quickshell support.",
            color: "#10b981",
            author: "Darkkal44"
        }
    ]

    function openSuite(targetId) {
        var list = backend.catalogList || []
        for (var i = 0; i < list.length; i++) {
            if (list[i] && list[i].id === targetId) {
                root.packageSelected(list[i])
                return
            }
        }
        backend.postNotification("warning", "Package Not Available", "The package '" + targetId + "' was not found in the current catalog index.")
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        // ── 1. Page Header ──
        ColumnLayout {
            spacing: 2
            Layout.fillWidth: true

            Text {
                text: "Curated Collections & Suites"
                font.pixelSize: 22
                font.bold: true
                color: theme.text
            }

            Text {
                text: "Curated desktop configurations linked directly to official repository packages"
                font.pixelSize: 13
                color: theme.textSecondary
            }
        }

        // ── 2. Information Banner on Bundle Selection ──
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: bannerCol.implicitHeight + 18
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.border
            border.width: 1

            ColumnLayout {
                id: bannerCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 2

                RowLayout {
                    spacing: 8
                    Text { text: "ℹ️"; font.pixelSize: 13 }
                    Text {
                        text: "Suite Details & Transaction Safety"
                        font.pixelSize: 12
                        font.bold: true
                        color: theme.text
                    }
                }

                Text {
                    text: "Selecting a collection opens its underlying package detail record. Installation runs through the transactional manifest installer with automatic snapshot protection."
                    font.pixelSize: 11
                    color: theme.textSecondary
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        // ── 3. Collections Grid ──
        GridView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            cellWidth: Math.floor(width / (width >= 900 ? 2 : 1))
            cellHeight: 180

            model: root.suites

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
                        onClicked: root.openSuite(modelData.targetPackageId)
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
                                text: "ID: " + modelData.targetPackageId
                                font.pixelSize: 10
                                font.family: "monospace"
                                color: theme.textMuted
                            }
                        }

                        Text {
                            text: modelData.title
                            font.pixelSize: 14
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
                            maximumLineCount: 2
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                        }

                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                text: "Author: " + modelData.author
                                font.pixelSize: 10
                                color: theme.textMuted
                            }

                            Item { Layout.fillWidth: true }

                            Text {
                                text: "Inspect Package Details →"
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
