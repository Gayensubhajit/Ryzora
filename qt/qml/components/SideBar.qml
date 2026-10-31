import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    color: theme.sidebarBg
    border.color: theme.borderSubtle
    border.width: 1

    property string currentTab: "discover"
    property bool isCollapsed: Qt.application.arguments.indexOf("--collapsed") !== -1
    property int compactWidth: 72
    property int expandedWidth: 240
    property int minExpandedWidth: 210
    property int maxExpandedWidth: 380
    property bool isDragging: false

    signal tabSelected(string tab)
    signal toggleCollapse()
    signal openAbout()

    Layout.preferredWidth: isCollapsed ? compactWidth : expandedWidth
    Layout.minimumWidth: isCollapsed ? compactWidth : minExpandedWidth
    Layout.maximumWidth: isCollapsed ? compactWidth : maxExpandedWidth

    Behavior on Layout.preferredWidth {
        enabled: !root.isDragging
        NumberAnimation { duration: theme.reduceMotion ? 0 : 150; easing.type: Easing.InOutQuad }
    }

    function persistSidebarSettings() {
        if (typeof backend !== "undefined" && backend && backend.settingsData) {
            var updated = Object.assign({}, backend.settingsData, {
                "sidebar_width": Math.round(root.expandedWidth),
                "sidebar_collapsed": root.isCollapsed
            })
            backend.saveSettings(updated)
        }
    }

    function toggleSidebar() {
        root.isCollapsed = !root.isCollapsed
        root.persistSidebarSettings()
    }

    Shortcut {
        sequence: "Ctrl+B"
        onActivated: root.toggleSidebar()
    }

    Shortcut {
        sequence: "F9"
        onActivated: root.toggleSidebar()
    }

    Component.onCompleted: {
        if (Qt.application.arguments.indexOf("--collapsed") !== -1) {
            root.isCollapsed = true
        } else if (typeof backend !== "undefined" && backend && backend.settingsData) {
            if (backend.settingsData.sidebar_collapsed !== undefined) {
                root.isCollapsed = backend.settingsData.sidebar_collapsed
            }
            if (backend.settingsData.sidebar_width && backend.settingsData.sidebar_width >= root.minExpandedWidth && backend.settingsData.sidebar_width <= root.maxExpandedWidth) {
                root.expandedWidth = backend.settingsData.sidebar_width
            }
        }
    }

    Connections {
        target: backend
        function onSettingsChanged() {
            if (backend.settingsData && !root.isDragging) {
                if (backend.settingsData.sidebar_width && backend.settingsData.sidebar_width >= root.minExpandedWidth && backend.settingsData.sidebar_width <= root.maxExpandedWidth) {
                    root.expandedWidth = backend.settingsData.sidebar_width
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── 1. Brand Header ──
        Rectangle {
            Layout.fillWidth: true
            height: 56
            color: "transparent"
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: root.isCollapsed ? 8 : 14
                anchors.rightMargin: root.isCollapsed ? 6 : 12
                spacing: root.isCollapsed ? 4 : 8

                // Clickable Logo + Wordmark Area
                Item {
                    Layout.fillWidth: !root.isCollapsed
                    Layout.fillHeight: true
                    implicitWidth: root.isCollapsed ? 30 : logoRow.implicitWidth
                    implicitHeight: 34

                    RowLayout {
                        id: logoRow
                        anchors.fill: parent
                        spacing: 10

                        Image {
                            id: logoImg
                            source: "qrc:/resources/icons/ryzora_logo.svg"
                            width: root.isCollapsed ? 30 : 34
                            height: root.isCollapsed ? 30 : 34
                            Layout.preferredWidth: root.isCollapsed ? 30 : 34
                            Layout.preferredHeight: root.isCollapsed ? 30 : 34
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            Layout.alignment: Qt.AlignVCenter
                        }

                        Text {
                            visible: !root.isCollapsed
                            text: "RYZORA"
                            font.family: theme.fontFamily
                            font.pixelSize: 13
                            font.weight: theme.weightSemibold
                            font.letterSpacing: 1.0
                            color: theme.text
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                            Layout.alignment: Qt.AlignVCenter
                        }
                    }

                    ToolTip.visible: brandMouse.containsMouse
                    ToolTip.text: "Ryzora · Discover"
                    ToolTip.delay: 400

                    MouseArea {
                        id: brandMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.tabSelected("discover")
                    }
                }

                // Collapse / Expand Toggle Button (ALWAYS visible in BOTH expanded and compact modes)
                Rectangle {
                    id: toggleBtn
                    objectName: "sidebarToggleBtn"
                    width: root.isCollapsed ? 24 : 26
                    height: root.isCollapsed ? 24 : 26
                    Layout.preferredWidth: root.isCollapsed ? 24 : 26
                    Layout.preferredHeight: root.isCollapsed ? 24 : 26
                    radius: theme.radiusSm
                    color: colMouse.containsMouse ? theme.surfaceHover : "transparent"
                    border.color: colMouse.containsMouse ? theme.borderSubtle : "transparent"
                    border.width: 1
                    Layout.alignment: Qt.AlignVCenter | Qt.AlignRight

                    RyzoraIcon {
                        anchors.centerIn: parent
                        name: root.isCollapsed ? "chevron-right" : "chevron-left"
                        size: 11
                        color: colMouse.containsMouse ? theme.text : theme.textMuted
                    }

                    ToolTip.visible: colMouse.containsMouse
                    ToolTip.text: root.isCollapsed ? "Expand sidebar (Ctrl+B / F9)" : "Collapse sidebar (Ctrl+B / F9)"
                    ToolTip.delay: 300

                    MouseArea {
                        id: colMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.toggleSidebar()
                    }
                }
            }
        }

        // ── 2. Navigation Items (Scrollable) ──
        Flickable {
            id: sideBarFlickable
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: navCol.implicitHeight + 16
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            ScrollBar.vertical: ScrollBar {
                id: sideBarScroll
                policy: ScrollBar.AsNeeded
                width: 4
                contentItem: Rectangle {
                    implicitWidth: 4
                    radius: 2
                    color: sideBarScroll.pressed ? theme.accent : (sideBarScroll.hovered ? theme.textMuted : theme.borderStrong)
                }
            }

            WheelHandler {
                target: parent
                orientation: Qt.Vertical
                onWheel: function(event) {
                    var step = 48
                    if (event.angleDelta.y > 0) {
                        sideBarFlickable.contentY = Math.max(0, sideBarFlickable.contentY - step)
                    } else {
                        sideBarFlickable.contentY = Math.min(Math.max(0, sideBarFlickable.contentHeight - sideBarFlickable.height), sideBarFlickable.contentY + step)
                    }
                }
            }

            ColumnLayout {
                id: navCol
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.margins: root.isCollapsed ? 4 : 8
                spacing: 2

                                // Group 1: STORE
                SectionHeader { label: "STORE"; collapsed: root.isCollapsed }

                NavButton {
                    label: "Discover"
                    iconName: "compass"
                    tabId: "discover"
                    isActive: root.currentTab === "discover" || root.currentTab === "collections" || root.currentTab === "creators" || root.currentTab === "repositories"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("discover")
                }

                NavButton {
                    label: "Apps"
                    iconName: "apps"
                    tabId: "apps"
                    badgeCount: backend ? backend.activeAppOperationsCount : 0
                    isActive: root.currentTab === "apps"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("apps")
                }

                NavButton {
                    label: "Library"
                    iconName: "library"
                    tabId: "library"
                    badgeCount: backend.libraryCount
                    isActive: root.currentTab === "library"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("library")
                }

                NavButton {
                    label: "Updates"
                    iconName: "arrow_sync"
                    tabId: "updates"
                    badgeCount: backend.updatesCount
                    isActive: root.currentTab === "updates"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("updates")
                }

                // Group 2: PERSONALIZE
                SectionHeader { label: "PERSONALIZE"; collapsed: root.isCollapsed }

                NavButton {
                    label: "Studio"
                    iconName: "sliders"
                    tabId: "studio"
                    isActive: root.currentTab === "studio" || root.currentTab === "lockscreens" || root.currentTab === "fastfetch"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("studio")
                }

                // Group 3: TOOLS
                SectionHeader { label: "TOOLS"; collapsed: root.isCollapsed }

                NavButton {
                    label: "Backups"
                    iconName: "archive"
                    tabId: "backups"
                    badgeCount: backend.snapshotsCount
                    isActive: root.currentTab === "backups"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("backups")
                }

                NavButton {
                    label: "Diagnostics"
                    iconName: "pulse"
                    tabId: "system"
                    isActive: root.currentTab === "system" || root.currentTab === "compat_lab" || root.currentTab === "integrity" || root.currentTab === "activity" || root.currentTab === "author"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("system")
                }

                NavButton {
                    label: "Settings"
                    iconName: "settings"
                    tabId: "settings"
                    isActive: root.currentTab === "settings" || root.currentTab === "storage"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("settings")
                }
            }
        }
    }

    component SectionHeader : Item {
        property string label: ""
        property bool collapsed: false
        Layout.fillWidth: true
        height: collapsed ? 10 : 22

        // Divider in collapsed mode
        Rectangle {
            visible: collapsed
            anchors.centerIn: parent
            width: 24
            height: 1
            color: theme.borderSubtle
        }

        // Label in expanded mode
        Text {
            visible: !collapsed
            text: label
            font.family: theme.fontFamily
            font.pixelSize: 11
            font.weight: theme.weightMedium
            font.letterSpacing: 1.0
            color: theme.navSectionLabel
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    component NavButton : Rectangle {
        id: navBtn
        objectName: "navBtn_" + navBtn.tabId
        property string label: ""
        property string iconName: "compass"
        property string tabId: ""
        property int badgeCount: 0
        property string badgeText: ""
        property bool isActive: false
        property bool collapsed: false
        signal clicked()

        Layout.fillWidth: true
        implicitHeight: 36
        height: 36
        radius: theme.radiusMd
        color: isActive ? theme.navItemActiveBg : (btnMouse.containsMouse ? theme.surfaceHover : "transparent")
        border.color: navBtn.activeFocus ? theme.accent : "transparent"
        border.width: navBtn.activeFocus ? 1 : 0
        activeFocusOnTab: true

        Keys.onReturnPressed: navBtn.clicked()
        Keys.onSpacePressed: navBtn.clicked()

        Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }

        // Left accent bar on active
        Rectangle {
            visible: navBtn.isActive
            width: 3
            height: 18
            radius: 1.5
            color: theme.accent
            anchors.left: parent.left
            anchors.leftMargin: 3
            anchors.verticalCenter: parent.verticalCenter
            z: 2
        }

        ToolTip.visible: navBtn.collapsed && btnMouse.containsMouse
        ToolTip.text: navBtn.label + (navBtn.badgeCount > 0 ? " (" + navBtn.badgeCount + ")" : (navBtn.badgeText.length > 0 ? " [" + navBtn.badgeText + "]" : ""))
        ToolTip.delay: 250

        // Expanded Layout
        RowLayout {
            visible: !navBtn.collapsed
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            spacing: 12

            RyzoraIcon {
                name: (navBtn.iconName && navBtn.iconName.length > 0) ? navBtn.iconName : "compass"
                size: 20
                filled: isActive
                color: isActive ? theme.accent : (btnMouse.containsMouse ? theme.text : theme.textSecondary)
                Layout.alignment: Qt.AlignVCenter
            }

            Text {
                text: navBtn.label
                font.family: theme.fontFamily
                font.pixelSize: theme.fontNav
                font.weight: isActive ? theme.weightSemibold : (btnMouse.containsMouse ? theme.weightMedium : theme.weightRegular)
                color: isActive ? theme.navItemActiveText : (btnMouse.containsMouse ? theme.navItemHover : theme.navItem)
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                elide: Text.ElideRight
            }

            // Numeric Badge
            Rectangle {
                visible: navBtn.badgeCount > 0
                implicitWidth: badgeLabel.implicitWidth + 10
                implicitHeight: 16
                radius: 8
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                Text {
                    id: badgeLabel
                    anchors.centerIn: parent
                    text: navBtn.badgeCount.toString()
                    font.pixelSize: 9
                    font.bold: true
                    font.family: "monospace"
                    color: theme.textSecondary
                }
            }

            // String Badge (e.g. "DEV")
            Rectangle {
                visible: navBtn.badgeText.length > 0
                implicitWidth: textBadgeLabel.implicitWidth + 8
                implicitHeight: 16
                radius: 4
                color: "#2e1065"
                border.color: "#7c3aed"
                border.width: 1

                Text {
                    id: textBadgeLabel
                    anchors.centerIn: parent
                    text: navBtn.badgeText
                    font.pixelSize: 8
                    font.bold: true
                    font.family: "monospace"
                    color: "#c4b5fd"
                }
            }
        }

        // Collapsed Layout (Centered icon + small dot indicator)
        Item {
            visible: navBtn.collapsed
            anchors.fill: parent

            RyzoraIcon {
                anchors.centerIn: parent
                name: (navBtn.iconName && navBtn.iconName.length > 0) ? navBtn.iconName : "compass"
                size: 20
                filled: isActive
                color: isActive ? theme.accent : (btnMouse.containsMouse ? theme.text : theme.textSecondary)
            }

            Rectangle {
                visible: navBtn.badgeCount > 0 || navBtn.badgeText.length > 0
                width: 6
                height: 6
                radius: 3
                color: theme.accent
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.topMargin: 4
                anchors.rightMargin: 4
            }
        }

        MouseArea {
            id: btnMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: navBtn.clicked()
        }
    }

    // ── Draggable Resize Divider (Dolphin style) ──
    Rectangle {
        id: resizeDividerVisual
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 2
        color: root.isDragging ? theme.accent : (resizeMouseArea.containsMouse ? theme.borderStrong : "transparent")
        z: 99
    }

    MouseArea {
        id: resizeMouseArea
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 8
        enabled: !root.isCollapsed
        cursorShape: Qt.SplitHCursor
        hoverEnabled: true
        z: 100
        property int startX: 0
        property int startW: 0

        onPressed: function(mouse) {
            root.isDragging = true
            startX = mouse.x
            startW = root.expandedWidth
        }
        onPositionChanged: function(mouse) {
            if (pressed) {
                var delta = mouse.x - startX
                var newW = Math.max(root.minExpandedWidth, Math.min(root.maxExpandedWidth, startW + delta))
                root.expandedWidth = newW
            }
        }
        onReleased: {
            root.isDragging = false
            root.persistSidebarSettings()
        }
    }
}
