import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    height: 50
    color: theme.bg
    border.color: theme.borderSubtle
    border.width: 1
    z: 20

    signal navigateBack()
    signal navigateForward()
    property bool canGoBack: false
    property bool canGoForward: false
    signal searchChanged(string text)
    signal searchSubmitted(string text)
    signal cancelSearchRequested()
    signal refreshClicked()
    signal libraryClicked()
    property string searchQuery: ""
    property string currentTab: "hub"

    readonly property string placeholderText: {
        if (currentTab === "apps") return "Search applications, packages, Flatpaks… (Ctrl+K)"
        if (currentTab === "discover") return "Search community themes, wallpapers, lockscreens… (Ctrl+K)"
        if (currentTab === "library") return "Search installed applications… (Ctrl+K)"
        if (currentTab === "updates") return "Search pending updates… (Ctrl+K)"
        if (currentTab === "backups") return "Search configuration snapshots… (Ctrl+K)"
        if (currentTab === "studio") return "Search desktop customizations… (Ctrl+K)"
        if (currentTab === "system") return "Search diagnostics, probes, and logs… (Ctrl+K)"
        return "Search Ryzora… (Ctrl+K)"
    }

    readonly property bool isSearchFocused: searchInput.activeFocus
    function unfocusSearch() {
        searchInput.focus = false
    }

    Shortcut {
        sequences: [StandardKey.Find, "Ctrl+K"]
        onActivated: {
            searchInput.forceActiveFocus()
            searchInput.selectAll()
        }
    }

    // Dismiss search focus on tapping outside searchContainer in TopBar without swallowing clicks
    TapHandler {
        onTapped: function(eventPoint) {
            if (!searchInput.activeFocus) return
            var pt = eventPoint.position
            var sX = searchContainer.x
            var sY = searchContainer.y
            var sW = searchContainer.width
            var sH = searchContainer.height
            if (pt.x < sX || pt.x > sX + sW || pt.y < sY || pt.y > sY + sH) {
                searchInput.focus = false
            }
        }
    }

    // ── 1. Leading Group: Back & Forward Navigation ──
    RowLayout {
        id: leadingRow
        anchors.left: parent.left
        anchors.leftMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        spacing: 4
        z: 30

        Rectangle {
            width: 32
            height: 32
            radius: theme.radiusSm
            color: backMouse.containsMouse && root.canGoBack ? theme.surfaceHover : "transparent"
            border.color: backMouse.containsMouse && root.canGoBack ? theme.border : "transparent"
            border.width: 1
            opacity: root.canGoBack ? 1.0 : 0.35

            ToolTip.visible: backMouse.containsMouse
            ToolTip.text: "Navigate back (Alt+Left)"
            ToolTip.delay: 300

            RyzoraIcon {
                anchors.centerIn: parent
                name: "chevron-left"
                size: 14
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

            ToolTip.visible: fwdMouse.containsMouse
            ToolTip.text: "Navigate forward (Alt+Right)"
            ToolTip.delay: 300

            RyzoraIcon {
                anchors.centerIn: parent
                name: "chevron-right"
                size: 14
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

    // ── 2. Center Group: Visually Centered Global Search ──
    Rectangle {
        id: searchContainer
        objectName: "searchContainer"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        // Mathematically centered while clamping to avoid overlapping leading or trailing groups
        width: Math.min(540, Math.max(200, root.width - (leadingRow.width + trailingRow.width + 48)))
        height: 36
        radius: theme.radiusMd
        color: theme.surface
        border.color: searchInput.activeFocus ? theme.accent : theme.borderSubtle
        border.width: searchInput.activeFocus ? 1.5 : 1
        z: 30

        Behavior on border.color { ColorAnimation { duration: 100 } }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 11
            anchors.rightMargin: 6
            spacing: 8

            RyzoraIcon {
                name: "search"
                size: 14
                color: searchInput.activeFocus ? theme.accent : theme.textMuted
            }

            TextInput {
                id: searchInput
                objectName: "globalSearchInput"
                Layout.fillWidth: true
                text: root.searchQuery
                font.pixelSize: 13
                font.family: theme.fontFamily
                color: theme.text
                selectByMouse: true
                clip: true
                activeFocusOnTab: true

                Text {
                    anchors.fill: parent
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.placeholderText
                    font.pixelSize: 13
                    font.family: theme.fontFamily
                    color: theme.textMuted
                    visible: !searchInput.text && !searchInput.activeFocus
                    elide: Text.ElideRight
                }

                Keys.onEscapePressed: {
                    if (searchInput.text.length > 0) {
                        searchInput.text = ""
                        root.searchQuery = ""
                        root.cancelSearchRequested()
                        if (backend) backend.cancelSearch()
                    }
                    searchInput.focus = false
                }

                Keys.onReturnPressed: {
                    root.searchSubmitted(searchInput.text)
                }

                Keys.onEnterPressed: {
                    root.searchSubmitted(searchInput.text)
                }

                onTextChanged: {
                    root.searchQuery = text
                    root.searchChanged(text)
                }
            }

            // Real Interactive Clear/Cancel X Button
            Rectangle {
                id: clearBtn
                objectName: "clearSearchButton"
                visible: searchInput.text.length > 0
                width: 24
                height: 24
                radius: 12
                color: clearMouse.containsMouse ? theme.surfaceHover : "transparent"
                border.color: clearMouse.containsMouse ? theme.borderSubtle : "transparent"
                border.width: 1

                ToolTip.visible: clearMouse.containsMouse
                ToolTip.text: "Clear search (Escape)"
                ToolTip.delay: 300

                RyzoraIcon {
                    anchors.centerIn: parent
                    name: "dismiss"
                    size: 11
                    color: clearMouse.containsMouse ? theme.text : theme.textMuted
                }

                MouseArea {
                    id: clearMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        searchInput.text = ""
                        root.searchQuery = ""
                        root.cancelSearchRequested()
                        if (backend) backend.cancelSearch()
                        searchInput.forceActiveFocus()
                    }
                }
            }
        }
    }

    // ── 3. Trailing Group: Active Transactions & Appearance ──
    RowLayout {
        id: trailingRow
        anchors.right: parent.right
        anchors.rightMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8
        z: 30

        // Active Installations Indicator & Popover
        Rectangle {
            id: installIndicatorBtn
            visible: Boolean(backend && backend.activeAppOperationsCount > 0)
            implicitWidth: installIndRow.implicitWidth + 18
            height: 32
            radius: theme.radiusMd
            color: installIndMouse.containsMouse || installPopover.visible
                ? (theme.resolvedTheme === "light" ? "#DBEAFE" : "#1E3A8A")
                : theme.surfaceElevated
            border.color: theme.accent
            border.width: 1

            RowLayout {
                id: installIndRow
                anchors.centerIn: parent
                spacing: 6

                RyzoraIcon {
                    name: "arrow-clockwise"
                    size: 13
                    color: theme.accent
                    RotationAnimation on rotation {
                        from: 0
                        to: 360
                        duration: 1200
                        loops: Animation.Infinite
                        running: installIndicatorBtn.visible
                    }
                }

                Text {
                    text: (backend ? backend.activeAppOperationsCount : 0) + " installing"
                    font.pixelSize: theme.fontNav
                    font.bold: true
                    color: theme.accent
                }
            }

            MouseArea {
                id: installIndMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    if (installPopover.visible) installPopover.close()
                    else installPopover.open()
                }
            }

            InstallManagerPopover {
                id: installPopover
                objectName: "installPopover"
                x: installIndicatorBtn.width - width
                y: installIndicatorBtn.height + 6
            }
        }

        // Appearance Popover Quick Switch
        Rectangle {
            id: appBtn
            objectName: "appearanceButton"
            width: 32
            height: 32
            radius: theme.radiusSm
            color: appMouse.containsMouse || popover.visible ? theme.surfaceHover : theme.surfaceElevated
            border.color: popover.visible ? theme.accent : theme.borderSubtle
            border.width: 1

            ToolTip.visible: appMouse.containsMouse && !popover.visible
            ToolTip.text: "Theme: " + (theme.appearanceMode === "light" ? "Light" : (theme.appearanceMode === "dark" ? "Dark" : "Auto (System)"))
            ToolTip.delay: 300

            RyzoraIcon {
                anchors.centerIn: parent
                name: theme.appearanceMode === "light" ? "weather-sunny" : (theme.appearanceMode === "dark" ? "weather-moon" : "desktop")
                size: 15
                filled: theme.appearanceMode !== "system"
                color: popover.visible ? theme.accent : theme.text
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

    // Dismiss overlay for outside clicks
    MouseArea {
        anchors.fill: parent
        z: 90
        visible: popover.visible
        onClicked: popover.visible = false
    }

    // Dismiss popover on Escape
    Shortcut {
        sequence: "Escape"
        enabled: popover.visible
        onActivated: popover.visible = false
    }

    // Appearance Popover Menu (Directly anchored to appBtn, closes on selection)
    Rectangle {
        id: popover
        objectName: "appearancePopover"
        visible: false
        width: 220
        height: 224
        anchors.top: parent.bottom
        anchors.right: parent.right
        anchors.margins: 6
        radius: theme.radiusLg
        color: theme.surfaceElevated
        border.color: theme.borderStrong
        border.width: 1
        z: 1000

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
                        { id: "dark", label: "Dark", icon: "weather-moon" },
                        { id: "light", label: "Light", icon: "weather-sunny" },
                        { id: "system", label: "Auto", icon: "desktop" }
                    ]
                    delegate: Rectangle {
                        property bool isCur: theme.appearanceMode === modelData.id
                        Layout.fillWidth: true
                        height: 28
                        radius: theme.radiusSm
                        color: isCur ? theme.navItemActiveBg : (pm.containsMouse ? theme.surfaceHover : "transparent")
                        border.color: isCur ? theme.accent : theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 5
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
                            onClicked: {
                                theme.setAppearanceMode(modelData.id)
                                popover.visible = false
                            }
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
                spacing: 5

                Repeater {
                    model: ["blue", "silver", "purple", "green", "orange", "rose", "cyan"]
                    delegate: Rectangle {
                        property bool isSel: theme.accentName === modelData
                        width: 22
                        height: 22
                        radius: 11
                        color: theme.swatches[modelData] || "#3b82f6"
                        border.color: isSel ? "#ffffff" : "transparent"
                        border.width: isSel ? 2 : 0

                        RyzoraIcon {
                            visible: isSel
                            anchors.centerIn: parent
                            name: "checkmark"
                            size: 10
                            color: "#ffffff"
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                theme.setAccent(modelData)
                                popover.visible = false
                            }
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
                text: "Active: " + (theme.swatchNames[theme.accentName] || theme.accentName)
                font.pixelSize: 10
                color: theme.textMuted
            }
        }
    }
}
