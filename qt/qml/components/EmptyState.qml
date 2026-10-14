import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property string icon: ""
    property string iconName: icon.length > 0 ? icon : "package"
    property string title: "No items found"
    property string message: "There are currently no items available in this view."
    property string actionLabel: ""
    signal actionClicked()

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 12

        RyzoraIcon {
            Layout.alignment: Qt.AlignHCenter
            name: root.iconName
            size: 36
            color: theme.textMuted
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: root.title
            font.pixelSize: theme.fontLg
            font.bold: true
            color: theme.text
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: root.message
            font.pixelSize: theme.fontBase
            color: theme.textSecondary
            horizontalAlignment: Text.AlignHCenter
            Layout.maximumWidth: 420
            wrapMode: Text.WordWrap
        }

        Rectangle {
            visible: root.actionLabel !== ""
            Layout.alignment: Qt.AlignHCenter
            width: btnText.width + 24
            height: 34
            radius: theme.radiusMd
            color: btnMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
            border.color: theme.accent
            border.width: 1

            Text {
                id: btnText
                anchors.centerIn: parent
                text: root.actionLabel
                font.pixelSize: theme.fontBase
                font.bold: true
                color: theme.accent
            }

            MouseArea {
                id: btnMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.actionClicked()
            }
        }
    }
}
