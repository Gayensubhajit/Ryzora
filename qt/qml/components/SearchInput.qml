import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    height: 38
    radius: theme.radiusMd
    color: theme.surface
    border.color: textInput.activeFocus ? theme.accent : theme.border
    border.width: textInput.activeFocus ? 1.5 : 1

    property alias text: textInput.text
    property alias placeholderText: placeholder.text
    signal textModified(string newText)
    signal clearClicked()

    Behavior on border.color {
        ColorAnimation { duration: theme.reduceMotion ? 0 : 120 }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 10
        spacing: 8

        RyzoraIcon {
            name: "search"
            size: 13
            color: theme.textMuted
        }

        TextInput {
            id: textInput
            Layout.fillWidth: true
            color: theme.text
            font.pixelSize: theme.fontBase
            selectByMouse: true
            clip: true
            onTextChanged: root.textModified(text)

            Text {
                id: placeholder
                anchors.fill: parent
                text: "Search packages, presets, or tags…"
                color: theme.textMuted
                font.pixelSize: theme.fontBase
                visible: !textInput.text && !textInput.activeFocus
            }
        }

        Rectangle {
            visible: textInput.text !== ""
            width: 20
            height: 20
            radius: 10
            color: clearMouse.containsMouse ? theme.surfaceHover : "transparent"

            RyzoraIcon {
                anchors.centerIn: parent
                name: "x"
                size: 9
                color: theme.textMuted
            }

            MouseArea {
                id: clearMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    textInput.text = ""
                    root.clearClicked()
                }
            }
        }
    }
}
