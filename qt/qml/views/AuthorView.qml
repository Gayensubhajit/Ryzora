import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

Item {
    id: root

    property string pkgName: "My Custom Lock Screen"
    property string pkgId: "custom-lockscreen"
    property string pkgVersion: "1.0.0"
    property string pkgCategory: "lockscreens"
    property string pkgTarget: "SilentSDDM"

    // Real dynamic validation rules
    readonly property bool isNameValid: root.pkgName.trim().length >= 3
    readonly property bool isSlugValid: /^[a-z0-9]+(-[a-z0-9]+)*$/.test(root.pkgId.trim())
    readonly property bool isVersionValid: /^\d+\.\d+\.\d+$/.test(root.pkgVersion.trim())
    readonly property bool isFormValid: isNameValid && isSlugValid && isVersionValid

    readonly property var targetDependencies: {
        if (root.pkgTarget === "SilentSDDM") {
            return { "display_manager": "sddm", "sddm_theme": "silentsddm" }
        } else if (root.pkgTarget === "Quickshell") {
            return { "compositor": "hyprland", "protocol": "ext-session-lock-v1" }
        } else if (root.pkgTarget === "Dual Engine") {
            return { "display_manager": "sddm", "compositor": "hyprland", "protocol": "ext-session-lock-v1" }
        } else {
            return { "runner": "fastfetch" }
        }
    }

    readonly property string generatedManifest: JSON.stringify({
        "id": root.pkgId.trim(),
        "name": root.pkgName.trim(),
        "version": root.pkgVersion.trim(),
        "category": root.pkgCategory,
        "target": root.pkgTarget,
        "author": "Current User",
        "schema_version": "1.0.0",
        "dependencies": root.targetDependencies
    }, null, 2)

    function exportToDrafts() {
        var res = backend.exportManifest(root.generatedManifest, root.pkgId)
        if (res.indexOf("OK:") === 0) {
            var outPath = res.substring(3)
            backend.postNotification("success", "Manifest Exported", "Successfully wrote validated manifest to " + outPath)
        } else {
            backend.postNotification("error", "Export Failed", res)
        }
    }

    FileDialog {
        id: exportDialog
        title: "Export Package Manifest"
        fileMode: FileDialog.SaveFile
        nameFilters: ["JSON Files (*.json)", "All Files (*)"]
        defaultSuffix: "json"
        onAccepted: {
            var path = selectedFile.toString()
            var res = backend.exportManifest(root.generatedManifest, root.pkgId, path)
            if (res.indexOf("OK:") === 0) {
                var outPath = res.substring(3)
                backend.postNotification("success", "Manifest Exported", "Successfully wrote validated manifest to " + outPath)
            } else {
                backend.postNotification("error", "Export Failed", res)
            }
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
                    text: "Package Authoring Studio"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Create, validate, and preview Ryzora package manifest specifications"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            RowLayout {
                spacing: 8

                Button {
                    text: "📁 Save to Drafts"
                    enabled: root.isFormValid
                    onClicked: root.exportToDrafts()
                }

                Button {
                    text: "✨ Export As…"
                    enabled: root.isFormValid
                    onClicked: exportDialog.open()
                }
            }
        }

        // ── 2. Two-Column Editor Layout ──
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 20

            // Left Column: Configuration Form & Live Validation
            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 420
                radius: theme.radiusLg
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                ScrollView {
                    anchors.fill: parent
                    anchors.margins: 18
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
                            Text { text: "Package ID (Kebab-case slug)"; font.pixelSize: 12; font.bold: true; color: theme.text }
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
                                model: ["SilentSDDM", "Quickshell", "Fastfetch", "Dual Engine"]
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
                            text: "LIVE VALIDATION CRITERIA"
                            font.pixelSize: 11
                            font.bold: true
                            font.letterSpacing: 1.0
                            color: theme.navSectionLabel
                        }

                        RowLayout {
                            spacing: 8
                            Text {
                                text: root.isNameValid ? "✓" : "✕"
                                font.pixelSize: 12
                                font.bold: true
                                color: root.isNameValid ? "#10b981" : "#ef4444"
                            }
                            Text {
                                text: root.isNameValid ? "Display name valid (>=3 characters)" : "Name must be at least 3 characters"
                                font.pixelSize: 11
                                color: root.isNameValid ? theme.textSecondary : "#fca5a5"
                            }
                        }

                        RowLayout {
                            spacing: 8
                            Text {
                                text: root.isSlugValid ? "✓" : "✕"
                                font.pixelSize: 12
                                font.bold: true
                                color: root.isSlugValid ? "#10b981" : "#ef4444"
                            }
                            Text {
                                text: root.isSlugValid ? "Package ID slug valid (lowercase-hyphen)" : "ID must be lowercase kebab-case (e.g. 'my-package')"
                                font.pixelSize: 11
                                color: root.isSlugValid ? theme.textSecondary : "#fca5a5"
                            }
                        }

                        RowLayout {
                            spacing: 8
                            Text {
                                text: root.isVersionValid ? "✓" : "✕"
                                font.pixelSize: 12
                                font.bold: true
                                color: root.isVersionValid ? "#10b981" : "#ef4444"
                            }
                            Text {
                                text: root.isVersionValid ? "SemVer 2.0.0 format valid (X.Y.Z)" : "Version must follow SemVer format (e.g. '1.0.0')"
                                font.pixelSize: 11
                                color: root.isVersionValid ? theme.textSecondary : "#fca5a5"
                            }
                        }

                        RowLayout {
                            spacing: 8
                            Text {
                                text: "✓"
                                font.pixelSize: 12
                                font.bold: true
                                color: "#10b981"
                            }
                            Text {
                                text: "Target paths sandboxed (no parent directory escape)"
                                font.pixelSize: 11
                                color: theme.textSecondary
                            }
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
                            text: "manifest.json (Dynamic Form Binding)"
                            font.pixelSize: 13
                            font.bold: true
                            color: theme.text
                        }

                        Item { Layout.fillWidth: true }

                        Rectangle {
                            height: 20
                            width: valBadgeTxt.implicitWidth + 12
                            radius: 4
                            color: root.isFormValid ? "#1e3a2f" : "#451a03"
                            border.color: root.isFormValid ? "#10b981" : theme.warning
                            border.width: 1
                            Text {
                                id: valBadgeTxt
                                anchors.centerIn: parent
                                text: root.isFormValid ? "VALID MANIFEST" : "VALIDATION ERRORS"
                                font.pixelSize: 9
                                font.bold: true
                                color: root.isFormValid ? "#6ee7b7" : "#fdba74"
                            }
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
                            text: root.generatedManifest
                            font.family: "monospace"
                            font.pixelSize: 12
                            color: root.isFormValid ? "#38bdf8" : "#94a3b8"
                        }
                    }
                }
            }
        }
    }
}
