import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: card
    width: 250
    height: 268
    radius: theme.radiusLg
    color: mouseArea.containsMouse ? theme.surfaceElevated : theme.surface
    border.color: mouseArea.containsMouse ? theme.borderStrong : theme.borderSubtle
    border.width: 1
    clip: true

    property var itemData: null
    signal cardClicked(var itemData)
    signal historyClicked(var itemData)

    Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 100 } }
    Behavior on border.color { ColorAnimation { duration: theme.reduceMotion ? 0 : 100 } }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            if (card.itemData) {
                card.cardClicked(card.itemData)
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── 1. 16:9 Aspect Video Media Preview ──
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(135, Math.round(card.width * 9 / 16))
            color: "#0a0c10"
            clip: true

            Image {
                id: prevImg
                anchors.fill: parent
                source: (card.itemData && card.itemData.preview_url && card.itemData.preview_url.indexOf("://") !== -1) ? card.itemData.preview_url : ""
                fillMode: (card.itemData && card.itemData.subfilter === "emblem") ? Image.PreserveAspectFit : Image.PreserveAspectCrop
                asynchronous: true
                smooth: true
                mipmap: true
                visible: status === Image.Ready

                scale: mouseArea.containsMouse ? 1.05 : 1.0
                Behavior on scale { NumberAnimation { duration: theme.reduceMotion ? 0 : 250; easing.type: Easing.OutQuad } }
            }

            // Fallback artwork
            Rectangle {
                anchors.fill: parent
                visible: prevImg.status !== Image.Ready
                gradient: Gradient {
                    
                    GradientStop { position: 0.0; color: (card.itemData && (card.itemData.package_type === "lockscreen" || card.itemData.category === "lockscreens")) ? "#1e1b4b" : "#1e293b" }
                    GradientStop { position: 1.0; color: "#090d16" }
                }

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 8
                    Text {
                        text: (card.itemData && (card.itemData.package_type === "lockscreen" || card.itemData.category === "lockscreens")) ? "🔒" : "⚡"
                        font.pixelSize: 20
                    }
                    Text {
                        text: card.itemData ? card.itemData.name : ""
                        font.pixelSize: 11
                        font.bold: true
                        color: "#94a3b8"
                        elide: Text.ElideRight
                    }
                }
            }

            // Top-Left Badges
            RowLayout {
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.margins: 6
                spacing: 4
                z: 10

                // Emblem
                Rectangle {
                    visible: card.itemData && (card.itemData.subfilter === "emblem" || (card.itemData.tags && card.itemData.tags.indexOf("emblem") !== -1))
                    height: 20; width: embTxt.width + 10; radius: 4
                    color: "#33ef4444"; border.color: "#66ef4444"
                    Text { id: embTxt; anchors.centerIn: parent; text: "EMBLEM"; font.pixelSize: theme.fontBadge; font.bold: true; color: "#fca5a5" }
                }

                // Official
                Rectangle {
                    visible: card.itemData && (card.itemData.author === "Darkkal44" || card.itemData.author === "Ryzora")
                    height: 20; width: offTxt.width + 10; radius: 4
                    color: "#33f59e0b"; border.color: "#66f59e0b"
                    Text { id: offTxt; anchors.centerIn: parent; text: "OFFICIAL"; font.pixelSize: theme.fontBadge; font.bold: true; color: "#fde68a" }
                }
            }

            // Top-Right Badges
            RowLayout {
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 6
                spacing: 4
                z: 10

                // Active
                Rectangle {
                    visible: card.itemData && card.itemData.is_active
                    height: 20; width: actLabel.width + 14; radius: 9
                    color: "#10b981"; border.color: "#34d399"
                    RowLayout {
                        anchors.centerIn: parent; spacing: 2
                        Text { text: "✓"; font.pixelSize: 9; font.bold: true; color: "#ffffff" }
                        Text { id: actLabel; text: "ACTIVE"; font.pixelSize: theme.fontBadge; font.bold: true; color: "#ffffff" }
                    }
                }

                // Installed
                Rectangle {
                    visible: card.itemData && card.itemData.is_installed && !card.itemData.is_active
                    height: 20; width: instLabel.width + 14; radius: 9
                    color: "#1e3a2f"; border.color: "#10b981"
                    RowLayout {
                        anchors.centerIn: parent; spacing: 2
                        Text { text: "✓"; font.pixelSize: 9; font.bold: true; color: "#6ee7b7" }
                        Text { id: instLabel; text: "INSTALLED"; font.pixelSize: theme.fontBadge; font.bold: true; color: "#6ee7b7" }
                    }
                }
            }
        }

        // ── 2. Compact & Clean Metadata Section ──
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "transparent"

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 3

                // Title + Category
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    Text {
                        text: card.itemData ? card.itemData.name : ""
                        font.pixelSize: theme.fontCardTitle
                        font.bold: true
                        color: mouseArea.containsMouse ? "#ffffff" : theme.text
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Text {
                        text: card.itemData ? (card.itemData.category || card.itemData.package_type || "").toUpperCase() : ""
                        font.pixelSize: theme.fontBadge
                        font.family: "monospace"
                        color: theme.textFaint
                    }
                }

                // Subtitle / Description (1 line clamp)
                Text {
                    text: card.itemData ? (card.itemData.description || "Active configuration on your Linux system.") : ""
                    font.pixelSize: theme.fontCardDesc
                    color: theme.textMuted
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                // Author
                Text {
                    text: (card.itemData && card.itemData.author) ? (card.itemData.author + " · " + (card.itemData.version || "1.0.0")) : "Community"
                    font.pixelSize: theme.fontMeta
                    font.family: "monospace"
                    color: theme.textFaint
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                Item { Layout.fillHeight: true }

                // Divider
                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: theme.borderSubtle
                }

                // Quiet Compatibility & Snapshot Row
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    // Compatibility green indicator
                    RowLayout {
                        spacing: 4
                        Rectangle { width: 5; height: 5; radius: 2.5; color: "#10b981" }
                        Text {
                            text: "Ready"
                            font.pixelSize: theme.fontMeta
                            font.bold: true
                            color: "#34d399"
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Version History Action Link
                    Rectangle {
                        height: 20
                        width: histText.width + 12
                        radius: 4
                        color: histArea.containsMouse ? theme.surfaceHover : "transparent"

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 3
                            Text { text: "⏱"; font.pixelSize: 9; color: theme.textMuted }
                            Text {
                                id: histText
                                text: "History"
                                font.pixelSize: theme.fontMeta
                                color: histArea.containsMouse ? theme.accent : theme.textMuted
                            }
                        }

                        MouseArea {
                            id: histArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (card.itemData) {
                                    card.historyClicked(card.itemData)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
