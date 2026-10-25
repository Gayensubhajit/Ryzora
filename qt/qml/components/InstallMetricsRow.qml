import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property string downloadedValue: "Calculating…"
    property string downloadedHelper: "Total size not yet available"
    property string speedValue: "Measuring…"
    property string speedHelper: "Based on real byte transfer"
    property string etaValue: "Estimating…"
    property string etaHelper: "Updated during download"
    property string stepValue: "Step 1 of 1"
    property string stepHelper: "In progress"

    implicitHeight: metricsGrid.implicitHeight

    GridLayout {
        id: metricsGrid
        anchors.fill: parent
        columns: root.width < 560 ? 2 : 4
        columnSpacing: 10
        rowSpacing: 10

        // Metric 1: Downloaded
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: cardCol1.implicitHeight + 16
            radius: 8
            color: theme.surfaceElevated
            border.color: theme.borderSubtle
            border.width: 1

            ColumnLayout {
                id: cardCol1
                anchors.fill: parent
                anchors.margins: 10
                spacing: 2

                Text {
                    text: "Downloaded"
                    font.pixelSize: 11
                    font.weight: Font.Medium
                    color: theme.textMuted
                }

                Text {
                    text: root.downloadedValue
                    font.pixelSize: 14
                    font.weight: Font.Bold
                    color: theme.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                Text {
                    text: root.downloadedHelper
                    font.pixelSize: 10
                    color: theme.textMuted
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }
        }

        // Metric 2: Download speed
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: cardCol2.implicitHeight + 16
            radius: 8
            color: theme.surfaceElevated
            border.color: theme.borderSubtle
            border.width: 1

            ColumnLayout {
                id: cardCol2
                anchors.fill: parent
                anchors.margins: 10
                spacing: 2

                Text {
                    text: "Download speed"
                    font.pixelSize: 11
                    font.weight: Font.Medium
                    color: theme.textMuted
                }

                Text {
                    text: root.speedValue
                    font.pixelSize: 14
                    font.weight: Font.Bold
                    color: theme.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                Text {
                    text: root.speedHelper
                    font.pixelSize: 10
                    color: theme.textMuted
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }
        }

        // Metric 3: Time remaining
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: cardCol3.implicitHeight + 16
            radius: 8
            color: theme.surfaceElevated
            border.color: theme.borderSubtle
            border.width: 1

            ColumnLayout {
                id: cardCol3
                anchors.fill: parent
                anchors.margins: 10
                spacing: 2

                Text {
                    text: "Time remaining"
                    font.pixelSize: 11
                    font.weight: Font.Medium
                    color: theme.textMuted
                }

                Text {
                    text: root.etaValue
                    font.pixelSize: 14
                    font.weight: Font.Bold
                    color: theme.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                Text {
                    text: root.etaHelper
                    font.pixelSize: 10
                    color: theme.textMuted
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }
        }

        // Metric 4: Installation step
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: cardCol4.implicitHeight + 16
            radius: 8
            color: theme.surfaceElevated
            border.color: theme.borderSubtle
            border.width: 1

            ColumnLayout {
                id: cardCol4
                anchors.fill: parent
                anchors.margins: 10
                spacing: 2

                Text {
                    text: "Installation step"
                    font.pixelSize: 11
                    font.weight: Font.Medium
                    color: theme.textMuted
                }

                Text {
                    text: root.stepValue
                    font.pixelSize: 14
                    font.weight: Font.Bold
                    color: theme.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                Text {
                    text: root.stepHelper
                    font.pixelSize: 10
                    color: theme.textMuted
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }
        }
    }
}
