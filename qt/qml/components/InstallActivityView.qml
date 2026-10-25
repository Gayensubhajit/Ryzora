import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: root

    property string stage: "DOWNLOADING"
    property string currentItem: ""
    property string diagnosticDetails: ""
    property string rawMessage: ""
    property bool isExpanded: false

    spacing: 8

    // Accordion Toggle Button
    Rectangle {
        Layout.fillWidth: true
        height: 28
        radius: 6
        color: toggleMouse.containsMouse ? theme.surfaceHover : "transparent"

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 4
            anchors.rightMargin: 8
            spacing: 6

            RyzoraIcon {
                name: root.isExpanded ? "chevron-up" : "chevron-down"
                size: 13
                color: theme.textSecondary
            }

            Text {
                text: root.isExpanded ? "Hide installation activity" : "Installation activity"
                font.pixelSize: 12
                font.weight: Font.Medium
                color: theme.textSecondary
            }

            Item { Layout.fillWidth: true }

            // Phase indicator badge
            Rectangle {
                radius: 4
                height: 18
                implicitWidth: badgeText.implicitWidth + 10
                color: theme.surfaceHover
                border.color: theme.borderSubtle
                border.width: 1

                Text {
                    id: badgeText
                    anchors.centerIn: parent
                    text: root.stage !== "" ? root.stage.toUpperCase() : "IN PROGRESS"
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    color: theme.accent
                }
            }
        }

        MouseArea {
            id: toggleMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: root.isExpanded = !root.isExpanded
        }
    }

    // Expandable Details Container
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: detailsCol.implicitHeight + 20
        radius: 8
        color: theme.surfaceElevated
        border.color: theme.borderSubtle
        border.width: 1
        visible: root.isExpanded

        ColumnLayout {
            id: detailsCol
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                RyzoraIcon {
                    name: "terminal"
                    size: 14
                    color: theme.accent
                }

                Text {
                    text: root.currentItem !== "" ? root.currentItem : "Resolving dependencies and environment"
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    color: theme.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }

            Text {
                text: {
                    if (root.diagnosticDetails !== "") return root.diagnosticDetails
                    if (root.rawMessage !== "") return root.rawMessage
                    return "Resolving dependencies, downloading content, deploying and finalizing."
                }
                font.pixelSize: 12
                color: theme.textSecondary
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }
}
