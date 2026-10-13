import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: card
    radius: theme.radiusLg
    color: mouseArea.containsMouse ? theme.surfaceHover : theme.surface
    border.color: card.activeFocus ? theme.accent : (mouseArea.containsMouse ? theme.borderStrong : theme.borderSubtle)
    border.width: 1
    clip: true
    activeFocusOnTab: true

    property var itemData: null
    signal cardClicked(var itemData)

    Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 120 } }
    Behavior on border.color { ColorAnimation { duration: theme.reduceMotion ? 0 : 120 } }

    Keys.onReturnPressed: if (card.itemData) card.cardClicked(card.itemData)
    Keys.onSpacePressed: if (card.itemData) card.cardClicked(card.itemData)

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

        // ── 1. Image Preview Section (75–80% of Card Area) ──
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            // Real Artwork Image Preview
            Image {
                id: previewImg
                anchors.fill: parent
                source: (card.itemData && card.itemData.preview_url && card.itemData.preview_url.indexOf("://") !== -1) ? card.itemData.preview_url : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                smooth: true
                mipmap: true
                visible: status === Image.Ready

                scale: mouseArea.containsMouse ? 1.02 : 1.0
                Behavior on scale { NumberAnimation { duration: theme.reduceMotion ? 0 : 250; easing.type: Easing.OutQuad } }
            }

            // Restrained Category Placeholder (Neutral native fallback)
            Rectangle {
                anchors.fill: parent
                visible: previewImg.status !== Image.Ready
                color: "#181A20"

                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 6

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: {
                            if (!card.itemData || !card.itemData.category) return "📦"
                            var cat = card.itemData.category.toLowerCase()
                            if (cat === "lockscreens") return "🔒"
                            if (cat === "fastfetch") return "⚡"
                            if (cat === "themes") return "🎨"
                            if (cat === "wallpapers") return "🖼️"
                            if (cat === "rices") return "🍱"
                            return "📦"
                        }
                        font.pixelSize: 24
                        opacity: 0.65
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: card.itemData ? (card.itemData.category ? card.itemData.category.toUpperCase() : "PACKAGE") : "PACKAGE"
                        font.pixelSize: 10
                        font.letterSpacing: 1.0
                        font.bold: true
                        color: theme.textMuted
                    }
                }
            }

            // Subtle inner bottom border line separating image from metadata
            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 1
                color: theme.borderSubtle
            }

            // Subtle Status Indicator (Top-Right: Small Dot + Optional Text)
            Rectangle {
                visible: card.itemData && (card.itemData.is_active || card.itemData.is_installed)
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                height: 18
                width: statusRow.implicitWidth + 10
                radius: 9
                color: Qt.rgba(0, 0, 0, 0.65)
                border.color: card.itemData && card.itemData.is_active ? theme.accent : "#30343C"
                border.width: 1

                RowLayout {
                    id: statusRow
                    anchors.centerIn: parent
                    spacing: 4

                    Rectangle {
                        width: 6
                        height: 6
                        radius: 3
                        color: card.itemData && card.itemData.is_active ? "#10b981" : "#6488E8"
                    }

                    Text {
                        text: card.itemData && card.itemData.is_active ? "Active" : "Installed"
                        font.pixelSize: 10
                        font.bold: true
                        color: "#E8EAF0"
                    }
                }
            }
        }

        // ── 2. Minimal Metadata Section (20–25% of Card Area) ──
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            Layout.minimumHeight: 56
            Layout.maximumHeight: 56
            color: "transparent"

            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                anchors.topMargin: 8
                anchors.bottomMargin: 8
                spacing: 3

                // Title (14–16px semibold, soft off-white)
                Text {
                    text: card.itemData ? card.itemData.name : ""
                    font.pixelSize: theme.fontCardTitle
                    font.bold: true
                    color: mouseArea.containsMouse ? theme.accent : theme.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                // Single short metadata line (11–12px muted gray) + Details action
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    Text {
                        text: {
                            if (!card.itemData) return ""
                            var cat = card.itemData.category ? (card.itemData.category.charAt(0).toUpperCase() + card.itemData.category.slice(1)) : "Theme"
                            var author = card.itemData.author || "Community"
                            return cat + " · " + author
                        }
                        font.pixelSize: theme.fontMeta
                        color: theme.textSecondary
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    // Hover Details Indicator
                    Text {
                        visible: mouseArea.containsMouse
                        text: "Details ›"
                        font.pixelSize: 11
                        font.bold: true
                        color: theme.accent
                    }
                }
            }
        }
    }
}
