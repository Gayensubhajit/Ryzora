import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    anchors.fill: parent
    visible: isOpen
    opacity: isOpen ? 1.0 : 0.0
    enabled: isOpen
    z: 1000

    property var packageData: null
    property bool isOpen: false
    property string pendingAction: ""

    function syncPackageData() {
        if (!root.packageData) return
        var pkgId = root.packageData.id
        var updated = null
        var list = backend.catalogList
        for (var i = 0; i < list.length; ++i) {
            if (list[i].id === pkgId) {
                updated = list[i]
                break
            }
        }
        if (!updated) {
            var lib = backend.libraryList
            for (var j = 0; j < lib.length; ++j) {
                if (lib[j].id === pkgId) {
                    updated = lib[j]
                    break
                }
            }
        }
        if (updated) {
            root.packageData = Object.assign({}, updated)
        }
    }

    Connections {
        target: backend
        function onPackageTested(packageId, success, message) {
            root.pendingAction = ""
            root.syncPackageData()
        }
        function onTargetDeactivated(target, success, message) {
            root.pendingAction = ""
            root.syncPackageData()
        }
        function onTargetActivated(packageId, target, success, message) {
            root.pendingAction = ""
            root.syncPackageData()
        }
        function onPackageInstalled(packageId, success, message) {
            root.pendingAction = ""
            root.syncPackageData()
        }
        function onPackageUninstalled(packageId, success, message) {
            root.pendingAction = ""
            root.syncPackageData()
        }
        function onCatalogChanged() { root.syncPackageData() }
        function onLibraryChanged() { root.syncPackageData() }
        function onActiveConfigurationChanged() { root.syncPackageData() }
    }

    signal closeRequested()
    signal actionRequested(string action, var data)

    focus: isOpen
    Keys.onEscapePressed: root.closeRequested()

    Behavior on opacity {
        NumberAnimation { duration: theme.reduceMotion ? 0 : 200 }
    }

    // Backdrop Dimmer
    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, 0.45)

        MouseArea {
            anchors.fill: parent
            onClicked: root.closeRequested()
        }
    }

    // Slide-in Drawer Container
    Rectangle {
        id: drawerPanel
        width: Math.min(500, root.width * 0.90)
        height: root.height
        y: 0
        x: root.isOpen ? root.width - width : root.width
        color: theme.surface
        border.color: theme.borderSubtle
        border.width: 1

        Behavior on x {
            NumberAnimation { duration: theme.reduceMotion ? 0 : 250; easing.type: Easing.OutCubic }
        }

        MouseArea {
            anchors.fill: parent
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            // Drawer Header (macOS Quiet Title Bar)
            Rectangle {
                Layout.fillWidth: true
                height: 68
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 24
                    anchors.rightMargin: 20
                    spacing: 14

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            spacing: 8
                            Text {
                                text: (root.packageData && root.packageData.name) ? String(root.packageData.name) : "Package Details"
                                font.pixelSize: 18
                                font.bold: true
                                color: theme.text
                                elide: Text.ElideRight
                                Layout.maximumWidth: 340
                            }
                            Rectangle {
                                visible: root.packageData && Boolean(root.packageData.version)
                                height: 18
                                width: verTxt.implicitWidth + 8
                                radius: 4
                                color: theme.surfaceActive
                                border.color: theme.borderSubtle
                                border.width: 1

                                Text {
                                    id: verTxt
                                    anchors.centerIn: parent
                                    text: (root.packageData && root.packageData.version) ? "v" + String(root.packageData.version) : ""
                                    font.pixelSize: 10
                                    color: theme.textSecondary
                                }
                            }
                        }

                        Text {
                            text: {
                                if (!root.packageData) return ""
                                var cat = root.packageData.category ? (root.packageData.category.charAt(0).toUpperCase() + root.packageData.category.slice(1)) : "Customization"
                                var author = root.packageData.author ? (typeof root.packageData.author === "object" ? (root.packageData.author.name || "Community") : String(root.packageData.author)) : "Community"
                                return cat + " · by " + author
                            }
                            font.pixelSize: 12
                            color: theme.textSecondary
                        }
                    }

                    Rectangle {
                        width: 30
                        height: 30
                        radius: 15
                        color: closeBtnMouse.containsMouse ? theme.surfaceHover : "transparent"
                        border.color: closeBtnMouse.containsMouse ? theme.borderSubtle : "transparent"
                        border.width: 1

                        RyzoraIcon {
                            anchors.centerIn: parent
                            name: "x"
                            size: 12
                            color: closeBtnMouse.containsMouse ? theme.text : theme.textSecondary
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
                    width: drawerPanel.width - 48
                    x: 24
                    spacing: 18

                    Item { height: 4 }

                    // Genuine Artwork Preview Hero
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.min(230, Math.round((drawerPanel.width - 48) * 10 / 16))
                        radius: theme.radiusLg
                        color: theme.resolvedTheme === "light" ? "#F0F2F6" : "#181A20"
                        clip: true
                        border.color: theme.borderSubtle
                        border.width: 1

                        Image {
                            id: drawerHeroImg
                            anchors.fill: parent
                            source: {
                                if (!root.packageData || !root.packageData.preview_url) return ""
                                var u = String(root.packageData.preview_url).trim()
                                if (u.length === 0) return ""
                                if (u.indexOf("file://") === 0) {
                                    if (u.indexOf("file:///") !== 0 && u.indexOf("file://localhost/") !== 0) {
                                        return "file:///" + u.substring(7)
                                    }
                                    return u
                                }
                                if (u.indexOf("http://") === 0 || u.indexOf("https://") === 0) return u
                                if (u.indexOf("/") === 0) return "file://" + u
                                return ""
                            }
                            fillMode: (root.packageData && (root.packageData.subfilter === "emblem" || root.packageData.category === "fastfetch")) ? Image.PreserveAspectFit : Image.PreserveAspectCrop
                            asynchronous: true
                            cache: true
                            smooth: true
                            mipmap: true
                            visible: status === Image.Ready
                        }

                        // Placeholder when image is absent or loading
                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 8
                            visible: drawerHeroImg.status !== Image.Ready
                            RyzoraIcon {
                                name: {
                                    if (!root.packageData || !root.packageData.category) return "package"
                                    var cat = String(root.packageData.category).toLowerCase()
                                    if (cat === "lockscreens") return "lock"
                                    if (cat === "fastfetch") return "terminal"
                                    if (cat === "themes" || cat === "appearance") return "palette"
                                    if (cat === "wallpapers") return "image"
                                    return "package"
                                }
                                size: 28
                                color: theme.textMuted
                                opacity: drawerHeroImg.status === Image.Loading ? 0.35 : 0.65
                                Layout.alignment: Qt.AlignHCenter
                            }
                            Text {
                                text: {
                                    if (drawerHeroImg.status === Image.Loading) return "Loading Artwork…"
                                    return root.packageData ? (root.packageData.name || "Preview Artwork") : "Preview Artwork"
                                }
                                font.pixelSize: 13
                                font.bold: true
                                color: theme.textMuted
                            }
                        }
                    }

                    // Status & Category Badges
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        // Status Badge
                        Rectangle {
                            height: 24
                            implicitWidth: statBadgeRow.implicitWidth + 16
                            radius: 12
                            color: {
                                if (!root.packageData) return theme.surfaceElevated
                                if (root.packageData.is_active) return theme.resolvedTheme === "light" ? "#DCFCE7" : "#064E3B"
                                if (root.packageData.is_installed) return theme.resolvedTheme === "light" ? "#EFF6FF" : "#1E3A8A"
                                return theme.surfaceElevated
                            }
                            border.color: {
                                if (!root.packageData) return theme.borderSubtle
                                if (root.packageData.is_active) return "#10B981"
                                if (root.packageData.is_installed) return "#3B82F6"
                                return theme.borderSubtle
                            }
                            border.width: 1

                            RowLayout {
                                id: statBadgeRow
                                anchors.centerIn: parent
                                spacing: 5

                                Rectangle {
                                    width: 6
                                    height: 6
                                    radius: 3
                                    color: {
                                        if (!root.packageData) return theme.textMuted
                                        if (root.packageData.is_active) return "#10B981"
                                        if (root.packageData.is_installed) return "#3B82F6"
                                        return theme.textMuted
                                    }
                                }

                                Text {
                                    text: {
                                        if (!root.packageData) return ""
                                        if (root.packageData.is_active) {
                                            var cat = String(root.packageData.category || "").toLowerCase()
                                            return cat === "lockscreens" ? "Active on Desktop" : "Active on System"
                                        }
                                        if (root.packageData.is_tested) return "Tested · Verified"
                                        if (root.packageData.is_installed) return "Installed · Ready"
                                        return "Available in Catalog"
                                    }
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: {
                                        if (!root.packageData) return theme.textSecondary
                                        if (root.packageData.is_active) return theme.resolvedTheme === "light" ? "#047857" : "#34D399"
                                        if (root.packageData.is_installed) return theme.resolvedTheme === "light" ? "#1D4ED8" : "#93C5FD"
                                        return theme.textSecondary
                                    }
                                }
                            }
                        }

                        // Category Pill
                        Rectangle {
                            height: 24
                            implicitWidth: catText.implicitWidth + 16
                            radius: 12
                            color: theme.surfaceElevated
                            border.color: theme.borderSubtle
                            border.width: 1

                            Text {
                                id: catText
                                anchors.centerIn: parent
                                text: {
                                    if (!root.packageData) return ""
                                    var cat = root.packageData.category ? root.packageData.category.toUpperCase() : "PACKAGE"
                                    if (root.packageData.subfilter) return cat + " · " + String(root.packageData.subfilter).toUpperCase()
                                    return cat
                                }
                                font.pixelSize: 10
                                font.bold: true
                                color: theme.textMuted
                            }
                        }

                        Item { Layout.fillWidth: true }
                    }

                    // Description Section
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text {
                            text: "About"
                            font.pixelSize: 13
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            text: (root.packageData && root.packageData.description && root.packageData.description.trim().length > 0) ? String(root.packageData.description) : "No description provided."
                            font.pixelSize: 13
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
                            anchors.margins: 14
                            spacing: 8

                            MetaRow { label: "Category"; value: (root.packageData && root.packageData.category) ? (root.packageData.category.charAt(0).toUpperCase() + root.packageData.category.slice(1)) : "—" }
                            MetaRow { label: "Author"; value: (root.packageData && root.packageData.author) ? (typeof root.packageData.author === "object" ? (root.packageData.author.name || "Community") : String(root.packageData.author)) : "Community" }
                            MetaRow { label: "Package ID"; value: (root.packageData && root.packageData.id) ? String(root.packageData.id) : "—" }
                            MetaRow {
                                label: "Type"
                                value: {
                                    if (!root.packageData) return "—"
                                    var cat = String(root.packageData.category || "").toLowerCase()
                                    if (cat === "lockscreens") return "Lockscreen Theme"
                                    if (cat === "fastfetch") return "Fastfetch Terminal Preset"
                                    if (cat === "wallpapers") return "Desktop Wallpaper"
                                    if (cat === "rices") return "Hyprland Rice Setup"
                                    if (cat === "themes") return "GTK / Qt Desktop Theme"
                                    return "Theme Package"
                                }
                            }
                            MetaRow {
                                label: "Status"
                                value: {
                                    if (!root.packageData) return "—"
                                    return root.packageData.is_installed ? "Installed (Preview Ready)" : "Available in Catalog"
                                }
                            }
                            MetaRow { label: "Dependencies"; value: (root.packageData && root.packageData.dependencies && root.packageData.dependencies.length > 0) ? root.packageData.dependencies.join(", ") : "None (Standalone)" }
                        }
                    }

                    // Tags Section
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        visible: root.packageData ? (Boolean(root.packageData.tags) && root.packageData.tags.length > 0) : false

                        Text {
                            text: "Tags"
                            font.pixelSize: 13
                            font.bold: true
                            color: theme.text
                        }

                        Flow {
                            Layout.fillWidth: true
                            spacing: 6

                            Repeater {
                                model: (root.packageData && root.packageData.tags) ? root.packageData.tags : []
                                delegate: Rectangle {
                                    height: 22
                                    width: tagLabel.width + 12
                                    radius: 6
                                    color: theme.surfaceActive
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    Text {
                                        id: tagLabel
                                        anchors.centerIn: parent
                                        text: "#" + String(modelData)
                                        font.pixelSize: 11
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
                height: 68
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 10

                    // 1. Not installed -> Install button
                    RyzoraButton {
                        visible: Boolean(root.packageData && !root.packageData.is_installed)
                        Layout.fillWidth: true
                        variant: "primary"
                        size: "md"
                        iconName: "arrow-download"
                        iconFilled: true
                        text: "Install Package"
                        loading: root.pendingAction === "install"
                        loadingText: "Installing…"
                        onClicked: {
                            if (root.packageData && root.pendingAction === "") {
                                root.pendingAction = "install"
                                root.actionRequested("install", root.packageData)
                            }
                        }
                    }

                    // 2. Installed & Inactive -> Apply button
                    Rectangle {
                        visible: Boolean(root.packageData && root.packageData.is_installed && !root.packageData.is_active && String(root.packageData.category || "").toLowerCase() !== "lockscreens")
                        Layout.fillWidth: true
                        height: 36
                        radius: 8
                        color: applyMouse.containsMouse ? theme.accentHover : theme.accent

                        Text {
                            anchors.centerIn: parent
                            text: {
                                var cat = root.packageData ? String(root.packageData.category || "").toLowerCase() : ""
                                if (root.pendingAction === "apply") {
                                    return cat === "lockscreens" ? "Applying Lock Screen…" : "Applying to System…"
                                }
                                return cat === "lockscreens" ? "Apply Lock Screen" : "Apply to System"
                            }
                            font.pixelSize: 13
                            font.bold: true
                            color: "#FFFFFF"
                        }

                        MouseArea {
                            id: applyMouse
                            anchors.fill: parent
                            hoverEnabled: root.pendingAction === ""
                            enabled: root.pendingAction === ""
                            cursorShape: root.pendingAction === "" ? Qt.PointingHandCursor : Qt.WaitCursor
                            onClicked: {
                                if (root.packageData && root.pendingAction === "") {
                                    root.pendingAction = "apply"
                                    root.actionRequested("apply", root.packageData)
                                }
                            }
                        }
                    }

                    // 3. Installed & Active -> Deactivate button
                    Rectangle {
                        id: drawerDeactBtn
                        objectName: "drawerDeactivateButton"
                        visible: Boolean(root.packageData && root.packageData.is_active && String(root.packageData.category || "").toLowerCase() !== "lockscreens")
                        Layout.fillWidth: true
                        height: 36
                        radius: 8
                        color: deactMouse.containsMouse ? (theme.resolvedTheme === "light" ? "#FEE2E2" : "#451a03") : (theme.resolvedTheme === "light" ? "#FEF3C7" : "#2E1A0A")
                        border.color: deactMouse.containsMouse ? "#EF4444" : "#F59E0B"
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: {
                                var cat = root.packageData ? String(root.packageData.category || "").toLowerCase() : ""
                                if (root.pendingAction === "deactivate") {
                                    return cat === "lockscreens" ? "Deactivating Lock Screen…" : "Deactivating Target…"
                                }
                                return cat === "lockscreens" ? "Deactivate Lock Screen" : "Deactivate Target"
                            }
                            font.pixelSize: 13
                            font.bold: true
                            color: deactMouse.containsMouse ? "#DC2626" : (theme.resolvedTheme === "light" ? "#B45309" : "#F59E0B")
                        }

                        MouseArea {
                            id: deactMouse
                            objectName: "drawerDeactivateMouseArea"
                            anchors.fill: parent
                            hoverEnabled: root.pendingAction === ""
                            enabled: root.pendingAction === ""
                            cursorShape: root.pendingAction === "" ? Qt.PointingHandCursor : Qt.WaitCursor
                            onClicked: {
                                if (root.packageData && root.pendingAction === "") {
                                    var cat = String(root.packageData.category || "").toLowerCase();
                                    var target = cat === "lockscreens" ? "session_lock" : null;
                                    console.log("[QML Drawer] Deactivate clicked! package:", root.packageData ? root.packageData.id : "null", "target:", target);
                                    root.pendingAction = "deactivate"
                                    root.actionRequested("deactivate", root.packageData, target)
                                }
                            }
                        }
                    }

                    // 3b. Test Preview Button
                    Rectangle {
                        visible: Boolean(root.packageData && root.packageData.is_installed)
                        Layout.fillWidth: true
                        height: 36
                        radius: 8
                        color: drawerTestMouse.containsMouse ? theme.surfaceActive : theme.surfaceElevated
                        border.color: drawerTestMouse.containsMouse ? theme.accent : theme.borderSubtle
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: root.pendingAction === "test" ? "Launching Preview…" : "Test Preview"
                            font.pixelSize: 13
                            font.bold: true
                            color: drawerTestMouse.containsMouse ? theme.accent : theme.text
                        }

                        MouseArea {
                            id: drawerTestMouse
                            anchors.fill: parent
                            hoverEnabled: root.pendingAction === ""
                            enabled: root.pendingAction === ""
                            cursorShape: root.pendingAction === "" ? Qt.PointingHandCursor : Qt.WaitCursor
                            onClicked: {
                                if (root.packageData && root.pendingAction === "") {
                                    root.pendingAction = "test"
                                    root.actionRequested("test", root.packageData)
                                }
                            }
                        }
                    }

                    // 4. Uninstall Button (Visible when installed)
                    // 4. Uninstall Button (Destructive variant with Fluent trash icon)
                    RyzoraButton {
                        visible: Boolean(root.packageData && root.packageData.is_installed)
                        variant: "destructive"
                        size: "md"
                        iconName: "delete"
                        text: "Uninstall"
                        loading: root.pendingAction === "uninstall"
                        loadingText: "Uninstalling…"
                        enabled: Boolean(root.packageData && !root.packageData.is_active && root.pendingAction === "")
                        onClicked: {
                            if (root.packageData && root.pendingAction === "") {
                                root.pendingAction = "uninstall"
                                root.actionRequested("uninstall", root.packageData)
                            }
                        }
                    }

                    // 5. Workspace Quick Launch (Fastfetch preview / Lock Screen studio)
                    Rectangle {
                        visible: Boolean(root.packageData && (root.packageData.category === "fastfetch" || root.packageData.category === "lockscreens"))
                        width: wsLabel.implicitWidth + 24
                        height: 36
                        radius: 8
                        color: wsMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: theme.accent
                        border.width: 1

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon {
                                name: (root.packageData && root.packageData.category === "fastfetch") ? "zap" : "sliders"
                                size: 12
                                color: theme.accent
                            }
                            Text {
                                id: wsLabel
                                text: (root.packageData && root.packageData.category === "fastfetch") ? "Preview" : "Studio"
                                font.pixelSize: 12
                                font.bold: true
                                color: theme.accent
                            }
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
                        width: 76
                        height: 36
                        radius: 8
                        color: dismissMouse.containsMouse ? theme.surfaceHover : theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: "Done"
                            font.pixelSize: 12
                            font.bold: true
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
