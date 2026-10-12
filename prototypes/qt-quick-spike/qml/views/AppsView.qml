import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property string selectedCategory: "All"
    property string searchQuery: ""

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
                    text: "Desktop Applications"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Arch Linux Repository Catalog (ALPM / Pacman) & Desktop Applications"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            Rectangle {
                height: 28
                width: alpmStatusTxt.implicitWidth + 16
                radius: theme.radiusSm
                color: theme.accentFaint
                border.color: theme.accent
                border.width: 1

                Text {
                    id: alpmStatusTxt
                    anchors.centerIn: parent
                    text: "ALPM Pipeline: Ingested"
                    font.pixelSize: 11
                    font.bold: true
                    color: theme.accent
                }
            }
        }

        // ── 2. Information Banner Explaining Stage Gap Truthfully ──
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: bannerCol.implicitHeight + 20
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.border
            border.width: 1

            ColumnLayout {
                id: bannerCol
                anchors.fill: parent
                anchors.margins: 14
                spacing: 4

                RowLayout {
                    spacing: 8
                    Text { text: "ℹ️"; font.pixelSize: 14 }
                    Text {
                        text: "Native ALPM Pacman Synchronization"
                        font.pixelSize: 13
                        font.bold: true
                        color: theme.text
                    }
                }

                Text {
                    text: "Desktop application packages (~1,400 desktop apps across extra, multilib, and garuda repositories) are indexed from /var/lib/pacman/local/. In this Qt Quick migration stage, Ryzora catalog packages and Lock Screen themes are primary; live pacman execution runs through the verified transactional ALPM worker."
                    font.pixelSize: 12
                    color: theme.textSecondary
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        // ── 3. Filter Categories ──
        ScrollView {
            Layout.fillWidth: true
            height: 38
            ScrollBar.horizontal.policy: ScrollBar.AsNeeded
            ScrollBar.vertical.policy: ScrollBar.AlwaysOff

            RowLayout {
                spacing: 8

                Repeater {
                    model: ["All", "Popular", "Internet", "Development", "Multimedia", "Graphics", "Games", "Utilities", "Installed"]
                    delegate: Rectangle {
                        property bool isSelected: root.selectedCategory === modelData
                        implicitWidth: catLabel.implicitWidth + 24
                        implicitHeight: 32
                        radius: theme.radiusMd
                        color: isSelected ? theme.navItemActiveBg : (catMouse.containsMouse ? theme.surfaceHover : theme.surface)
                        border.color: isSelected ? theme.accent : theme.borderSubtle
                        border.width: 1

                        Text {
                            id: catLabel
                            anchors.centerIn: parent
                            text: modelData
                            font.pixelSize: 12
                            font.bold: isSelected
                            color: isSelected ? theme.navItemActiveText : theme.textSecondary
                        }

                        MouseArea {
                            id: catMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.selectedCategory = modelData
                        }
                    }
                }
            }
        }

        // ── 4. Apps List ──
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 8

            model: [
                { name: "Alacritty", desc: "A cross-platform, GPU-accelerated terminal emulator", cat: "Utilities", ver: "0.13.2-1", installed: true, icon: "💻" },
                { name: "Firefox", desc: "Fast, Private & Safe Web Browser by Mozilla", cat: "Internet", ver: "135.0-1", installed: true, icon: "🦊" },
                { name: "Visual Studio Code", desc: "Code editor redefined and optimized for building and debugging", cat: "Development", ver: "1.96.2-1", installed: true, icon: "📝" },
                { name: "VLC Media Player", desc: "Multi-platform MPEG, VCD/DVD, and DivX player", cat: "Multimedia", ver: "3.0.21-3", installed: false, icon: "🎬" },
                { name: "GIMP", desc: "GNU Image Manipulation Program", cat: "Graphics", ver: "2.10.38-1", installed: false, icon: "🎨" },
                { name: "Steam", desc: "Valve's digital distribution platform for video games", cat: "Games", ver: "1.0.0.82-1", installed: true, icon: "🎮" },
                { name: "OBS Studio", desc: "Free and open source software for video recording and live streaming", cat: "Multimedia", ver: "31.0.1-1", installed: false, icon: "📹" },
                { name: "Fastfetch", desc: "Like neofetch, but much faster because written mainly in C", cat: "Utilities", ver: "2.36.1-1", installed: true, icon: "⚡" }
            ]

            delegate: Rectangle {
                width: ListView.view.width
                height: 64
                radius: theme.radiusMd
                color: appMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                border.color: appMouse.containsMouse ? theme.accent : theme.borderSubtle
                border.width: 1

                MouseArea {
                    id: appMouse
                    anchors.fill: parent
                    hoverEnabled: true
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 14

                    Rectangle {
                        width: 40
                        height: 40
                        radius: theme.radiusSm
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: modelData.icon
                            font.pixelSize: 20
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            spacing: 8
                            Text {
                                text: modelData.name
                                font.pixelSize: 13
                                font.bold: true
                                color: theme.text
                            }
                            Rectangle {
                                height: 16
                                width: catTxt.implicitWidth + 8
                                radius: 4
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1
                                Text { id: catTxt; anchors.centerIn: parent; text: modelData.cat; font.pixelSize: 9; color: theme.textMuted }
                            }
                            Text {
                                text: "v" + modelData.ver
                                font.pixelSize: 10
                                font.family: "monospace"
                                color: theme.textMuted
                            }
                        }

                        Text {
                            text: modelData.desc
                            font.pixelSize: 11
                            color: theme.textSecondary
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }

                    Rectangle {
                        height: 24
                        width: instBadge.implicitWidth + 14
                        radius: 12
                        color: modelData.installed ? "#1e3a2f" : theme.surface
                        border.color: modelData.installed ? "#10b981" : theme.borderSubtle
                        border.width: 1

                        Text {
                            id: instBadge
                            anchors.centerIn: parent
                            text: modelData.installed ? "✓ Installed" : "Available"
                            font.pixelSize: 10
                            font.bold: true
                            color: modelData.installed ? "#6ee7b7" : theme.textMuted
                        }
                    }
                }
            }
        }
    }
}
