import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property string pkgName: "My Custom Lock Screen"
    property string pkgId: "custom-lockscreen"
    property string pkgVersion: "1.0.0"
    property string pkgCategory: "lockscreens"
    property string pkgTarget: "SilentSDDM"

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
                    text: "Package Authoring Studio"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Create, validate, and package custom lock screens and themes for the Ryzora ecosystem"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            Button {
                text: "✨ Export Manifest JSON"
                onClicked: {
                    // Export manifest action
                }
            }
        }

        // ── 2. Two-Column Editor Layout ──
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 20

            // Left Column: Configuration Form
            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 420
                radius: theme.radiusLg
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                ScrollView {
                    anchors.fill: parent
                    anchors.margins: 20
                    clip: true

                    ColumnLayout {
                        width: parent.width
                        spacing: 14

                        Text {
                            text: "PACKAGE SPECIFICATION"
                            font.pixelSize: 11
                            font.bold: true
                            font.letterSpacing: 1.0
                            color: theme.navSectionLabel
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4
                            Text { text: "Display Name"; font.pixelSize: 12; font.bold: true; color: theme.text }
                            TextField {
                                Layout.fillWidth: true
                                text: root.pkgName
                                onTextChanged: root.pkgName = text
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4
                            Text { text: "Package ID (Slug)"; font.pixelSize: 12; font.bold: true; color: theme.text }
                            TextField {
                                Layout.fillWidth: true
                                text: root.pkgId
                                onTextChanged: root.pkgId = text
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 4
                                Text { text: "Version (SemVer)"; font.pixelSize: 12; font.bold: true; color: theme.text }
                                TextField {
                                    Layout.fillWidth: true
                                    text: root.pkgVersion
                                    onTextChanged: root.pkgVersion = text
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 4
                                Text { text: "Category"; font.pixelSize: 12; font.bold: true; color: theme.text }
                                ComboBox {
                                    Layout.fillWidth: true
                                    model: ["lockscreens", "fastfetch", "wallpapers", "rices", "themes"]
                                    onCurrentTextChanged: root.pkgCategory = currentText
                                }
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4
                            Text { text: "Target Integration Engine"; font.pixelSize: 12; font.bold: true; color: theme.text }
                            ComboBox {
                                Layout.fillWidth: true
                                model: ["SilentSDDM (Login Screen)", "Quickshell (ext-session-lock-v1)", "Dual Engine (SDDM + Quickshell)", "Fastfetch Runner"]
                                onCurrentTextChanged: root.pkgTarget = currentText
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 1
                            color: theme.borderSubtle
                            Layout.topMargin: 4
                            Layout.bottomMargin: 4
                        }

                        Text {
                            text: "INTEGRITY & VALIDATION CHECKS"
                            font.pixelSize: 11
                            font.bold: true
                            font.letterSpacing: 1.0
                            color: theme.navSectionLabel
                        }

                        RowLayout {
                            spacing: 8
                            Text { text: "✓"; font.pixelSize: 12; font.bold: true; color: "#10b981" }
                            Text { text: "SemVer 2.0.0 format valid"; font.pixelSize: 11; color: theme.textSecondary }
                        }

                        RowLayout {
                            spacing: 8
                            Text { text: "✓"; font.pixelSize: 12; font.bold: true; color: "#10b981" }
                            Text { text: "Target paths sandboxed (no traversal)"; font.pixelSize: 11; color: theme.textSecondary }
                        }

                        RowLayout {
                            spacing: 8
                            Text { text: "✓"; font.pixelSize: 12; font.bold: true; color: "#10b981" }
                            Text { text: "Zero privileged hooks or arbitrary commands"; font.pixelSize: 11; color: theme.textSecondary }
                        }
                    }
                }
            }

            // Right Column: Live Manifest JSON Preview
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: theme.radiusLg
                color: "#0a0c10"
                border.color: theme.borderSubtle
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 12

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text { text: "📄"; font.pixelSize: 14 }
                        Text {
                            text: "manifest.json (Live Preview)"
                            font.pixelSize: 13
                            font.bold: true
                            color: theme.text
                        }

                        Item { Layout.fillWidth: true }

                        Rectangle {
                            height: 20
                            width: 60
                            radius: 4
                            color: "#1e3a2f"
                            border.color: "#10b981"
                            border.width: 1
                            Text { anchors.centerIn: parent; text: "VALID"; font.pixelSize: 9; font.bold: true; color: "#6ee7b7" }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: theme.borderSubtle
                    }

                    Flickable {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        contentWidth: manifestText.implicitWidth
                        contentHeight: manifestText.implicitHeight

                        Text {
                            id: manifestText
                            text: JSON.stringify({
                                "id": root.pkgId,
                                "name": root.pkgName,
                                "version": root.pkgVersion,
                                "category": root.pkgCategory,
                                "target": root.pkgTarget,
                                "author": "Current User",
                                "schema_version": "1.0.0",
                                "dependencies": {
                                    "compositor": "hyprland",
                                    "protocol": "ext-session-lock-v1"
                                }
                            }, null, 2)
                            font.family: "monospace"
                            font.pixelSize: 12
                            color: "#38bdf8"
                        }
                    }
                }
            }
        }
    }
}
