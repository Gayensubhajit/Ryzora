import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: card
    width: 270
    height: 290
    radius: theme.radiusXl
    color: theme.surface
    border.color: mouseArea.containsMouse ? theme.accent : (card.itemData && card.itemData.is_active ? "#10b981" : (card.itemData && card.itemData.is_installed ? "#059669" : theme.borderSubtle))
    border.width: (card.itemData && (card.itemData.is_active || card.itemData.is_installed)) ? 1.5 : 1
    clip: true

    property var itemData: null
    signal cardClicked(var itemData)

    Behavior on border.color { ColorAnimation { duration: theme.reduceMotion ? 0 : 150 } }

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

        // ── 1. 16:9 Media Preview Section ──
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.round(card.width * 9 / 16)
            color: "#0a0c10"
            clip: true

            // Real Artwork Image Preview
            Image {
                id: previewImg
                anchors.fill: parent
                source: (card.itemData && card.itemData.preview_url) ? card.itemData.preview_url : ""
                fillMode: (card.itemData && card.itemData.subfilter === "emblem") ? Image.PreserveAspectFit : Image.PreserveAspectCrop
                asynchronous: true
                smooth: true
                mipmap: true
                visible: status === Image.Ready

                scale: mouseArea.containsMouse ? 1.04 : 1.0
                Behavior on scale { NumberAnimation { duration: theme.reduceMotion ? 0 : 250; easing.type: Easing.OutQuad } }
            }

            // Fallback Generative Artwork
            Rectangle {
                anchors.fill: parent
                visible: previewImg.status !== Image.Ready
                gradient: Gradient {
                    
                    GradientStop { position: 0.0; color: (card.itemData && card.itemData.category === "lockscreens") ? "#1e1b4b" : (card.itemData && card.itemData.category === "fastfetch") ? "#1e293b" : "#172554" }
                    GradientStop { position: 1.0; color: "#090d16" }
                }

                // Generative details
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 12

                    RowLayout {
                        Layout.fillWidth: true
                        Rectangle {
                            width: 24; height: 24; radius: theme.radiusSm
                            color: "#33000000"; border.color: "#33ffffff"
                            Text {
                                anchors.centerIn: parent
                                text: (card.itemData && card.itemData.category === "lockscreens") ? "🔒" :
                                      (card.itemData && card.itemData.category === "fastfetch") ? "⚡" : "📦"
                                font.pixelSize: 12
                            }
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: (card.itemData && card.itemData.subfilter) ? card.itemData.subfilter.toUpperCase() : ""
                            font.pixelSize: 10; font.bold: true; font.family: "monospace"
                            color: "#94a3b8"
                        }
                    }

                    Item { Layout.fillHeight: true }

                    Text {
                        text: card.itemData ? ("~/.config/" + card.itemData.name.toLowerCase().replace(/\s+/g, "-")) : ""
                        font.pixelSize: 9; font.family: "monospace"
                        color: "#64748b"; elide: Text.ElideRight; Layout.fillWidth: true
                    }
                }
            }

            // Bottom vignette shadow overlay
            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 36
                gradient: Gradient {
                    GradientStop { position: 0.0; color: "transparent" }
                    GradientStop { position: 1.0; color: "#99000000" }
                }
            }

            // Top-Left Badges
            RowLayout {
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.margins: 8
                spacing: 4
                z: 10

                // Emblem Badge
                Rectangle {
                    visible: card.itemData && (card.itemData.subfilter === "emblem" || (card.itemData.tags && card.itemData.tags.indexOf("emblem") !== -1))
                    height: 18; width: emblemTxt.width + 10; radius: 4
                    color: "#33ef4444"; border.color: "#66ef4444"
                    Text { id: emblemTxt; anchors.centerIn: parent; text: "EMBLEM"; font.pixelSize: 9; font.bold: true; color: "#fca5a5" }
                }

                // Official / Verified Badge
                Rectangle {
                    visible: card.itemData && (card.itemData.author === "Darkkal44" || card.itemData.author === "Ryzora")
                    height: 18; width: authTxt.width + 10; radius: 4
                    color: "#33f59e0b"; border.color: "#66f59e0b"
                    Text { id: authTxt; anchors.centerIn: parent; text: "OFFICIAL"; font.pixelSize: 9; font.bold: true; color: "#fde68a" }
                }
            }

            // Top-Right Badges
            RowLayout {
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                spacing: 4
                z: 10

                // Active Badge
                Rectangle {
                    visible: card.itemData && card.itemData.is_active
                    height: 20; width: activeTxt.width + 14; radius: 10
                    color: "#10b981"; border.color: "#34d399"
                    RowLayout {
                        anchors.centerIn: parent; spacing: 3
                        Text { text: "✓"; font.pixelSize: 10; font.bold: true; color: "#ffffff" }
                        Text { id: activeTxt; text: "ACTIVE"; font.pixelSize: 9; font.bold: true; color: "#ffffff" }
                    }
                }

                // Installed Badge
                Rectangle {
                    visible: card.itemData && card.itemData.is_installed && !card.itemData.is_active
                    height: 20; width: instTxt.width + 14; radius: 10
                    color: "#1e3a2f"; border.color: "#10b981"
                    RowLayout {
                        anchors.centerIn: parent; spacing: 3
                        Text { text: "✓"; font.pixelSize: 10; font.bold: true; color: "#6ee7b7" }
                        Text { id: instTxt; text: "INSTALLED"; font.pixelSize: 9; font.bold: true; color: "#6ee7b7" }
                    }
                }
            }
        }

        // ── 2. Card Content & Metadata Section ──
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "transparent"

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 4

                // Title
                Text {
                    text: card.itemData ? card.itemData.name : ""
                    font.pixelSize: 14
                    font.bold: true
                    color: mouseArea.containsMouse ? theme.accent : theme.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    Behavior on color { ColorAnimation { duration: 100 } }
                }

                // Description (2 lines clamp)
                Text {
                    text: card.itemData ? (card.itemData.description || "Linux desktop customization configuration.") : ""
                    font.pixelSize: 11
                    color: theme.textSecondary
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                }

                // Divider
                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: theme.borderSubtle
                    Layout.topMargin: 4
                    Layout.bottomMargin: 4
                }

                // Footer Row: Author & Metrics
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Text {
                        text: (card.itemData && card.itemData.author) ? card.itemData.author : "Community"
                        font.pixelSize: 11
                        color: theme.textMuted
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    // Rating if available
                    RowLayout {
                        visible: card.itemData && card.itemData.rating > 0
                        spacing: 2
                        Text { text: "★"; font.pixelSize: 11; color: "#f59e0b" }
                        Text {
                            text: (card.itemData && card.itemData.rating) ? Number(card.itemData.rating).toFixed(1) : ""
                            font.pixelSize: 11
                            font.bold: true
                            color: theme.text
                        }
                    }

                    // Download count
                    RowLayout {
                        visible: card.itemData && card.itemData.download_count > 0
                        spacing: 2
                        Text { text: "↓"; font.pixelSize: 11; color: theme.textMuted }
                        Text {
                            text: {
                                if (!card.itemData || !card.itemData.download_count) return ""
                                var c = card.itemData.download_count
                                if (c >= 1000) return (c / 1000).toFixed(1) + "k"
                                return c.toString()
                            }
                            font.pixelSize: 10
                            font.family: "monospace"
                            color: theme.textSecondary
                        }
                    }
                }
            }
        }
    }
}
