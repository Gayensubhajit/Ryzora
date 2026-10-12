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
                text: "Community Creators"
                font.pixelSize: 22
                font.bold: true
                color: theme.text
            }

            Text {
                text: "Verified package maintainers, PGP signing keys, and package contributors"
                font.pixelSize: 13
                color: theme.textSecondary
            }
        }

        // ── 2. Creators Grid ──
        GridView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            cellWidth: Math.floor(width / (width >= 800 ? 2 : 1))
            cellHeight: 180

            model: [
                {
                    name: "Subhajit Gayen",
                    handle: "subhajit",
                    role: "Core Maintainer · Ryzora Founder",
                    verified: true,
                    key: "0x4A82F921B6E09C57",
                    packagesCount: 14,
                    rating: "4.9",
                    bio: "Creator of Ryzora, Clockwork Orbital lock screen, and dr460nized Fastfetch presets."
                },
                {
                    name: "Garuda Linux Team",
                    handle: "garuda-linux",
                    role: "Distribution Maintainers",
                    verified: true,
                    key: "0xD1E2837A5C91FE44",
                    packagesCount: 22,
                    rating: "4.8",
                    bio: "Official Garuda Linux distribution assets, dr460nized icons, and Wayland configurations."
                },
                {
                    name: "SilentByte",
                    handle: "silentbyte",
                    role: "Verified Contributor",
                    verified: true,
                    key: "0x89C1340B2D44E721",
                    packagesCount: 8,
                    rating: "4.7",
                    bio: "Nord theme specialist, SilentSDDM video wallpapers, and Hypridle hooks."
                },
                {
                    name: "Community Contributors",
                    handle: "community",
                    role: "Community Pool",
                    verified: false,
                    key: "Unsigned / Sandbox",
                    packagesCount: 43,
                    rating: "4.5",
                    bio: "Open community submitted rices, Fastfetch specs, and experimental QML widgets."
                }
            ]

            delegate: Item {
                width: GridView.view.cellWidth
                height: GridView.view.cellHeight

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 6
                    radius: theme.radiusLg
                    color: crMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                    border.color: crMouse.containsMouse ? theme.accent : theme.borderSubtle
                    border.width: 1

                    MouseArea {
                        id: crMouse
                        anchors.fill: parent
                        hoverEnabled: true
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 8

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Rectangle {
                                width: 38
                                height: 38
                                radius: 19
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1

                                Text {
                                    anchors.centerIn: parent
                                    text: modelData.name.charAt(0)
                                    font.pixelSize: 16
                                    font.bold: true
                                    color: theme.accent
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1

                                RowLayout {
                                    spacing: 6
                                    Text {
                                        text: modelData.name
                                        font.pixelSize: 14
                                        font.bold: true
                                        color: theme.text
                                    }
                                    Rectangle {
                                        visible: modelData.verified
                                        height: 16
                                        width: 16
                                        radius: 8
                                        color: "#1e3a2f"
                                        Text { anchors.centerIn: parent; text: "✓"; font.pixelSize: 10; font.bold: true; color: "#6ee7b7" }
                                    }
                                }

                                Text {
                                    text: "@" + modelData.handle + " · " + modelData.role
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                }
                            }
                        }

                        Text {
                            text: modelData.bio
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
                                text: "PGP Key: " + modelData.key
                                font.pixelSize: 10
                                font.family: "monospace"
                                color: theme.textMuted
                            }

                            Item { Layout.fillWidth: true }

                            Text {
                                text: modelData.packagesCount + " packages · ★ " + modelData.rating
                                font.pixelSize: 11
                                font.bold: true
                                color: theme.textSecondary
                            }
                        }
                    }
                }
            }
        }
    }
}
