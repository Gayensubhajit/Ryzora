import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    visible: false
    anchors.fill: parent
    color: "#80000000"
    z: 100

    signal closeRequested()

    MouseArea {
        anchors.fill: parent
        onClicked: root.closeRequested()
    }

    Rectangle {
        width: Math.min(460, parent.width - 40)
        height: 380
        anchors.centerIn: parent
        radius: theme.radiusLg
        color: theme.surfaceElevated
        border.color: theme.borderStrong
        border.width: 1

        MouseArea {
            anchors.fill: parent
            // Prevent click-through
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 24
            spacing: 16

            RowLayout {
                Layout.fillWidth: true
                spacing: 14

                Rectangle {
                    width: 44
                    height: 44
                    radius: theme.radiusMd
                    color: theme.accent

                    Text {
                        anchors.centerIn: parent
                        text: "R"
                        font.pixelSize: 22
                        font.bold: true
                        font.family: "monospace"
                        color: "#ffffff"
                    }
                }

                ColumnLayout {
                    spacing: 2
                    Layout.fillWidth: true

                    Text {
                        text: "Ryzora"
                        font.pixelSize: 18
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Version 0.1.0 · Qt 6 Quick / QML + Rust"
                        font.pixelSize: 11
                        color: theme.textMuted
                    }
                }

                Button {
                    text: "✕"
                    flat: true
                    onClicked: root.closeRequested()
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: "Linux Lock Screen & Theme Ecosystem Store"
                    font.pixelSize: 13
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Engineered for Garuda Linux and modern Wayland desktops. Provides transactional installation, cryptographic manifest verification, snapshot restore points, and multi-target display integration for SilentSDDM and Quickshell."
                    font.pixelSize: 12
                    color: theme.textSecondary
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                RowLayout {
                    Text { text: "Desktop Target:"; font.pixelSize: 11; color: theme.textMuted }
                    Text { text: (backend.systemInfo && backend.systemInfo.desktop) ? backend.systemInfo.desktop : "Hyprland"; font.pixelSize: 11; font.bold: true; color: theme.text }
                }
                RowLayout {
                    Text { text: "Session Protocol:"; font.pixelSize: 11; color: theme.textMuted }
                    Text { text: "Wayland (ext-session-lock-v1)"; font.pixelSize: 11; font.bold: true; color: theme.text }
                }
                RowLayout {
                    Text { text: "Core Engine:"; font.pixelSize: 11; color: theme.textMuted }
                    Text { text: "ryzora-core v0.1.0"; font.pixelSize: 11; font.bold: true; color: theme.text }
                }
            }

            Item { Layout.fillHeight: true }

            RowLayout {
                Layout.fillWidth: true
                Item { Layout.fillWidth: true }
                Button {
                    text: "Close"
                    onClicked: root.closeRequested()
                }
            }
        }
    }
}
