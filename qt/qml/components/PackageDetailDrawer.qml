import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    anchors.fill: parent
    visible: opacity > 0
    opacity: isOpen ? 1.0 : 0.0
    z: 1000

    property var packageData: null
    property bool isOpen: false

    signal closeRequested()
    signal actionRequested(string action, var data)

    Behavior on opacity {
        NumberAnimation { duration: theme.reduceMotion ? 0 : 200 }
    }

    // Backdrop Dimmer
    Rectangle {
        anchors.fill: parent
        color: "#99000000"

        MouseArea {
            anchors.fill: parent
            onClicked: root.closeRequested()
        }
    }

    // Slide-in Drawer Container
    Rectangle {
        id: drawerPanel
        width: Math.min(480, root.width * 0.85)
        height: root.height
        anchors.right: parent.right
        color: theme.surface
        border.color: theme.border
        border.width: 1

        x: root.isOpen ? root.width - width : root.width

        Behavior on x {
            NumberAnimation { duration: theme.reduceMotion ? 0 : 250; easing.type: Easing.OutCubic }
        }

        MouseArea {
            anchors.fill: parent
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            // Drawer Header
            Rectangle {
                Layout.fillWidth: true
                height: 64
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 20
                    anchors.rightMargin: 16
                    spacing: 12

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            spacing: 8
                            Text {
                                text: (root.packageData && root.packageData.name) ? String(root.packageData.name) : "Package Details"
                                font.pixelSize: theme.fontLg
                                font.bold: true
                                color: theme.text
                                elide: Text.ElideRight
                                Layout.maximumWidth: 320
                            }
                            Text {
                                text: (root.packageData && root.packageData.version) ? "v" + String(root.packageData.version) : ""
                                font.pixelSize: theme.fontSm
                                color: theme.textMuted
                            }
                        }

                        Text {
                            text: (root.packageData && root.packageData.id) ? "ID: " + String(root.packageData.id) : ""
                            font.pixelSize: theme.fontXs
                            color: theme.textFaint
                        }
                    }

                    Rectangle {
                        width: 32
                        height: 32
                        radius: theme.radiusMd
                        color: closeBtnMouse.containsMouse ? theme.surfaceHover : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: "✕"
                            font.pixelSize: 14
                            color: theme.textSecondary
                        }

                        MouseArea {
                            id: closeBtnMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.closeRequested()
                        }
                    }
                }
            }

            // Scrollable Content
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                ScrollBar.vertical.policy: ScrollBar.AsNeeded

                ColumnLayout {
                    width: drawerPanel.width - 40
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing: 20

                    Item { height: 4 }

                    // Badges Bar
                    RowLayout {
                        spacing: 8

                        Rectangle {
                            height: 22
                            width: catText.width + 12
                            radius: theme.radiusSm
                            color: theme.accentFaint
                            border.color: theme.accent

                            Text {
                                id: catText
                                anchors.centerIn: parent
                                text: {
                                    if (!root.packageData) return ""
                                    var cat = root.packageData.category || "GENERAL"
                                    if (root.packageData.subfilter) return cat.toUpperCase() + " • " + String(root.packageData.subfilter).toUpperCase()
                                    return cat.toUpperCase()
                                }
                                font.pixelSize: theme.fontXs
                                font.bold: true
                                color: theme.accent
                            }
                        }

                        Rectangle {
                            visible: root.packageData ? Boolean(root.packageData.is_installed) : false
                            height: 22
                            width: instBadgeText.width + 12
                            radius: theme.radiusSm
                            color: theme.successBg
                            border.color: theme.success

                            Text {
                                id: instBadgeText
                                anchors.centerIn: parent
                                text: {
                                    if (!root.packageData) return ""
                                    var t = root.packageData.active_targets
                                    if (t && t.sddm_login) return "★ SDDM Login Active"
                                    if (t && t.session_lock) return "★ Session Lock Active"
                                    if (t && t.sddm_lock) return "★ SDDM Lock Active"
                                    if (root.packageData.is_active) return "★ Active Integration"
                                    return "✓ Installed"
                                }
                                font.pixelSize: theme.fontXs
                                font.bold: true
                                color: (root.packageData && root.packageData.is_active) ? "#a5b4fc" : theme.success
                            }
                        }

                        Rectangle {
                            visible: root.packageData ? !Boolean(root.packageData.is_fixture) : false
                            height: 22
                            width: verifBadgeText.width + 12
                            radius: theme.radiusSm
                            color: "#1e1b4b"
                            border.color: "#6366f1"

                            Text {
                                id: verifBadgeText
                                anchors.centerIn: parent
                                text: "🛡️ Verified Source"
                                font.pixelSize: theme.fontXs
                                font.bold: true
                                color: "#a5b4fc"
                            }
                        }

                        Item { Layout.fillWidth: true }
                    }

                    // Description Section
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text {
                            text: "Description"
                            font.pixelSize: theme.fontBase
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            text: (root.packageData && root.packageData.description) ? String(root.packageData.description) : "No description provided."
                            font.pixelSize: theme.fontBase
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }

                    // Metadata Card
                    Rectangle {
                        Layout.fillWidth: true
                        height: metaCol.implicitHeight + 24
                        radius: theme.radiusMd
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            id: metaCol
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 8

                            MetaRow { label: "Author"; value: (root.packageData && root.packageData.author) ? (typeof root.packageData.author === "object" ? (root.packageData.author.name || "—") : String(root.packageData.author)) : "—" }
                            MetaRow { label: "Package ID"; value: (root.packageData && root.packageData.id) ? String(root.packageData.id) : "—" }
                            MetaRow { label: "Category"; value: (root.packageData && root.packageData.category) ? String(root.packageData.category) : "—" }
                            MetaRow { label: "Subfilter"; value: (root.packageData && root.packageData.subfilter) ? String(root.packageData.subfilter) : "none" }
                            MetaRow {
                                label: "State"
                                value: {
                                    if (!root.packageData) return "—"
                                    if (root.packageData.is_active) {
                                        var t = root.packageData.active_targets
                                        var targets = []
                                        if (t && t.session_lock) targets.push("Session Lock")
                                        if (t && t.sddm_login) targets.push("SDDM Login")
                                        if (t && t.sddm_lock) targets.push("SDDM Lock")
                                        return targets.length > 0 ? "Active (" + targets.join(", ") + ")" : "Active"
                                    }
                                    return root.packageData.is_installed ? "Installed (Ready)" : "Available in Catalog"
                                }
                            }
                            MetaRow {
                                label: "Rating"
                                value: (root.packageData && root.packageData.rating !== undefined && root.packageData.rating !== null) ? (Number(root.packageData.rating).toFixed(1) + " / 5.0") : "—"
                            }
                            MetaRow {
                                label: "Downloads"
                                value: (root.packageData && root.packageData.download_count !== undefined && root.packageData.download_count !== null) ? String(root.packageData.download_count) : "—"
                            }
                            MetaRow { label: "Integrity"; value: (root.packageData && root.packageData.is_fixture) ? "Synthetic Fixture" : "Sha256 Verified" }
                        }
                    }

                    // Tags Section
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        visible: root.packageData ? (Boolean(root.packageData.tags) && root.packageData.tags.length > 0) : false

                        Text {
                            text: "Tags"
                            font.pixelSize: theme.fontBase
                            font.bold: true
                            color: theme.text
                        }

                        Flow {
                            Layout.fillWidth: true
                            spacing: 6

                            Repeater {
                                model: (root.packageData && root.packageData.tags) ? root.packageData.tags : []
                                delegate: Rectangle {
                                    height: 20
                                    width: tagLabel.width + 10
                                    radius: theme.radiusSm
                                    color: theme.surfaceActive
                                    border.color: theme.borderStrong
                                    border.width: 1

                                    Text {
                                        id: tagLabel
                                        anchors.centerIn: parent
                                        text: "#" + String(modelData)
                                        font.pixelSize: theme.fontXs
                                        color: theme.textSecondary
                                    }
                                }
                            }
                        }
                    }

                    Item { height: 16 }
                }
            }

            // Drawer Action Footer
            Rectangle {
                Layout.fillWidth: true
                height: 64
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 10

                    // 1. Not installed -> Install button
                    Rectangle {
                        visible: Boolean(root.packageData && !root.packageData.is_installed)
                        Layout.fillWidth: true
                        height: 38
                        radius: theme.radiusMd
                        color: installMouse.containsMouse ? theme.accentHover : theme.accent

                        Text {
                            anchors.centerIn: parent
                            text: "Install Package"
                            font.pixelSize: theme.fontBase
                            font.bold: true
                            color: "#0a0a0c"
                        }

                        MouseArea {
                            id: installMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.packageData) {
                                    root.actionRequested("install", root.packageData)
                                }
                            }
                        }
                    }

                    // 2. Installed & Inactive -> Apply button
                    Rectangle {
                        visible: Boolean(root.packageData && root.packageData.is_installed && !root.packageData.is_active)
                        Layout.fillWidth: true
                        height: 38
                        radius: theme.radiusMd
                        color: applyMouse.containsMouse ? theme.accentHover : theme.accent

                        Text {
                            anchors.centerIn: parent
                            text: "Apply to System"
                            font.pixelSize: theme.fontBase
                            font.bold: true
                            color: "#0a0a0c"
                        }

                        MouseArea {
                            id: applyMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.packageData) {
                                    root.actionRequested("apply", root.packageData)
                                }
                            }
                        }
                    }

                    // 3. Installed & Active -> Deactivate button
                    Rectangle {
                        visible: Boolean(root.packageData && root.packageData.is_active)
                        Layout.fillWidth: true
                        height: 38
                        radius: theme.radiusMd
                        color: deactMouse.containsMouse ? "#451a03" : theme.surfaceElevated
                        border.color: deactMouse.containsMouse ? "#f59e0b" : "#b45309"
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: "Deactivate Target"
                            font.pixelSize: theme.fontBase
                            font.bold: true
                            color: "#f59e0b"
                        }

                        MouseArea {
                            id: deactMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.packageData) {
                                    root.actionRequested("deactivate", root.packageData)
                                }
                            }
                        }
                    }

                    // 4. Uninstall Button (Visible when installed)
                    Rectangle {
                        visible: Boolean(root.packageData && root.packageData.is_installed)
                        width: 90
                        height: 38
                        radius: theme.radiusMd
                        color: uninstMouse.containsMouse ? "#3b1219" : theme.surfaceElevated
                        border.color: uninstMouse.containsMouse ? "#ef4444" : theme.borderStrong
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: "Uninstall"
                            font.pixelSize: theme.fontBase
                            font.bold: true
                            color: (root.packageData && root.packageData.is_active) ? theme.textFaint : (uninstMouse.containsMouse ? "#f87171" : theme.textMuted)
                        }

                        MouseArea {
                            id: uninstMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: (root.packageData && root.packageData.is_active) ? Qt.ForbiddenCursor : Qt.PointingHandCursor
                            onClicked: {
                                if (root.packageData) {
                                    root.actionRequested("uninstall", root.packageData)
                                }
                            }
                        }
                    }

                    // 5. Workspace Quick Launch (Fastfetch preview / Lock Screen studio)
                    Rectangle {
                        visible: Boolean(root.packageData && (root.packageData.category === "fastfetch" || root.packageData.category === "lockscreens"))
                        width: wsLabel.implicitWidth + 24
                        height: 38
                        radius: theme.radiusMd
                        color: wsMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                        border.color: theme.accent
                        border.width: 1

                        Text {
                            id: wsLabel
                            anchors.centerIn: parent
                            text: (root.packageData && root.packageData.category === "fastfetch") ? "⚡ Preview" : "🔒 Studio"
                            font.pixelSize: theme.fontBase
                            font.bold: true
                            color: theme.accent
                        }

                        MouseArea {
                            id: wsMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.packageData) {
                                    if (root.packageData.category === "fastfetch") {
                                        root.actionRequested("open_fastfetch", root.packageData)
                                    } else {
                                        root.actionRequested("open_lockscreen", root.packageData)
                                    }
                                }
                            }
                        }
                    }

                    // 6. Dismiss Button
                    Rectangle {
                        width: 80
                        height: 38
                        radius: theme.radiusMd
                        color: dismissMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                        border.color: theme.border
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: "Close"
                            font.pixelSize: theme.fontBase
                            color: theme.text
                        }

                        MouseArea {
                            id: dismissMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.closeRequested()
                        }
                    }
                }
            }
        }
    }

    component MetaRow : RowLayout {
        property string label: ""
        property string value: ""
        Layout.fillWidth: true
        spacing: 8

        Text {
            text: label
            font.pixelSize: theme.fontSm
            color: theme.textMuted
            Layout.minimumWidth: 90
        }

        Text {
            text: value
            font.pixelSize: theme.fontSm
            color: theme.text
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }
}
