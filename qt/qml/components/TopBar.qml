import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    height: 52
    color: theme.bg
    border.color: theme.borderSubtle
    border.width: 1
    z: 20

    signal navigateBack()
    signal navigateForward()
    property bool canGoBack: false
    property bool canGoForward: false
    signal refreshClicked()
    signal searchChanged(string text)
    signal libraryClicked()
    property string searchQuery: ""
    property string currentTab: "hub"
    readonly property string placeholderText: currentTab === "apps" ? "Search applications…" : "Search rices, themes, lockscreens…"

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 16
        anchors.rightMargin: 16
        spacing: 10

        // ── 1. History Navigation (Back / Forward) ──
        RowLayout {
            spacing: 4

            Rectangle {
                width: 32
                height: 32
                radius: theme.radiusSm
                color: backMouse.containsMouse && root.canGoBack ? theme.surfaceHover : "transparent"
                border.color: backMouse.containsMouse && root.canGoBack ? theme.border : "transparent"
                border.width: 1
                opacity: root.canGoBack ? 1.0 : 0.35

                Text {
                    anchors.centerIn: parent
                    text: "‹"
                    font.pixelSize: 20
                    font.bold: true
                    color: root.canGoBack ? theme.text : theme.textMuted
                }

                MouseArea {
                    id: backMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: root.canGoBack ? Qt.PointingHandCursor : Qt.ArrowCursor
                    onClicked: {
                        if (root.canGoBack) root.navigateBack()
                    }
                }
            }

            Rectangle {
                width: 32
                height: 32
                radius: theme.radiusSm
                color: fwdMouse.containsMouse && root.canGoForward ? theme.surfaceHover : "transparent"
                border.color: fwdMouse.containsMouse && root.canGoForward ? theme.border : "transparent"
                border.width: 1
                opacity: root.canGoForward ? 1.0 : 0.35

                Text {
                    anchors.centerIn: parent
                    text: "›"
                    font.pixelSize: 20
                    font.bold: true
                    color: root.canGoForward ? theme.text : theme.textMuted
                }

                MouseArea {
                    id: fwdMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: root.canGoForward ? Qt.PointingHandCursor : Qt.ArrowCursor
                    onClicked: {
                        if (root.canGoForward) root.navigateForward()
                    }
                }
            }
        }

        // ── 2. Refresh Button ──
        Rectangle {
            width: 32
            height: 32
            radius: theme.radiusSm
            color: refMouse.containsMouse ? theme.surfaceHover : "transparent"
            border.color: refMouse.containsMouse ? theme.border : "transparent"
            border.width: 1

            Text {
                anchors.centerIn: parent
                text: "⟳"
                font.pixelSize: 16
                color: refMouse.containsMouse ? theme.text : theme.textSecondary
            }

            MouseArea {
                id: refMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.refreshClicked()
            }
        }

        // ── 3. Search Bar ──
        Rectangle {
            Layout.preferredWidth: Math.min(360, Math.max(180, root.width * 0.3))
            Layout.fillWidth: false
            height: 34
            radius: theme.radiusMd
            color: theme.surface
            border.color: searchInput.activeFocus ? theme.accent : theme.borderSubtle
            border.width: 1

            Behavior on border.color { ColorAnimation { duration: 100 } }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                spacing: 8

                RyzoraIcon {
                    name: "search"
                    size: 13
                    color: theme.textMuted
                }

                TextInput {
                    id: searchInput
                    Layout.fillWidth: true
                    text: root.searchQuery
                    font.pixelSize: theme.fontNav
                    color: theme.text
                    selectByMouse: true
                    clip: true

                    Text {
                        anchors.fill: parent
                        text: root.placeholderText
                        font.pixelSize: theme.fontNav
                        color: theme.textMuted
                        visible: !searchInput.text && !searchInput.activeFocus
                    }

                    onTextChanged: {
                        root.searchQuery = text
                        root.searchChanged(text)
                    }
                }

                Item {
                    visible: searchInput.text.length > 0
                    width: 14; height: 14
                    RyzoraIcon {
                        anchors.centerIn: parent
                        name: "x"
                        size: 9
                        color: clearMouse.containsMouse ? theme.text : theme.textMuted
                    }
                    MouseArea {
                        id: clearMouse
                        anchors.fill: parent
                        anchors.margins: -4
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            searchInput.text = ""
                        }
                    }
                }
            }
        }

        // Draggable Window Spacer
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
        }

        // ── 4. Library Shortcut Button ──
        Rectangle {
            implicitWidth: libRow.implicitWidth + 20
            height: 32
            radius: theme.radiusMd
            color: libMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                id: libRow
                anchors.centerIn: parent
                spacing: 6

                RyzoraIcon {
                    name: "book"
                    size: 13
                    color: theme.text
                }

                Text {
                    text: "Library"
                    font.pixelSize: theme.fontNav
                    font.bold: true
                    color: theme.text
                }
            }

            MouseArea {
                id: libMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.libraryClicked()
            }
        }

        // ── 5. Appearance Popover Quick Switch ──
        Rectangle {
            id: appBtn
            width: 32
            height: 32
            radius: theme.radiusSm
            color: appMouse.containsMouse || popover.visible ? theme.surfaceHover : theme.surfaceElevated
            border.color: theme.borderSubtle
            border.width: 1

            RyzoraIcon {
                anchors.centerIn: parent
                name: theme.appearanceMode === "light" ? "sun" : (theme.appearanceMode === "dark" ? "moon" : "desktop")
                size: 14
                color: theme.text
            }

            MouseArea {
                id: appMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: popover.visible = !popover.visible
            }
        }
    }

    // Appearance Popover Menu
    Rectangle {
        id: popover
        visible: false
        width: 210
        height: 220
        anchors.top: parent.bottom
        anchors.right: parent.right
        anchors.margins: 6
        radius: theme.radiusLg
        color: theme.surfaceElevated
        border.color: theme.borderStrong
        border.width: 1
        z: 100

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            Text {
                text: "APPEARANCE"
                font.pixelSize: 10
                font.bold: true
                font.letterSpacing: 1.0
                color: theme.navSectionLabel
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 4

                Repeater {
                    model: [
                        { id: "dark", label: "Dark", icon: "moon" },
                        { id: "light", label: "Light", icon: "sun" },
                        { id: "system", label: "Auto", icon: "desktop" }
                    ]
                    delegate: Rectangle {
                        property bool isCur: theme.appearanceMode === modelData.id
                        Layout.fillWidth: true
                        height: 30
                        radius: theme.radiusSm
                        color: isCur ? theme.navItemActiveBg : (pm.containsMouse ? theme.surfaceHover : "transparent")
                        border.color: isCur ? theme.accent : theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon {
                                name: modelData.icon
                                size: 12
                                color: isCur ? theme.accent : theme.textSecondary
                            }
                            Text {
                                text: modelData.label
                                font.pixelSize: 11
                                font.bold: isCur
                                color: isCur ? theme.text : theme.textSecondary
                            }
                        }

                        MouseArea {
                            id: pm
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: theme.setAppearanceMode(modelData.id)
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            Text {
                text: "ACCENT COLOR"
                font.pixelSize: 10
                font.bold: true
                font.letterSpacing: 1.0
                color: theme.navSectionLabel
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Repeater {
                    model: ["blue", "silver", "purple", "green", "orange"]
                    delegate: Rectangle {
                        property bool isSel: theme.accentName === modelData
                        width: 24
                        height: 24
                        radius: 12
                        color: theme.swatches[modelData]
                        border.color: isSel ? "#ffffff" : "transparent"
                        border.width: isSel ? 2 : 0

                        RyzoraIcon {
                            visible: isSel
                            anchors.centerIn: parent
                            name: "check"
                            size: 11
                            color: "#ffffff"
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: theme.setAccent(modelData)
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            Text {
                text: "Active: " + theme.swatchNames[theme.accentName]
                font.pixelSize: 10
                color: theme.textMuted
            }
        }
    }
}
