import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    signal backRequested()
    signal packageSelected(var pkg)
    signal actionRequested(string action, var data, var targetOverride)

    property var packageData: null
    property string fromRoute: "discover"
    property string pendingAction: ""
    property string errorMessage: ""

    onPackageDataChanged: {
        root.errorMessage = ""
        root.pendingAction = ""
        if (detailScroll) {
            detailScroll.ScrollBar.vertical.position = 0
        }
    }

    onPendingActionChanged: {
        if (root.pendingAction !== "") {
            pendingTimeout.restart()
        } else {
            pendingTimeout.stop()
        }
    }

    Timer {
        id: pendingTimeout
        interval: 8000
        repeat: false
        onTriggered: {
            if (root.pendingAction !== "") {
                if (root.pendingAction === "test" && root.errorMessage === "") {
                    root.errorMessage = "Preview launch timed out."
                }
                root.pendingAction = ""
            }
        }
    }

    function syncPackageData() {
        if (!root.packageData || !root.packageData.id) return
        var pkgId = String(root.packageData.id)
        var updated = null
        var list = backend.catalogList || []
        for (var i = 0; i < list.length; ++i) {
            if (list[i] && String(list[i].id) === pkgId) {
                updated = list[i]
                break
            }
        }
        if (!updated) {
            var lib = backend.libraryList || []
            for (var j = 0; j < lib.length; ++j) {
                if (lib[j] && String(lib[j].id) === pkgId) {
                    updated = lib[j]
                    break
                }
            }
        }
        if (updated) {
            root.packageData = updated
        }
    }

    Connections {
        target: backend
        function onPackageTested(packageId, success, message) {
            if (root.packageData && packageId && String(packageId) !== String(root.packageData.id)) return
            root.pendingAction = ""
            pendingTimeout.stop()
            if (!success) {
                root.errorMessage = message || "Failed to launch preview."
            } else {
                root.errorMessage = ""
            }
            root.syncPackageData()
        }
        function onTargetActivated(packageId, target, success, message) {
            if (root.packageData && packageId && String(packageId) !== String(root.packageData.id)) return
            root.pendingAction = ""
            pendingTimeout.stop()
            if (!success) {
                root.errorMessage = message || "Failed to activate target."
            } else {
                root.errorMessage = ""
            }
            root.syncPackageData()
        }
        function onTargetDeactivated(target, success, message) {
            root.pendingAction = ""
            pendingTimeout.stop()
            if (!success) {
                root.errorMessage = message || "Failed to deactivate target."
            } else {
                root.errorMessage = ""
            }
            root.syncPackageData()
        }
        function onPackageInstalled(packageId, success, message) {
            if (root.packageData && packageId && String(packageId) !== String(root.packageData.id)) return
            root.pendingAction = ""
            pendingTimeout.stop()
            if (!success) {
                root.errorMessage = message || "Failed to install package."
            } else {
                root.errorMessage = ""
            }
            root.syncPackageData()
        }
        function onPackageUninstalled(packageId, success, message) {
            if (root.packageData && packageId && String(packageId) !== String(root.packageData.id)) return
            root.pendingAction = ""
            pendingTimeout.stop()
            if (!success) {
                root.errorMessage = message || "Failed to uninstall package."
            } else {
                root.errorMessage = ""
            }
            root.syncPackageData()
        }
        function onCatalogChanged() { root.syncPackageData() }
        function onLibraryChanged() { root.syncPackageData() }
        function onActiveConfigurationChanged() { root.syncPackageData() }
    }

    function isInstalled() {
        if (!root.packageData) return false
        return Boolean(root.packageData.is_installed)
    }

    function isTested() {
        if (!root.packageData) return false
        return Boolean(root.packageData.is_tested)
    }

    function isAnyActive() {
        if (!root.packageData) return false
        var cat = String(root.packageData.category || "").toLowerCase()
        if (cat === "lockscreens" || cat === "sddm") {
            return Boolean(root.packageData.active_targets && root.packageData.active_targets.sddm_login)
        }
        return Boolean(root.packageData.is_active)
    }

    function isLockscreen() {
        if (!root.packageData) return false
        var cat = String(root.packageData.category || "").toLowerCase()
        return cat === "lockscreens" || cat === "sddm"
    }

    function isFastfetch() {
        if (!root.packageData) return false
        return String(root.packageData.category || "").toLowerCase() === "fastfetch"
    }

    function supportsSddm() {
        if (!root.packageData) return false
        if (!root.isLockscreen()) return false
        if (root.packageData.supports_sddm !== undefined) return Boolean(root.packageData.supports_sddm)
        var id = String(root.packageData.id || "")
        if (id.indexOf("swaylock") !== -1 || id.indexOf("hyprlock") !== -1) return false
        return true
    }

    function getRelatedPackages() {
        if (!root.packageData) return []
        var all = backend.catalogList || []
        var related = []
        var currentCat = String(root.packageData.category || "").toLowerCase()
        var currentId = String(root.packageData.id || "")
        for (var i = 0; i < all.length && related.length < 4; ++i) {
            var item = all[i]
            if (item && String(item.id) !== currentId && String(item.category || "").toLowerCase() === currentCat) {
                related.push(item)
            }
        }
        return related
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── 1. Top Navigation & Breadcrumb Header (macOS Clean Bar) ──
        Rectangle {
            Layout.fillWidth: true
            height: 52
            color: theme.bg
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 24
                anchors.rightMargin: 24
                spacing: 14

                // Back Button (Restrained macOS Pill)
                Rectangle {
                    id: backBtn
                    objectName: "backButton"
                    height: 32
                    implicitWidth: backRow.implicitWidth + 20
                    radius: theme.radiusMd
                    color: backMouse.pressed
                        ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#383E4A")
                        : (backMouse.containsMouse
                            ? (theme.resolvedTheme === "light" ? "#F3F4F6" : "#2E3440")
                            : (theme.resolvedTheme === "light" ? "#FAFAFA" : "#252A34"))
                    border.color: backMouse.containsMouse
                        ? (theme.resolvedTheme === "light" ? "#9CA3AF" : "#5A6578")
                        : (theme.resolvedTheme === "light" ? "#E5E7EB" : "#3B4252")
                    border.width: 1

                    RowLayout {
                        id: backRow
                        anchors.centerIn: parent
                        spacing: 6

                        RyzoraIcon {
                            name: "chevron-left"
                            size: 13
                            color: theme.accent
                        }

                        Text {
                            text: root.fromRoute === "lockscreens" ? "Back to Lock Screens" : (root.fromRoute === "library" ? "Back to Library" : "Back to Discover")
                            font.pixelSize: 12
                            font.bold: true
                            color: theme.text
                        }
                    }

                    MouseArea {
                        id: backMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.backRequested()
                    }
                }

                // Vertical Divider
                Rectangle {
                    width: 1
                    height: 18
                    color: theme.borderSubtle
                }

                // Breadcrumb Path
                RowLayout {
                    spacing: 8
                    Layout.fillWidth: true

                    MouseArea {
                        implicitWidth: discTxt.implicitWidth
                        implicitHeight: discTxt.implicitHeight
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.backRequested()
                        Text {
                            id: discTxt
                            text: "Discover"
                            font.pixelSize: 12
                            color: parent.containsMouse ? theme.accent : theme.textMuted
                        }
                        hoverEnabled: true
                    }

                    RyzoraIcon {
                        name: "chevron-right"
                        size: 9
                        color: theme.textMuted
                    }

                    Text {
                        text: {
                            if (!root.packageData || !root.packageData.category) return "Customization"
                            var cat = String(root.packageData.category)
                            if (cat === "lockscreens") return "Lock Screens"
                            if (cat === "fastfetch") return "Fastfetch"
                            if (cat === "wallpapers") return "Wallpapers"
                            if (cat === "rices") return "Rices"
                            if (cat === "themes") return "Themes"
                            return cat.charAt(0).toUpperCase() + cat.slice(1)
                        }
                        font.pixelSize: 12
                        color: theme.textMuted
                    }

                    RyzoraIcon {
                        name: "chevron-right"
                        size: 9
                        color: theme.textMuted
                    }

                    Text {
                        text: root.packageData ? String(root.packageData.name) : ""
                        font.pixelSize: 12
                        font.bold: true
                        color: theme.text
                        elide: Text.ElideRight
                        Layout.maximumWidth: 280
                    }
                }

                // Header Status Pill (macOS restrained badge)
                Rectangle {
                    visible: root.packageData !== null
                    height: 26
                    implicitWidth: headStatRow.implicitWidth + 20
                    radius: 13
                    color: {
                        if (!root.packageData) return "transparent"
                        if (root.isAnyActive()) {
                            return theme.resolvedTheme === "light" ? "#ECFDF5" : "#064E3B"
                        }
                        if (root.isInstalled()) return theme.resolvedTheme === "light" ? "#EFF6FF" : "#1E293B"
                        return theme.resolvedTheme === "light" ? "#F9FAFB" : "#1F242D"
                    }
                    border.color: {
                        if (!root.packageData) return "transparent"
                        if (root.isAnyActive()) {
                            return theme.resolvedTheme === "light" ? "#A7F3D0" : "#059669"
                        }
                        if (root.isInstalled()) return theme.resolvedTheme === "light" ? "#BFDBFE" : "#3B82F6"
                        return theme.resolvedTheme === "light" ? "#E5E7EB" : "#374151"
                    }
                    border.width: 1

                    RowLayout {
                        id: headStatRow
                        anchors.centerIn: parent
                        spacing: 6

                        Rectangle {
                            width: 6
                            height: 6
                            radius: 3
                            color: {
                                if (!root.packageData) return "#9CA3AF"
                                if (root.isAnyActive()) return "#10B981"
                                if (root.isInstalled()) return "#3B82F6"
                                return "#9CA3AF"
                            }
                        }

                        Text {
                            text: {
                                if (!root.packageData) return ""
                                if (root.isAnyActive()) {
                                    var cat = String(root.packageData.category || "").toLowerCase()
                                    return (cat === "lockscreens" || cat === "sddm") ? "Active on Login Screen" : "Active on System"
                                }
                                if (root.isInstalled()) return "Installed"
                                return "Available"
                            }
                            font.pixelSize: 11
                            font.bold: true
                            color: {
                                if (!root.packageData) return theme.textSecondary
                                if (root.isAnyActive()) {
                                    return theme.resolvedTheme === "light" ? "#047857" : "#6EE7B7"
                                }
                                if (root.isInstalled()) {
                                    return theme.resolvedTheme === "light" ? "#1D4ED8" : "#93C5FD"
                                }
                                return theme.resolvedTheme === "light" ? "#4B5563" : "#9CA3AF"
                            }
                        }
                    }
                }
            }
        }

        // ── 2. Scrollable Detail Page Body ──
        ScrollView {
            id: detailScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: detailScroll.width
            contentHeight: detailScrollContent.implicitHeight + 40
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical.policy: ScrollBar.AsNeeded

            Item {
                id: detailScrollContent
                width: detailScroll.width
                implicitHeight: detailMainCol.implicitHeight + 40

                ColumnLayout {
                    id: detailMainCol
                    width: Math.min(1280, detailScroll.width - 64)
                    anchors.horizontalCenter: parent.horizontalCenter
                    y: 20
                    spacing: 28

                    // ── Hero Section (2-Column Desktop Grid) ──
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 28

                        // ── LEFT COLUMN: Media Preview & Art Showcase (~56% width) ──
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignTop
                            spacing: 12

                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.max(260, Math.min(460, detailScroll.width * 0.35))
                                radius: theme.radiusLg
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1
                                clip: true

                                // Ambient subtle blurred backdrop
                                Image {
                                    id: heroBgBlur
                                    anchors.fill: parent
                                    source: mainHeroImg.source
                                    fillMode: Image.PreserveAspectCrop
                                    asynchronous: true
                                    cache: true
                                    smooth: true
                                    opacity: 0.14
                                    visible: mainHeroImg.status === Image.Ready
                                }

                                Image {
                                    id: mainHeroImg
                                    anchors.fill: parent
                                    anchors.margins: (root.isFastfetch() || (root.packageData && root.packageData.subfilter === "emblem")) ? 24 : 0
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
                                    fillMode: (root.isFastfetch() || (root.packageData && root.packageData.subfilter === "emblem")) ? Image.PreserveAspectFit : Image.PreserveAspectCrop
                                    asynchronous: true
                                    cache: true
                                    smooth: true
                                    visible: status === Image.Ready
                                }

                                // Understated Fallback State when image is missing or loading
                                ColumnLayout {
                                    anchors.centerIn: parent
                                    spacing: 8
                                    visible: mainHeroImg.status !== Image.Ready

                                    RyzoraIcon {
                                        Layout.alignment: Qt.AlignHCenter
                                        name: {
                                            if (!root.packageData || !root.packageData.category) return "package"
                                            var cat = String(root.packageData.category).toLowerCase()
                                            if (cat === "lockscreens" || cat === "lockscreen") return "lock"
                                            if (cat === "fastfetch") return "terminal"
                                            if (cat === "themes" || cat === "theme") return "palette"
                                            if (cat === "wallpapers" || cat === "wallpaper") return "image"
                                            if (cat === "rices" || cat === "rice") return "sliders"
                                            return "package"
                                        }
                                        size: 38
                                        color: theme.textMuted
                                        opacity: mainHeroImg.status === Image.Loading ? 0.35 : 0.65
                                    }
                                    Text {
                                        Layout.alignment: Qt.AlignHCenter
                                        text: {
                                            if (mainHeroImg.status === Image.Loading) return "Loading Preview…"
                                            return root.packageData ? (root.packageData.name || "Preview Artwork") : "Preview Artwork"
                                        }
                                        font.pixelSize: 13
                                        font.bold: true
                                        color: theme.textMuted
                                    }
                                }
                            }

                            // Media Specification Badges
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 8

                                Rectangle {
                                    height: 24
                                    implicitWidth: fmtTxt.implicitWidth + 16
                                    radius: 6
                                    color: theme.surfaceElevated
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    Text {
                                        id: fmtTxt
                                        anchors.centerIn: parent
                                        text: "Ryzora Package Spec v1"
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                    }
                                }

                                Rectangle {
                                    height: 24
                                    implicitWidth: verifTxt.implicitWidth + 24
                                    radius: 6
                                    color: theme.surfaceElevated
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    RowLayout {
                                        anchors.centerIn: parent
                                        spacing: 4
                                        RyzoraIcon {
                                            name: "shield-check"
                                            size: 12
                                            color: theme.resolvedTheme === "light" ? "#047857" : "#34D399"
                                        }
                                        Text {
                                            id: verifTxt
                                            text: "Integrity Verified"
                                            font.pixelSize: 11
                                            font.bold: true
                                            color: theme.resolvedTheme === "light" ? "#047857" : "#34D399"
                                        }
                                    }
                                }

                                Rectangle {
                                    height: 24
                                    implicitWidth: formatTxt.implicitWidth + 16
                                    radius: 6
                                    color: theme.surfaceElevated
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    Text {
                                        id: formatTxt
                                        anchors.centerIn: parent
                                        text: {
                                            if (!root.packageData) return "Asset"
                                            if (root.isLockscreen()) return "Quickshell QML Component"
                                            if (root.isFastfetch()) return "JSON Terminal Config"
                                            var cat = String(root.packageData.category || "").toLowerCase()
                                            if (cat === "wallpapers") return "Wallpaper Asset"
                                            if (cat === "rices") return "Hyprland Rice Bundle"
                                            if (cat === "themes") return "GTK/Qt Desktop Theme"
                                            return "Customization Package"
                                        }
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                    }
                                }

                                Item { Layout.fillWidth: true }
                            }
                        }

                        // ── RIGHT COLUMN: Information, Actions & Technical Specs (~44% width) ──
                        ColumnLayout {
                            Layout.preferredWidth: Math.max(380, Math.min(520, detailScroll.width * 0.40))
                            Layout.minimumWidth: 360
                            Layout.maximumWidth: 540
                            Layout.alignment: Qt.AlignTop
                            spacing: 16

                            // Title, Version, Category, Author & Status Badges
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 6

                                Text {
                                    text: root.packageData ? String(root.packageData.name) : "Package Details"
                                    font.pixelSize: 22
                                    font.bold: true
                                    color: theme.text
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }

                                Text {
                                    text: {
                                        if (!root.packageData) return ""
                                        var parts = []
                                        if (root.packageData.version) parts.push("v" + root.packageData.version)
                                        var cat = root.packageData.category ? (root.packageData.category.charAt(0).toUpperCase() + root.packageData.category.slice(1)) : "Theme"
                                        if (cat === "Lockscreens") cat = "Lock Screens"
                                        parts.push(cat)
                                        var author = root.packageData.author ? (typeof root.packageData.author === "object" ? (root.packageData.author.name || "Community") : String(root.packageData.author)) : "Community"
                                        parts.push("by " + author)
                                        return parts.join(" · ")
                                    }
                                    font.pixelSize: 13
                                    color: theme.textSecondary
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }

                                // Status Badges Row: [Active] [Installed] [Preview verified]
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    // Active Pill
                                    Rectangle {
                                        visible: root.isAnyActive()
                                        height: 24
                                        implicitWidth: actPillTxt.implicitWidth + 24
                                        radius: 12
                                        color: theme.resolvedTheme === "light" ? "#ECFDF5" : "#064E3B"
                                        border.color: theme.resolvedTheme === "light" ? "#A7F3D0" : "#059669"
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 4
                                            Rectangle {
                                                width: 6
                                                height: 6
                                                radius: 3
                                                color: theme.resolvedTheme === "light" ? "#059669" : "#34D399"
                                            }
                                            Text {
                                                id: actPillTxt
                                                text: {
                                                    var cat = String(root.packageData ? root.packageData.category || "" : "").toLowerCase()
                                                    return (cat === "lockscreens" || cat === "sddm") ? "Active on Login Screen" : "Active on System"
                                                }
                                                font.pixelSize: 11
                                                font.bold: true
                                                color: theme.resolvedTheme === "light" ? "#047857" : "#6EE7B7"
                                            }
                                        }
                                    }

                                    // Installed Pill
                                    Rectangle {
                                        visible: root.isInstalled() && !root.isAnyActive()
                                        height: 24
                                        implicitWidth: instPillTxt.implicitWidth + 20
                                        radius: 12
                                        color: theme.resolvedTheme === "light" ? "#EFF6FF" : "#1E293B"
                                        border.color: theme.resolvedTheme === "light" ? "#BFDBFE" : "#3B82F6"
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 4
                                            RyzoraIcon {
                                                name: "check"
                                                size: 10
                                                color: theme.resolvedTheme === "light" ? "#1D4ED8" : "#93C5FD"
                                            }
                                            Text {
                                                id: instPillTxt
                                                text: "Installed"
                                                font.pixelSize: 11
                                                font.bold: true
                                                color: theme.resolvedTheme === "light" ? "#1D4ED8" : "#93C5FD"
                                            }
                                        }
                                    }

                                    // Preview Verified Pill (Lock screens)
                                    Rectangle {
                                        visible: root.isLockscreen() && root.isTested()
                                        height: 24
                                        implicitWidth: testedPillTxt.implicitWidth + 20
                                        radius: 12
                                        color: theme.resolvedTheme === "light" ? "#F5F3FF" : "#2E1065"
                                        border.color: theme.resolvedTheme === "light" ? "#DDD6FE" : "#7C3AED"
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 4
                                            RyzoraIcon {
                                                name: "eye"
                                                size: 11
                                                color: theme.resolvedTheme === "light" ? "#6D28D9" : "#C4B5FD"
                                            }
                                            Text {
                                                id: testedPillTxt
                                                text: "Preview verified"
                                                font.pixelSize: 11
                                                font.bold: true
                                                color: theme.resolvedTheme === "light" ? "#6D28D9" : "#C4B5FD"
                                            }
                                        }
                                    }

                                    // Available in Catalog Pill
                                    Rectangle {
                                        visible: !root.isInstalled()
                                        height: 24
                                        implicitWidth: availPillTxt.implicitWidth + 16
                                        radius: 12
                                        color: theme.resolvedTheme === "light" ? "#F3F4F6" : "#1F242D"
                                        border.color: theme.resolvedTheme === "light" ? "#E5E7EB" : "#374151"
                                        border.width: 1

                                        Text {
                                            id: availPillTxt
                                            anchors.centerIn: parent
                                            text: "Available in Catalog"
                                            font.pixelSize: 11
                                            font.bold: true
                                            color: theme.textMuted
                                        }
                                    }
                                }
                            }

                            // ── Primary Action Box (macOS Clean Panel) ──
                            Rectangle {
                                Layout.fillWidth: true
                                implicitHeight: actBoxCol.implicitHeight + 32
                                radius: theme.radiusMd
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1

                                // Progress Line Indicator during active pending operations
                                Rectangle {
                                    anchors.top: parent.top
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    height: 2.5
                                    color: theme.accent
                                    visible: root.pendingAction !== ""
                                }

                                ColumnLayout {
                                    id: actBoxCol
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.margins: 16
                                    spacing: 10

                                    // ── STATE 1: NOT INSTALLED -> GET BUTTON ──
                                    Rectangle {
                                        id: instBtn
                                        objectName: "installButton"
                                        visible: Boolean(root.packageData && !root.packageData.is_installed)
                                        Layout.fillWidth: true
                                        height: 38
                                        radius: theme.radiusSm
                                        color: instBtnMouse.pressed
                                            ? (theme.resolvedTheme === "light" ? "#0051A8" : "#005BB5")
                                            : (instBtnMouse.containsMouse
                                                ? (theme.resolvedTheme === "light" ? "#0062CC" : "#0071E3")
                                                : (theme.resolvedTheme === "light" ? "#007AFF" : "#0A84FF"))
                                        border.color: theme.resolvedTheme === "light" ? "#005BB5" : "#0A84FF"
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: root.pendingAction === "install" ? "refresh" : "download"
                                                size: 13
                                                color: "#FFFFFF"
                                            }
                                            Text {
                                                text: root.pendingAction === "install" ? "Installing…" : "Get"
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: "#FFFFFF"
                                            }
                                        }

                                        MouseArea {
                                            id: instBtnMouse
                                            objectName: "installMouseArea"
                                            anchors.fill: parent
                                            hoverEnabled: root.pendingAction === ""
                                            enabled: root.pendingAction === ""
                                            cursorShape: root.pendingAction === "" ? Qt.PointingHandCursor : Qt.WaitCursor
                                            onClicked: {
                                                if (root.packageData && root.pendingAction === "") {
                                                    root.errorMessage = ""; root.pendingAction = "install"
                                                    root.actionRequested("install", root.packageData, null)
                                                }
                                            }
                                        }
                                    }

                                    // ── STATE 2A: INSTALLED LOCKSCREEN (SDDM SUPPORTED) & INACTIVE -> APPLY TO LOGIN SCREEN ──
                                    Rectangle {
                                        id: primApplyBtn
                                        objectName: "primaryApplyButton"
                                        visible: Boolean(root.packageData && root.isInstalled() && !root.isAnyActive() && root.supportsSddm())
                                        Layout.fillWidth: true
                                        height: 38
                                        radius: theme.radiusSm
                                        color: primApplyMouse.pressed
                                            ? (theme.resolvedTheme === "light" ? "#0051A8" : "#005BB5")
                                            : (primApplyMouse.containsMouse
                                                ? (theme.resolvedTheme === "light" ? "#0062CC" : "#0071E3")
                                                : (theme.resolvedTheme === "light" ? "#007AFF" : "#0A84FF"))
                                        border.color: theme.resolvedTheme === "light" ? "#005BB5" : "#0A84FF"
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: root.pendingAction === "apply" ? "refresh" : "desktop"
                                                size: 13
                                                color: "#FFFFFF"
                                            }
                                            Text {
                                                text: root.pendingAction === "apply" ? "Applying…" : "Apply to Login Screen"
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: "#FFFFFF"
                                            }
                                        }

                                        MouseArea {
                                            id: primApplyMouse
                                            objectName: "primaryApplyMouseArea"
                                            anchors.fill: parent
                                            hoverEnabled: root.pendingAction === ""
                                            enabled: root.pendingAction === ""
                                            cursorShape: root.pendingAction === "" ? Qt.PointingHandCursor : Qt.WaitCursor
                                            onClicked: {
                                                if (root.packageData && root.pendingAction === "") {
                                                    root.errorMessage = ""; root.pendingAction = "apply"
                                                    root.actionRequested("apply", root.packageData, "sddm_login")
                                                }
                                            }
                                        }
                                    }

                                    // ── STATE 2B: INSTALLED LOCKSCREEN (SDDM SUPPORTED) & ACTIVE -> DEACTIVATE THEME ──
                                    Rectangle {
                                        id: primDeactBtn
                                        objectName: "primaryDeactivateButton"
                                        visible: Boolean(root.packageData && root.isInstalled() && root.isAnyActive() && root.supportsSddm())
                                        Layout.fillWidth: true
                                        height: 38
                                        radius: theme.radiusSm
                                        color: primDeactMouse.pressed
                                            ? (theme.resolvedTheme === "light" ? "#FCA5A5" : "#4A1B1E")
                                            : (primDeactMouse.containsMouse
                                                ? (theme.resolvedTheme === "light" ? "#FEE2E2" : "#3B181A")
                                                : (theme.resolvedTheme === "light" ? "#FFFFFF" : "#2C313A"))
                                        border.color: primDeactMouse.containsMouse ? "#EF4444" : (theme.resolvedTheme === "light" ? "#D1D5DB" : "#474F5D")
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: root.pendingAction === "deactivate" ? "refresh" : "stop"
                                                size: 12
                                                color: primDeactMouse.containsMouse ? (theme.resolvedTheme === "light" ? "#DC2626" : "#FCA5A5") : (theme.resolvedTheme === "light" ? "#111827" : "#F9FAFB")
                                            }
                                            Text {
                                                text: root.pendingAction === "deactivate" ? "Deactivating…" : "Deactivate Theme"
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: primDeactMouse.containsMouse ? (theme.resolvedTheme === "light" ? "#DC2626" : "#FCA5A5") : (theme.resolvedTheme === "light" ? "#111827" : "#F9FAFB")
                                            }
                                        }

                                        MouseArea {
                                            id: primDeactMouse
                                            objectName: "primaryDeactivateMouseArea"
                                            anchors.fill: parent
                                            hoverEnabled: root.pendingAction === ""
                                            enabled: root.pendingAction === ""
                                            cursorShape: root.pendingAction === "" ? Qt.PointingHandCursor : Qt.WaitCursor
                                            onClicked: {
                                                if (root.packageData && root.pendingAction === "") {
                                                    root.errorMessage = ""; root.pendingAction = "deactivate"
                                                    root.actionRequested("deactivate", root.packageData, "sddm_login")
                                                }
                                            }
                                        }
                                    }

                                    // ── STATE 2C: INSTALLED LOCKSCREEN -> PREVIEW THEME BUTTON ──
                                    Rectangle {
                                        id: testBtn
                                        objectName: "testButton"
                                        visible: Boolean(root.packageData && root.isInstalled() && root.isLockscreen())
                                        Layout.fillWidth: true
                                        height: 36
                                        radius: theme.radiusSm
                                        color: {
                                            if (primApplyBtn.visible) {
                                                return testBtnMouse.pressed
                                                    ? (theme.resolvedTheme === "light" ? "#D1D5DB" : "#404856")
                                                    : (testBtnMouse.containsMouse
                                                        ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#363C47")
                                                        : (theme.resolvedTheme === "light" ? "#F3F4F6" : "#242830"))
                                            } else {
                                                return testBtnMouse.pressed
                                                    ? (theme.resolvedTheme === "light" ? "#0051A8" : "#005BB5")
                                                    : (testBtnMouse.containsMouse
                                                        ? (theme.resolvedTheme === "light" ? "#0062CC" : "#0071E3")
                                                        : (theme.resolvedTheme === "light" ? "#007AFF" : "#0A84FF"))
                                            }
                                        }
                                        border.color: {
                                            if (primApplyBtn.visible) {
                                                return testBtnMouse.containsMouse ? (theme.resolvedTheme === "light" ? "#9CA3AF" : "#60697B") : (theme.resolvedTheme === "light" ? "#D1D5DB" : "#3E4450")
                                            } else {
                                                return theme.resolvedTheme === "light" ? "#005BB5" : "#0A84FF"
                                            }
                                        }
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: root.pendingAction === "test" ? "refresh" : "play"
                                                size: 12
                                                color: primApplyBtn.visible ? (theme.resolvedTheme === "light" ? "#111827" : "#F9FAFB") : "#FFFFFF"
                                            }
                                            Text {
                                                text: root.pendingAction === "test" ? "Opening Preview…" : "Preview Theme"
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: primApplyBtn.visible ? (theme.resolvedTheme === "light" ? "#111827" : "#F9FAFB") : "#FFFFFF"
                                            }
                                        }

                                        MouseArea {
                                            id: testBtnMouse
                                            objectName: "testMouseArea"
                                            anchors.fill: parent
                                            hoverEnabled: root.pendingAction === ""
                                            enabled: root.pendingAction === ""
                                            cursorShape: root.pendingAction === "" ? Qt.PointingHandCursor : Qt.WaitCursor
                                            onClicked: {
                                                if (root.packageData && root.pendingAction === "") {
                                                    console.log("[QML PackageDetailView] Test clicked! package:", root.packageData.id);
                                                    root.errorMessage = ""; root.pendingAction = "test"
                                                    root.actionRequested("test", root.packageData, null)
                                                }
                                            }
                                        }
                                    }

                                    // ── STATE 3: INSTALLED FASTFETCH -> OPEN IN FASTFETCH STUDIO ──
                                    Rectangle {
                                        id: openFfBtn
                                        objectName: "openFastfetchButton"
                                        visible: Boolean(root.packageData && root.isInstalled() && root.isFastfetch())
                                        Layout.fillWidth: true
                                        height: 38
                                        radius: theme.radiusSm
                                        color: openFfMouse.pressed
                                            ? (theme.resolvedTheme === "light" ? "#0051A8" : "#005BB5")
                                            : (openFfMouse.containsMouse
                                                ? (theme.resolvedTheme === "light" ? "#0062CC" : "#0071E3")
                                                : (theme.resolvedTheme === "light" ? "#007AFF" : "#0A84FF"))
                                        border.color: theme.resolvedTheme === "light" ? "#005BB5" : "#0A84FF"
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: "terminal"
                                                size: 13
                                                color: "#FFFFFF"
                                            }
                                            Text {
                                                text: "Open in Fastfetch Studio"
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: "#FFFFFF"
                                            }
                                        }

                                        MouseArea {
                                            id: openFfMouse
                                            objectName: "openFastfetchMouseArea"
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                if (root.packageData) {
                                                    root.actionRequested("open_fastfetch", root.packageData, null)
                                                }
                                            }
                                        }
                                    }

                                    // ── STATE 4: INSTALLED OTHER (WALLPAPERS, RICES, THEMES) -> OPEN IN LIBRARY ──
                                    Rectangle {
                                        id: openLibBtn
                                        objectName: "openLibraryButton"
                                        visible: Boolean(root.packageData && root.isInstalled() && !root.isLockscreen() && !root.isFastfetch())
                                        Layout.fillWidth: true
                                        height: 38
                                        radius: theme.radiusSm
                                        color: openLibMouse.pressed
                                            ? (theme.resolvedTheme === "light" ? "#D1D5DB" : "#404856")
                                            : (openLibMouse.containsMouse
                                                ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#363C47")
                                                : (theme.resolvedTheme === "light" ? "#F3F4F6" : "#242830"))
                                        border.color: openLibMouse.containsMouse
                                            ? (theme.resolvedTheme === "light" ? "#9CA3AF" : "#60697B")
                                            : (theme.resolvedTheme === "light" ? "#D1D5DB" : "#3E4450")
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: "book"
                                                size: 13
                                                color: theme.text
                                            }
                                            Text {
                                                text: "Open in Library"
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: theme.text
                                            }
                                        }

                                        MouseArea {
                                            id: openLibMouse
                                            objectName: "openLibraryMouseArea"
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                if (root.packageData) {
                                                    root.actionRequested("open_library", root.packageData, null)
                                                }
                                            }
                                        }
                                    }

                                    // ── COMMON ACTION: UNINSTALL PACKAGE ──
                                    Rectangle {
                                        id: uninstBtn
                                        objectName: "uninstallButton"
                                        visible: Boolean(root.packageData && root.isInstalled())
                                        Layout.fillWidth: true
                                        height: 32
                                        radius: theme.radiusSm
                                        color: "transparent"
                                        border.color: uninstBtnMouse.containsMouse && !root.isAnyActive()
                                            ? (theme.resolvedTheme === "light" ? "#FCA5A5" : "#7F1D1D")
                                            : "transparent"
                                        border.width: 1
                                        opacity: root.isAnyActive() ? 0.45 : 1.0

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 6
                                            RyzoraIcon {
                                                name: root.pendingAction === "uninstall" ? "refresh" : "trash"
                                                size: 12
                                                color: uninstBtnMouse.containsMouse && !root.isAnyActive()
                                                ? (theme.resolvedTheme === "light" ? "#DC2626" : "#F87171")
                                                : theme.textMuted
                                            }
                                            Text {
                                                text: root.pendingAction === "uninstall" ? "Uninstalling…" : "Uninstall"
                                                font.pixelSize: 12
                                                font.bold: true
                                                color: uninstBtnMouse.containsMouse && !root.isAnyActive()
                                                    ? (theme.resolvedTheme === "light" ? "#DC2626" : "#F87171")
                                                    : theme.textSecondary
                                            }
                                        }

                                        MouseArea {
                                            id: uninstBtnMouse
                                            objectName: "uninstallMouseArea"
                                            anchors.fill: parent
                                            hoverEnabled: root.pendingAction === "" && !root.isAnyActive()
                                            enabled: root.pendingAction === "" && !root.isAnyActive()
                                            cursorShape: root.isAnyActive() ? Qt.ForbiddenCursor : (root.pendingAction === "" ? Qt.PointingHandCursor : Qt.WaitCursor)
                                            onClicked: {
                                                if (root.packageData && root.pendingAction === "" && !root.isAnyActive()) {
                                                    console.log("[QML PackageDetailView] Uninstall clicked! package:", root.packageData.id);
                                                    root.errorMessage = ""; root.pendingAction = "uninstall"
                                                    root.actionRequested("uninstall", root.packageData, null)
                                                }
                                            }
                                        }
                                    }

                                    // Notice when active theme cannot be uninstalled
                                    Text {
                                        visible: Boolean(root.packageData && root.isInstalled() && root.isAnyActive())
                                        Layout.fillWidth: true
                                        text: "Active package cannot be uninstalled. Deactivate it first to restore system defaults safely."
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                        horizontalAlignment: Text.AlignHCenter
                                        wrapMode: Text.WordWrap
                                    }

                                    // ── ERROR BANNER ──
                                    Rectangle {
                                        id: actErrorBanner
                                        objectName: "actionErrorBanner"
                                        visible: root.errorMessage !== ""
                                        Layout.fillWidth: true
                                        radius: theme.radiusSm
                                        color: theme.resolvedTheme === "light" ? "#FEF2F2" : "#3B181A"
                                        border.color: theme.resolvedTheme === "light" ? "#FCA5A5" : "#7F1D1D"
                                        border.width: 1
                                        implicitHeight: actErrRow.implicitHeight + 16

                                        RowLayout {
                                            id: actErrRow
                                            anchors.fill: parent
                                            anchors.margins: 10
                                            spacing: 8
                                            RyzoraIcon {
                                                name: "alert"
                                                size: 14
                                                color: theme.resolvedTheme === "light" ? "#DC2626" : "#F87171"
                                            }
                                            Text {
                                                Layout.fillWidth: true
                                                text: root.errorMessage
                                                font.pixelSize: 12
                                                wrapMode: Text.WordWrap
                                                color: theme.resolvedTheme === "light" ? "#991B1B" : "#FCA5A5"
                                            }
                                            MouseArea {
                                                implicitWidth: 16
                                                implicitHeight: 16
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: root.errorMessage = ""
                                                RyzoraIcon {
                                                    anchors.centerIn: parent
                                                    name: "x"
                                                    size: 10
                                                    color: theme.resolvedTheme === "light" ? "#991B1B" : "#FCA5A5"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // ── Description ("About this Customization") ──
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 6

                                Text {
                                    text: "About this Customization"
                                    font.pixelSize: 14
                                    font.bold: true
                                    color: theme.text
                                }

                                Text {
                                    text: (root.packageData && root.packageData.description && root.packageData.description.trim().length > 0)
                                        ? String(root.packageData.description)
                                        : "No description provided for this package."
                                    font.pixelSize: 13
                                    lineHeight: 1.35
                                    color: theme.textSecondary
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }

                            // ── Technical Specifications Card (Compact & Secondary) ──
                            Rectangle {
                                Layout.fillWidth: true
                                implicitHeight: specCol.implicitHeight + 24
                                radius: theme.radiusMd
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1

                                ColumnLayout {
                                    id: specCol
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.margins: 12
                                    spacing: 8

                                    SpecRow {
                                        label: "Category"
                                        value: {
                                            if (!root.packageData || !root.packageData.category) return "—"
                                            var cat = String(root.packageData.category)
                                            if (cat === "lockscreens") return "Lock Screens"
                                            if (cat === "fastfetch") return "Fastfetch"
                                            if (cat === "wallpapers") return "Wallpapers"
                                            if (cat === "rices") return "Rices"
                                            if (cat === "themes") return "Themes"
                                            return cat.charAt(0).toUpperCase() + cat.slice(1)
                                        }
                                    }
                                    SpecRow {
                                        label: "Author"
                                        value: (root.packageData && root.packageData.author)
                                            ? (typeof root.packageData.author === "object" ? (root.packageData.author.name || "Community") : String(root.packageData.author))
                                            : "Community"
                                    }
                                    SpecRow {
                                        label: "Package ID"
                                        value: (root.packageData && root.packageData.id) ? String(root.packageData.id) : "—"
                                    }
                                    SpecRow {
                                        label: "Version"
                                        value: (root.packageData && root.packageData.version) ? ("v" + root.packageData.version) : "v1.0.0"
                                    }
                                    SpecRow {
                                        label: "Compatibility"
                                        value: {
                                            if (!root.packageData) return "—"
                                            if (root.isLockscreen()) {
                                                return root.supportsSddm() ? "SDDM Login Greeter (Quickshell)" : "Quickshell Desktop Preview"
                                            }
                                            if (root.isFastfetch()) return "Fastfetch v1.0+ · Terminal"
                                            var cat = String(root.packageData.category || "").toLowerCase()
                                            if (cat === "wallpapers") return "Desktop Wallpaper (Image)"
                                            if (cat === "rices") return "Hyprland Wayland Compositor"
                                            if (cat === "themes") return "GTK / Qt Desktop Theme"
                                            return "Ryzora Package Spec v1"
                                        }
                                    }
                                    SpecRow {
                                        label: "Dependencies"
                                        value: (root.packageData && root.packageData.dependencies && root.packageData.dependencies.length > 0)
                                            ? root.packageData.dependencies.join(", ")
                                            : "None (Standalone)"
                                    }
                                    SpecRow {
                                        label: "Status"
                                        value: {
                                            if (!root.packageData) return "—"
                                            if (root.isAnyActive()) return "Active on System"
                                            if (!root.isInstalled()) return "Available in Catalog"
                                            if (root.isTested()) return "Installed · Preview Verified"
                                            return "Installed"
                                        }
                                    }
                                }
                            }

                            // ── Tags Section ──
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 6
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
                                            implicitWidth: tagTxt.width + 16
                                            radius: 6
                                            color: theme.surfaceActive
                                            border.color: theme.borderSubtle
                                            border.width: 1

                                            Text {
                                                id: tagTxt
                                                anchors.centerIn: parent
                                                text: "#" + String(modelData)
                                                font.pixelSize: 11
                                                color: theme.textSecondary
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── 3. Subtle Section Divider ──
                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: theme.borderSubtle
                    }

                    // ── 4. Related Content Section ──
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 14
                        visible: root.getRelatedPackages().length > 0

                        ColumnLayout {
                            spacing: 2
                            Text {
                                text: {
                                    if (!root.packageData || !root.packageData.category) return "Related Customizations"
                                    var cat = String(root.packageData.category)
                                    if (cat === "lockscreens") return "More Lock Screens"
                                    if (cat === "fastfetch") return "More Fastfetch Presets"
                                    if (cat === "wallpapers") return "More Wallpapers"
                                    if (cat === "rices") return "More Rice Setups"
                                    if (cat === "themes") return "More Desktop Themes"
                                    return "More in " + (cat.charAt(0).toUpperCase() + cat.slice(1))
                                }
                                font.pixelSize: 18
                                font.bold: true
                                color: theme.text
                            }
                            Text {
                                text: "Explore complementary themes, presets, and community customizations."
                                font.pixelSize: 12
                                color: theme.textSecondary
                            }
                        }

                        // Responsive Row of Related Cards
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16

                            Repeater {
                                model: root.getRelatedPackages()
                                delegate: Item {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 250
                                    Layout.minimumWidth: 180

                                    StoreCard {
                                        anchors.fill: parent
                                        itemData: modelData
                                        onCardClicked: function(data) {
                                            root.packageSelected(data)
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Item { height: 28 }
                }
            }
        }
    }

    component SpecRow : RowLayout {
        property string label: ""
        property string value: ""
        Layout.fillWidth: true
        spacing: 8

        Text {
            text: label
            font.pixelSize: theme.fontSm
            color: theme.textMuted
            Layout.minimumWidth: 95
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
