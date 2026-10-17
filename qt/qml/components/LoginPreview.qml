import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import QtMultimedia
import QtQuick.Effects

// Shared lockscreen preview used by Studio and the package detail page.
//  - SilentSDDM & Qylock themes: renders the theme's own Main.qml against inert greeter stubs.
//  - Everything else: poster/video background + simulated login composition driven by `config`.
// Playback is tied to `active` only (never to hover).
Rectangle {
    id: root
    property var packageData: null
    property var config: ({})
    property bool active: visible
    property bool hasUnsaved: false

    readonly property real aspect: Screen.width > 0 && Screen.height > 0 ? Math.max(1.5, Math.min(2.4, Screen.width / Screen.height)) : 16 / 9
    readonly property string pkgId: packageData && packageData.id ? String(packageData.id) : ""
    readonly property string realUrl: pkgId !== "" ? backend.themePreviewMainUrl(pkgId) : ""
    readonly property bool isReal: realUrl !== "" && realLoader.status !== Loader.Error
    readonly property string videoSource: videoFor(packageData)
    readonly property bool hasVideo: videoSource !== ""
    property bool hasFirstFrame: false
    readonly property bool videoPlaying: root.hasFirstFrame || (player.playbackState === MediaPlayer.PlayingState && player.error === MediaPlayer.NoError && player.mediaStatus !== MediaPlayer.NoMedia)
    readonly property bool isPlaying: isReal ? (realLoader.status === Loader.Ready) : videoPlaying
    property bool isLoginScreenState: false

    onPackageDataChanged: hasFirstFrame = false
    onVideoSourceChanged: hasFirstFrame = false

    function isTruthy(v) {
        if (v === true || v === 1) return true;
        if (v === false || v === 0) return false;
        if (v === undefined || v === null) return false;
        var s = String(v).trim().toLowerCase();
        return s === "true" || s === "1" || s === "yes" || s === "on";
    }
    property string modeLabel: {
        if (realUrl !== "" && realLoader.status === Loader.Error) return "Poster fallback (Engine error)"
        if (isReal && realLoader.status === Loader.Ready) return "Live theme render"
        if (hasVideo) return "Simulated login · live video"
        return "Simulated login"
    }
    property bool showBadge: true
    property bool interactive: true

    Timer {
        id: reloadDebounceTimer
        interval: 150
        repeat: false
        onTriggered: {
            if (root.active && root.realUrl !== "") {
                var u = root.realUrl
                realLoader.source = ""
                realLoader.source = u
            }
        }
    }

    onPkgIdChanged: {
        root.isLoginScreenState = false
        if (root.pkgId !== "" && root.config && typeof backend !== "undefined" && backend) {
            backend.updatePreviewDraft(root.pkgId, root.config)
        }
    }

    onConfigChanged: {
        if (root.pkgId !== "" && root.config && typeof backend !== "undefined" && backend) {
            backend.updatePreviewDraft(root.pkgId, root.config)
            reloadDebounceTimer.restart()
        }
    }

    implicitHeight: width / aspect
    color: "#05060a"
    clip: true
    border.color: "#22ffffff"
    border.width: 1

    function fileUrl(u) {
        if (u.indexOf("file://") === 0) {
            if (u.indexOf("file:///") !== 0 && u.indexOf("file://localhost/") !== 0) return "file:///" + u.substring(7)
            return u
        }
        if (u.indexOf("http://") === 0 || u.indexOf("https://") === 0) return u
        if (u.indexOf("/") === 0) return "file://" + u
        return ""
    }
    function posterFor(t) {
        if (!t) return ""
        var pid = String(t.id || "")
        if (pid.indexOf("silentsddm-") === 0) {
            var name = pid.substring(11)
            var localPosters = [
                "file://" + backend.resolveHomePath() + "/.local/share/ryzora/lockscreens/silentsddm/posters/" + pid + ".png",
                "file:///usr/share/sddm/themes/ryzora-silent/backgrounds/" + name + ".png",
                "file:///usr/share/sddm/themes/ryzora-silent/backgrounds/" + name + ".jpg"
            ]
            for (var i = 0; i < localPosters.length; ++i) {
                return localPosters[i]
            }
        }
        return fileUrl(String(t.preview_url || "").trim())
    }

    function videoFor(t) {
        if (!t) return ""
        var vu = t.video_url ? String(t.video_url).trim() : ""
        if (vu.length === 0 && t.id) {
            var pid = String(t.id)
            if (pid.indexOf("silentsddm-") === 0) {
                var name = pid.substring(11)
                vu = "file://" + backend.resolveHomePath() + "/.local/share/ryzora/lockscreens/silentsddm/wallpapers/" + name + ".mp4"
            } else {
                var lists = [backend.catalogList || [], backend.libraryList || []]
                for (var li = 0; li < lists.length && vu.length === 0; ++li)
                    for (var ci = 0; ci < lists[li].length; ++ci)
                        if (lists[li][ci].id === t.id && lists[li][ci].video_url) { vu = String(lists[li][ci].video_url).trim(); break }
            }
        }
        if (vu.length === 0 && t.preview_url) {
            var pu = String(t.preview_url).trim()
            if (pu.indexOf(".mp4") >= 0 || pu.indexOf(".webm") >= 0 || pu.indexOf(".mov") >= 0) {
                vu = pu
            }
        }
        return vu.length === 0 ? "" : fileUrl(vu)
    }

    // Background poster image: always mounted underneath to eliminate black flashes
    Image {
        id: bgPoster
        anchors.fill: parent
        source: root.packageData ? root.posterFor(root.packageData) : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: true
        z: -1
        visible: status === Image.Ready
    }

    // ── Real theme renderer (SilentSDDM & Qylock) ──
    Item {
        id: stage
        visible: root.isReal
        readonly property real baseW: 1920
        readonly property real baseH: 1080
        width: baseW; height: baseH
        scale: root.width > 0 ? (root.width / baseW) : 1.0
        x: 0
        y: Math.max(0, (root.height - (baseH * scale)) / 2)
        transformOrigin: Item.TopLeft
        clip: true

        Loader {
            id: realLoader
            anchors.fill: parent
            active: root.active && root.realUrl !== ""
            source: active ? root.realUrl : ""
            asynchronous: false
            onLoaded: {
                if (realLoader.item && realLoader.item.state !== undefined) {
                    realLoader.item.state = root.isLoginScreenState ? "loginState" : "lockState"
                }
            }
            onStatusChanged: if (status === Loader.Error) console.warn("Real theme preview failed:", root.realUrl)
        }

        // Click on preview stage to toggle between lock screen and login panel
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton
            hoverEnabled: false
            enabled: root.interactive
            z: 100
            onClicked: {
                root.isLoginScreenState = !root.isLoginScreenState
                if (realLoader.item && realLoader.item.state !== undefined) {
                    realLoader.item.state = root.isLoginScreenState ? "loginState" : "lockState"
                }
                overlay.showLoginPanel = root.isLoginScreenState
            }
        }
    }

    // ── Simulated composition fallback ──
    Item {
        id: sim
        anchors.fill: parent
        visible: !root.isReal

        Item {
            id: bgContainer
            anchors.fill: parent
            visible: false
            Image {
                id: poster
                anchors.fill: parent
                source: root.packageData ? root.posterFor(root.packageData) : ""
                fillMode: String(root.config.fillMode || "fill") === "fit" ? Image.PreserveAspectFit
                        : (String(root.config.fillMode) === "stretch" ? Image.Stretch : Image.PreserveAspectCrop)
                asynchronous: true
                cache: true
                visible: status === Image.Ready && (!root.hasVideo || !root.hasFirstFrame)
            }
            VideoOutput {
                id: vout
                anchors.fill: parent
                visible: root.hasVideo && root.hasFirstFrame
                fillMode: String(root.config.fillMode || "fill") === "fit" ? VideoOutput.PreserveAspectFit
                        : (String(root.config.fillMode) === "stretch" ? VideoOutput.Stretch : VideoOutput.PreserveAspectCrop)
            }
        }

        MultiEffect {
            source: bgContainer
            anchors.fill: bgContainer
            visible: !root.isReal
            blurEnabled: Number(root.config.blur || 0) > 0
            blur: Math.min(1.0, Math.max(0.0, Number(root.config.blur || 0) / 100.0))
            blurMax: 48
            brightness: {
                var b = String(root.config.brightness || "neutral")
                if (b === "dark") return -0.4
                if (b === "dim") return -0.2
                if (b === "bright") return 0.2
                return 0.0
            }
            saturation: {
                var s = String(root.config.saturation || "neutral")
                if (s === "grayscale") return -1.0
                if (s === "muted") return -0.5
                if (s === "vibrant") return 0.3
                return 0.0
            }
        }

        MediaPlayer {
            id: player
            videoOutput: vout
            audioOutput: AudioOutput { muted: true; volume: 0.0 }
            loops: MediaPlayer.Infinite
            source: (root.active && !root.isReal && root.hasVideo && root.isTruthy(root.config.enableAnimations !== undefined ? root.config.enableAnimations : true)) ? root.videoSource : ""
            Component.onCompleted: if (root.active && !root.isReal && source != "") play()
            onPlaybackStateChanged: {
                if (playbackState === MediaPlayer.PlayingState) {
                    root.hasFirstFrame = true
                }
            }
            onMediaStatusChanged: if (mediaStatus === MediaPlayer.LoadedMedia && root.active && !root.isReal) play()
            onSourceChanged: {
                root.hasFirstFrame = false
                if (source == "") stop()
                else if (root.active && !root.isReal) play()
            }
        }

        Item {
            id: overlay
            anchors.fill: parent
            property var cfg: root.config || ({})
            property string br: String(cfg.brightness || "neutral")
            property date now: new Date()
            property bool showLoginPanel: false
            Timer { interval: 1000; running: root.active && !root.isReal; repeat: true; onTriggered: overlay.now = new Date() }

            MouseArea {
                anchors.fill: parent
                onClicked: overlay.showLoginPanel = !overlay.showLoginPanel
            }

            Rectangle {
                anchors.fill: parent
                color: overlay.br === "bright" ? "#ffffff" : "#000000"
                opacity: overlay.br === "dark" ? 0.55 : (overlay.br === "dim" ? 0.3 : (overlay.br === "bright" ? 0.12 : 0.0))
            }
            Rectangle {
                anchors.fill: parent
                gradient: Gradient {
                    GradientStop { position: 0.0; color: "#00000000" }
                    GradientStop { position: 1.0; color: "#99000000" }
                }
            }

            // Lock Screen Mode: Clock & Date
            ColumnLayout {
                id: clockCol
                visible: !overlay.showLoginPanel && (root.isTruthy(overlay.cfg.clockDisplay !== undefined ? overlay.cfg.clockDisplay : true) || root.isTruthy(overlay.cfg.dateDisplay !== undefined ? overlay.cfg.dateDisplay : true))
                spacing: 2
                property string pos: String(overlay.cfg.clockPosition || "top-center")
                property real m: 24
                x: pos.indexOf("left") >= 0 ? m : (pos.indexOf("right") >= 0 ? parent.width - width - m : (parent.width - width) / 2)
                y: pos.indexOf("top") >= 0 ? m : (pos.indexOf("bottom") >= 0 ? parent.height - height - m : (parent.height - height) / 2)
                Text {
                    visible: root.isTruthy(overlay.cfg.clockDisplay !== undefined ? overlay.cfg.clockDisplay : true)
                    Layout.alignment: Qt.AlignHCenter
                    text: Qt.formatTime(overlay.now, String(overlay.cfg.clockFormat || "hh:mm"))
                    font.pixelSize: Math.max(28, overlay.height * 0.16)
                    font.weight: Font.Light
                    color: "#ffffff"
                    style: Text.Raised; styleColor: "#66000000"
                }
                Text {
                    visible: root.isTruthy(overlay.cfg.dateDisplay !== undefined ? overlay.cfg.dateDisplay : true)
                    Layout.alignment: Qt.AlignHCenter
                    text: Qt.formatDate(overlay.now, String(overlay.cfg.dateFormat || "dddd, MMMM dd, yyyy"))
                    font.pixelSize: Math.max(11, overlay.height * 0.045)
                    color: "#e5e7eb"
                    style: Text.Raised; styleColor: "#66000000"
                }
            }

            // Lock Screen message
            Text {
                visible: !overlay.showLoginPanel && root.isTruthy(overlay.cfg.messageDisplay !== undefined ? overlay.cfg.messageDisplay : true)
                anchors.bottom: parent.bottom
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottomMargin: 20
                text: String(overlay.cfg.messageText || "Press any key to unlock")
                font.pixelSize: Math.max(10, overlay.height * 0.038)
                color: Qt.rgba(1, 1, 1, 0.75)
                style: Text.Raised; styleColor: "#66000000"
            }

            // Login State Mode: User Avatar & Password Box
            ColumnLayout {
                id: loginCol
                visible: overlay.showLoginPanel
                spacing: 10
                property string pos: String(overlay.cfg.loginAreaPosition || "center")
                x: pos === "left" ? 36 : (pos === "right" ? parent.width - width - 36 : (parent.width - width) / 2)
                y: (parent.height - height) / 2
                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: Math.max(48, overlay.height * 0.2); Layout.preferredHeight: Layout.preferredWidth
                    radius: String(overlay.cfg.avatarShape || "circle") === "square" ? 8 : (String(overlay.cfg.avatarShape) === "rounded" ? width * 0.25 : width / 2)
                    color: Qt.rgba(1, 1, 1, 0.18)
                    border.color: Qt.rgba(1, 1, 1, 0.5); border.width: 1.5
                    RyzoraIcon { anchors.centerIn: parent; name: "user"; size: parent.width * 0.5; color: "#ffffff" }
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "silentbyte"
                    font.pixelSize: Math.max(11, overlay.height * 0.045)
                    font.bold: true
                    color: "#ffffff"
                }
                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: Math.max(130, overlay.width * 0.28); Layout.preferredHeight: Math.max(26, overlay.height * 0.09)
                    radius: height / 2
                    color: Qt.rgba(1, 1, 1, 0.16)
                    border.color: Qt.rgba(1, 1, 1, 0.35); border.width: 1
                    Text { anchors.centerIn: parent; text: "•  •  •  •  •  •"; color: "#ffffff"; font.pixelSize: 11 }
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Click anywhere to return to lock screen"
                    font.pixelSize: Math.max(9, overlay.height * 0.032)
                    color: "#9ca3af"
                }
            }
        }
    }

    // Playback failure notice
    Rectangle {
        visible: !root.isReal && root.hasHideError
        z: 4
        anchors.centerIn: parent
        radius: 8; color: "#cc000000"
        width: errTxt.implicitWidth + 24; height: errTxt.implicitHeight + 14
        Text { id: errTxt; anchors.centerIn: parent; text: "Video playback failed: " + player.errorString; color: "#fca5a5"; font.pixelSize: 11 }
    }
    readonly property bool hasHideError: hasVideo && player.error !== MediaPlayer.NoError

    // Real theme engine failure notice
    Rectangle {
        visible: root.realUrl !== "" && realLoader.status === Loader.Error
        z: 25
        anchors.centerIn: parent
        radius: 8
        color: Qt.rgba(0, 0, 0, 0.85)
        border.color: "#ef4444"
        border.width: 1
        width: realErrCol.implicitWidth + 28
        height: realErrCol.implicitHeight + 18
        ColumnLayout {
            id: realErrCol
            anchors.centerIn: parent
            spacing: 4
            Text {
                text: "Theme Engine Notice"
                font.pixelSize: 11
                font.bold: true
                color: "#f87171"
                Layout.alignment: Qt.AlignHCenter
            }
            Text {
                text: "Live engine render failed; displaying static poster fallback."
                font.pixelSize: 10
                color: "#e5e7eb"
                Layout.alignment: Qt.AlignHCenter
            }
        }
    }

    // Explicit Reversible State Toggle Button (Lock Screen / Login Panel)
    Rectangle {
        id: stateToggleBtn
        visible: root.showBadge && root.interactive
        z: 10
        anchors.top: parent.top; anchors.left: parent.left; anchors.margins: 12
        height: 24; width: toggleRow.implicitWidth + 18; radius: 12
        color: toggleMouse.containsMouse ? Qt.rgba(0, 0, 0, 0.85) : Qt.rgba(0, 0, 0, 0.65)
        border.color: toggleMouse.containsMouse ? "#60a5fa" : Qt.rgba(1, 1, 1, 0.25)
        border.width: 1

        RowLayout {
            id: toggleRow
            anchors.centerIn: parent
            spacing: 6
            RyzoraIcon {
                name: root.isLoginScreenState ? "lock" : "user"
                size: 11
                color: "#60a5fa"
            }
            Text {
                text: root.isLoginScreenState ? "Show Lock Screen" : "Show Login Panel"
                color: "#FFFFFF"
                font.pixelSize: 10
                font.bold: true
            }
        }

        MouseArea {
            id: toggleMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                root.isLoginScreenState = !root.isLoginScreenState
                if (realLoader.item && realLoader.item.state !== undefined) {
                    realLoader.item.state = root.isLoginScreenState ? "loginState" : "lockState"
                }
                overlay.showLoginPanel = root.isLoginScreenState
            }
        }
    }

    // Mode / state badge
    Rectangle {
        visible: root.showBadge
        z: 5
        anchors.top: parent.top; anchors.right: parent.right; anchors.margins: 12
        height: 24; width: badgeRow.implicitWidth + 18; radius: 12
        color: Qt.rgba(0, 0, 0, 0.65); border.color: Qt.rgba(1, 1, 1, 0.2); border.width: 1
        RowLayout {
            id: badgeRow
            anchors.centerIn: parent
            spacing: 5
            Rectangle { Layout.preferredWidth: 6; Layout.preferredHeight: 6; radius: 3
                color: root.hasUnsaved ? "#f59e0b" : "#10b981" }
            Text {
                text: (root.hasUnsaved ? "Draft · " : "") + root.modeLabel
                font.pixelSize: 10; font.bold: true; color: "#ffffff"
            }
        }
    }
}
