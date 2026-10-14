import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: card
    radius: 10
    clip: true
    color: mouseArea.containsMouse ? theme.surfaceHover : theme.surface
    border.color: mouseArea.containsMouse ? theme.accent : theme.borderSubtle
    border.width: 1

    property var itemData: null

    signal cardClicked(var itemData)
    signal actionClicked(string action, var itemData)

    Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }
    Behavior on border.color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }

    Keys.onReturnPressed: if (card.itemData) card.cardClicked(card.itemData)
    Keys.onSpacePressed: if (card.itemData) card.cardClicked(card.itemData)

    MouseArea {
        id: mouseArea
        objectName: "storeCardMouseArea"
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        z: 1
        onClicked: {
            if (card.itemData) {
                console.log("[QML StoreCard] Card clicked: " + card.itemData.id);
                card.cardClicked(card.itemData);
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── 1. Image Preview Section (Dominant Landscape Area) ──
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            // Clean Fallback Graphic when artwork is loading or unavailable
            Rectangle {
                anchors.fill: parent
                visible: previewImg.status !== Image.Ready
                color: theme.resolvedTheme === "light" ? "#F0F2F5" : "#1A1D23"

                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 6

                    RyzoraIcon {
                        Layout.alignment: Qt.AlignHCenter
                        name: {
                            if (!card.itemData || !card.itemData.category) return "package"
                            var cat = String(card.itemData.category).toLowerCase()
                            if (cat === "lockscreens" || cat === "lockscreen") return "lock"
                            if (cat === "fastfetch") return "terminal"
                            if (cat === "themes" || cat === "theme") return "palette"
                            if (cat === "wallpapers" || cat === "wallpaper") return "image"
                            if (cat === "rices" || cat === "rice") return "sliders"
                            return "package"
                        }
                        size: 24
                        color: theme.textMuted
                        opacity: previewImg.status === Image.Loading ? 0.40 : 0.70
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: {
                            if (previewImg.status === Image.Loading) return "LOADING…"
                            return card.itemData ? (card.itemData.category ? String(card.itemData.category).toUpperCase() : "PACKAGE") : "PACKAGE"
                        }
                        font.pixelSize: 10
                        font.letterSpacing: 1.0
                        font.bold: true
                        color: theme.textMuted
                    }
                }
            }

            // Real Artwork Image Preview
            Image {
                id: previewImg
                anchors.fill: parent
                source: {
                    if (!card.itemData || !card.itemData.preview_url) return ""
                    var u = String(card.itemData.preview_url).trim()
                    if (!u || u === "file://" || u === "file:///" || u === "http://" || u === "https://") return ""
                    if (u.indexOf("file:///") === 0) return u
                    if (u.indexOf("file://") === 0) {
                        var rest = u.substring(7).trim()
                        if (!rest || rest === "/") return ""
                        if (rest.indexOf("/") === 0) return "file://" + rest
                        return ""
                    }
                    if (u.indexOf("http://") === 0) return u.length > 7 ? u : ""
                    if (u.indexOf("https://") === 0) return u.length > 8 ? u : ""
                    if (u.indexOf("/") === 0) return "file://" + u
                    return ""
                }
                fillMode: (card.itemData && (card.itemData.subfilter === "emblem" || card.itemData.category === "fastfetch")) ? Image.PreserveAspectFit : Image.PreserveAspectCrop
                asynchronous: true
                cache: true
                sourceSize.width: 640
                sourceSize.height: 480
                smooth: true
                visible: status === Image.Ready
            }

            // Category Badge Pill (Top-Left)
            Rectangle {
                visible: Boolean(card.itemData && card.itemData.category)
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.margins: 8
                height: 18
                width: catBadgeText.implicitWidth + 12
                radius: 4
                color: Qt.rgba(0, 0, 0, 0.55)
                border.color: Qt.rgba(255, 255, 255, 0.12)
                border.width: 1

                Text {
                    id: catBadgeText
                    anchors.centerIn: parent
                    text: {
                        if (!card.itemData || !card.itemData.category) return "PACKAGE"
                        return String(card.itemData.category).toUpperCase()
                    }
                    font.pixelSize: 9
                    font.letterSpacing: 0.5
                    font.bold: true
                    color: "#FFFFFF"
                }
            }

            // Subtle divider line separating image from metadata
            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 1
                color: theme.borderSubtle
            }

            // Subtle Status Indicator (Top-Right: Small Dot + Status Text)
            Rectangle {
                visible: Boolean(card.itemData && (card.itemData.is_active || card.itemData.is_installed))
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                height: 20
                width: statusRow.implicitWidth + 14
                radius: 10
                color: Qt.rgba(0, 0, 0, 0.65)
                border.color: card.itemData && card.itemData.is_active ? theme.accent : Qt.rgba(255, 255, 255, 0.22)
                border.width: 1

                RowLayout {
                    id: statusRow
                    anchors.centerIn: parent
                    spacing: 5

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
                        color: "#FFFFFF"
                    }
                }
            }
        }

        // ── 2. Minimal Metadata & Contextual Action Section ──
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 58
            Layout.minimumHeight: 58
            Layout.maximumHeight: 58
            color: "transparent"

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                anchors.topMargin: 8
                anchors.bottomMargin: 8
                spacing: 8

                // Title & Subtitle Info (Fills remaining width)
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: card.itemData ? String(card.itemData.name) : ""
                        font.pixelSize: 13
                        font.bold: true
                        color: mouseArea.containsMouse ? theme.accent : theme.text
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Text {
                        text: {
                            if (!card.itemData) return ""
                            var cat = card.itemData.category ? (String(card.itemData.category).charAt(0).toUpperCase() + String(card.itemData.category).slice(1)) : "Customization"
                            var author = card.itemData.author ? (typeof card.itemData.author === "object" ? (card.itemData.author.name || "Community") : String(card.itemData.author)) : "Community"
                            return cat + " · " + author
                        }
                        font.pixelSize: 11
                        color: theme.textSecondary
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                // PureDesk / macOS Contextual Action Pill Beneath Preview
                Rectangle {
                    height: 24
                    width: cardActionText.implicitWidth + 18
                    radius: 12
                    color: {
                        if (!card.itemData) return "transparent"
                        if (card.itemData.is_active) return theme.resolvedTheme === "light" ? "#DCFCE7" : "#064E3B"
                        if (card.itemData.is_installed) return theme.resolvedTheme === "light" ? "#EFF6FF" : "#1E3A8A"
                        return pillMouse.containsMouse ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#2A2F3A") : (theme.resolvedTheme === "light" ? "#F3F4F6" : "#22262E")
                    }
                    border.color: {
                        if (!card.itemData) return "transparent"
                        if (card.itemData.is_active) return "#10B981"
                        if (card.itemData.is_installed) return "#3B82F6"
                        return pillMouse.containsMouse ? theme.accent : theme.borderSubtle
                    }
                    border.width: 1

                    Text {
                        id: cardActionText
                        anchors.centerIn: parent
                        text: {
                            if (!card.itemData) return ""
                            if (card.itemData.is_active) return "ACTIVE"
                            if (card.itemData.is_installed) return "OPEN"
                            return "GET"
                        }
                        font.pixelSize: 10
                        font.bold: true
                        color: {
                            if (!card.itemData) return theme.textMuted
                            if (card.itemData.is_active) return theme.resolvedTheme === "light" ? "#047857" : "#34D399"
                            if (card.itemData.is_installed) return theme.resolvedTheme === "light" ? "#1D4ED8" : "#93C5FD"
                            return theme.accent
                        }
                    }

                    MouseArea {
                        id: pillMouse
                        objectName: "storeCardPillMouse"
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        z: 2
                        onClicked: {
                            if (!card.itemData) return;
                            if (!card.itemData.is_installed) {
                                card.actionClicked("install", card.itemData);
                            } else {
                                card.cardClicked(card.itemData);
                            }
                        }
                    }
                }
            }
        }
    }
}
