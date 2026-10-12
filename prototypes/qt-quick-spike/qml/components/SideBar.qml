import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    color: theme.sidebarBg
    border.color: theme.borderSubtle
    border.width: 1

    property string currentTab: "discover"
    property bool isCollapsed: false
    signal tabSelected(string tab)
    signal toggleCollapse()
    signal openAbout()

    Layout.preferredWidth: isCollapsed ? 58 : 218
    Layout.minimumWidth: isCollapsed ? 58 : 208
    Layout.maximumWidth: isCollapsed ? 58 : 230

    Behavior on Layout.preferredWidth {
        NumberAnimation { duration: theme.reduceMotion ? 0 : 150; easing.type: Easing.InOutQuad }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── 1. Brand Header ──
        Rectangle {
            Layout.fillWidth: true
            height: 52
            color: "transparent"
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: root.isCollapsed ? 12 : 16
                anchors.rightMargin: root.isCollapsed ? 12 : 12
                spacing: 10

                Rectangle {
                    width: 28
                    height: 28
                    radius: theme.radiusMd
                    color: theme.accent

                    Text {
                        anchors.centerIn: parent
                        text: "R"
                        font.pixelSize: 13
                        font.bold: true
                        font.family: "monospace"
                        color: "#ffffff"
                    }
                }

                Text {
                    visible: !root.isCollapsed
                    text: "RYZORA"
                    font.pixelSize: 12
                    font.bold: true
                    font.letterSpacing: 1.5
                    color: theme.text
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                }

                // Collapse Button
                Rectangle {
                    visible: !root.isCollapsed
                    width: 26
                    height: 26
                    radius: theme.radiusSm
                    color: colMouse.containsMouse ? theme.surfaceHover : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: "◀"
                        font.pixelSize: 10
                        color: theme.textMuted
                    }

                    MouseArea {
                        id: colMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.isCollapsed = true
                    }
                }
            }
        }

        // ── 2. Navigation Items (Scrollable) ──
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: navCol.implicitHeight + 16
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            ColumnLayout {
                id: navCol
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.margins: root.isCollapsed ? 4 : 8
                spacing: 2

                // Group 1: STORE
                SectionHeader { label: "STORE"; collapsed: root.isCollapsed }

                NavButton {
                    label: "Home"
                    iconSymbol: "🌐"
                    tabId: "hub"
                    isActive: root.currentTab === "hub"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("hub")
                }

                NavButton {
                    label: "Discover"
                    iconSymbol: "🧭"
                    tabId: "discover"
                    isActive: root.currentTab === "discover"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("discover")
                }

                NavButton {
                    label: "Apps"
                    iconSymbol: "🪟"
                    tabId: "apps"
                    isActive: root.currentTab === "apps"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("apps")
                }

                NavButton {
                    label: "Library"
                    iconSymbol: "📚"
                    tabId: "library"
                    badgeCount: backend.libraryCount
                    isActive: root.currentTab === "library"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("library")
                }

                NavButton {
                    label: "Updates"
                    iconSymbol: "⬆️"
                    tabId: "updates"
                    badgeCount: backend.updatesCount
                    isActive: root.currentTab === "updates"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("updates")
                }

                NavButton {
                    label: "Collections"
                    iconSymbol: "🔖"
                    tabId: "collections"
                    isActive: root.currentTab === "collections"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("collections")
                }

                // Group 2: COMMUNITY
                SectionHeader { label: "COMMUNITY"; collapsed: root.isCollapsed }

                NavButton {
                    label: "Repositories"
                    iconSymbol: "🗄️"
                    tabId: "repositories"
                    isActive: root.currentTab === "repositories"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("repositories")
                }

                NavButton {
                    label: "Creators"
                    iconSymbol: "👥"
                    tabId: "creators"
                    isActive: root.currentTab === "creators"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("creators")
                }

                // Group 3: TOOLS
                SectionHeader { label: "TOOLS"; collapsed: root.isCollapsed }

                NavButton {
                    label: "Backups"
                    iconSymbol: "🛡️"
                    tabId: "backups"
                    badgeCount: backend.snapshotsCount
                    isActive: root.currentTab === "backups"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("backups")
                }

                NavButton {
                    label: "Integrity"
                    iconSymbol: "🔒"
                    tabId: "integrity"
                    isActive: root.currentTab === "integrity"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("integrity")
                }

                NavButton {
                    label: "System"
                    iconSymbol: "💻"
                    tabId: "system"
                    isActive: root.currentTab === "system"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("system")
                }

                NavButton {
                    label: "Activity"
                    iconSymbol: "🔔"
                    tabId: "activity"
                    isActive: root.currentTab === "activity"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("activity")
                }

                NavButton {
                    label: "Pkg Creator"
                    iconSymbol: "📦"
                    tabId: "author"
                    isActive: root.currentTab === "author"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("author")
                }

                NavButton {
                    label: "Compat Lab"
                    iconSymbol: "🧪"
                    tabId: "compat_lab"
                    badgeText: "DEV"
                    isActive: root.currentTab === "compat_lab"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("compat_lab")
                }

                NavButton {
                    label: "Settings"
                    iconSymbol: "⚙️"
                    tabId: "settings"
                    isActive: root.currentTab === "settings"
                    collapsed: root.isCollapsed
                    onClicked: root.tabSelected("settings")
                }
            }
        }

        // ── 3. Footer System Card & About ──
        Rectangle {
            Layout.fillWidth: true
            height: root.isCollapsed ? 80 : 86
            color: "transparent"
            border.color: theme.borderSubtle
            border.width: 1

            // Expanded Footer
            ColumnLayout {
                visible: !root.isCollapsed
                anchors.fill: parent
                anchors.margins: 8
                spacing: 4

                Rectangle {
                    Layout.fillWidth: true
                    height: 42
                    radius: theme.radiusSm
                    color: sysMouse.containsMouse ? theme.surfaceHover : "transparent"

                    MouseArea {
                        id: sysMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.tabSelected("system")
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 1

                        Text {
                            text: (backend.systemInfo && backend.systemInfo.desktop) ? backend.systemInfo.desktop : "Hyprland"
                            font.pixelSize: 12
                            font.bold: true
                            color: theme.text
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }

                        Text {
                            text: "wayland · Connected"
                            font.pixelSize: 10
                            color: theme.textMuted
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 24
                    radius: theme.radiusSm
                    color: aboutMouse.containsMouse ? theme.surfaceHover : "transparent"

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        text: "About Ryzora"
                        font.pixelSize: 11
                        color: aboutMouse.containsMouse ? theme.text : theme.textMuted
                    }

                    MouseArea {
                        id: aboutMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.openAbout()
                    }
                }
            }

            // Collapsed Footer
            ColumnLayout {
                visible: root.isCollapsed
                anchors.fill: parent
                anchors.margins: 6
                spacing: 4

                Rectangle {
                    width: 38
                    height: 34
                    radius: theme.radiusSm
                    color: sysIconMouse.containsMouse ? theme.surfaceHover : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: "💻"
                        font.pixelSize: 14
                    }

                    MouseArea {
                        id: sysIconMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.tabSelected("system")
                    }
                }

                Rectangle {
                    width: 38
                    height: 28
                    radius: theme.radiusSm
                    color: expMouse.containsMouse ? theme.surfaceHover : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: "▶"
                        font.pixelSize: 10
                        color: theme.textMuted
                    }

                    MouseArea {
                        id: expMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.isCollapsed = false
                    }
                }
            }
        }
    }

    component SectionHeader : Item {
        property string label: ""
        property bool collapsed: false
        Layout.fillWidth: true
        height: collapsed ? 12 : 24

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
            font.pixelSize: 10
            font.bold: true
            font.letterSpacing: 1.0
            color: theme.navSectionLabel
            anchors.left: parent.left
            anchors.leftMargin: 10
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    component NavButton : Rectangle {
        id: navBtn
        property string label: ""
        property string iconSymbol: ""
        property string tabId: ""
        property int badgeCount: 0
        property string badgeText: ""
        property bool isActive: false
        property bool collapsed: false
        signal clicked()

        Layout.fillWidth: true
        implicitHeight: 34
        height: 34
        radius: theme.radiusMd
        color: isActive ? theme.navItemActiveBg : (btnMouse.containsMouse ? theme.surfaceHover : "transparent")

        Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }

        // Expanded Layout
        RowLayout {
            visible: !navBtn.collapsed
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            spacing: 10

            Text {
                text: navBtn.iconSymbol
                font.pixelSize: 13
                color: isActive ? theme.accent : theme.textMuted
                Layout.alignment: Qt.AlignVCenter
            }

            Text {
                text: navBtn.label
                font.pixelSize: 12
                font.bold: isActive
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

            Text {
                anchors.centerIn: parent
                text: navBtn.iconSymbol
                font.pixelSize: 14
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
}
