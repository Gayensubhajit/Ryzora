import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    property string currentSubpage: "lockscreen" // Default to Lockscreen Studio
    property var activeDetailTheme: null
    property var themeConfig: ({})
    property var draftConfig: ({})
    property string pendingAction: ""
    property string actionError: ""
    property string actionSuccess: ""

    onActiveDetailThemeChanged: {
        if (activeDetailTheme) {
            root.themeConfig = backend.getThemeConfig(activeDetailTheme.id)
            var draft = {}
            if (root.themeConfig.settings) {
                for (var i = 0; i < root.themeConfig.settings.length; ++i) {
                    var s = root.themeConfig.settings[i]
                    draft[s.id] = s.current_value
                }
            }
            root.draftConfig = draft
            root.actionError = ""
            root.actionSuccess = ""
        } else {
            root.themeConfig = ({})
            root.draftConfig = ({})
            root.actionError = ""
            root.actionSuccess = ""
        }
    }

    // Fastfetch visual state
    property string selectedFastfetchPreset: backend.selectedPreset || "garuda"
    property string selectedFastfetchLogo: "garuda"
    property string fastfetchColorAccent: "cyan"
    property string fastfetchSeparator: ":"

    // Fastfetch modules
    property var activeModules: ({
        "os": true, "host": true, "kernel": true, "uptime": true,
        "packages": true, "shell": true, "display": true, "de": true,
        "wm": true, "theme": true, "icons": true, "terminal": true,
        "cpu": true, "gpu": true, "memory": true, "disk": false
    })

    // Wallpapers options
    property string wallpaperFit: "fill" // "fill" | "fit" | "stretch" | "center"
    property var selectedWallpaper: null

    signal navigateRequested(string tabName)

    function selectSubpage(page, pkg) {
        root.currentSubpage = page
        root.actionError = ""
        root.pendingAction = ""
        if (page === "lockscreen") {
            root.activeDetailTheme = pkg || null
        } else if (page === "wallpapers") {
            root.selectedWallpaper = pkg || null
        }
    }

    function openThemeDetail(pkg) {
        root.currentSubpage = "lockscreen"
        root.activeDetailTheme = pkg
        root.actionError = ""
    }

    // Safety timeout so buttons never remain stuck
    Timer {
        id: pendingTimer
        interval: 8000
        repeat: false
        onTriggered: {
            if (root.pendingAction !== "") {
                if (root.pendingAction === "preview") {
                    root.actionError = "Preview launch timed out."
                }
                root.pendingAction = ""
            }
        }
    }

    onPendingActionChanged: {
        if (root.pendingAction !== "") {
            pendingTimer.restart()
        } else {
            pendingTimer.stop()
        }
    }

    Connections {
        target: backend
        function onPackageTested(packageId, success, message) {
            root.pendingAction = ""
            pendingTimer.stop()
            if (!success) {
                root.actionError = message || "Failed to launch preview."
            } else {
                root.actionError = ""
            }
        }
        function onTargetActivated(pkg, target, success, message) {
            root.pendingAction = ""
            pendingTimer.stop()
            if (!success) {
                root.actionError = message || "Failed to activate SDDM greeter."
            } else {
                root.actionError = ""
                if (root.activeDetailTheme) {
                    root.themeConfig = backend.getThemeConfig(root.activeDetailTheme.id)
                }
            }
        }
        function onTargetDeactivated(target, success, message) {
            root.pendingAction = ""
            pendingTimer.stop()
            if (!success) {
                root.actionError = message || "Failed to deactivate SDDM greeter."
            } else {
                root.actionError = ""
                if (root.activeDetailTheme) {
                    root.themeConfig = backend.getThemeConfig(root.activeDetailTheme.id)
                }
            }
        }
        function onPackageUninstalled(pkg, success, message) {
            root.pendingAction = ""
            pendingTimer.stop()
            if (success && root.activeDetailTheme && root.activeDetailTheme.id === pkg) {
                root.activeDetailTheme = null
            } else if (!success) {
                root.actionError = message
            }
        }
        function onFastfetchChanged() {
            if (!backend.isFastfetchLoading) {
                root.pendingAction = ""
            }
        }
        function onLibraryChanged() {
            if (root.activeDetailTheme) {
                var lib = backend.libraryList
                for (var i = 0; i < lib.length; ++i) {
                    if (lib[i].id === root.activeDetailTheme.id) {
                        root.activeDetailTheme = Object.assign({}, lib[i])
                        break
                    }
                }
            }
        }
    }

    // Filter installed SDDM-compatible lockscreen themes
    function getInstalledSddmThemes() {
        var res = []
        var list = backend.libraryList
        for (var i = 0; i < list.length; ++i) {
            var item = list[i]
            var cat = String(item.category || item.package_type || "").toLowerCase()
            if ((cat === "lockscreens" || cat === "lockscreen" || cat === "sddm") && item.supports_sddm) {
                res.push(item)
            }
        }
        return res
    }

    function isThemeActive(item) {
        if (!item) return false
        if (item.active_targets && item.active_targets.sddm_login) return true
        if (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active) {
            return backend.activeConfiguration.active_package_id === item.id
        }
        return false
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // ══════════════════════════════════════════════════════════════════════
        // 1. RYOKU-STYLE PERSISTENT CATEGORY SIDEBAR
        // ══════════════════════════════════════════════════════════════════════
        Rectangle {
            Layout.preferredWidth: 200
            Layout.minimumWidth: 175
            Layout.maximumWidth: 220
            Layout.fillHeight: true
            color: theme.sidebarBg
            border.color: theme.borderSubtle
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 4

                // Section Label
                Text {
                    text: "CUSTOMIZATION"
                    font.pixelSize: 10
                    font.bold: true
                    font.letterSpacing: 1.2
                    color: theme.textMuted
                    Layout.leftMargin: 8
                    Layout.topMargin: 4
                    Layout.bottomMargin: 6
                }

                // Subpage Nav Items (Real Icons, No Emoji)
                StudioSubpageButton {
                    iconName: "sliders"
                    title: "Overview"
                    desc: "Studio dashboard"
                    isActive: root.currentSubpage === "overview"
                    onClicked: root.selectSubpage("overview", null)
                }

                StudioSubpageButton {
                    iconName: "lock"
                    title: "Lockscreen"
                    desc: "SDDM login themes"
                    isActive: root.currentSubpage === "lockscreen"
                    onClicked: root.selectSubpage("lockscreen", null)
                }

                StudioSubpageButton {
                    iconName: "terminal"
                    title: "Fastfetch"
                    desc: "Terminal modules"
                    isActive: root.currentSubpage === "fastfetch"
                    onClicked: {
                        root.selectSubpage("fastfetch", null)
                        if (!backend.fastfetchHtml || backend.fastfetchHtml.length === 0) {
                            backend.requestFastfetchPreview(root.selectedFastfetchPreset)
                        }
                    }
                }

                StudioSubpageButton {
                    iconName: "image"
                    title: "Wallpapers"
                    desc: "Backgrounds & fit"
                    isActive: root.currentSubpage === "wallpapers"
                    onClicked: root.selectSubpage("wallpapers", null)
                }

                StudioSubpageButton {
                    iconName: "palette"
                    title: "Appearance"
                    desc: "Colors & theme"
                    isActive: root.currentSubpage === "appearance"
                    onClicked: root.selectSubpage("appearance", null)
                }

                Item { Layout.fillHeight: true }

                // Quick Status Card in Left Nav
                Rectangle {
                    Layout.fillWidth: true
                    height: 64
                    radius: theme.radiusMd
                    color: theme.surface
                    border.color: theme.borderSubtle
                    border.width: 1

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 3

                        RowLayout {
                            spacing: 6
                            Rectangle {
                                width: 6; height: 6; radius: 3
                                color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active) ? "#10b981" : "#64748b"
                            }
                            Text {
                                text: "SDDM Greeter"
                                font.pixelSize: 10
                                font.bold: true
                                color: theme.text
                            }
                        }

                        Text {
                            text: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                ? (backend.activeConfiguration.active_title || "Active Theme")
                                : "System Default"
                            font.pixelSize: 10
                            color: theme.textMuted
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }
                }
            }
        }

        // ══════════════════════════════════════════════════════════════════════
        // 2. MAIN WORKSPACE CONTENT AREA
        // ══════════════════════════════════════════════════════════════════════
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ScrollBar.vertical.policy: ScrollBar.AsNeeded

            ColumnLayout {
                width: parent.width
                anchors.margins: 28
                spacing: 22

                // Global Action Error Banner
                Rectangle {
                    visible: root.actionError !== ""
                    Layout.fillWidth: true
                    height: 38
                    radius: theme.radiusSm
                    color: Qt.rgba(239/255, 68/255, 68/255, 0.15)
                    border.color: "#ef4444"
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 8
                        RyzoraIcon { name: "shield"; size: 14; color: "#f87171" }
                        Text {
                            text: root.actionError
                            font.pixelSize: 12
                            font.bold: true
                            color: "#f87171"
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }
                        Rectangle {
                            width: 20; height: 20; radius: 10
                            color: "transparent"
                            RyzoraIcon { anchors.centerIn: parent; name: "x"; size: 10; color: "#f87171" }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.actionError = "" }
                        }
                    }
                }

                // ──────────────────────────────────────────────────────────────
                // SUBPAGE: LOCKSCREEN STUDIO (Ryoku Arch Layout)
                // ──────────────────────────────────────────────────────────────
                ColumnLayout {
                    visible: root.currentSubpage === "lockscreen"
                    Layout.fillWidth: true
                    spacing: 20

                    // ── Case A: Theme Gallery View (Main Landing) ──
                    ColumnLayout {
                        visible: root.activeDetailTheme === null
                        Layout.fillWidth: true
                        spacing: 18

                        // 1. Compact Header Row with Action Buttons
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16

                            ColumnLayout {
                                spacing: 4
                                Layout.fillWidth: true

                                RowLayout {
                                    spacing: 8
                                    Text {
                                        text: "—  PERSONALIZATION"
                                        font.pixelSize: 11
                                        font.bold: true
                                        font.letterSpacing: 1.5
                                        color: theme.textMuted
                                    }
                                }

                                Text {
                                    text: "Lockscreen"
                                    font.pixelSize: 26
                                    font.bold: true
                                    color: theme.text
                                }

                                Text {
                                    text: "Pick the skin your lock and sign-in screens wear. It only swaps the look, never your login; applying reskins the sign-in screen too, so you will be asked for your password."
                                    font.pixelSize: 12
                                    color: theme.textMuted
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                    Layout.maximumWidth: 800
                                }
                            }

                            // Top-Right Action Buttons (Ryoku style)
                            RowLayout {
                                spacing: 8
                                Layout.alignment: Qt.AlignRight | Qt.AlignTop

                                HeaderPillButton {
                                    label: "FILES"
                                    iconName: "sliders"
                                    onClicked: root.navigateRequested("library")
                                }

                                HeaderPillButton {
                                    label: "UPDATES"
                                    iconName: "refresh"
                                    onClicked: root.navigateRequested("updates")
                                }

                                HeaderPillButton {
                                    label: "BROWSE STORE"
                                    iconName: "store"
                                    onClicked: root.navigateRequested("discover")
                                }
                            }
                        }

                        // 2. Restrained Security / Status Strip (Ryoku style)
                        Rectangle {
                            Layout.fillWidth: true
                            height: 40
                            radius: theme.radiusSm
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 14
                                anchors.rightMargin: 14
                                spacing: 10

                                RowLayout {
                                    spacing: 6
                                    RyzoraIcon { name: "key"; size: 13; color: theme.textMuted }
                                    Text {
                                        text: "Sign-in, Fingerprint & Security key"
                                        font.pixelSize: 11
                                        font.bold: true
                                        color: theme.text
                                    }
                                    RyzoraIcon { name: "chevron-right"; size: 10; color: theme.textMuted }
                                }

                                Item { Layout.fillWidth: true }

                                RowLayout {
                                    spacing: 8
                                    Rectangle {
                                        width: 6; height: 6; radius: 3
                                        color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active) ? "#10b981" : "#64748b"
                                    }
                                    Text {
                                        text: "SDDM drop-in verified · polkit helper active · desktop lock (Dusky/Hyprlock) untouched"
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                    }
                                }
                            }
                        }

                        // 3. Responsive Gallery of Installed SDDM Theme Cards
                        Flow {
                            id: galleryFlow
                            Layout.fillWidth: true
                            spacing: 18
                            visible: root.getInstalledSddmThemes().length > 0

                            Repeater {
                                model: root.getInstalledSddmThemes()

                                Rectangle {
                                    id: cardRect
                                    width: 300
                                    height: 290
                                    radius: theme.radiusMd
                                    color: root.isThemeActive(modelData)
                                        ? (theme.resolvedTheme === "light" ? "#F0F4FF" : Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.08))
                                        : (cardMouse.containsMouse ? theme.surfaceElevated : theme.surface)
                                    border.color: root.isThemeActive(modelData)
                                        ? theme.accent
                                        : (cardMouse.containsMouse ? theme.borderStrong : theme.borderSubtle)
                                    border.width: 1

                                    Behavior on border.color { ColorAnimation { duration: 100 } }
                                    Behavior on color { ColorAnimation { duration: 100 } }

                                    ColumnLayout {
                                        anchors.fill: parent
                                        anchors.margins: 12
                                        spacing: 8

                                        // ── Landscape Preview Box (16:9) ──
                                        Rectangle {
                                            Layout.fillWidth: true
                                            Layout.preferredHeight: 155
                                            radius: theme.radiusSm
                                            color: theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            border.width: 1
                                            clip: true

                                            Image {
                                                id: thumbImg
                                                anchors.fill: parent
                                                source: {
                                                    var u = String(modelData.preview_url || "").trim()
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
                                                fillMode: Image.PreserveAspectCrop
                                                asynchronous: true
                                                smooth: true
                                                visible: status === Image.Ready
                                            }

                                            // Fallback Artwork (Clean and Theme-Aware)
                                            Rectangle {
                                                anchors.fill: parent
                                                visible: thumbImg.status !== Image.Ready
                                                color: theme.surfaceElevated

                                                ColumnLayout {
                                                    anchors.centerIn: parent
                                                    spacing: 4
                                                    RyzoraIcon { name: "lock"; size: 22; color: theme.textMuted; Layout.alignment: Qt.AlignHCenter }
                                                    Text {
                                                        text: modelData.name
                                                        font.pixelSize: 11
                                                        font.bold: true
                                                        color: theme.textMuted
                                                        Layout.alignment: Qt.AlignHCenter
                                                    }
                                                }
                                            }

                                            // Subtle Quick Preview Scrim on Hover (Ryoku style)
                                            Rectangle {
                                                anchors { left: parent.left; bottom: parent.bottom; margins: 8 }
                                                height: 22
                                                width: pvRow.implicitWidth + 12
                                                radius: 4
                                                color: Qt.rgba(0, 0, 0, 0.70)
                                                border.color: Qt.rgba(255, 255, 255, 0.20)
                                                border.width: 1
                                                opacity: cardMouse.containsMouse ? 1 : 0
                                                Behavior on opacity { NumberAnimation { duration: 120 } }

                                                RowLayout {
                                                    id: pvRow
                                                    anchors.centerIn: parent
                                                    spacing: 4
                                                    RyzoraIcon { name: "play"; size: 9; color: "#ffffff" }
                                                    Text {
                                                        text: "Preview"
                                                        font.pixelSize: 10
                                                        font.bold: true
                                                        color: "#ffffff"
                                                    }
                                                }
                                            }
                                        }

                                        // ── Metadata & Status Section ──
                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 3

                                            // Top Row: Ordinal + Active Indicator
                                            RowLayout {
                                                Layout.fillWidth: true

                                                Text {
                                                    text: (index < 9 ? "0" : "") + (index + 1)
                                                    font.pixelSize: 12
                                                    font.bold: true
                                                    font.family: "monospace"
                                                    color: theme.textMuted
                                                }

                                                Text {
                                                    text: "·  " + (modelData.author || "COMMUNITY").toUpperCase()
                                                    font.pixelSize: 9
                                                    font.bold: true
                                                    font.letterSpacing: 0.8
                                                    color: theme.textMuted
                                                    elide: Text.ElideRight
                                                    Layout.fillWidth: true
                                                }

                                                // Restrained Active Indicator (small dot + label)
                                                Rectangle {
                                                    visible: root.isThemeActive(modelData)
                                                    height: 18
                                                    width: pillText.implicitWidth + 14
                                                    radius: 4
                                                    color: theme.resolvedTheme === "light" ? "#DCFCE7" : Qt.rgba(16/255, 185/255, 129/255, 0.15)
                                                    border.color: theme.resolvedTheme === "light" ? "#86EFAC" : "#10b981"
                                                    border.width: 1

                                                    RowLayout {
                                                        anchors.centerIn: parent
                                                        spacing: 4
                                                        Rectangle {
                                                            width: 5; height: 5; radius: 2.5
                                                            color: "#10b981"
                                                        }
                                                        Text {
                                                            id: pillText
                                                            text: "ACTIVE"
                                                            font.pixelSize: 8
                                                            font.bold: true
                                                            font.letterSpacing: 0.6
                                                            color: theme.resolvedTheme === "light" ? "#15803D" : "#34d399"
                                                        }
                                                    }
                                                }
                                            }

                                            // Theme Title
                                            Text {
                                                text: modelData.name
                                                font.pixelSize: 14
                                                font.bold: true
                                                color: cardMouse.containsMouse ? theme.accent : theme.text
                                                elide: Text.ElideRight
                                                Layout.fillWidth: true
                                            }

                                            // Description / Summary Snippet
                                            Text {
                                                text: modelData.description || "Interactive SDDM login greeter theme with custom typography."
                                                font.pixelSize: 11
                                                color: theme.textSecondary
                                                maximumLineCount: 2
                                                wrapMode: Text.WordWrap
                                                elide: Text.ElideRight
                                                Layout.fillWidth: true
                                            }
                                        }

                                        Item { Layout.fillHeight: true }
                                    }

                                    MouseArea {
                                        id: cardMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: root.openThemeDetail(modelData)
                                    }
                                }
                            }
                        }

                        // 4. Empty State (When no SDDM themes installed)
                        Rectangle {
                            visible: root.getInstalledSddmThemes().length === 0
                            Layout.fillWidth: true
                            height: 220
                            radius: theme.radiusMd
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1

                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 12

                                RyzoraIcon { name: "lock"; size: 36; color: theme.textMuted; Layout.alignment: Qt.AlignHCenter }

                                Text {
                                    text: "No SDDM login greeter themes installed"
                                    font.pixelSize: 14
                                    font.bold: true
                                    color: theme.text
                                    Layout.alignment: Qt.AlignHCenter
                                }

                                Text {
                                    text: "Explore SDDM-compatible login themes in the Ryzora store to install and preview them."
                                    font.pixelSize: 12
                                    color: theme.textMuted
                                    Layout.alignment: Qt.AlignHCenter
                                }

                                ActionButton {
                                    label: "Browse Store"
                                    iconName: "store"
                                    Layout.alignment: Qt.AlignHCenter
                                    onClicked: root.navigateRequested("discover")
                                }
                            }
                        }
                    }

                    // ── Case B: Theme Detail View (When a theme is clicked) ──
                    ColumnLayout {
                        visible: root.activeDetailTheme !== null
                        Layout.fillWidth: true
                        spacing: 20

                        // Breadcrumb Navigation Back to Gallery & Status Badge
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Rectangle {
                                width: 32; height: 32; radius: 16
                                color: backMouse.containsMouse ? theme.surfaceHover : theme.surface
                                border.color: backMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
                                border.width: 1

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    name: "chevron-left"
                                    size: 14
                                    color: backMouse.containsMouse ? theme.accent : theme.text
                                }

                                MouseArea {
                                    id: backMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        root.activeDetailTheme = null
                                        root.actionError = ""
                                    }
                                }
                            }

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: "Lockscreen Studio"
                                    font.pixelSize: 12
                                    color: theme.textMuted
                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: root.activeDetailTheme = null
                                    }
                                }
                                Text { text: "/"; font.pixelSize: 12; color: theme.borderStrong }
                                Text {
                                    text: "Themes"
                                    font.pixelSize: 12
                                    color: theme.textMuted
                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: root.activeDetailTheme = null
                                    }
                                }
                                Text { text: "/"; font.pixelSize: 12; color: theme.borderStrong }
                                Text {
                                    text: root.activeDetailTheme ? root.activeDetailTheme.name : ""
                                    font.pixelSize: 12
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            Item { Layout.fillWidth: true }

                            // SDDM Login Status Pill
                            Rectangle {
                                property bool isActive: root.isThemeActive(root.activeDetailTheme)
                                height: 26
                                width: topStatusTxt.implicitWidth + 24
                                radius: 13
                                color: isActive ? Qt.rgba(16/255, 185/255, 129/255, 0.15) : theme.surfaceElevated
                                border.color: isActive ? "#10b981" : theme.borderSubtle
                                border.width: 1

                                RowLayout {
                                    anchors.centerIn: parent
                                    spacing: 6
                                    Rectangle {
                                        width: 6; height: 6; radius: 3
                                        color: parent.parent.isActive ? "#10b981" : theme.textMuted
                                    }
                                    Text {
                                        id: topStatusTxt
                                        text: parent.parent.isActive ? "ACTIVE AT LOGIN (SDDM)" : "INSTALLED · READY TO APPLY"
                                        font.pixelSize: 10
                                        font.bold: true
                                        font.letterSpacing: 0.6
                                        color: parent.parent.isActive ? "#34d399" : theme.textMuted
                                    }
                                }
                            }
                        }

                        // Action Error Alert Banner (if any)
                        Rectangle {
                            visible: root.actionError !== ""
                            Layout.fillWidth: true
                            height: 40
                            radius: theme.radiusSm
                            color: Qt.rgba(239/255, 68/255, 68/255, 0.15)
                            border.color: "#ef4444"
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 10
                                spacing: 8
                                RyzoraIcon { name: "x"; size: 14; color: "#f87171" }
                                Text {
                                    text: root.actionError
                                    font.pixelSize: 12
                                    color: "#fca5a5"
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                                MouseArea {
                                    width: 18; height: 18
                                    cursorShape: Qt.PointingHandCursor
                                    RyzoraIcon { anchors.centerIn: parent; name: "x"; size: 10; color: "#fca5a5" }
                                    onClicked: root.actionError = ""
                                }
                            }
                        }

                        // Hero Detail Card (Balanced Desktop Layout)
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: heroRow.implicitHeight + 40
                            radius: theme.radiusLg
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                id: heroRow
                                anchors.fill: parent
                                anchors.margins: 20
                                spacing: 24

                                // Large Theme Preview (16:9)
                                Rectangle {
                                    Layout.preferredWidth: 380
                                    Layout.preferredHeight: 214
                                    radius: theme.radiusMd
                                    color: theme.surfaceElevated
                                    border.color: theme.borderSubtle
                                    border.width: 1
                                    clip: true

                                    Image {
                                        id: detailHeroImg
                                        anchors.fill: parent
                                        source: {
                                            if (!root.activeDetailTheme) return ""
                                            var u = String(root.activeDetailTheme.preview_url || "").trim()
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
                                        fillMode: Image.PreserveAspectCrop
                                        visible: status === Image.Ready
                                    }

                                    Rectangle {
                                        anchors.fill: parent
                                        visible: detailHeroImg.status !== Image.Ready
                                        color: theme.surfaceElevated
                                        ColumnLayout {
                                            anchors.centerIn: parent
                                            spacing: 6
                                            RyzoraIcon { name: "lock"; size: 32; color: theme.textMuted; Layout.alignment: Qt.AlignHCenter }
                                            Text {
                                                text: root.activeDetailTheme ? root.activeDetailTheme.name : ""
                                                font.pixelSize: 12
                                                font.bold: true
                                                color: theme.textMuted
                                                Layout.alignment: Qt.AlignHCenter
                                            }
                                        }
                                    }
                                }

                                // Detail Information & Actions Area
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 10

                                    // Author & Version Tag
                                    Text {
                                        text: ((root.activeDetailTheme && root.activeDetailTheme.author) ? String(root.activeDetailTheme.author).toUpperCase() : "COMMUNITY") + "  ·  v" + (root.activeDetailTheme ? (root.activeDetailTheme.version || "1.0.0") : "1.0.0") + "  ·  SDDM Theme"
                                        font.pixelSize: 10
                                        font.bold: true
                                        font.letterSpacing: 0.8
                                        color: theme.textMuted
                                    }

                                    Text {
                                        text: root.activeDetailTheme ? root.activeDetailTheme.name : ""
                                        font.pixelSize: 20
                                        font.bold: true
                                        color: theme.text
                                    }

                                    Text {
                                        text: root.activeDetailTheme ? (root.activeDetailTheme.description || "") : ""
                                        font.pixelSize: 12
                                        color: theme.textSecondary
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }

                                    // Compact System Status Row
                                    RowLayout {
                                        spacing: 16
                                        Layout.topMargin: 4
                                        Layout.bottomMargin: 4

                                        RowLayout {
                                            spacing: 6
                                            Rectangle {
                                                width: 6; height: 6; radius: 3
                                                color: root.isThemeActive(root.activeDetailTheme) ? "#10b981" : theme.textMuted
                                            }
                                            Text {
                                                text: root.isThemeActive(root.activeDetailTheme) ? "Active at SDDM Login" : "Installed · Ready to apply"
                                                font.pixelSize: 11
                                                font.bold: true
                                                color: root.isThemeActive(root.activeDetailTheme) ? (theme.resolvedTheme === "light" ? "#15803D" : "#34d399") : theme.textMuted
                                            }
                                        }

                                        Text { text: "·"; font.pixelSize: 11; color: theme.borderStrong }

                                        Text {
                                            text: "Desktop lock: Dusky / Hyprlock (Untouched)"
                                            font.pixelSize: 11
                                            color: theme.textMuted
                                        }
                                    }

                                    // Compact Action Buttons
                                    RowLayout {
                                        spacing: 10

                                        // Apply Theme Button (When not active)
                                        Rectangle {
                                            visible: !root.isThemeActive(root.activeDetailTheme)
                                            height: 34
                                            width: applyBtnTxt.implicitWidth + 28
                                            radius: theme.radiusSm
                                            color: applyBtnM.containsMouse ? "#059669" : "#10b981"

                                            RowLayout {
                                                anchors.centerIn: parent
                                                spacing: 6
                                                RyzoraIcon { name: "check"; size: 12; color: "#ffffff" }
                                                Text {
                                                    id: applyBtnTxt
                                                    text: root.pendingAction === "apply" ? "Applying…" : "Apply Theme"
                                                    font.pixelSize: 12
                                                    font.bold: true
                                                    color: "#ffffff"
                                                }
                                            }

                                            MouseArea {
                                                id: applyBtnM
                                                anchors.fill: parent
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: {
                                                    if (root.activeDetailTheme && root.pendingAction === "") {
                                                        root.actionError = ""
                                                        root.pendingAction = "apply"
                                                        backend.applyTarget(root.activeDetailTheme.id, "sddm_login")
                                                    }
                                                }
                                            }
                                        }

                                        // Deactivate Theme Button (When active)
                                        Rectangle {
                                            visible: root.isThemeActive(root.activeDetailTheme)
                                            height: 34
                                            width: deactBtnTxt.implicitWidth + 28
                                            radius: theme.radiusSm
                                            color: deactBtnM.containsMouse ? "#3b1219" : theme.surfaceElevated
                                            border.color: deactBtnM.containsMouse ? "#ef4444" : theme.borderStrong
                                            border.width: 1

                                            RowLayout {
                                                anchors.centerIn: parent
                                                spacing: 6
                                                RyzoraIcon { name: "x"; size: 12; color: deactBtnM.containsMouse ? "#f87171" : theme.textMuted }
                                                Text {
                                                    id: deactBtnTxt
                                                    text: root.pendingAction === "deactivate" ? "Deactivating…" : "Deactivate Theme"
                                                    font.pixelSize: 12
                                                    font.bold: true
                                                    color: deactBtnM.containsMouse ? "#f87171" : theme.textMuted
                                                }
                                            }

                                            MouseArea {
                                                id: deactBtnM
                                                anchors.fill: parent
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: {
                                                    if (root.pendingAction === "") {
                                                        root.actionError = ""
                                                        root.pendingAction = "deactivate"
                                                        backend.deactivateTarget("sddm_login")
                                                    }
                                                }
                                            }
                                        }

                                        // Preview Theme Button
                                        Rectangle {
                                            height: 34
                                            width: prevBtnTxt.implicitWidth + 26
                                            radius: theme.radiusSm
                                            color: prevBtnM.containsMouse ? theme.surfaceActive : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            border.width: 1

                                            RowLayout {
                                                anchors.centerIn: parent
                                                spacing: 6
                                                RyzoraIcon { name: "play"; size: 11; color: theme.text }
                                                Text {
                                                    id: prevBtnTxt
                                                    text: root.pendingAction === "preview" ? "Opening Preview…" : "Preview Theme"
                                                    font.pixelSize: 12
                                                    font.bold: true
                                                    color: theme.text
                                                }
                                            }

                                            MouseArea {
                                                id: prevBtnM
                                                anchors.fill: parent
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: {
                                                    if (root.activeDetailTheme && root.pendingAction === "") {
                                                        root.actionError = ""
                                                        root.pendingAction = "preview"
                                                        backend.testPackage(root.activeDetailTheme.id)
                                                    }
                                                }
                                            }
                                        }

                                        // Uninstall Package Button
                                        Rectangle {
                                            property bool isAct: root.isThemeActive(root.activeDetailTheme)
                                            height: 34
                                            width: uninstBtnTxt.implicitWidth + 24
                                            radius: theme.radiusSm
                                            opacity: isAct ? 0.45 : 1.0
                                            color: (!isAct && uninstBtnM.containsMouse) ? "#2b1114" : theme.surfaceElevated
                                            border.color: (!isAct && uninstBtnM.containsMouse) ? "#ef4444" : theme.borderSubtle
                                            border.width: 1

                                            RowLayout {
                                                anchors.centerIn: parent
                                                spacing: 6
                                                RyzoraIcon {
                                                    name: "trash"
                                                    size: 12
                                                    color: (!parent.parent.isAct && uninstBtnM.containsMouse) ? "#ef4444" : theme.textMuted
                                                }
                                                Text {
                                                    id: uninstBtnTxt
                                                    text: root.pendingAction === "uninstall" ? "Removing…" : "Uninstall"
                                                    font.pixelSize: 12
                                                    font.bold: true
                                                    color: (!parent.parent.isAct && uninstBtnM.containsMouse) ? "#ef4444" : theme.textMuted
                                                }
                                            }

                                            MouseArea {
                                                id: uninstBtnM
                                                anchors.fill: parent
                                                cursorShape: parent.isAct ? Qt.ForbiddenCursor : Qt.PointingHandCursor
                                                onClicked: {
                                                    if (parent.isAct) {
                                                        root.actionError = "Cannot uninstall active theme. Deactivate it first to restore system defaults safely."
                                                        return
                                                    }
                                                    if (root.activeDetailTheme && root.pendingAction === "") {
                                                        root.actionError = ""
                                                        root.pendingAction = "uninstall"
                                                        backend.uninstallPackage(root.activeDetailTheme.id)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Theme Configuration Section: Dynamically Built from Validated Capability Model
                        Rectangle {
                            visible: root.themeConfig && root.themeConfig.has_settings === true
                            Layout.fillWidth: true
                            implicitHeight: themeCustomCol.implicitHeight + 40
                            radius: theme.radiusLg
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1

                            ColumnLayout {
                                id: themeCustomCol
                                anchors.fill: parent
                                anchors.margins: 20
                                spacing: 16

                                RowLayout {
                                    spacing: 8
                                    RyzoraIcon { name: "sliders"; size: 14; color: theme.accent }
                                    Text {
                                        text: "THEME CUSTOMIZATION"
                                        font.pixelSize: 11
                                        font.bold: true
                                        font.letterSpacing: 1.0
                                        color: theme.accent
                                    }
                                }

                                Text {
                                    text: "Configure parameters declared and supported by this theme's QML engine. Changes apply immediately to local preview and SDDM login screen."
                                    font.pixelSize: 12
                                    color: theme.textSecondary
                                }

                                // Dynamic Settings Controls
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 14

                                    Repeater {
                                        model: (root.themeConfig && root.themeConfig.settings) ? root.themeConfig.settings : []
                                        delegate: RowLayout {
                                            Layout.fillWidth: true
                                            spacing: 16

                                            ColumnLayout {
                                                Layout.fillWidth: true
                                                spacing: 2
                                                RowLayout {
                                                    spacing: 6
                                                    Text {
                                                        text: modelData.label
                                                        font.pixelSize: 13
                                                        font.bold: true
                                                        color: theme.text
                                                    }
                                                    Rectangle {
                                                        visible: modelData.category !== undefined && modelData.category !== ""
                                                        height: 16
                                                        width: catTxt.implicitWidth + 8
                                                        radius: 3
                                                        color: theme.surfaceElevated
                                                        border.color: theme.borderSubtle
                                                        border.width: 1
                                                        Text {
                                                            id: catTxt
                                                            anchors.centerIn: parent
                                                            text: modelData.category || ""
                                                            font.pixelSize: 9
                                                            color: theme.textMuted
                                                        }
                                                    }
                                                }
                                                Text {
                                                    text: modelData.help || ""
                                                    font.pixelSize: 11
                                                    color: theme.textMuted
                                                    wrapMode: Text.WordWrap
                                                    Layout.fillWidth: true
                                                }
                                            }

                                            // 1. Enum options
                                            RowLayout {
                                                visible: modelData.type === "enum"
                                                spacing: 4
                                                Repeater {
                                                    model: modelData.options
                                                    delegate: Rectangle {
                                                        property bool isCur: String(root.draftConfig[modelData.id]) === String(modelData)
                                                        height: 30
                                                        width: Math.max(64, optTxt.implicitWidth + 20)
                                                        radius: theme.radiusSm
                                                        color: isCur ? theme.accent : (enumM.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                                                        border.color: isCur ? theme.accent : theme.borderSubtle
                                                        border.width: 1

                                                        Text {
                                                            id: optTxt
                                                            anchors.centerIn: parent
                                                            text: String(modelData).charAt(0).toUpperCase() + String(modelData).slice(1)
                                                            font.pixelSize: 11
                                                            font.bold: true
                                                            color: parent.isCur ? "#000000" : theme.text
                                                        }

                                                        MouseArea {
                                                            id: enumM
                                                            anchors.fill: parent
                                                            hoverEnabled: true
                                                            cursorShape: Qt.PointingHandCursor
                                                            onClicked: {
                                                                var d = Object.assign({}, root.draftConfig)
                                                                d[modelData.id] = String(modelData)
                                                                root.draftConfig = d
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            // 2. Boolean toggle
                                            RowLayout {
                                                visible: modelData.type === "bool"
                                                spacing: 4
                                                Rectangle {
                                                    property bool isCur: Boolean(root.draftConfig[modelData.id]) === true
                                                    height: 30
                                                    width: 72
                                                    radius: theme.radiusSm
                                                    color: isCur ? theme.accent : (bTrueM.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                                                    border.color: isCur ? theme.accent : theme.borderSubtle
                                                    border.width: 1
                                                    Text {
                                                        anchors.centerIn: parent
                                                        text: "Enabled"
                                                        font.pixelSize: 11
                                                        font.bold: true
                                                        color: parent.isCur ? "#000000" : theme.text
                                                    }
                                                    MouseArea {
                                                        id: bTrueM
                                                        anchors.fill: parent
                                                        hoverEnabled: true
                                                        cursorShape: Qt.PointingHandCursor
                                                        onClicked: {
                                                            var d = Object.assign({}, root.draftConfig)
                                                            d[modelData.id] = true
                                                            root.draftConfig = d
                                                        }
                                                    }
                                                }
                                                Rectangle {
                                                    property bool isCur: Boolean(root.draftConfig[modelData.id]) === false
                                                    height: 30
                                                    width: 72
                                                    radius: theme.radiusSm
                                                    color: isCur ? theme.accent : (bFalseM.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                                                    border.color: isCur ? theme.accent : theme.borderSubtle
                                                    border.width: 1
                                                    Text {
                                                        anchors.centerIn: parent
                                                        text: "Disabled"
                                                        font.pixelSize: 11
                                                        font.bold: true
                                                        color: parent.isCur ? "#000000" : theme.text
                                                    }
                                                    MouseArea {
                                                        id: bFalseM
                                                        anchors.fill: parent
                                                        hoverEnabled: true
                                                        cursorShape: Qt.PointingHandCursor
                                                        onClicked: {
                                                            var d = Object.assign({}, root.draftConfig)
                                                            d[modelData.id] = false
                                                            root.draftConfig = d
                                                        }
                                                    }
                                                }
                                            }

                                            // 3. Integer range
                                            RowLayout {
                                                visible: modelData.type === "int"
                                                spacing: 4
                                                Repeater {
                                                    model: (modelData.max !== undefined && modelData.min !== undefined) ? (modelData.max - modelData.min + 1) : 0
                                                    delegate: Rectangle {
                                                        property int val: modelData.min + index
                                                        property bool isCur: parseInt(root.draftConfig[modelData.id]) === val
                                                        height: 30
                                                        width: 34
                                                        radius: theme.radiusSm
                                                        color: isCur ? theme.accent : (intM.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                                                        border.color: isCur ? theme.accent : theme.borderSubtle
                                                        border.width: 1
                                                        Text {
                                                            anchors.centerIn: parent
                                                            text: String(parent.val)
                                                            font.pixelSize: 11
                                                            font.bold: true
                                                            color: parent.isCur ? "#000000" : theme.text
                                                        }
                                                        MouseArea {
                                                            id: intM
                                                            anchors.fill: parent
                                                            hoverEnabled: true
                                                            cursorShape: Qt.PointingHandCursor
                                                            onClicked: {
                                                                var d = Object.assign({}, root.draftConfig)
                                                                d[modelData.id] = parent.val
                                                                root.draftConfig = d
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Action feedback / Save Button Row
                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        visible: root.actionSuccess !== ""
                                        text: root.actionSuccess
                                        font.pixelSize: 11
                                        color: "#34d399"
                                        Layout.fillWidth: true
                                        elide: Text.ElideRight
                                    }
                                    Item { Layout.fillWidth: true; visible: root.actionSuccess === "" }

                                    Rectangle {
                                        height: 32
                                        width: saveCfgTxt.implicitWidth + 24
                                        radius: theme.radiusSm
                                        color: saveCfgM.containsMouse ? theme.surfaceActive : theme.surfaceElevated
                                        border.color: theme.accent
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 6
                                            RyzoraIcon { name: "check"; size: 11; color: theme.accent }
                                            Text {
                                                id: saveCfgTxt
                                                text: "Save Customization"
                                                font.pixelSize: 11
                                                font.bold: true
                                                color: theme.accent
                                            }
                                        }

                                        MouseArea {
                                            id: saveCfgM
                                            anchors.fill: parent
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                if (root.activeDetailTheme) {
                                                    root.actionError = ""
                                                    root.actionSuccess = ""
                                                    var res = backend.saveThemeConfig(root.activeDetailTheme.id, root.draftConfig)
                                                    if (res.success) {
                                                        root.actionSuccess = res.message || "Customization saved successfully."
                                                        root.themeConfig = backend.getThemeConfig(root.activeDetailTheme.id)
                                                    } else {
                                                        root.actionError = res.message || "Failed to save customization."
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // System Integration Invariant Note
                        Rectangle {
                            Layout.fillWidth: true
                            height: 48
                            radius: theme.radiusSm
                            color: theme.surface
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 10
                                RyzoraIcon { name: "shield-check"; size: 16; color: "#10b981" }
                                Text {
                                    text: "System Safety: Applying this theme installs files to /usr/share/sddm/themes and sets /etc/sddm.conf.d/zz-ryzora-theme.conf. Desktop locking (Dusky/Hyprlock) remains completely untouched."
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }
                }

                // ──────────────────────────────────────────────────────────────
                // SUBPAGE 1: OVERVIEW DASHBOARD
                // ──────────────────────────────────────────────────────────────
                ColumnLayout {
                    visible: root.currentSubpage === "overview"
                    Layout.fillWidth: true
                    spacing: 18

                    // Header
                    ColumnLayout {
                        spacing: 4
                        Text {
                            text: "—  PERSONALIZATION"
                            font.pixelSize: 11
                            font.bold: true
                            font.letterSpacing: 1.5
                            color: theme.textMuted
                        }
                        Text {
                            text: "Studio"
                            font.pixelSize: 26
                            font.bold: true
                            color: theme.text
                        }
                        Text {
                            text: "Your desktop, your style. Configure every part of your Ryzora experience."
                            font.pixelSize: 12
                            color: theme.textMuted
                        }
                    }

                    // 2x2 Grid of Rich Studio Cards
                    GridLayout {
                        Layout.fillWidth: true
                        columns: width >= 720 ? 2 : 1
                        rowSpacing: 16
                        columnSpacing: 16

                        // Card 1: Lockscreen Studio
                        StudioHubCard {
                            title: "Lockscreen"
                            desc: "Login screen themes and options"
                            iconName: "lock"
                            statusBadge: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active) ? "ACTIVE THEME" : "SYSTEM DEFAULT"
                            statusColor: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active) ? "#10b981" : "#64748b"
                            previewColor: "#1e1b4b"
                            onOpenClicked: root.selectSubpage("lockscreen", null)
                        }

                        // Card 2: Fastfetch Studio
                        StudioHubCard {
                            title: "Fastfetch"
                            desc: "Terminal layout and modules"
                            iconName: "terminal"
                            statusBadge: (root.selectedFastfetchPreset || "default").toUpperCase()
                            statusColor: theme.accent
                            previewColor: "#0f172a"
                            onOpenClicked: {
                                root.selectSubpage("fastfetch", null)
                                if (!backend.fastfetchHtml || backend.fastfetchHtml.length === 0) {
                                    backend.requestFastfetchPreview(root.selectedFastfetchPreset)
                                }
                            }
                        }

                        // Card 3: Wallpaper Studio
                        StudioHubCard {
                            title: "Wallpapers"
                            desc: "Backgrounds and collections"
                            iconName: "image"
                            statusBadge: "READY"
                            statusColor: "#8b5cf6"
                            previewColor: "#2e1065"
                            onOpenClicked: root.selectSubpage("wallpapers", null)
                        }

                        // Card 4: Appearance Studio
                        StudioHubCard {
                            title: "Appearance"
                            desc: "Colors and visual preferences"
                            iconName: "palette"
                            statusBadge: theme.appearanceMode.toUpperCase() + " · " + theme.accentName.toUpperCase()
                            statusColor: theme.accent
                            previewColor: "#18181b"
                            onOpenClicked: root.selectSubpage("appearance", null)
                        }
                    }

                    // Bottom info card
                    Rectangle {
                        Layout.fillWidth: true
                        height: 56
                        radius: theme.radiusMd
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 16
                            spacing: 12
                            RyzoraIcon { name: "info"; size: 16; color: theme.accent }
                            Text {
                                text: "Installed packages stay safely preserved in Library; live preview, configuration, and integration live here in Studio."
                                font.pixelSize: 12
                                color: theme.textMuted
                                Layout.fillWidth: true
                            }
                            ActionButton {
                                label: "View Library"
                                iconName: "sliders"
                                onClicked: root.navigateRequested("library")
                            }
                        }
                    }
                }

                // ──────────────────────────────────────────────────────────────
                // SUBPAGE 3: FASTFETCH STUDIO
                // ──────────────────────────────────────────────────────────────
                ColumnLayout {
                    visible: root.currentSubpage === "fastfetch"
                    Layout.fillWidth: true
                    spacing: 18

                    ColumnLayout {
                        spacing: 4
                        Text { text: "—  PERSONALIZATION"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; font.letterSpacing: 1.5 }
                        Text { text: "Fastfetch Studio"; font.pixelSize: 26; font.bold: true; color: theme.text }
                        Text { text: "Terminal layout and modules configuration."; font.pixelSize: 12; color: theme.textMuted }
                    }

                    // Presets bar
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: "Preset:"
                            font.pixelSize: 12
                            font.bold: true
                            color: theme.textMuted
                        }

                        ScrollView {
                            Layout.fillWidth: true
                            clip: true
                            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                            ScrollBar.vertical.policy: ScrollBar.AlwaysOff

                            RowLayout {
                                spacing: 6
                                Repeater {
                                    model: backend.availablePresets
                                    Rectangle {
                                        height: 28
                                        width: presetTxt.implicitWidth + 20
                                        radius: 6
                                        color: (root.selectedFastfetchPreset === modelData)
                                            ? Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.2)
                                            : (pMouse.containsMouse ? theme.surfaceElevated : theme.surface)
                                        border.color: (root.selectedFastfetchPreset === modelData)
                                            ? theme.accent
                                            : theme.borderSubtle
                                        border.width: 1

                                        Text {
                                            id: presetTxt
                                            anchors.centerIn: parent
                                            text: modelData
                                            font.pixelSize: 11
                                            font.bold: root.selectedFastfetchPreset === modelData
                                            color: (root.selectedFastfetchPreset === modelData) ? theme.accent : theme.text
                                        }

                                        MouseArea {
                                            id: pMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                root.selectedFastfetchPreset = modelData
                                                backend.requestFastfetchPreview(modelData)
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Refresh Button
                        Rectangle {
                            height: 28
                            width: 84
                            radius: 6
                            color: theme.surface
                            border.color: theme.borderSubtle

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 4
                                RyzoraIcon { name: "refresh"; size: 11; color: theme.textMuted }
                                Text { text: "Refresh"; font.pixelSize: 11; font.bold: true; color: theme.text }
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: backend.requestFastfetchPreview(root.selectedFastfetchPreset)
                            }
                        }
                    }

                    // Main Canvas + Module Options
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 20

                        // Terminal Output Canvas
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.minimumWidth: 340
                            height: 400
                            radius: theme.radiusLg
                            color: "#08080c"
                            border.color: backend.isProcessActive() ? theme.accent : theme.borderStrong
                            border.width: 1
                            clip: true

                            // Terminal Titlebar
                            Rectangle {
                                id: termTitleBar
                                anchors.top: parent.top
                                anchors.left: parent.left
                                anchors.right: parent.right
                                height: 32
                                color: "#111116"
                                border.color: "#1e1e24"
                                border.width: 1

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 12
                                    anchors.rightMargin: 12
                                    spacing: 8

                                    // macOS Style Window Dots
                                    RowLayout {
                                        spacing: 6
                                        Rectangle { width: 10; height: 10; radius: 5; color: "#ff5f56" }
                                        Rectangle { width: 10; height: 10; radius: 5; color: "#ffbd2e" }
                                        Rectangle { width: 10; height: 10; radius: 5; color: "#27c93f" }
                                    }

                                    Item { Layout.fillWidth: true }

                                    Text {
                                        text: "fastfetch -c " + (root.selectedFastfetchPreset || "default")
                                        font.pixelSize: 11
                                        font.family: "monospace"
                                        color: "#71717a"
                                    }

                                    Item { Layout.fillWidth: true }
                                }
                            }

                            // Terminal Output Area
                            Flickable {
                                anchors.top: termTitleBar.bottom
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                anchors.margins: 14
                                contentWidth: termText.implicitWidth
                                contentHeight: termText.implicitHeight
                                clip: true

                                Text {
                                    id: termText
                                    text: backend.fastfetchHtml || "<span style='color:#71717a;'>Press Refresh to load fastfetch output...</span>"
                                    textFormat: Text.RichText
                                    font.family: "monospace"
                                    font.pixelSize: 11
                                    color: "#f4f4f5"
                                }
                            }
                        }
                    }
                }

                // ──────────────────────────────────────────────────────────────
                // SUBPAGE 4: WALLPAPERS STUDIO
                // ──────────────────────────────────────────────────────────────
                ColumnLayout {
                    visible: root.currentSubpage === "wallpapers"
                    Layout.fillWidth: true
                    spacing: 18

                    ColumnLayout {
                        spacing: 4
                        Text { text: "—  PERSONALIZATION"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; font.letterSpacing: 1.5 }
                        Text { text: "Wallpaper Studio"; font.pixelSize: 26; font.bold: true; color: theme.text }
                        Text { text: "Backgrounds and collections."; font.pixelSize: 12; color: theme.textMuted }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 360
                        radius: theme.radiusLg
                        color: "#0a0a10"
                        border.color: theme.borderStrong
                        border.width: 1
                        clip: true

                        Rectangle {
                            anchors.fill: parent
                            gradient: Gradient {
                                GradientStop { position: 0.0; color: "#0c4a6e" }
                                GradientStop { position: 0.5; color: "#1e1b4b" }
                                GradientStop { position: 1.0; color: "#09090b" }
                            }
                        }

                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 8
                            RyzoraIcon { name: "image"; size: 48; color: "#ffffff"; Layout.alignment: Qt.AlignHCenter }
                            Text {
                                text: root.selectedWallpaper ? root.selectedWallpaper.name : "Deep Space Nebula 8K"
                                font.pixelSize: 18
                                font.bold: true
                                color: "#ffffff"
                                Layout.alignment: Qt.AlignHCenter
                            }
                            Text {
                                text: "Resolution: 7680 × 4320 · Fit: " + root.wallpaperFit.toUpperCase()
                                font.pixelSize: 12
                                color: "#94a3b8"
                                Layout.alignment: Qt.AlignHCenter
                            }
                        }
                    }
                }

                // ──────────────────────────────────────────────────────────────
                // SUBPAGE 5: APPEARANCE STUDIO
                // ──────────────────────────────────────────────────────────────
                ColumnLayout {
                    visible: root.currentSubpage === "appearance"
                    Layout.fillWidth: true
                    spacing: 18

                    ColumnLayout {
                        spacing: 4
                        Text { text: "—  PERSONALIZATION"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; font.letterSpacing: 1.5 }
                        Text { text: "Appearance Studio"; font.pixelSize: 26; font.bold: true; color: theme.text }
                        Text { text: "Colors and visual preferences."; font.pixelSize: 12; color: theme.textMuted }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        radius: theme.radiusLg
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 20
                            spacing: 16

                            Text {
                                text: "THEME PREFERENCE"
                                font.pixelSize: 10
                                font.bold: true
                                font.letterSpacing: 1.2
                                color: theme.textMuted
                            }

                            RowLayout {
                                spacing: 12
                                ThemeModeCard {
                                    modeName: "dark"
                                    label: "Dark"
                                    iconName: "lock"
                                    isSelected: theme.appearanceMode === "dark"
                                    onClicked: {
                                        theme.appearanceMode = "dark"
                                        root.persistAppearance()
                                    }
                                }
                                ThemeModeCard {
                                    modeName: "light"
                                    label: "Light"
                                    iconName: "palette"
                                    isSelected: theme.appearanceMode === "light"
                                    onClicked: {
                                        theme.appearanceMode = "light"
                                        root.persistAppearance()
                                    }
                                }
                                ThemeModeCard {
                                    modeName: "system"
                                    label: "System Auto"
                                    iconName: "desktop"
                                    isSelected: theme.appearanceMode === "system"
                                    onClicked: {
                                        theme.appearanceMode = "system"
                                        root.persistAppearance()
                                    }
                                }
                            }

                            Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                            Text {
                                text: "ACCENT COLOR"
                                font.pixelSize: 10
                                font.bold: true
                                font.letterSpacing: 1.2
                                color: theme.textMuted
                            }

                            RowLayout {
                                spacing: 14
                                Repeater {
                                    model: ["blue", "silver", "purple", "green", "orange"]
                                    Rectangle {
                                        width: 38; height: 38; radius: 19
                                        color: theme.swatches[modelData] || "#6488E8"
                                        border.color: theme.accentName === modelData ? "#ffffff" : "transparent"
                                        border.width: 2

                                        Rectangle {
                                            anchors.centerIn: parent
                                            width: 12; height: 12; radius: 6
                                            color: "#ffffff"
                                            visible: theme.accentName === modelData
                                        }

                                        MouseArea {
                                            anchors.fill: parent
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                theme.accentName = modelData
                                                root.persistAppearance()
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    function persistAppearance() {
        var appObj = {
            "mode": theme.appearanceMode,
            "accent": theme.accentName,
            "reduce_motion": theme.reduceMotion,
            "reduce_transparency": theme.reduceTransparency,
            "increase_contrast": theme.increaseContrast
        }
        backend.saveAppearance(appObj)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // REUSABLE SUBCOMPONENTS (Clean Vector Icons, No Emoji)
    // ──────────────────────────────────────────────────────────────────────────
    component StudioSubpageButton : Rectangle {
        id: subBtn
        property string iconName: "sliders"
        property string title: ""
        property string desc: ""
        property bool isActive: false
        signal clicked()

        Layout.fillWidth: true
        height: 50
        radius: theme.radiusMd
        color: isActive
            ? Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.15)
            : (subMouse.containsMouse ? theme.surfaceHover : "transparent")
        border.color: isActive ? theme.accent : "transparent"
        border.width: 1

        RowLayout {
            anchors.fill: parent
            anchors.margins: 10
            spacing: 10

            RyzoraIcon {
                name: subBtn.iconName
                size: 15
                color: subBtn.isActive ? theme.accent : theme.textMuted
            }

            ColumnLayout {
                spacing: 1
                Layout.fillWidth: true
                Text {
                    text: subBtn.title
                    font.pixelSize: 12
                    font.bold: subBtn.isActive
                    color: subBtn.isActive ? theme.accent : theme.text
                }
                Text {
                    text: subBtn.desc
                    font.pixelSize: 10
                    color: theme.textMuted
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }
        }

        MouseArea {
            id: subMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: subBtn.clicked()
        }
    }

    component StudioHubCard : Rectangle {
        id: hubCard
        property string title: ""
        property string desc: ""
        property string iconName: "sliders"
        property string statusBadge: ""
        property color statusColor: theme.accent
        property color previewColor: "#1e1b4b"
        signal openClicked()

        Layout.fillWidth: true
        height: 180
        radius: theme.radiusLg
        color: cardMouse.containsMouse ? theme.surfaceElevated : theme.surface
        border.color: cardMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Rectangle {
                    width: 36; height: 36; radius: theme.radiusSm
                    color: Qt.rgba(hubCard.statusColor.r, hubCard.statusColor.g, hubCard.statusColor.b, 0.15)
                    RyzoraIcon { anchors.centerIn: parent; name: hubCard.iconName; size: 18; color: hubCard.statusColor }
                }

                ColumnLayout {
                    spacing: 1
                    Layout.fillWidth: true
                    Text { text: hubCard.title; font.pixelSize: 14; font.bold: true; color: theme.text }
                    Text { text: hubCard.desc; font.pixelSize: 11; color: theme.textMuted }
                }

                Rectangle {
                    height: 20
                    width: stBadgeTxt.implicitWidth + 14
                    radius: 4
                    color: Qt.rgba(hubCard.statusColor.r, hubCard.statusColor.g, hubCard.statusColor.b, 0.2)
                    border.color: hubCard.statusColor
                    border.width: 1
                    Text {
                        id: stBadgeTxt
                        anchors.centerIn: parent
                        text: hubCard.statusBadge
                        font.pixelSize: 9
                        font.bold: true
                        color: hubCard.statusColor
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: theme.radiusSm
                color: hubCard.previewColor
                border.color: Qt.rgba(255, 255, 255, 0.08)
                clip: true

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 8
                    RyzoraIcon { name: hubCard.iconName; size: 16; color: "#94a3b8" }
                    Text { text: "Interactive Studio Editor"; font.pixelSize: 11; color: "#cbd5e1" }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Item { Layout.fillWidth: true }
                RowLayout {
                    spacing: 4
                    Text {
                        text: "Open Studio"
                        font.pixelSize: 12
                        font.bold: true
                        color: cardMouse.containsMouse ? theme.accent : theme.textSecondary
                    }
                    RyzoraIcon { name: "arrow-right"; size: 12; color: cardMouse.containsMouse ? theme.accent : theme.textSecondary }
                }
            }
        }

        MouseArea {
            id: cardMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: hubCard.openClicked()
        }
    }

    component HeaderPillButton : Rectangle {
        id: pillBtn
        property string label: ""
        property string iconName: ""
        signal clicked()

        implicitHeight: 28
        implicitWidth: pRow.implicitWidth + 20
        radius: 6
        color: pMouse.containsMouse ? theme.surfaceHover : theme.surface
        border.color: pMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
        border.width: 1

        RowLayout {
            id: pRow
            anchors.centerIn: parent
            spacing: 6

            RyzoraIcon {
                visible: pillBtn.iconName !== ""
                name: pillBtn.iconName
                size: 11
                color: theme.textSecondary
            }

            Text {
                text: pillBtn.label
                font.pixelSize: 10
                font.bold: true
                font.letterSpacing: 0.8
                color: theme.text
            }
        }

        MouseArea {
            id: pMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: pillBtn.clicked()
        }
    }

    component ThemeModeCard : Rectangle {
        id: modeCard
        property string modeName: ""
        property string label: ""
        property string iconName: ""
        property bool isSelected: false
        signal clicked()

        Layout.fillWidth: true
        height: 72
        radius: theme.radiusMd
        color: isSelected
            ? Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.15)
            : (mArea.containsMouse ? theme.surfaceElevated : theme.surface)
        border.color: isSelected ? theme.accent : theme.borderSubtle
        border.width: 1

        ColumnLayout {
            anchors.centerIn: parent
            spacing: 4
            RyzoraIcon { name: modeCard.iconName; size: 20; color: modeCard.isSelected ? theme.accent : theme.textMuted; Layout.alignment: Qt.AlignHCenter }
            Text {
                text: modeCard.label
                font.pixelSize: 12
                font.bold: modeCard.isSelected
                color: modeCard.isSelected ? theme.accent : theme.text
                Layout.alignment: Qt.AlignHCenter
            }
        }

        MouseArea {
            id: mArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: modeCard.clicked()
        }
    }

    component ActionButton : Rectangle {
        id: actBtn
        property string label: ""
        property string iconName: ""
        signal clicked()

        implicitHeight: 32
        implicitWidth: btnRow.implicitWidth + 24
        radius: theme.radiusSm
        color: actMouse.containsMouse ? theme.surfaceElevated : "transparent"
        border.color: theme.borderSubtle
        border.width: 1

        RowLayout {
            id: btnRow
            anchors.centerIn: parent
            spacing: 6

            RyzoraIcon {
                visible: actBtn.iconName !== ""
                name: actBtn.iconName
                size: 12
                color: theme.textSecondary
            }

            Text {
                text: actBtn.label
                font.pixelSize: 11
                font.bold: true
                color: actMouse.containsMouse ? theme.text : theme.textSecondary
            }
        }

        MouseArea {
            id: actMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: actBtn.clicked()
        }
    }
}
