import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia
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

    // Lockscreen themes use the shared LoginPreview (its own autoplaying player); this view's
    // legacy player stays idle for them so two decoders never fight over one source.
    readonly property bool useLoginPreview: !!root.packageData && root.isLockscreen() && root.supportsSddm()
    readonly property string videoSource: {
        if (!root.packageData || root.useLoginPreview) return ""
        var vu = root.packageData.video_url ? String(root.packageData.video_url).trim() : ""
        if (vu.length === 0) return ""
        if (vu.indexOf("file://") === 0) {
            if (vu.indexOf("file:///") !== 0 && vu.indexOf("file://localhost/") !== 0) {
                return "file:///" + vu.substring(7)
            }
            return vu
        }
        if (vu.indexOf("http://") === 0 || vu.indexOf("https://") === 0) return vu
        if (vu.indexOf("/") === 0) return "file://" + vu
        return ""
    }
    readonly property bool hasVideo: videoSource !== ""
    property bool hasFirstFrame: false
    readonly property bool isNarrow: detailScroll.width < 880
    property bool isEngineInstalled: (typeof backend !== "undefined" && backend) ? backend.isSilentEngineInstalled : false
    property bool engineActionPending: false
    readonly property bool isVideoPlaying: root.hasVideo && (root.hasFirstFrame || videoPlayer.playbackState === MediaPlayer.PlayingState) && videoPlayer.error === MediaPlayer.NoError

    MediaPlayer {
        id: videoPlayer
        videoOutput: videoOutput
        audioOutput: AudioOutput {
            muted: true
            volume: 0.0
        }
        loops: MediaPlayer.Infinite
        source: root.videoSource
        onErrorOccurred: function(error, errorString) {
            console.warn("VideoPlayer error [" + error + "]: " + errorString)
        }
        onPlaybackStateChanged: {
            if (playbackState === MediaPlayer.PlayingState) {
                root.hasFirstFrame = true
            }
        }
        onSourceChanged: {
            root.hasFirstFrame = false
            if (source !== "") {
                play()
            } else {
                stop()
            }
        }
        Component.onDestruction: stop()
    }

    onPackageDataChanged: {
        root.hasFirstFrame = false
        root.syncThemeConfig()
        root.errorMessage = ""
        root.pendingAction = ""
        root.engineActionPending = false
        if (typeof backend !== "undefined" && backend) {
            root.isEngineInstalled = backend.isSilentEngineInstalled
        }
        if (root.hasVideo && root.videoSource !== "") {
            videoPlayer.source = root.videoSource
            videoPlayer.play()
        } else {
            videoPlayer.stop()
            videoPlayer.source = ""
        }
        if (detailScroll) {
            detailScroll.ScrollBar.vertical.position = 0
        }
    }

    onVisibleChanged: {
        if (visible) {
            if (root.hasVideo && root.videoSource !== "" && videoPlayer.playbackState !== MediaPlayer.PlayingState) {
                videoPlayer.play()
            }
        } else {
            if (videoPlayer.playbackState === MediaPlayer.PlayingState) {
                videoPlayer.stop()
            }
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

    function applyCurrentPackage() {
        if (root.packageData && root.pendingAction === "") {
            if (root.isSilentSddm() && !root.isEngineInstalled) {
                root.errorMessage = "SilentSDDM base theme engine is not installed at /usr/share/sddm/themes/ryzora-silent. Please click 'Install SilentSDDM Base Theme' above before applying."
                return
            }
            root.errorMessage = ""; root.pendingAction = "apply"
            root.actionRequested("apply", root.packageData, "sddm_login")
        }
    }

    function installBaseEngine() {
        root.engineActionPending = true
        root.errorMessage = ""
        backend.installSilentEngine()
    }

    property var themeConfig: ({})
    property var draftConfig: ({})
    property string customSuccess: ""
    property string customError: ""

    function syncThemeConfig() {
        if (root.packageData) {
            root.themeConfig = backend.getThemeConfig(root.packageData.id)
            var draft = {}
            if (root.themeConfig && root.themeConfig.settings) {
                for (var i = 0; i < root.themeConfig.settings.length; ++i) {
                    var s = root.themeConfig.settings[i]
                    draft[s.id] = s.current_value
                }
            }
            root.draftConfig = draft
        } else {
            root.themeConfig = ({})
            root.draftConfig = ({})
        }
        root.customSuccess = ""
        root.customError = ""
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
            root.packageData = Object.assign({}, updated)
        }
        root.syncThemeConfig()
    }

    Connections {
        target: backend
        function onSilentEngineInstalledChanged() {
            if (backend) {
                root.isEngineInstalled = backend.isSilentEngineInstalled
            }
        }
        function onSilentEngineInstalled(success, message) {
            root.engineActionPending = false
            if (backend) {
                root.isEngineInstalled = backend.isSilentEngineInstalled
            }
            if (!success) {
                root.errorMessage = message || "SilentSDDM base theme engine installation failed."
            } else {
                root.errorMessage = ""
            }
            root.syncPackageData()
        }
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
        if (root.isLockscreen()) {
            return Boolean(root.packageData.active_targets && root.packageData.active_targets.sddm_login)
        }
        return Boolean(root.packageData.is_active)
    }

    function isLockscreen() {
        if (!root.packageData) return false
        var cat = String(root.packageData.category || "").toLowerCase()
        var pt = String(root.packageData.package_type || "").toLowerCase()
        var id = String(root.packageData.id || "")
        return cat === "lockscreens" || cat === "lockscreen" || cat === "sddm" ||
               pt === "lockscreens" || pt === "lockscreen" || pt === "sddm" ||
               id.indexOf("lockscreen-") === 0 || id.indexOf("silentsddm-") === 0 || id.indexOf("custom:") === 0
    }

    function isFastfetch() {
        if (!root.packageData) return false
        var cat = String(root.packageData.category || "").toLowerCase()
        var pt = String(root.packageData.package_type || "").toLowerCase()
        var id = String(root.packageData.id || "")
        return cat === "fastfetch" || pt === "fastfetch" || id.indexOf("fastfetch-") === 0
    }

    function supportsSddm() {
        if (!root.packageData) return false
        if (!root.isLockscreen()) return false
        if (root.packageData.supports_sddm !== undefined) return Boolean(root.packageData.supports_sddm)
        var id = String(root.packageData.id || "")
        if (id.indexOf("swaylock") !== -1 || id.indexOf("hyprlock") !== -1) return false
        return true
    }

    function isSilentSddm() {
        if (!root.packageData) return false
        var sf = String(root.packageData.subfilter || "")
        var id = String(root.packageData.id || "")
        return sf === "silentsddm" || id.indexOf("silentsddm-") === 0
    }

    function isConfigurableLockscreen() {
        if (!root.isLockscreen()) return false
        return true
    }

    function getStudioSubpage() {
        if (!root.packageData) return "overview"
        var cat = String(root.packageData.category || "").toLowerCase()
        var pt = String(root.packageData.package_type || "").toLowerCase()
        var sf = String(root.packageData.subfilter || "").toLowerCase()
        var id = String(root.packageData.id || "")
        var isSilent = sf === "silentsddm" || id.indexOf("silentsddm-") === 0
        if (isSilent || cat === "lockscreens" || cat === "lockscreen" || cat === "sddm" || pt === "lockscreen" || pt === "lockscreens" || root.packageData.supports_sddm) {
            return "lockscreen"
        }
        if (cat === "fastfetch" || pt === "fastfetch") {
            return "fastfetch"
        }
        if (cat === "wallpapers" || cat === "wallpaper" || pt === "wallpaper" || cat === "rices" || cat === "themes") {
            return "wallpapers"
        }
        return "overview"
    }

    function getStudioButtonLabel() {
        var sp = root.getStudioSubpage()
        if (sp === "lockscreen") return "Open in Lockscreen Studio"
        if (sp === "fastfetch") return "Open in Fastfetch Studio"
        if (sp === "wallpapers") return "Open in Wallpaper Studio"
        return "Open in Studio"
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

        // ── 1. Top Navigation Header (macOS Clean Breadcrumb Bar) ──
        Rectangle {
            id: navHeader
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
                        onClicked: {
                            if (videoPlayer.playbackState === MediaPlayer.PlayingState) {
                                videoPlayer.stop()
                            }
                            root.backRequested()
                        }
                    }
                }

                // Vertical Divider
                Rectangle {
                    width: 1
                    height: 18
                    color: theme.borderSubtle
                }

                // Breadcrumb Path (Discover > Category > Item Name)
                RowLayout {
                    spacing: 8
                    Layout.fillWidth: true

                    MouseArea {
                        implicitWidth: discTxt.implicitWidth
                        implicitHeight: discTxt.implicitHeight
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (videoPlayer.playbackState === MediaPlayer.PlayingState) {
                                videoPlayer.stop()
                            }
                            root.backRequested()
                        }
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
                        Layout.maximumWidth: 320
                    }
                }
            }
        }

        // ── 2. Scrollable Body Content (Full Available Viewport Width) ──
        Flickable {
            id: detailScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: detailScroll.width
            contentHeight: detailScrollContent.implicitHeight + 40
            clip: true
            ScrollBar.horizontal: ScrollBar { policy: ScrollBar.AlwaysOff }
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            Item {
                id: detailScrollContent
                width: detailScroll.width
                implicitHeight: detailMainCol.implicitHeight + 40

                ColumnLayout {
                    id: detailMainCol
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Math.max(24, Math.min(32, (detailScroll.width - 1440) / 2))
                    anchors.rightMargin: Math.max(24, Math.min(32, (detailScroll.width - 1440) / 2))
                    y: 20
                    spacing: 28

                    // ── Hero Section (2-Column Desktop Grid or 1-Column Responsive) ──
                    GridLayout {
                        Layout.fillWidth: true
                        columns: root.isNarrow ? 1 : 2
                        columnSpacing: 28
                        rowSpacing: 24

                        // ── LEFT COLUMN: Media Preview & Art Showcase (~58% width) ──
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.preferredWidth: root.isNarrow ? detailMainCol.width : Math.floor(detailMainCol.width * 0.58)
                            Layout.minimumWidth: root.isNarrow ? 260 : 420
                            Layout.alignment: Qt.AlignTop
                            spacing: 12

                            Rectangle {
                                id: heroPreviewBox
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.min(520, Math.max(280, Math.round(width * 9 / 16)))
                                radius: theme.radiusLg
                                color: theme.surfaceElevated
                                border.color: theme.borderSubtle
                                border.width: 1
                                clip: true

                                LoginPreview {
                                    id: detailLoginPreview
                                    anchors.fill: parent
                                    z: 5
                                    visible: root.useLoginPreview
                                    radius: 0
                                    border.width: 0
                                    packageData: root.packageData
                                    config: root.draftConfig
                                    active: root.visible && root.useLoginPreview
                                }

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

                                // Poster Image / Static Preview
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
                                    visible: !root.useLoginPreview && status === Image.Ready && (!root.hasVideo || !root.hasFirstFrame || videoPlayer.error !== MediaPlayer.NoError)
                                }

                                // Qt Multimedia Live Video Output
                                VideoOutput {
                                    id: videoOutput
                                    anchors.fill: parent
                                    fillMode: VideoOutput.PreserveAspectCrop
                                    visible: root.hasVideo && (root.hasFirstFrame || videoPlayer.playbackState === MediaPlayer.PlayingState || videoPlayer.playbackState === MediaPlayer.PausedState)
                                }

                                // Playback Control Overlay Badge
                                Rectangle {
                                    id: playPauseOverlay
                                    visible: root.hasVideo
                                    anchors.bottom: parent.bottom
                                    anchors.left: parent.left
                                    anchors.margins: 12
                                    height: 28
                                    implicitWidth: playPauseRow.implicitWidth + 18
                                    radius: 14
                                    color: playPauseMouse.containsMouse
                                        ? (theme.resolvedTheme === "light" ? "rgba(255,255,255,0.92)" : "rgba(15,23,42,0.85)")
                                        : (theme.resolvedTheme === "light" ? "rgba(255,255,255,0.78)" : "rgba(15,23,42,0.70)")
                                    border.color: theme.resolvedTheme === "light" ? "rgba(0,0,0,0.12)" : "rgba(255,255,255,0.18)"
                                    border.width: 1

                                    RowLayout {
                                        id: playPauseRow
                                        anchors.centerIn: parent
                                        spacing: 6

                                        RyzoraIcon {
                                            name: root.isVideoPlaying ? "pause" : "play"
                                            size: 11
                                            color: theme.text
                                        }
                                        Text {
                                            text: root.isVideoPlaying ? "Live Video Preview" : (videoPlayer.playbackState === MediaPlayer.PausedState ? "Paused" : "Play Video")
                                            font.pixelSize: 11
                                            font.bold: true
                                            color: theme.text
                                        }
                                    }

                                    MouseArea {
                                        id: playPauseMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            if (videoPlayer.playbackState === MediaPlayer.PlayingState) {
                                                videoPlayer.pause()
                                            } else {
                                                videoPlayer.play()
                                            }
                                        }
                                    }
                                }

                                // Fallback State when media is loading or missing
                                ColumnLayout {
                                    anchors.centerIn: parent
                                    spacing: 8
                                    visible: !root.useLoginPreview && mainHeroImg.status !== Image.Ready && !root.isVideoPlaying

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
                                            if (root.hasVideo) return "1080p Live Video Wallpaper"
                                            if (root.isSilentSddm()) return "SilentSDDM Background Asset"
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

                                // Dynamic Preview Status Indicator
                                RowLayout {
                                    spacing: 6
                                    Rectangle {
                                        width: 6
                                        height: 6
                                        radius: 3
                                        color: {
                                            if (root.isVideoPlaying) return "#10B981"
                                            if (root.hasVideo && videoPlayer.error !== MediaPlayer.NoError) return "#EF4444"
                                            if (mainHeroImg.status === Image.Ready) return "#10B981"
                                            if (mainHeroImg.status === Image.Loading || (root.hasVideo && videoPlayer.playbackState === MediaPlayer.PlayingState)) return "#F59E0B"
                                            return "#64748B"
                                        }
                                    }
                                    Text {
                                        text: {
                                            if (root.isVideoPlaying) return "Live Video Preview (Playing)"
                                            if (root.hasVideo && videoPlayer.error !== MediaPlayer.NoError) return "Video Unavailable (Poster Fallback)"
                                            if (root.hasVideo && videoPlayer.playbackState === MediaPlayer.PausedState) return "Video Paused"
                                            if (root.hasVideo && mainHeroImg.status === Image.Ready) return "Video Poster Ready"
                                            if (mainHeroImg.status === Image.Ready) return "Preview Ready"
                                            if (mainHeroImg.status === Image.Loading) return "Loading Preview…"
                                            return "Preview Unavailable"
                                        }
                                        font.pixelSize: 11
                                        color: theme.textMuted
                                    }
                                }
                            }
                        }

                        // ── RIGHT COLUMN: Information, Actions & Technical Specs (~42% width) ──
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.preferredWidth: root.isNarrow ? detailMainCol.width : Math.floor(detailMainCol.width * 0.42)
                            Layout.minimumWidth: root.isNarrow ? 260 : 340
                            Layout.maximumWidth: root.isNarrow ? 9999 : 560
                            Layout.alignment: Qt.AlignTop
                            spacing: 16

                            // Title & Single Authoritative Status Badge Row
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 6

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 12

                                    Text {
                                        text: root.packageData ? String(root.packageData.name) : "Package Details"
                                        font.pixelSize: 22
                                        font.bold: true
                                        color: theme.text
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }

                                    // Single Authoritative Status Badge (Beside Package Title)
                                    Rectangle {
                                        id: authStatusPill
                                        objectName: "authStatusPill"
                                        visible: root.packageData !== null
                                        height: 26
                                        implicitWidth: authStatRow.implicitWidth + 18
                                        radius: 13
                                        color: {
                                            if (!root.packageData) return "transparent"
                                            if (root.isAnyActive()) return theme.resolvedTheme === "light" ? "#ECFDF5" : "#064E3B"
                                            if (root.isInstalled()) {
                                                if (root.isSilentSddm() && !root.isEngineInstalled) {
                                                    return theme.resolvedTheme === "light" ? "#FEF3C7" : "#451A03"
                                                }
                                                return theme.resolvedTheme === "light" ? "#EFF6FF" : "#1E293B"
                                            }
                                            return theme.resolvedTheme === "light" ? "#F9FAFB" : "#242830"
                                        }
                                        border.color: {
                                            if (!root.packageData) return "transparent"
                                            if (root.isAnyActive()) return theme.resolvedTheme === "light" ? "#A7F3D0" : "#059669"
                                            if (root.isInstalled()) {
                                                if (root.isSilentSddm() && !root.isEngineInstalled) {
                                                    return theme.resolvedTheme === "light" ? "#FCD34D" : "#B45309"
                                                }
                                                return theme.resolvedTheme === "light" ? "#BFDBFE" : "#3B82F6"
                                            }
                                            return theme.resolvedTheme === "light" ? "#E5E7EB" : "#3B4252"
                                        }
                                        border.width: 1

                                        RowLayout {
                                            id: authStatRow
                                            anchors.centerIn: parent
                                            spacing: 5

                                            Rectangle {
                                                width: 6
                                                height: 6
                                                radius: 3
                                                color: {
                                                    if (!root.packageData) return "#9CA3AF"
                                                    if (root.isAnyActive()) return theme.resolvedTheme === "light" ? "#059669" : "#34D399"
                                                    if (root.isInstalled()) {
                                                        if (root.isSilentSddm() && !root.isEngineInstalled) {
                                                            return theme.resolvedTheme === "light" ? "#D97706" : "#FBBF24"
                                                        }
                                                        return theme.resolvedTheme === "light" ? "#2563EB" : "#60A5FA"
                                                    }
                                                    return theme.textMuted
                                                }
                                            }

                                            Text {
                                                text: {
                                                    if (!root.packageData) return ""
                                                    if (root.isAnyActive()) {
                                                        var cat = String(root.packageData.category || "").toLowerCase()
                                                        return (cat === "lockscreens" || cat === "sddm") ? "Active on Login Screen" : "Active on System"
                                                    }
                                                    if (root.isInstalled()) {
                                                        if (root.isSilentSddm() && !root.isEngineInstalled) {
                                                            return "Installed (Engine Required)"
                                                        }
                                                        return "Installed"
                                                    }
                                                    return "Available"
                                                }
                                                font.pixelSize: 11
                                                font.bold: true
                                                color: {
                                                    if (!root.packageData) return theme.textMuted
                                                    if (root.isAnyActive()) return theme.resolvedTheme === "light" ? "#047857" : "#6EE7B7"
                                                    if (root.isInstalled()) {
                                                        if (root.isSilentSddm() && !root.isEngineInstalled) {
                                                            return theme.resolvedTheme === "light" ? "#92400E" : "#FDE68A"
                                                        }
                                                        return theme.resolvedTheme === "light" ? "#1D4ED8" : "#93C5FD"
                                                    }
                                                    return theme.textMuted
                                                }
                                            }
                                        }
                                    }
                                }

                                // Secondary Line: Version · Category · Author
                                Text {
                                    text: {
                                        if (!root.packageData) return ""
                                        var parts = []
                                        if (root.packageData.version) parts.push("v" + root.packageData.version)
                                        var isSilent = root.isSilentSddm()
                                        var cat = isSilent ? "SDDM Wallpaper" : (root.packageData.category ? (root.packageData.category.charAt(0).toUpperCase() + root.packageData.category.slice(1)) : "Theme")
                                        if (cat === "Lockscreens") cat = "Lock Screen"
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
                            }

                            // ── ACTION PANEL ──
                            Rectangle {
                                id: actionBox
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

                                    // ── PREREQUISITE CARD: SILENTSDDM BASE THEME ENGINE REQUIRED ──
                                    Rectangle {
                                        id: prerequisiteEngineCard
                                        objectName: "prerequisiteEngineCard"
                                        visible: Boolean(root.packageData && root.isInstalled() && !root.isAnyActive() && root.isSilentSddm() && !root.isEngineInstalled)
                                        Layout.fillWidth: true
                                        radius: theme.radiusSm
                                        color: theme.resolvedTheme === "light" ? "#FFFBEB" : "rgba(180, 83, 9, 0.16)"
                                        border.color: theme.resolvedTheme === "light" ? "#FCD34D" : "rgba(245, 158, 11, 0.40)"
                                        border.width: 1
                                        implicitHeight: prereqCol.implicitHeight + 20

                                        ColumnLayout {
                                            id: prereqCol
                                            anchors.fill: parent
                                            anchors.margins: 10
                                            spacing: 8

                                            RowLayout {
                                                spacing: 8
                                                Layout.fillWidth: true
                                                RyzoraIcon {
                                                    name: "alert"
                                                    size: 14
                                                    color: "#F59E0B"
                                                }
                                                Text {
                                                    text: "Base Engine Required"
                                                    font.pixelSize: 12
                                                    font.bold: true
                                                    color: theme.resolvedTheme === "light" ? "#B45309" : "#FBBF24"
                                                }
                                            }

                                            Text {
                                                Layout.fillWidth: true
                                                text: "SilentSDDM wallpapers run inside the 'ryzora-silent' base theme. The engine must be installed to /usr/share/sddm/themes/ryzora-silent before applying."
                                                font.pixelSize: 11
                                                color: theme.resolvedTheme === "light" ? "#92400E" : "#CBD5E1"
                                                wrapMode: Text.WordWrap
                                            }

                                            Rectangle {
                                                id: installEngineBtn
                                                objectName: "installEngineButton"
                                                Layout.fillWidth: true
                                                height: 32
                                                radius: theme.radiusSm
                                                color: instEngMouse.pressed
                                                    ? "#B45309"
                                                    : (instEngMouse.containsMouse ? "#D97706" : (theme.resolvedTheme === "light" ? "#D97706" : "#B45309"))

                                                RowLayout {
                                                    anchors.centerIn: parent
                                                    spacing: 6
                                                    RyzoraIcon {
                                                        name: root.engineActionPending ? "refresh" : "download"
                                                        size: 12
                                                        color: "#FFFFFF"
                                                    }
                                                    Text {
                                                        text: root.engineActionPending ? "Installing Base Engine…" : "Install SilentSDDM Base Theme"
                                                        font.pixelSize: 11
                                                        font.bold: true
                                                        color: "#FFFFFF"
                                                    }
                                                }

                                                MouseArea {
                                                    id: instEngMouse
                                                    objectName: "installEngineMouseArea"
                                                    anchors.fill: parent
                                                    hoverEnabled: !root.engineActionPending
                                                    enabled: !root.engineActionPending
                                                    cursorShape: !root.engineActionPending ? Qt.PointingHandCursor : Qt.WaitCursor
                                                    onClicked: root.installBaseEngine()
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
                                        color: {
                                            var needEngine = root.isSilentSddm() && !root.isEngineInstalled
                                            if (needEngine) {
                                                return primApplyMouse.containsMouse
                                                    ? (theme.resolvedTheme === "light" ? "#F3F4F6" : "#334155")
                                                    : (theme.resolvedTheme === "light" ? "#F9FAFB" : "#1E293B")
                                            }
                                            return primApplyMouse.pressed
                                                ? (theme.resolvedTheme === "light" ? "#0051A8" : "#005BB5")
                                                : (primApplyMouse.containsMouse
                                                    ? (theme.resolvedTheme === "light" ? "#0062CC" : "#0071E3")
                                                    : (theme.resolvedTheme === "light" ? "#007AFF" : "#0A84FF"))
                                        }
                                        border.color: {
                                            var needEngine = root.isSilentSddm() && !root.isEngineInstalled
                                            if (needEngine) {
                                                return theme.resolvedTheme === "light" ? "#D1D5DB" : "#475569"
                                            }
                                            return theme.resolvedTheme === "light" ? "#005BB5" : "#0A84FF"
                                        }
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: root.pendingAction === "apply" ? "refresh" : "desktop"
                                                size: 13
                                                color: (root.isSilentSddm() && !root.isEngineInstalled) ? theme.textMuted : "#FFFFFF"
                                            }
                                            Text {
                                                text: {
                                                    if (root.pendingAction === "apply") return "Applying…"
                                                    if (root.isSilentSddm() && !root.isEngineInstalled) return "Apply to Login Screen (Engine Required)"
                                                    return "Apply to Login Screen"
                                                }
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: (root.isSilentSddm() && !root.isEngineInstalled) ? theme.textMuted : "#FFFFFF"
                                            }
                                        }

                                        MouseArea {
                                            id: primApplyMouse
                                            objectName: "primaryApplyMouseArea"
                                            anchors.fill: parent
                                            hoverEnabled: root.pendingAction === ""
                                            enabled: root.pendingAction === ""
                                            cursorShape: root.pendingAction === "" ? Qt.PointingHandCursor : Qt.WaitCursor
                                            onClicked: root.applyCurrentPackage()
                                        }
                                    }

                                    // ── STATE 2B: ACTIVE LOCKSCREEN -> ACTIVE BADGE & RESTORE BUTTON ──
                                    Rectangle {
                                        id: primDeactBtn
                                        objectName: "primaryDeactivateButton"
                                        visible: Boolean(root.packageData && root.isInstalled() && root.isAnyActive() && root.supportsSddm())
                                        Layout.fillWidth: true
                                        height: 38
                                        radius: theme.radiusSm
                                        color: theme.resolvedTheme === "light" ? "#ECFDF5" : "#064E3B"
                                        border.color: primDeactMouse.containsMouse
                                            ? (theme.resolvedTheme === "light" ? "#FCA5A5" : "#EF4444")
                                            : (theme.resolvedTheme === "light" ? "#A7F3D0" : "#059669")
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: primDeactMouse.containsMouse ? "x" : (root.pendingAction === "deactivate" ? "refresh" : "check")
                                                size: 13
                                                color: primDeactMouse.containsMouse
                                                    ? (theme.resolvedTheme === "light" ? "#DC2626" : "#F87171")
                                                    : (theme.resolvedTheme === "light" ? "#047857" : "#34D399")
                                            }
                                            Text {
                                                text: root.pendingAction === "deactivate"
                                                    ? "Restoring Default Greeter…"
                                                    : (primDeactMouse.containsMouse ? "Deactivate (Restore Default SDDM)" : "Active on Login Screen")
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: primDeactMouse.containsMouse
                                                    ? (theme.resolvedTheme === "light" ? "#DC2626" : "#F87171")
                                                    : (theme.resolvedTheme === "light" ? "#047857" : "#34D399")
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

                                    // ── STATE 2C: PREVIEW THEME BUTTON ──
                                    Rectangle {
                                        id: testBtn
                                        objectName: "testButton"
                                        visible: Boolean(root.packageData && root.isInstalled() && !root.isFastfetch())
                                        Layout.fillWidth: true
                                        height: 38
                                        radius: theme.radiusSm
                                        color: testBtnMouse.pressed
                                            ? (theme.resolvedTheme === "light" ? "#D1D5DB" : "#404856")
                                            : (testBtnMouse.containsMouse
                                                ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#363C47")
                                                : (theme.resolvedTheme === "light" ? "#F3F4F6" : "#242830"))
                                        border.color: testBtnMouse.containsMouse
                                            ? (theme.resolvedTheme === "light" ? "#9CA3AF" : "#60697B")
                                            : (theme.resolvedTheme === "light" ? "#D1D5DB" : "#3E4450")
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: root.pendingAction === "test" ? "refresh" : "play"
                                                size: 12
                                                color: theme.text
                                            }
                                            Text {
                                                text: root.pendingAction === "test" ? "Opening Preview…" : "Preview Theme"
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: theme.text
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

                                    // ── STATE 2D: OPEN IN STUDIO (For Installed Packages with Studio Support, excluding Fastfetch which uses openFastfetchButton) ──
                                    Rectangle {
                                        id: openStudioBtn
                                        objectName: "openStudioButton"
                                        visible: Boolean(root.packageData && root.isInstalled() && !root.isFastfetch() && root.getStudioSubpage() !== "overview")
                                        Layout.fillWidth: true
                                        height: 38
                                        radius: theme.radiusSm
                                        color: openStudioMouse.pressed
                                            ? (theme.resolvedTheme === "light" ? "#D1D5DB" : "#404856")
                                            : (openStudioMouse.containsMouse
                                                ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#363C47")
                                                : (theme.resolvedTheme === "light" ? "#F3F4F6" : "#242830"))
                                        border.color: openStudioMouse.containsMouse
                                            ? (theme.resolvedTheme === "light" ? "#9CA3AF" : "#60697B")
                                            : (theme.resolvedTheme === "light" ? "#D1D5DB" : "#3E4450")
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 8
                                            RyzoraIcon {
                                                name: "sliders"
                                                size: 13
                                                color: theme.text
                                            }
                                            Text {
                                                text: root.getStudioButtonLabel()
                                                font.pixelSize: 13
                                                font.bold: true
                                                color: theme.text
                                            }
                                        }

                                        MouseArea {
                                            id: openStudioMouse
                                            objectName: "openStudioMouseArea"
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                if (root.packageData) {
                                                    if (videoPlayer.playbackState === MediaPlayer.PlayingState) {
                                                        videoPlayer.stop()
                                                    }
                                                    root.actionRequested("open_studio", root.packageData, null)
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

                                    // Subtle Divider before Destructive Actions
                                    Rectangle {
                                        visible: Boolean(root.packageData && root.isInstalled())
                                        Layout.fillWidth: true
                                        height: 1
                                        color: theme.borderSubtle
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
                                                    size: 12
                                                    color: theme.resolvedTheme === "light" ? "#DC2626" : "#F87171"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // ── Theme Customization Card (Matches Tauri Lock Screen Config Panel) ──
                            Rectangle {
                                id: detailThemeConfigCard
                                visible: Boolean(root.packageData && root.isInstalled() && (root.isLockscreen() || root.isSilentSddm()) && root.themeConfig && root.themeConfig.has_settings === true)
                                Layout.fillWidth: true
                                implicitHeight: detailThemeCol.implicitHeight + 36
                                radius: theme.radiusMd
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1

                                ColumnLayout {
                                    id: detailThemeCol
                                    anchors.fill: parent
                                    anchors.margins: 18
                                    spacing: 14

                                    RowLayout {
                                        spacing: 8
                                        RyzoraIcon { name: "sliders"; size: 14; color: theme.accent }
                                        Text {
                                            text: "LOGIN SCREEN CUSTOMIZATION"
                                            font.pixelSize: 11
                                            font.bold: true
                                            font.letterSpacing: 1.0
                                            color: theme.accent
                                        }
                                        Item { Layout.fillWidth: true }
                                        Text {
                                            visible: root.customSuccess !== ""
                                            text: root.customSuccess
                                            font.pixelSize: 11
                                            color: "#34d399"
                                        }
                                        Text {
                                            visible: root.customError !== ""
                                            text: root.customError
                                            font.pixelSize: 11
                                            color: "#f87171"
                                        }
                                    }

                                    Text {
                                        text: "Customize visual parameters and layout directly for this login screen. Changes apply immediately to live SDDM and preview."
                                        font.pixelSize: 12
                                        color: theme.textSecondary
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }

                                    // Dynamic Settings Controls
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 12

                                        Repeater {
                                            model: (root.themeConfig && root.themeConfig.settings) ? root.themeConfig.settings : []
                                            delegate: RowLayout {
                                                Layout.fillWidth: true
                                                spacing: 14

                                                ColumnLayout {
                                                    Layout.fillWidth: true
                                                    spacing: 2
                                                    RowLayout {
                                                        spacing: 6
                                                        Text {
                                                            text: modelData.label
                                                            font.pixelSize: 12
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
                                                            height: 28
                                                            width: Math.max(56, optTxt.implicitWidth + 16)
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
                                                        height: 28
                                                        width: 64
                                                        radius: theme.radiusSm
                                                        color: isCur ? theme.accent : (bTrueM.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                                                        border.color: isCur ? theme.accent : theme.borderSubtle
                                                        border.width: 1
                                                        Text {
                                                            anchors.centerIn: parent
                                                            text: "Enabled"
                                                            font.pixelSize: 10
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
                                                        height: 28
                                                        width: 64
                                                        radius: theme.radiusSm
                                                        color: isCur ? theme.accent : (bFalseM.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                                                        border.color: isCur ? theme.accent : theme.borderSubtle
                                                        border.width: 1
                                                        Text {
                                                            anchors.centerIn: parent
                                                            text: "Disabled"
                                                            font.pixelSize: 10
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
                                                    spacing: 8

                                                    RowLayout {
                                                        visible: (modelData.max !== undefined && modelData.min !== undefined && (modelData.max - modelData.min <= 10))
                                                        spacing: 4
                                                        Repeater {
                                                            model: (modelData.max !== undefined && modelData.min !== undefined && (modelData.max - modelData.min <= 10)) ? (modelData.max - modelData.min + 1) : 0
                                                            delegate: Rectangle {
                                                                property int val: modelData.min + index
                                                                property bool isCur: parseInt(root.draftConfig[modelData.id]) === val
                                                                height: 28
                                                                width: 32
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

                                                    RowLayout {
                                                        visible: (modelData.max !== undefined && modelData.min !== undefined && (modelData.max - modelData.min > 10))
                                                        spacing: 10
                                                        Slider {
                                                            id: intSlider
                                                            from: modelData.min || 0
                                                            to: modelData.max || 100
                                                            stepSize: 1
                                                            value: parseInt(root.draftConfig[modelData.id] !== undefined ? root.draftConfig[modelData.id] : (modelData.default_value || modelData.min || 0))
                                                            onMoved: {
                                                                var d = Object.assign({}, root.draftConfig)
                                                                d[modelData.id] = Math.round(value)
                                                                root.draftConfig = d
                                                            }
                                                        }
                                                        Rectangle {
                                                            height: 26
                                                            width: Math.max(34, valTxt.implicitWidth + 12)
                                                            radius: 4
                                                            color: theme.surfaceElevated
                                                            border.color: theme.borderSubtle
                                                            border.width: 1
                                                            Text {
                                                                id: valTxt
                                                                anchors.centerIn: parent
                                                                text: String(Math.round(intSlider.value))
                                                                font.pixelSize: 11
                                                                font.bold: true
                                                                color: theme.accent
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Save Button Row
                                    RowLayout {
                                        Layout.fillWidth: true
                                        Item { Layout.fillWidth: true }

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
                                                    if (root.packageData) {
                                                        root.customSuccess = ""
                                                        root.customError = ""
                                                        var res = backend.saveThemeConfig(root.packageData.id, root.draftConfig)
                                                        if (res.success) {
                                                            root.customSuccess = "Customization saved!"
                                                            root.syncThemeConfig()
                                                        } else {
                                                            root.customError = res.message || "Failed to save."
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // ── About & Information Card ──
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 8

                                Text {
                                    text: "About"
                                    font.pixelSize: 15
                                    font.bold: true
                                    color: theme.text
                                }

                                Text {
                                    text: {
                                        if (root.isSilentSddm()) {
                                            return "Minimalist SDDM login screen wallpaper background from upstream SilentSDDM integration. Seamlessly materializes into the system greeter background directory upon installation."
                                        }
                                        return (root.packageData && root.packageData.description && root.packageData.description.trim().length > 0)
                                            ? String(root.packageData.description)
                                            : "No description provided for this package."
                                    }
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
                                            if (root.isSilentSddm()) return "SDDM Wallpaper Asset"
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
                                            if (root.isSilentSddm()) return "SDDM Login Greeter Background"
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
                                        isPageVisible: root.visible && (root.parent ? root.parent.visible : true)
                                        allowAutoplay: true
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
