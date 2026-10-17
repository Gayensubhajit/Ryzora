import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia
import "../components"

Item {
    id: root

    property string currentSubpage: "lockscreen" // Default to Lockscreen Studio
    property string returnRoute: ""
    property var activeDetailTheme: null
    property var installedThemes: []
    property bool isSecurityExpanded: false

    function updateInstalledThemes() {
        var res = []
        var list = backend.libraryList || []
        for (var i = 0; i < list.length; ++i) {
            var item = list[i]
            if (!item || !item.id) continue
            var pid = String(item.id)
            var cat = String(item.category || item.package_type || "").toLowerCase()
            var isSilent = pid.indexOf("silentsddm-") === 0 || String(item.subfilter || "") === "silentsddm"
            var isQylock = pid.indexOf("lockscreen-qylock-") === 0 || String(item.subfilter || "") === "qylock"
            var isCustom = pid.indexOf("custom:") === 0
            var isLock = cat === "lockscreens" || cat === "lockscreen" || cat === "sddm" || isSilent || isQylock
            var isWp = String(item.package_type || "") === "wallpaper" && !isSilent && !item.supports_sddm
            var supportsSddm = Boolean(item.supports_sddm) || isSilent || (isQylock && !pid.includes("swaylock") && !pid.includes("hyprlock"))

            if (isLock && !isWp && !isCustom && supportsSddm) {
                res.push(item)
            }
        }
        root.installedThemes = res
    }

    onVisibleChanged: {
        if (visible) {
            updateInstalledThemes()
            if (!backend.libraryList || backend.libraryList.length === 0) {
                backend.loadLibrary(false)
            }
        }
    }

    Component.onCompleted: {
        if (!backend.libraryList || backend.libraryList.length === 0) {
            backend.loadLibrary(false)
        }
        updateInstalledThemes()
    }
    property var themeConfig: ({})
    property var draftConfig: ({})

    function isTruthy(v) {
        if (v === true || v === 1) return true;
        if (v === false || v === 0) return false;
        if (v === undefined || v === null) return false;
        var s = String(v).trim().toLowerCase();
        return s === "true" || s === "1" || s === "yes" || s === "on";
    }
    property string pendingAction: ""

    function triggerApply() {
        if (root.activeDetailTheme && root.pendingAction === "") {
            root.actionError = ""
            root.actionSuccess = ""
            if (root.hasUnsavedChanges) {
                var saveRes = backend.saveThemeConfig(root.activeDetailTheme.id, root.draftConfig)
                if (!saveRes.success) {
                    root.actionError = saveRes.message || "Failed to save customization before applying."
                    return
                }
                root.themeConfig = backend.getThemeConfig(root.activeDetailTheme.id)
            }
            root.pendingAction = "apply"
            backend.applyTarget(root.activeDetailTheme.id, "sddm_login")
        }
    }

    function triggerDeactivate() {
        if (root.pendingAction === "") {
            root.actionError = ""
            root.actionSuccess = ""
            root.pendingAction = "deactivate"
            backend.deactivateTarget("sddm_login")
        }
    }

    function triggerPreview() {
        if (root.activeDetailTheme && root.pendingAction === "") {
            root.actionError = ""
            root.pendingAction = "preview"
            backend.testPackage(root.activeDetailTheme.id)
        }
    }

    function triggerSave() {
        if (root.activeDetailTheme && root.pendingAction === "") {
            root.actionError = ""
            root.actionSuccess = ""
            var res = backend.saveThemeConfig(root.activeDetailTheme.id, root.draftConfig)
            if (res.success) {
                root.actionSuccess = res.message || "Customization saved successfully."
                root.themeConfig = backend.getThemeConfig(root.activeDetailTheme.id)
                backend.updatePreviewDraft(root.activeDetailTheme.id, root.draftConfig)
            } else {
                root.actionError = res.message || "Failed to save customization."
            }
        }
    }

    function triggerReset() {
        if (root.activeDetailTheme && root.pendingAction === "") {
            root.actionError = ""
            root.actionSuccess = ""
            var res = backend.resetThemeConfig(root.activeDetailTheme.id)
            if (res.success) {
                root.themeConfig = res
                var d = {}
                if (res.settings) {
                    for (var i = 0; i < res.settings.length; ++i) {
                        d[res.settings[i].id] = res.settings[i].current_value
                    }
                }
                root.draftConfig = d
                backend.updatePreviewDraft(root.activeDetailTheme.id, d)
                root.actionSuccess = "Settings restored to theme defaults."
            } else {
                root.actionError = res.message || "Failed to reset theme settings."
            }
        }
    }
    property string actionError: ""
    property string actionSuccess: ""

    readonly property bool hasUnsavedChanges: {
        if (!root.themeConfig || !root.themeConfig.settings || !root.draftConfig) return false
        for (var i = 0; i < root.themeConfig.settings.length; ++i) {
            var s = root.themeConfig.settings[i]
            var savedVal = s.current_value
            var curVal = root.draftConfig[s.id]
            if (curVal !== undefined && String(curVal) !== String(savedVal)) {
                return true
            }
        }
        return false
    }

    function updateDraft(key, value) {
        var d = Object.assign({}, root.draftConfig)
        d[key] = value
        root.draftConfig = d
        if (root.activeDetailTheme && root.activeDetailTheme.id) {
            backend.updatePreviewDraft(root.activeDetailTheme.id, d)
        }
    }

    onDraftConfigChanged: {
        if (root.activeDetailTheme && root.activeDetailTheme.id) {
            backend.updatePreviewDraft(root.activeDetailTheme.id, root.draftConfig)
        }
    }

    function getCategorySettings(catName) {
        if (!root.themeConfig || !root.themeConfig.settings) return []
        var res = []
        for (var i = 0; i < root.themeConfig.settings.length; ++i) {
            if (root.themeConfig.settings[i].category === catName) {
                res.push(root.themeConfig.settings[i])
            }
        }
        return res
    }

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

    function goBackInStudio() {
        root.actionError = ""
        root.actionSuccess = ""
        if (root.returnRoute !== "") {
            var r = root.returnRoute
            root.returnRoute = ""
            root.activeDetailTheme = null
            root.navigateRequested(r)
            return
        }
        if (root.activeDetailTheme !== null) {
            root.activeDetailTheme = null
            return
        }
        if (root.currentSubpage !== "lockscreen") {
            root.selectSubpage("lockscreen", null)
            return
        }
        if (typeof window !== "undefined" && window && window.navHistoryIndex > 0) {
            window.goBack()
        }
    }

    function selectSubpage(page, pkg) {
        root.currentSubpage = page
        root.actionError = ""
        root.actionSuccess = ""
        root.pendingAction = ""
        root.updateInstalledThemes()

        if (page === "lockscreen") {
            var pkgId = typeof pkg === "string" ? pkg : (pkg ? pkg.id : "")
            if (pkgId && pkgId.length > 0) {
                var found = null
                var lib = backend.libraryList || []
                for (var i = 0; i < lib.length; ++i) {
                    if (lib[i].id === pkgId || lib[i].id.replace(/^lockscreen-qylock-/, "") === pkgId || pkgId.replace(/^lockscreen-qylock-/, "") === lib[i].id) {
                        found = lib[i]
                        break
                    }
                }
                if (found) {
                    root.activeDetailTheme = found
                } else {
                    var catFound = null
                    var catList = backend.catalogList || []
                    for (var j = 0; j < catList.length; ++j) {
                        if (catList[j].id === pkgId || catList[j].id.replace(/^lockscreen-qylock-/, "") === pkgId || pkgId.replace(/^lockscreen-qylock-/, "") === catList[j].id) {
                            catFound = catList[j]
                            break
                        }
                    }
                    if (catFound) {
                        root.activeDetailTheme = catFound
                        root.actionError = "Theme '" + catFound.name + "' is not installed yet. Install it to configure theme options."
                    } else if (pkg && typeof pkg === "object") {
                        root.activeDetailTheme = pkg
                    } else {
                        root.activeDetailTheme = null
                    }
                }
            } else {
                root.activeDetailTheme = null
            }
        } else if (page === "fastfetch") {
            root.activeDetailTheme = null
            var ffId = typeof pkg === "string" ? pkg : (pkg ? pkg.id : "")
            if (ffId) {
                var preset = ffId.replace(/^fastfetch-/, "")
                root.selectedFastfetchPreset = preset
                backend.requestFastfetchPreview(preset)
            }
        } else if (page === "wallpapers") {
            root.activeDetailTheme = null
            var wpId = typeof pkg === "string" ? pkg : (pkg ? pkg.id : "")
            var wpFound = null
            var all = backend.catalogList || []
            for (var k = 0; k < all.length; ++k) {
                if (all[k].id === wpId) {
                    wpFound = all[k]
                    break
                }
            }
            root.selectedWallpaper = wpFound || (typeof pkg === "object" ? pkg : null)
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
            root.updateInstalledThemes()
            if (root.activeDetailTheme) {
                var lib = backend.libraryList || []
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
        if (!root.installedThemes || root.installedThemes.length === 0) {
            root.updateInstalledThemes()
        }
        return root.installedThemes
    }

    function isThemeSddmActive(item) {
        if (!item) return false
        if (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active) {
            return backend.activeConfiguration.active_package_id === item.id
        }
        return Boolean(item.active_targets && item.active_targets.sddm_login)
    }

    function isThemeLockActive(item) {
        if (!item) return false
        if (backend.activeConfiguration && backend.activeConfiguration.session_lock_active) {
            return backend.activeConfiguration.active_lock_package_id === item.id
        }
        return Boolean(item.active_targets && item.active_targets.session_lock)
    }

    function isThemeActive(item) {
        return isThemeSddmActive(item)
    }

    // ══════════════════════════════════════════════════════════════════════
    // COMPACT HORIZONTAL STUDIO NAVIGATION TABS (macOS System Settings style)
    // ══════════════════════════════════════════════════════════════════════
    Rectangle {
        id: studioTabsBar
        visible: root.activeDetailTheme === null
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: 48
        z: 10
        color: theme.surface
        border.color: theme.borderSubtle
        border.width: 1

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 24
            anchors.rightMargin: 24
            spacing: 8

            // Segmented / Pill Navigation Tabs
            RowLayout {
                spacing: 4

                StudioTabButton {
                    iconName: "lock"
                    label: "Lockscreen"
                    isActive: root.currentSubpage === "lockscreen"
                    onClicked: root.selectSubpage("lockscreen", null)
                }

                StudioTabButton {
                    iconName: "terminal"
                    label: "Fastfetch"
                    isActive: root.currentSubpage === "fastfetch"
                    onClicked: {
                        root.selectSubpage("fastfetch", null)
                        if (!backend.fastfetchHtml || backend.fastfetchHtml.length === 0) {
                            backend.requestFastfetchPreview(root.selectedFastfetchPreset)
                        }
                    }
                }

                StudioTabButton {
                    iconName: "image"
                    label: "Wallpapers"
                    isActive: root.currentSubpage === "wallpapers"
                    onClicked: root.selectSubpage("wallpapers", null)
                }

                StudioTabButton {
                    iconName: "palette"
                    label: "Appearance"
                    isActive: root.currentSubpage === "appearance"
                    onClicked: root.selectSubpage("appearance", null)
                }
            }

            Item { Layout.fillWidth: true }

            // Subtle inline active greeter status badge on the right
            Rectangle {
                property bool isSddmAct: Boolean(backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                height: 24
                radius: 12
                implicitWidth: quickSddmRow.implicitWidth + 16
                color: isSddmAct ? Qt.rgba(16/255, 185/255, 129/255, 0.12) : Qt.rgba(100/255, 116/255, 139/255, 0.12)
                border.color: isSddmAct ? "#10B981" : theme.borderSubtle
                border.width: 1

                RowLayout {
                    id: quickSddmRow
                    anchors.centerIn: parent
                    spacing: 6
                    Rectangle {
                        width: 6; height: 6; radius: 3
                        color: parent.parent.isSddmAct ? "#10B981" : "#64748B"
                    }
                    Text {
                        text: parent.parent.isSddmAct
                            ? "SDDM: " + (backend.activeConfiguration.active_title || "Active Theme")
                            : "SDDM: Default"
                        font.pixelSize: 10
                        font.bold: true
                        color: parent.parent.isSddmAct ? (theme.resolvedTheme === "light" ? "#047857" : "#34D399") : theme.textMuted
                    }
                }
            }
        }
    }

            // Persistent top toolbar for theme editor (macOS style)
            Rectangle {
                id: editorToolbar
                visible: root.currentSubpage === "lockscreen" && root.activeDetailTheme !== null
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                height: 52
                z: 10
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 20
                    anchors.rightMargin: 20
                    spacing: 12

                    // Back to gallery button
                    Rectangle {
                        width: 32
                        height: 32
                        radius: theme.radiusSm
                        color: backBtnM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        RyzoraIcon {
                            anchors.centerIn: parent
                            name: "chevron-left"
                            size: 14
                            color: theme.text
                        }

                        MouseArea {
                            id: backBtnM
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.goBackInStudio()
                        }
                    }

                    // Theme Name
                    Text {
                        text: root.activeDetailTheme ? root.activeDetailTheme.name : ""
                        font.pixelSize: 16
                        font.bold: true
                        color: theme.text
                        elide: Text.ElideRight
                        Layout.maximumWidth: 260
                    }

                    // Single Truthful Status Badge
                    Rectangle {
                        property bool isSddmAct: root.isThemeSddmActive(root.activeDetailTheme)
                        property bool isLockAct: root.isThemeLockActive(root.activeDetailTheme)
                        height: 22
                        width: statusRowHdr.implicitWidth + 14
                        radius: 11
                        color: isSddmAct
                            ? (theme.resolvedTheme === "light" ? "#DCFCE7" : "#064E3B")
                            : (isLockAct
                                ? (theme.resolvedTheme === "light" ? "#EDE9FE" : Qt.rgba(139/255, 92/255, 246/255, 0.25))
                                : (theme.resolvedTheme === "light" ? "#F1F5F9" : "#1E293B"))
                        border.color: isSddmAct ? "#10B981" : (isLockAct ? "#8B5CF6" : theme.borderSubtle)
                        border.width: 1

                        RowLayout {
                            id: statusRowHdr
                            anchors.centerIn: parent
                            spacing: 5

                            Rectangle {
                                width: 6; height: 6; radius: 3
                                color: parent.parent.isSddmAct ? "#10B981" : (parent.parent.isLockAct ? "#8B5CF6" : "#64748B")
                            }

                            Text {
                                text: parent.parent.isSddmAct ? "Active at SDDM Login" : (parent.parent.isLockAct ? "Active on Desktop Lock" : "Installed")
                                font.pixelSize: 10
                                font.bold: true
                                color: parent.parent.isSddmAct
                                    ? (theme.resolvedTheme === "light" ? "#047857" : "#34D399")
                                    : (parent.parent.isLockAct
                                        ? (theme.resolvedTheme === "light" ? "#6D28D9" : "#A78BFA")
                                        : theme.textSecondary)
                            }
                        }
                    }

                    // Unsaved Changes Indicator
                    Rectangle {
                        visible: root.hasUnsavedChanges
                        height: 22
                        width: unsavedRow.implicitWidth + 12
                        radius: 11
                        color: theme.resolvedTheme === "light" ? "#FEF3C7" : "#78350F"
                        border.color: "#F59E0B"
                        border.width: 1

                        RowLayout {
                            id: unsavedRow
                            anchors.centerIn: parent
                            spacing: 4

                            Rectangle { width: 5; height: 5; radius: 2.5; color: "#F59E0B" }
                            Text {
                                text: "Unsaved Changes"
                                font.pixelSize: 10
                                font.bold: true
                                color: theme.resolvedTheme === "light" ? "#B45309" : "#FDE68A"
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Status / Action feedback message
                    Text {
                        visible: (root.actionError !== "" || root.actionSuccess !== "" || root.pendingAction !== "") && width > 40
                        text: {
                            if (root.pendingAction === "apply") return "Applying to SDDM greeter…"
                            if (root.pendingAction === "preview") return "Launching fullscreen preview…"
                            if (root.pendingAction === "deactivate") return "Restoring default SDDM…"
                            if (root.actionError !== "") return root.actionError
                            if (root.actionSuccess !== "") return root.actionSuccess
                            return ""
                        }
                        font.pixelSize: 11
                        font.bold: root.actionError !== ""
                        color: root.actionError !== "" ? "#EF4444" : (root.actionSuccess !== "" ? "#10B981" : "#3B82F6")
                        elide: Text.ElideRight
                        Layout.maximumWidth: 300
                    }

                    // Save Button in Header (only when unsaved changes exist)
                    Rectangle {
                        visible: root.hasUnsavedChanges
                        height: 28
                        width: saveTxtHdr.implicitWidth + 20
                        radius: theme.radiusSm
                        color: theme.accent
                        opacity: root.pendingAction !== "" ? 0.5 : 1.0

                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 5
                            RyzoraIcon { name: "check"; size: 12; color: "#FFFFFF" }
                            Text {
                                id: saveTxtHdr
                                text: "Save Changes"
                                font.pixelSize: 11
                                font.bold: true
                                color: "#FFFFFF"
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.triggerSave()
                        }
                    }
                }
            }

            ScrollView {
                id: studioScroll

                WheelHandler {
                    target: null
                    acceptedDevices: PointerDevice.Mouse
                    blocking: true
                    onWheel: (event) => {
                        var fl = studioScroll.contentItem
                        if (fl && fl.contentHeight > fl.height) {
                            var delta = event.angleDelta.y
                            var step = -delta * (80.0 / 120.0) * 1.18
                            fl.contentY = Math.max(0, Math.min(fl.contentHeight - fl.height, fl.contentY + step))
                        }
                    }
                }
                anchors.top: editorToolbar.visible ? editorToolbar.bottom : (studioTabsBar.visible ? studioTabsBar.bottom : parent.top)
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
            clip: true
            ScrollBar.vertical.policy: ScrollBar.AsNeeded
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                x: 24
                width: Math.max(300, studioScroll.availableWidth - 48)
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

                        // 1. Compact Header Row (macOS System Settings style)
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Text {
                                    text: "Lockscreen"
                                    font.pixelSize: 22
                                    font.bold: true
                                    color: theme.text
                                }

                                Text {
                                    text: "Installed SDDM login greeter themes and animated lockscreen skins."
                                    font.pixelSize: 12
                                    color: theme.textMuted
                                }
                            }

                            // Single Browse Store action pill
                            Rectangle {
                                height: 32
                                radius: theme.radiusSm
                                implicitWidth: browseStoreRow.implicitWidth + 20
                                color: browseStoreMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                border.color: browseStoreMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
                                border.width: 1

                                RowLayout {
                                    id: browseStoreRow
                                    anchors.centerIn: parent
                                    spacing: 6
                                    RyzoraIcon { name: "store"; size: 12; color: theme.text }
                                    Text {
                                        text: "Browse Store"
                                        font.pixelSize: 11
                                        font.weight: Font.Medium
                                        color: theme.text
                                    }
                                }

                                MouseArea {
                                    id: browseStoreMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.navigateRequested("discover")
                                }
                            }
                        }

                        // 3. Responsive Gallery of Installed SDDM Theme Cards (Full Width Grid & Hover Effects)
                        GridLayout {
                            id: galleryGrid
                            Layout.fillWidth: true
                            rowSpacing: 18
                            columnSpacing: 18
                            columns: Math.max(1, Math.floor((width + 18) / 320))
                            visible: root.installedThemes.length > 0

                            Repeater {
                                model: root.installedThemes
                                StoreCard {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 230
                                    Layout.minimumHeight: 210
                                    Layout.minimumWidth: 260
                                    itemData: modelData
                                    hideActionButton: true
                                    hideCategoryBadge: true
                                    isPageVisible: root.visible && root.currentSubpage === "lockscreen" && root.activeDetailTheme === null
                                    allowAutoplay: true
                                    onCardClicked: (pkg) => root.openThemeDetail(pkg)
                                }
                            }
                        }

                        // 4. Empty State (When no SDDM themes installed)
                        Rectangle {
                            visible: root.installedThemes.length === 0
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
                        spacing: 18

                        // 1. Action Error Banner (if any)
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
                                RyzoraIcon { name: "x"; size: 14; color: "#f87171" }
                                Text {
                                    text: root.actionError
                                    font.pixelSize: 12
                                    color: "#fca5a5"
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                                Rectangle {
                                    width: 18; height: 18; radius: 9
                                    color: "transparent"
                                    RyzoraIcon { anchors.centerIn: parent; name: "x"; size: 10; color: "#fca5a5" }
                                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.actionError = "" }
                                }
                            }
                        }

                        // 2. Responsive Hero Section (Left Preview + Right Context Summary)
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 18
                            Layout.alignment: Qt.AlignTop

                            // Left Pane: Left-aligned Preview Container using available width (aspect 16:9)
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredWidth: studioScroll.width >= 920 ? 3 : 1
                                Layout.minimumWidth: 320
                                radius: theme.radiusMd
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1
                                implicitHeight: previewCol.implicitHeight + 20

                                ColumnLayout {
                                    id: previewCol
                                    anchors.fill: parent
                                    anchors.margins: 10
                                    spacing: 8

                                    LoginPreview {
                                        id: studioPreview
                                        Layout.fillWidth: true
                                        Layout.preferredHeight: Math.round(width / aspect)
                                        radius: theme.radiusSm
                                        border.color: theme.borderSubtle
                                        packageData: root.activeDetailTheme
                                        config: root.draftConfig
                                        hasUnsaved: root.hasUnsavedChanges
                                        active: root.visible && root.activeDetailTheme !== null
                                    }

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 6
                                        RyzoraIcon { name: "info"; size: 12; color: theme.textMuted }
                                        Text {
                                            text: studioPreview.isReal
                                                ? "Rendered with theme’s live engine. Edits update preview dynamically."
                                                : "Simulated login composition driven by your draft settings."
                                            font.pixelSize: 11
                                            color: theme.textMuted
                                            Layout.fillWidth: true
                                            elide: Text.ElideRight
                                        }
                                    }

                                    // Quick Actions Row under Preview Canvas (fallback for narrow screens when sidebar is hidden)
                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 8
                                        visible: studioScroll.width < 920

                                        Rectangle {
                                            Layout.fillWidth: true
                                            height: 32
                                            radius: theme.radiusSm
                                            color: fsQuickMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            border.width: 1

                                            RowLayout {
                                                anchors.centerIn: parent
                                                spacing: 6
                                                RyzoraIcon { name: "maximize-2"; size: 12; color: theme.textSecondary }
                                                Text {
                                                    text: "Fullscreen Preview"
                                                    font.pixelSize: 11
                                                    font.weight: Font.Medium
                                                    color: theme.text
                                                }
                                            }

                                            MouseArea {
                                                id: fsQuickMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: root.triggerPreview()
                                            }
                                        }

                                        Rectangle {
                                            Layout.fillWidth: true
                                            height: 32
                                            radius: theme.radiusSm
                                            property bool isApplied: root.isThemeSddmActive(root.activeDetailTheme)
                                            property bool needsApply: !isApplied || root.hasUnsavedChanges
                                            color: (!needsApply && isApplied) ? Qt.rgba(16/255, 185/255, 129/255, 0.15) : (applyQuickMouse.containsMouse ? theme.accentHover : theme.accent)
                                            border.color: (!needsApply && isApplied) ? "#10B981" : "transparent"
                                            border.width: (!needsApply && isApplied) ? 1 : 0

                                            RowLayout {
                                                anchors.centerIn: parent
                                                spacing: 6
                                                RyzoraIcon {
                                                    name: (!parent.parent.needsApply && parent.parent.isApplied) ? "check" : "shield"
                                                    size: 12
                                                    color: (!parent.parent.needsApply && parent.parent.isApplied) ? "#10B981" : "#FFFFFF"
                                                }
                                                Text {
                                                    text: root.pendingAction === "apply" ? "Applying…" : (root.hasUnsavedChanges ? "Save & Apply" : (parent.parent.isApplied ? "Active on SDDM" : "Apply to Login Screen"))
                                                    font.pixelSize: 11
                                                    font.weight: Font.SemiBold
                                                    color: (!parent.parent.needsApply && parent.parent.isApplied) ? "#10B981" : "#FFFFFF"
                                                }
                                            }

                                            MouseArea {
                                                id: applyQuickMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                enabled: root.pendingAction !== "apply" && parent.needsApply
                                                onClicked: root.triggerApply()
                                            }
                                        }
                                    }
                                }
                            }

                            // Right Pane: Contextual Theme Summary Card (when width >= 920)
                            Rectangle {
                                visible: studioScroll.width >= 920
                                Layout.preferredWidth: 260
                                Layout.minimumWidth: 220
                                Layout.maximumWidth: 320
                                Layout.fillHeight: true
                                radius: theme.radiusMd
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 14
                                    spacing: 10

                                    Text {
                                        text: "ABOUT THEME"
                                        font.pixelSize: 10
                                        font.bold: true
                                        font.letterSpacing: 1.2
                                        color: theme.textMuted
                                    }

                                    Text {
                                        text: root.activeDetailTheme ? root.activeDetailTheme.name : ""
                                        font.pixelSize: 16
                                        font.bold: true
                                        color: theme.text
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }

                                    Text {
                                        text: root.activeDetailTheme ? (root.activeDetailTheme.description || "Authentic SDDM login theme.") : ""
                                        font.pixelSize: 11
                                        color: theme.textSecondary
                                        wrapMode: Text.WordWrap
                                        maximumLineCount: 3
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }

                                    Rectangle {
                                        Layout.fillWidth: true
                                        height: 1
                                        color: theme.borderSubtle
                                    }

                                    ColumnLayout {
                                        spacing: 6
                                        Layout.fillWidth: true

                                        RowLayout {
                                            spacing: 6
                                            Text { text: "Author:"; font.pixelSize: 11; color: theme.textMuted; Layout.preferredWidth: 55 }
                                            Text {
                                                text: root.activeDetailTheme ? (typeof root.activeDetailTheme.author === "object" ? (root.activeDetailTheme.author.name || "Community") : String(root.activeDetailTheme.author || "Community")) : "Community"
                                                font.pixelSize: 11; font.bold: true; color: theme.text
                                                elide: Text.ElideRight
                                                Layout.fillWidth: true
                                            }
                                        }

                                        RowLayout {
                                            spacing: 6
                                            Text { text: "Target:"; font.pixelSize: 11; color: theme.textMuted; Layout.preferredWidth: 55 }
                                            Text { text: "SDDM Greeter"; font.pixelSize: 11; font.bold: true; color: theme.text }
                                        }

                                        RowLayout {
                                            spacing: 6
                                            Text { text: "Status:"; font.pixelSize: 11; color: theme.textMuted; Layout.preferredWidth: 55 }
                                            Text {
                                                text: root.isThemeSddmActive(root.activeDetailTheme) ? "Active at SDDM Login" : (root.isThemeLockActive(root.activeDetailTheme) ? "Active on Desktop Lock" : "Installed")
                                                font.pixelSize: 11
                                                font.bold: true
                                                color: root.isThemeSddmActive(root.activeDetailTheme) ? "#10b981" : (root.isThemeLockActive(root.activeDetailTheme) ? "#8b5cf6" : theme.textSecondary)
                                                elide: Text.ElideRight
                                                Layout.fillWidth: true
                                            }
                                        }
                                    }

                                    Rectangle {
                                        Layout.fillWidth: true
                                        height: 1
                                        color: theme.borderSubtle
                                    }

                                    // Action Buttons in Side Panel
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 8

                                        // Primary CTA: Apply to SDDM Login Screen
                                        Rectangle {
                                            id: applyCtaBtn
                                            Layout.fillWidth: true
                                            height: 38
                                            radius: theme.radiusSm
                                            property bool isApplied: root.isThemeSddmActive(root.activeDetailTheme)
                                            property bool isBusy: root.pendingAction === "apply"
                                            property bool needsApply: !isApplied || root.hasUnsavedChanges
                                            color: (!needsApply && isApplied)
                                                   ? Qt.rgba(16/255, 185/255, 129/255, 0.15)
                                                   : (applyCtaMouse.containsMouse ? theme.accentHover : theme.accent)
                                            border.color: (!needsApply && isApplied) ? "#10B981" : "transparent"
                                            border.width: (!needsApply && isApplied) ? 1 : 0

                                            RowLayout {
                                                anchors.centerIn: parent
                                                spacing: 8
                                                RyzoraIcon {
                                                    name: (!applyCtaBtn.needsApply && applyCtaBtn.isApplied) ? "check" : "shield"
                                                    size: 14
                                                    color: (!applyCtaBtn.needsApply && applyCtaBtn.isApplied) ? "#10B981" : "#FFFFFF"
                                                }
                                                Text {
                                                    text: applyCtaBtn.isBusy ? "Applying…" : (root.hasUnsavedChanges ? "Save & Apply Customization" : (applyCtaBtn.isApplied ? "Applied to SDDM" : "Apply to Login Screen"))
                                                    font.pixelSize: 12
                                                    font.weight: Font.SemiBold
                                                    color: (!applyCtaBtn.needsApply && applyCtaBtn.isApplied) ? "#10B981" : "#FFFFFF"
                                                }
                                            }

                                            MouseArea {
                                                id: applyCtaMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                enabled: !applyCtaBtn.isBusy && applyCtaBtn.needsApply
                                                onClicked: root.triggerApply()
                                            }
                                        }

                                        // Deactivate Button (Visible if applied)
                                        Rectangle {
                                            visible: root.isThemeSddmActive(root.activeDetailTheme)
                                            Layout.fillWidth: true
                                            height: 32
                                            radius: theme.radiusSm
                                            color: deactMouse.containsMouse ? theme.surfaceHover : "transparent"
                                            border.color: theme.borderSubtle
                                            border.width: 1

                                            RowLayout {
                                                anchors.centerIn: parent
                                                spacing: 6
                                                RyzoraIcon { name: "x"; size: 12; color: "#EF4444" }
                                                Text {
                                                    text: root.pendingAction === "deactivate" ? "Deactivating…" : "Deactivate (Restore Default)"
                                                    font.pixelSize: 11
                                                    font.weight: Font.Medium
                                                    color: "#EF4444"
                                                }
                                            }

                                            MouseArea {
                                                id: deactMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                enabled: root.pendingAction !== "deactivate"
                                                onClicked: root.triggerDeactivate()
                                            }
                                        }

                                        // Fullscreen Test Button
                                        Rectangle {
                                            Layout.fillWidth: true
                                            height: 32
                                            radius: theme.radiusSm
                                            color: fsBtnMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            border.width: 1

                                            RowLayout {
                                                anchors.centerIn: parent
                                                spacing: 6
                                                RyzoraIcon { name: "maximize-2"; size: 12; color: theme.textSecondary }
                                                Text {
                                                    text: "Test Fullscreen Preview"
                                                    font.pixelSize: 11
                                                    font.weight: Font.Medium
                                                    color: theme.text
                                                }
                                            }

                                            MouseArea {
                                                id: fsBtnMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: root.triggerPreview()
                                            }
                                        }

                                        // Save / Reset block for settings changes
                                        RowLayout {
                                            visible: root.hasUnsavedChanges || Boolean(root.themeConfig && root.themeConfig.is_modified)
                                            Layout.fillWidth: true
                                            spacing: 8

                                            Rectangle {
                                                Layout.fillWidth: true
                                                height: 30
                                                radius: theme.radiusSm
                                                color: saveSideMouse.containsMouse ? theme.accentHover : theme.accent
                                                Text {
                                                    anchors.centerIn: parent
                                                    text: "Save Settings"
                                                    font.pixelSize: 11
                                                    font.bold: true
                                                    color: "#FFFFFF"
                                                }
                                                MouseArea {
                                                    id: saveSideMouse
                                                    anchors.fill: parent
                                                    cursorShape: Qt.PointingHandCursor
                                                    onClicked: root.triggerSave()
                                                }
                                            }

                                            Rectangle {
                                                Layout.fillWidth: true
                                                height: 30
                                                radius: theme.radiusSm
                                                color: resetSideMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                                border.color: theme.borderSubtle
                                                border.width: 1
                                                Text {
                                                    anchors.centerIn: parent
                                                    text: "Reset"
                                                    font.pixelSize: 11
                                                    color: theme.textSecondary
                                                }
                                                MouseArea {
                                                    id: resetSideMouse
                                                    anchors.fill: parent
                                                    cursorShape: Qt.PointingHandCursor
                                                    onClicked: root.triggerReset()
                                                }
                                            }
                                        }
                                    }

                                    Item { Layout.fillHeight: true }
                                }
                            }
                        }

                        // 3. Responsive Grouped Settings Cards
                        // 3A. Wide Layout (width >= 860): 2 Columns
                        RowLayout {
                            visible: Boolean(root.themeConfig && root.themeConfig.has_settings === true && studioScroll.width >= 860)
                            Layout.fillWidth: true
                            spacing: 16
                            Layout.alignment: Qt.AlignTop

                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.preferredWidth: 1
                                Layout.alignment: Qt.AlignTop
                                spacing: 16

                                SettingsGroupCard {
                                    title: "Background"
                                    iconName: "image"
                                    settingsList: root.getCategorySettings("Background")
                                }

                                SettingsGroupCard {
                                    title: "Login Panel"
                                    iconName: "user"
                                    settingsList: root.getCategorySettings("Login Panel")
                                }

                                SettingsGroupCard {
                                    title: "Appearance"
                                    iconName: "sliders"
                                    settingsList: root.getCategorySettings("Appearance")
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.preferredWidth: 1
                                Layout.alignment: Qt.AlignTop
                                spacing: 16

                                SettingsGroupCard {
                                    title: "Clock & Date"
                                    iconName: "clock"
                                    settingsList: root.getCategorySettings("Clock & Date")
                                }

                                SettingsGroupCard {
                                    title: "Unlock Hint"
                                    iconName: "lock"
                                    settingsList: root.getCategorySettings("Unlock Hint")
                                }

                                SettingsGroupCard {
                                    title: "Animation"
                                    iconName: "play"
                                    settingsList: root.getCategorySettings("Animation")
                                }
                            }
                        }

                        // 3B. Narrow Layout (width < 860): Stacked 1 Column
                        ColumnLayout {
                            visible: Boolean(root.themeConfig && root.themeConfig.has_settings === true && studioScroll.width < 860)
                            Layout.fillWidth: true
                            spacing: 16

                            SettingsGroupCard {
                                title: "Background"
                                iconName: "image"
                                settingsList: root.getCategorySettings("Background")
                            }

                            SettingsGroupCard {
                                title: "Login Panel"
                                iconName: "user"
                                settingsList: root.getCategorySettings("Login Panel")
                            }

                            SettingsGroupCard {
                                title: "Clock & Date"
                                iconName: "clock"
                                settingsList: root.getCategorySettings("Clock & Date")
                            }

                            SettingsGroupCard {
                                title: "Unlock Hint"
                                iconName: "lock"
                                settingsList: root.getCategorySettings("Unlock Hint")
                            }

                            SettingsGroupCard {
                                title: "Appearance"
                                iconName: "sliders"
                                settingsList: root.getCategorySettings("Appearance")
                            }

                            SettingsGroupCard {
                                title: "Animation"
                                iconName: "play"
                                settingsList: root.getCategorySettings("Animation")
                            }
                        }

                        // 4. Informational card when theme has no configurable parameters
                        Rectangle {
                            visible: Boolean(root.activeDetailTheme && root.themeConfig && root.themeConfig.has_settings === false && !root.themeConfig.error)
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
                                RyzoraIcon { name: "sliders"; size: 14; color: theme.textMuted }
                                Text {
                                    text: "Default Configuration: This theme operates with integrated styling and does not require additional configuration parameters."
                                    font.pixelSize: 11
                                    color: theme.textSecondary
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                            }
                        }

                        // 5. System Integration Safety Note
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
                // SUBPAGE 3: FASTFETCH STUDIO
                // ──────────────────────────────────────────────────────────────
                ColumnLayout {
                    visible: root.currentSubpage === "fastfetch"
                    Layout.fillWidth: true
                    spacing: 18

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12
                        Rectangle {
                            width: 32; height: 32; radius: 16
                            Layout.alignment: Qt.AlignTop
                            color: ffBackM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                            border.color: ffBackM.containsMouse ? theme.borderStrong : theme.borderSubtle
                            border.width: 1
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "chevron-left"
                                size: 14
                                color: ffBackM.containsMouse ? theme.accent : theme.text
                            }
                            MouseArea {
                                id: ffBackM
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.goBackInStudio()
                            }
                        }
                        ColumnLayout {
                            spacing: 4
                            Layout.fillWidth: true
                            Text { text: "—  PERSONALIZATION"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; font.letterSpacing: 1.5 }
                            Text { text: "Fastfetch Studio"; font.pixelSize: 26; font.bold: true; color: theme.text }
                            Text { text: "Terminal layout and modules configuration."; font.pixelSize: 12; color: theme.textMuted }
                        }
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

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12
                        Rectangle {
                            width: 32; height: 32; radius: 16
                            Layout.alignment: Qt.AlignTop
                            color: wpBackM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                            border.color: wpBackM.containsMouse ? theme.borderStrong : theme.borderSubtle
                            border.width: 1
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "chevron-left"
                                size: 14
                                color: wpBackM.containsMouse ? theme.accent : theme.text
                            }
                            MouseArea {
                                id: wpBackM
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.goBackInStudio()
                            }
                        }
                        ColumnLayout {
                            spacing: 4
                            Layout.fillWidth: true
                            Text { text: "—  PERSONALIZATION"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; font.letterSpacing: 1.5 }
                            Text { text: "Wallpaper Studio"; font.pixelSize: 26; font.bold: true; color: theme.text }
                            Text { text: "Backgrounds and collections."; font.pixelSize: 12; color: theme.textMuted }
                        }
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

                        Image {
                            id: wpCanvasImg
                            anchors.fill: parent
                            source: {
                                if (!root.selectedWallpaper) return ""
                                var u = String(root.selectedWallpaper.preview_url || "").trim()
                                if (u.length === 0) return ""
                                if (u.indexOf("file://") === 0) return u
                                if (u.indexOf("http://") === 0 || u.indexOf("https://") === 0) return u
                                if (u.indexOf("/") === 0) return "file://" + u
                                return ""
                            }
                            fillMode: root.wallpaperFit === "fit" ? Image.PreserveAspectFit : (root.wallpaperFit === "stretch" ? Image.Stretch : Image.PreserveAspectCrop)
                            visible: status === Image.Ready
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

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12
                        Rectangle {
                            width: 32; height: 32; radius: 16
                            Layout.alignment: Qt.AlignTop
                            color: apBackM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                            border.color: apBackM.containsMouse ? theme.borderStrong : theme.borderSubtle
                            border.width: 1
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "chevron-left"
                                size: 14
                                color: apBackM.containsMouse ? theme.accent : theme.text
                            }
                            MouseArea {
                                id: apBackM
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.goBackInStudio()
                            }
                        }
                        ColumnLayout {
                            spacing: 4
                            Layout.fillWidth: true
                            Text { text: "—  PERSONALIZATION"; font.pixelSize: 11; font.bold: true; color: theme.textMuted; font.letterSpacing: 1.5 }
                            Text { text: "Appearance Studio"; font.pixelSize: 26; font.bold: true; color: theme.text }
                            Text { text: "Colors and visual preferences."; font.pixelSize: 12; color: theme.textMuted }
                        }
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
    component StudioTabButton : Rectangle {
        id: tabBtn
        property string iconName: "sliders"
        property string label: ""
        property bool isActive: false
        signal clicked()

        height: 32
        implicitWidth: tabRow.implicitWidth + 24
        radius: 6
        color: isActive
            ? (theme.resolvedTheme === "light" ? theme.surfaceElevated : Qt.rgba(255, 255, 255, 0.10))
            : (tabMouse.containsMouse ? (theme.resolvedTheme === "light" ? "#F1F5F9" : Qt.rgba(255, 255, 255, 0.05)) : "transparent")
        border.color: isActive ? theme.borderSubtle : "transparent"
        border.width: isActive ? 1 : 0

        RowLayout {
            id: tabRow
            anchors.centerIn: parent
            spacing: 6

            RyzoraIcon {
                name: tabBtn.iconName
                size: 13
                color: tabBtn.isActive ? theme.accent : (tabMouse.containsMouse ? theme.text : theme.textSecondary)
            }

            Text {
                text: tabBtn.label
                font.pixelSize: 12
                font.weight: tabBtn.isActive ? Font.SemiBold : Font.Normal
                color: tabBtn.isActive ? theme.text : (tabMouse.containsMouse ? theme.text : theme.textSecondary)
            }
        }

        MouseArea {
            id: tabMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: tabBtn.clicked()
        }
    }

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
                font.pixelSize: 11
                font.weight: Font.Medium
                font.letterSpacing: 0.2
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
    component SettingsGroupCard : Rectangle {
        id: sgc
        property string title: ""
        property string iconName: "sliders"
        property var settingsList: []

        visible: sgc.settingsList && sgc.settingsList.length > 0
        Layout.fillWidth: true
        radius: theme.radiusMd
        color: theme.surface
        border.color: theme.borderSubtle
        border.width: 1
        implicitHeight: groupCol.implicitHeight + 32

        ColumnLayout {
            id: groupCol
            anchors.fill: parent
            anchors.margins: 16
            spacing: 14

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                RyzoraIcon { name: sgc.iconName; size: 13; color: theme.accent }
                Text {
                    text: sgc.title
                    font.pixelSize: 12
                    font.bold: true
                    font.letterSpacing: 0.5
                    color: theme.text
                }
                Item { Layout.fillWidth: true }
                Rectangle {
                    height: 18
                    width: cntTxt.implicitWidth + 10
                    radius: 4
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1
                    Text {
                        id: cntTxt
                        anchors.centerIn: parent
                        text: sgc.settingsList ? (sgc.settingsList.length + " settings") : ""
                        font.pixelSize: 9
                        font.bold: true
                        color: theme.textMuted
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            Repeater {
                model: sgc.settingsList
                delegate: ColumnLayout {
                    id: settingCol
                    property var settingDef: modelData
                    Layout.fillWidth: true
                    spacing: 12

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 14

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3
                            Text {
                                text: settingCol.settingDef.label || ""
                                font.pixelSize: 13
                                font.bold: true
                                color: theme.text
                            }
                            Text {
                                text: settingCol.settingDef.help || ""
                                font.pixelSize: 11
                                color: theme.textMuted
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        // 1. Boolean Toggle Switch
                        Item {
                            visible: settingCol.settingDef.type === "bool"
                            width: 44
                            height: 24
                            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter

                            property bool checked: root.isTruthy(root.draftConfig[settingCol.settingDef.id])

                            Rectangle {
                                anchors.fill: parent
                                radius: 12
                                color: parent.checked ? (theme.resolvedTheme === "light" ? "#10B981" : "#059669") : (theme.resolvedTheme === "light" ? "#D1D5DB" : "#334155")
                                border.color: parent.checked ? "#10B981" : (swMouse.containsMouse ? theme.borderStrong : theme.borderSubtle)
                                border.width: 1

                                Behavior on color { ColorAnimation { duration: 150 } }

                                Rectangle {
                                    width: 18
                                    height: 18
                                    radius: 9
                                    y: 3
                                    x: parent.parent.checked ? 23 : 3
                                    color: "#FFFFFF"

                                    Behavior on x { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }
                                }
                            }

                            MouseArea {
                                id: swMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.updateDraft(settingCol.settingDef.id, !parent.checked)
                            }
                        }

                        // 2. Blur Slider with Numeric Readout
                        RowLayout {
                            visible: settingCol.settingDef.id === "blur"
                            spacing: 8
                            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter

                            Slider {
                                id: bSlider
                                implicitWidth: 120
                                from: settingCol.settingDef.min !== undefined ? settingCol.settingDef.min : 0
                                to: settingCol.settingDef.max !== undefined ? settingCol.settingDef.max : 100
                                stepSize: 1
                                value: parseInt(root.draftConfig[settingCol.settingDef.id] !== undefined ? root.draftConfig[settingCol.settingDef.id] : (settingCol.settingDef.default_value || 0))
                                onMoved: root.updateDraft(settingCol.settingDef.id, Math.round(value))
                            }

                            Rectangle {
                                height: 26
                                width: Math.max(46, bValTxt.implicitWidth + 12)
                                radius: 4
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1

                                Text {
                                    id: bValTxt
                                    anchors.centerIn: parent
                                    text: Math.round(bSlider.value) + " px"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: theme.accent
                                }
                            }
                        }

                        // 3. Clock Position 3x3 Pad
                        RowLayout {
                            visible: settingCol.settingDef.id === "clockPosition"
                            spacing: 8
                            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter

                            GridLayout {
                                columns: 3
                                rowSpacing: 3
                                columnSpacing: 3

                                Repeater {
                                    model: [
                                        { id: "top-left", tip: "Top Left" },
                                        { id: "top-center", tip: "Top Center" },
                                        { id: "top-right", tip: "Top Right" },
                                        { id: "center-left", tip: "Center Left" },
                                        { id: "center", tip: "Center" },
                                        { id: "center-right", tip: "Center Right" },
                                        { id: "bottom-left", tip: "Bottom Left" },
                                        { id: "bottom-center", tip: "Bottom Center" },
                                        { id: "bottom-right", tip: "Bottom Right" }
                                    ]
                                    delegate: Rectangle {
                                        property bool isCur: String(root.draftConfig[settingCol.settingDef.id]) === modelData.id
                                        width: 22
                                        height: 22
                                        radius: 4
                                        color: isCur ? theme.accent : (padMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                                        border.color: isCur ? theme.accent : theme.borderSubtle
                                        border.width: 1

                                        Rectangle {
                                            anchors.centerIn: parent
                                            width: isCur ? 7 : 4
                                            height: isCur ? 7 : 4
                                            radius: isCur ? 3.5 : 2
                                            color: parent.isCur ? (theme.resolvedTheme === "light" ? "#FFFFFF" : "#000000") : theme.textMuted
                                        }

                                        MouseArea {
                                            id: padMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: root.updateDraft(settingCol.settingDef.id, modelData.id)
                                        }
                                    }
                                }
                            }

                            Rectangle {
                                height: 26
                                width: Math.max(50, posValTxt.implicitWidth + 14)
                                radius: 4
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1

                                Text {
                                    id: posValTxt
                                    anchors.centerIn: parent
                                    text: {
                                        var p = String(root.draftConfig[settingCol.settingDef.id] || "top-center")
                                        return p.replace("-", " ").toUpperCase()
                                    }
                                    font.pixelSize: 10
                                    font.bold: true
                                    color: theme.text
                                }
                            }
                        }

                        // 4. Segmented Control for Other Enums
                        Rectangle {
                            visible: settingCol.settingDef.type === "enum" && settingCol.settingDef.id !== "clockPosition"
                            height: 30
                            radius: theme.radiusSm
                            color: theme.surfaceElevated
                            border.color: theme.borderSubtle
                            border.width: 1
                            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                            implicitWidth: segRow.implicitWidth + 6

                            RowLayout {
                                id: segRow
                                anchors.centerIn: parent
                                spacing: 2

                                Repeater {
                                    model: settingCol.settingDef.options || []
                                    delegate: Rectangle {
                                        id: segItem
                                        property bool isCur: String(root.draftConfig[settingCol.settingDef.id]) === String(modelData)
                                        height: 24
                                        radius: 4
                                        implicitWidth: Math.max(42, segTxt.implicitWidth + 12)
                                        color: isCur ? (theme.resolvedTheme === "light" ? "#FFFFFF" : theme.accent) : (segM.containsMouse ? theme.surfaceHover : "transparent")
                                        border.color: isCur ? (theme.resolvedTheme === "light" ? theme.border : theme.accent) : "transparent"
                                        border.width: isCur ? 1 : 0

                                        Text {
                                            id: segTxt
                                            anchors.centerIn: parent
                                            text: {
                                                var str = String(modelData)
                                                if (str === "Press any key to unlock") return "Full Prompt"
                                                if (str === "Press any key") return "Short Prompt"
                                                if (str === "Welcome") return "Welcome"
                                                if (str === "dddd, MMMM dd, yyyy") return "Full Date"
                                                if (str === "dddd, MMMM d") return "Day & Month"
                                                if (str === "MM/dd/yyyy") return "MM/DD/YY"
                                                if (str === "dd/MM/yyyy") return "DD/MM/YY"
                                                if (str === "hh:mm") return "24h"
                                                if (str === "hh:mm:ss") return "24h + Sec"
                                                if (str === "hh:mm AP") return "12h"
                                                if (str === "hh:mm:ss AP") return "12h + Sec"
                                                return str.charAt(0).toUpperCase() + str.slice(1)
                                            }
                                            font.pixelSize: 11
                                            font.bold: segItem.isCur
                                            color: segItem.isCur ? (theme.resolvedTheme === "light" ? theme.text : "#FFFFFF") : theme.textMuted
                                        }

                                        MouseArea {
                                            id: segM
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: root.updateDraft(settingCol.settingDef.id, String(modelData))
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        visible: index < sgc.settingsList.length - 1
                        Layout.fillWidth: true
                        height: 1
                        color: Qt.rgba(theme.borderSubtle.r, theme.borderSubtle.g, theme.borderSubtle.b, 0.4)
                    }
                }
            }
        }
    }

}
