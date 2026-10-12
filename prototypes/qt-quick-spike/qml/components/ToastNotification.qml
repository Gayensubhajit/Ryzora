import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    width: Math.min(420, parent ? parent.width - 40 : 400)
    height: contentCol.implicitHeight + 24
    radius: theme.radiusMd
    z: 9999

    property string notificationType: "info" // "success" | "error" | "warning" | "info"
    property string title: ""
    property string message: ""
    property bool isOpen: false

    color: notificationType === "error" ? theme.errorBg :
           notificationType === "success" ? theme.successBg :
           notificationType === "warning" ? theme.warningBg : theme.surfaceElevated

    border.color: notificationType === "error" ? theme.error :
                  notificationType === "success" ? theme.success :
                  notificationType === "warning" ? theme.warning : theme.accent
    border.width: 1

    opacity: isOpen ? 1.0 : 0.0
    visible: opacity > 0

    Behavior on opacity {
        NumberAnimation { duration: theme.reduceMotion ? 0 : 200 }
    }

    Timer {
        id: autoDismissTimer
        interval: 4000
        running: root.isOpen
        onTriggered: root.isOpen = false
    }

    function show(type, nTitle, nMsg) {
        notificationType = type
        title = nTitle
        message = nMsg
        isOpen = true
        autoDismissTimer.restart()
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 12

        Text {
            text: notificationType === "success" ? "✓" :
                  notificationType === "error" ? "⚠️" :
                  notificationType === "warning" ? "⚡" : "ℹ️"
            font.pixelSize: 18
            color: notificationType === "error" ? theme.error :
                   notificationType === "success" ? theme.success :
                   notificationType === "warning" ? theme.warning : theme.accent
        }

        ColumnLayout {
            id: contentCol
            Layout.fillWidth: true
            spacing: 2

            Text {
                text: root.title
                font.pixelSize: theme.fontBase
                font.bold: true
                color: theme.text
                Layout.fillWidth: true
            }

            Text {
                text: root.message
                font.pixelSize: theme.fontSm
                color: theme.textSecondary
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }

        Rectangle {
            width: 22
            height: 22
            radius: 11
            color: closeMouse.containsMouse ? Qt.rgba(1, 1, 1, 0.15) : "transparent"

            Text {
                anchors.centerIn: parent
                text: "✕"
                font.pixelSize: 11
                color: theme.textMuted
            }

            MouseArea {
                id: closeMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.isOpen = false
            }
        }
    }
}
