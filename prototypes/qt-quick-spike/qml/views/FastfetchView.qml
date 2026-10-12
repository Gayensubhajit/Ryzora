import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        // Header and Presets Bar
        RowLayout {
            Layout.fillWidth: true

            ColumnLayout {
                spacing: 2
                Text {
                    text: "Fastfetch Terminal Preview"
                    font.pixelSize: theme.fontLg
                    font.bold: true
                    color: theme.text
                }
                Text {
                    text: "Asynchronous subprocess execution with monotonic cancellation & timeout protection"
                    font.pixelSize: theme.fontSm
                    color: theme.textMuted
                }
            }

            Item { Layout.fillWidth: true }

            // Preset Buttons Bar
            ScrollView {
                Layout.maximumWidth: 550
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                ScrollBar.vertical.policy: ScrollBar.AlwaysOff

                RowLayout {
                    spacing: 6
                    Repeater {
                        model: backend.availablePresets
                        PresetButton {
                            label: modelData
                            preset: modelData
                        }
                    }
                }
            }

            // Cancel Button
            Rectangle {
                visible: backend.isFastfetchLoading
                width: 90
                height: 32
                radius: theme.radiusMd
                color: theme.errorBg
                border.color: theme.error

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 4
                    Text { text: "✕"; font.pixelSize: 11; color: theme.error }
                    Text { text: "Cancel"; font.pixelSize: theme.fontSm; font.bold: true; color: theme.error }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: backend.cancelFastfetch()
                }
            }
        }

        // Terminal Output Canvas
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: theme.radiusMd
            color: "#08080c"
            border.color: backend.isProcessActive() ? theme.accent : theme.border
            border.width: 1

            // Loading state banner inside terminal
            Rectangle {
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 12
                width: 160
                height: 28
                radius: theme.radiusSm
                color: "#aa0a0a0e"
                border.color: theme.accent
                visible: backend.isFastfetchLoading
                z: 10

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 8
                    BusyIndicator {
                        running: backend.isFastfetchLoading
                        implicitWidth: 16
                        implicitHeight: 16
                    }
                    Text {
                        text: "Executing Fastfetch…"
                        font.pixelSize: 10
                        color: theme.accent
                    }
                }
            }

            // Scrollable Rich Text Canvas
            ScrollView {
                anchors.fill: parent
                anchors.margins: 16
                clip: true

                TextEdit {
                    id: terminalText
                    readOnly: true
                    selectByMouse: true
                    textFormat: TextEdit.RichText
                    text: backend.fastfetchHtml
                    font.family: "JetBrains Mono, monospace"
                    font.pixelSize: 12
                    color: "#f1f5f9"
                    wrapMode: TextEdit.NoWrap
                }

                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                ScrollBar.horizontal: ScrollBar { policy: ScrollBar.AsNeeded }
            }

            // Terminal Footer
            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 32
                radius: theme.radiusMd
                color: theme.surfaceElevated
                border.color: theme.borderSubtle

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 16

                    Text {
                        text: "Active preset: " + backend.selectedPreset
                        font.pixelSize: 11
                        color: theme.textMuted
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: backend.isProcessActive() ? "Process PID: " + backend.activePid() + " (Running)" : "Process: Idle"
                        font.pixelSize: 11
                        color: backend.isProcessActive() ? theme.warning : theme.success
                    }
                }
            }
        }
    }

    component PresetButton : Rectangle {
        property string label: ""
        property string preset: ""
        property bool isSelected: backend.selectedPreset === preset

        height: 32
        width: btnLabel.width + 18
        radius: theme.radiusSm
        color: isSelected ? theme.accent : (pMouse.containsMouse ? theme.surfaceHover : theme.surface)
        border.color: isSelected ? theme.accent : theme.border

        Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 100 } }

        Text {
            id: btnLabel
            anchors.centerIn: parent
            text: label
            font.pixelSize: 11
            font.bold: isSelected
            color: isSelected ? "#0a0a0c" : theme.textSecondary
        }

        MouseArea {
            id: pMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: backend.requestFastfetchPreview(preset)
        }
    }
}
