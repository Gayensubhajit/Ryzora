import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia

Rectangle {
    id: card
    radius: 10
    clip: true
    implicitWidth: 280
    implicitHeight: 240
    color: mouseArea.containsMouse ? theme.surfaceHover : theme.surface
    border.color: mouseArea.containsMouse ? theme.accent : theme.borderSubtle
    border.width: 1

    property var itemData: null

    readonly property bool isLockscreen: {
        if (!card.itemData) return false
        var cat = String(card.itemData.category || "").toLowerCase()
        var pt = String(card.itemData.package_type || "").toLowerCase()
        var sf = String(card.itemData.subfilter || "").toLowerCase()
        var id = String(card.itemData.id || "")
        return cat === "lockscreens" || cat === "lockscreen" || cat === "sddm" ||
               pt === "lockscreens" || pt === "lockscreen" || pt === "sddm" ||
               sf === "silentsddm" || id.indexOf("lockscreen-") === 0 || id.indexOf("silentsddm-") === 0
    }

    readonly property string videoSource: {
        if (!card.itemData) return ""
        var vu = card.itemData.video_url ? String(card.itemData.video_url).trim() : ""
        if (vu.length === 0 && card.itemData.preview_video) {
            vu = String(card.itemData.preview_video).trim()
        }
        if (vu.length === 0 && card.itemData.id && typeof backend !== "undefined" && backend) {
            var lists = [backend.catalogList || [], backend.libraryList || []]
            for (var li = 0; li < lists.length && vu.length === 0; ++li) {
                for (var ci = 0; ci < lists[li].length; ++ci) {
                    if (lists[li][ci].id === card.itemData.id) {
                        if (lists[li][ci].video_url) { vu = String(lists[li][ci].video_url).trim(); break }
                        if (lists[li][ci].preview_video) { vu = String(lists[li][ci].preview_video).trim(); break }
                    }
                }
            }
        }
        if (vu.length === 0 && card.itemData.preview_url) {
            var pu = String(card.itemData.preview_url).trim()
            if (pu.indexOf(".mp4") >= 0 || pu.indexOf(".webm") >= 0 || pu.indexOf(".mov") >= 0) {
                vu = pu
            }
        }
        if (vu.length === 0) return ""
        if (vu.indexOf("file://") === 0) {
            if (vu.indexOf("file:///") !== 0 && vu.indexOf("file://localhost/") !== 0) {
                return "file:///" + vu.substring(7)
            }
            return vu
        }
        if (vu.indexOf("http://") === 0 || vu.indexOf("https://") === 0) return vu
        if (vu.indexOf("/") === 0) return "file://" + vu
        return "file://" + backend.resolveHomePath() + "/" + vu
    }
    readonly property bool hasVideo: videoSource !== ""
    readonly property bool isLocalVideo: videoSource.indexOf("file://") === 0
    property string actionLabel: ""
    property bool hideActionButton: false
    property bool hideCategoryBadge: false

    property bool inViewport: true
    property bool allowAutoplay: true
    property Item scrollContainer: null
    property bool isPageVisible: true
    property bool playVideoActive: false
    readonly property bool isHovered: mouseArea.containsMouse

    property string currentItemId: ""
    readonly property int staggerMs: {
        var s = currentItemId || (card.itemData && card.itemData.id ? String(card.itemData.id) : "")
        var h = 0
        for (var i = 0; i < s.length; ++i) {
            h = (h * 31 + s.charCodeAt(i)) & 0x7fffffff
        }
        return 45 + (h % 9) * 35
    }

    onItemDataChanged: {
        var newId = (card.itemData && card.itemData.id) ? String(card.itemData.id) : ""
        if (newId !== currentItemId) {
            currentItemId = newId
            if (videoLoader.item) {
                videoLoader.item.hasFirstFrame = false
            }
            playDebounceTimer.stop()
            card.playVideoActive = false
            card.releaseSlot()
            if (card.shouldPlayVideo) {
                playDebounceTimer.interval = card.staggerMs
                playDebounceTimer.restart()
            }
        }
    }

    function findScrollContainer() {
        var p = card.parent
        while (p) {
            if (p.interactive === false) {
                p = p.parent
                continue
            }
            if (p.contentY !== undefined) {
                return p
            }
            if (p.contentItem && p.contentItem.contentY !== undefined) {
                return p.contentItem
            }
            p = p.parent
        }
        return null
    }

    onParentChanged: {
        card.scrollContainer = card.findScrollContainer()
    }

    onIsPageVisibleChanged: {
        if (isPageVisible && shouldPlayVideo) {
            playDebounceTimer.interval = card.staggerMs
            playDebounceTimer.restart()
        } else if (!isPageVisible) {
            playDebounceTimer.stop()
            card.playVideoActive = false
            card.releaseSlot()
        }
    }

    Component.onCompleted: {
        card.scrollContainer = card.findScrollContainer()
        if (card.shouldPlayVideo) {
            playDebounceTimer.interval = card.staggerMs
            playDebounceTimer.restart()
        }
    }

    function checkViewport() {
        if (!card.visible || !card.isPageVisible) return false
        var sc = card.scrollContainer || card.findScrollContainer()
        if (sc && sc.height > 0) {
            var cy = sc.contentY
            if (cy <= 0) {
                var ptTop = card.mapToItem(sc, 0, 0)
                if (ptTop.y <= sc.height + 250) return true
            }
            var pt = card.mapToItem(sc, 0, 0)
            var cardH = card.height > 0 ? card.height : 220
            return (pt.y + cardH >= -250) && (pt.y <= sc.height + 250)
        }
        return true
    }

    // Direct reactive viewport detection: autoplays immediately on page load and continuously while in view
    readonly property bool isInViewport: {
        var _ = card.y + card.height + (card.parent ? card.parent.y : 0) + (card.scrollContainer ? card.scrollContainer.contentY + card.scrollContainer.height : 0)
        return checkViewport()
    }

    readonly property bool shouldPlayVideo: isPageVisible && isInViewport && hasVideo && videoSource !== "" && allowAutoplay

    property bool hasDecoderSlot: false

    function tryAcquireSlot() {
        if (hasDecoderSlot) return true
        if (backend && backend.acquireDecoderSlot) {
            if (backend.acquireDecoderSlot()) {
                hasDecoderSlot = true
                return true
            }
            return false
        }
        hasDecoderSlot = true
        return true
    }

    Connections {
        target: backend
        function onDecoderSlotAvailable() {
            if (card.shouldPlayVideo && !card.playVideoActive && !card.hasDecoderSlot) {
                playDebounceTimer.interval = card.staggerMs
                playDebounceTimer.restart()
            }
        }
    }

    function releaseSlot() {
        if (hasDecoderSlot) {
            hasDecoderSlot = false
            if (backend && backend.releaseDecoderSlot) {
                backend.releaseDecoderSlot()
            }
        }
    }

    Component.onDestruction: releaseSlot()

    Timer {
        id: playDebounceTimer
        interval: 80
        repeat: false
        onTriggered: {
            if (card.shouldPlayVideo && !card.playVideoActive) {
                if (card.tryAcquireSlot()) {
                    card.playVideoActive = true
                }
            }
        }
    }

    onShouldPlayVideoChanged: {
        if (shouldPlayVideo) {
            stopDebounceTimer.stop()
            if (!card.playVideoActive) {
                playDebounceTimer.interval = card.staggerMs
                playDebounceTimer.restart()
            }
        } else {
            playDebounceTimer.stop()
            stopDebounceTimer.restart()
        }
    }

    // Delayed teardown: avoids destroy/recreate flicker when visibility/viewport briefly toggles
    Timer {
        id: stopDebounceTimer
        interval: 700
        repeat: false
        onTriggered: {
            if (!card.shouldPlayVideo) {
                card.playVideoActive = false
                card.releaseSlot()
            }
        }
    }

    // Hover does not trigger or interrupt continuous autoplay

    signal cardClicked(var itemData)
    signal actionClicked(string action, var itemData)

    Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }
    Behavior on border.color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }

    Keys.onReturnPressed: if (card.itemData) card.cardClicked(card.itemData)
    Keys.onSpacePressed: if (card.itemData) card.cardClicked(card.itemData)

    MouseArea {
        id: mouseArea
        objectName: "storeCardMouseArea"
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        z: 1
        onClicked: {
            if (card.itemData) {
                console.log("[QML StoreCard] Card clicked: " + card.itemData.id);
                card.cardClicked(card.itemData);
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── 1. Image Preview Section (Dominant Landscape Area) ──
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            // Clean Fallback Graphic when artwork is loading or unavailable
            Rectangle {
                anchors.fill: parent
                visible: previewImg.status !== Image.Ready
                color: theme.resolvedTheme === "light" ? "#F0F2F5" : "#1A1D23"

                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 6

                    RyzoraIcon {
                        Layout.alignment: Qt.AlignHCenter
                        name: {
                            if (!card.itemData || !card.itemData.category) return "package"
                            var cat = String(card.itemData.category).toLowerCase()
                            if (cat === "lockscreens" || cat === "lockscreen") return "lock"
                            if (cat === "fastfetch") return "terminal"
                            if (cat === "themes" || cat === "theme") return "palette"
                            if (cat === "wallpapers" || cat === "wallpaper") return "image"
                            if (cat === "rices" || cat === "rice") return "sliders"
                            return "package"
                        }
                        size: 24
                        color: theme.textMuted
                        opacity: previewImg.status === Image.Loading ? 0.40 : 0.70
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: {
                            if (previewImg.status === Image.Loading && Boolean(card.itemData && card.itemData.preview_url)) return "LOADING…"
                            return card.itemData ? (card.itemData.category ? String(card.itemData.category).toUpperCase() : "PACKAGE") : "PACKAGE"
                        }
                        font.pixelSize: 10
                        font.letterSpacing: 1.0
                        font.bold: true
                        color: theme.textMuted
                    }
                }
            }

            Timer {
                id: loadingWatchdog
                interval: 3500
                running: previewImg.status === Image.Loading
                repeat: false
                onTriggered: {
                    if (previewImg.status === Image.Loading) {
                        previewImg.source = ""
                    }
                }
            }

            // Real Artwork Image Preview
            Image {
                id: previewImg
                anchors.fill: parent
                source: {
                    if (!card.itemData || !card.itemData.preview_url) return ""
                    var u = String(card.itemData.preview_url).trim()
                    if (!u || u === "file://" || u === "file:///" || u === "http://" || u === "https://") return ""
                    if (u.indexOf("file:///") === 0) return u
                    if (u.indexOf("file://") === 0) {
                        var rest = u.substring(7).trim()
                        if (!rest || rest === "/") return ""
                        if (rest.indexOf("/") === 0) return "file://" + rest
                        return ""
                    }
                    if (u.indexOf("http://") === 0) return u.length > 7 ? u : ""
                    if (u.indexOf("https://") === 0) return u.length > 8 ? u : ""
                    if (u.indexOf("/") === 0) return "file://" + u
                    return ""
                }
                fillMode: (card.itemData && (card.itemData.subfilter === "emblem" || card.itemData.category === "fastfetch")) ? Image.PreserveAspectFit : Image.PreserveAspectCrop
                asynchronous: true
                cache: true
                sourceSize.width: 384
                sourceSize.height: 240
                smooth: true
                visible: status === Image.Ready && (!videoLoader.item || !videoLoader.item.hasFirstFrame)
            }

            // Recorded Demo / Cursor Tutorial Video Preview with Bounded Concurrency
            Loader {
                id: videoLoader
                anchors.fill: parent
                active: card.playVideoActive
                sourceComponent: demoVideoComp
            }

            Component {
                id: demoVideoComp
                Item {
                    id: videoRoot
                    anchors.fill: parent
                    property bool hasFirstFrame: false
                    readonly property bool isPlaying: hasFirstFrame || (cardPlayer.playbackState === MediaPlayer.PlayingState)

                    VideoOutput {
                        id: cardVideoOutput
                        anchors.fill: parent
                        fillMode: VideoOutput.PreserveAspectCrop
                        visible: videoRoot.hasFirstFrame
                    }

                    MediaPlayer {
                        id: cardPlayer
                        videoOutput: cardVideoOutput
                        audioOutput: AudioOutput {
                            muted: true
                            volume: 0.0
                        }
                        loops: MediaPlayer.Infinite
                        source: card.videoSource
                        Component.onCompleted: play()
                        onPositionChanged: function() {
                            if (!videoRoot.hasFirstFrame && cardPlayer.position > 0) {
                                videoRoot.hasFirstFrame = true
                            }
                        }
                        onMediaStatusChanged: function() {
                            if (cardPlayer.mediaStatus === MediaPlayer.LoadedMedia || cardPlayer.mediaStatus === MediaPlayer.BufferedMedia) {
                                if (cardPlayer.playbackState !== MediaPlayer.PlayingState) cardPlayer.play()
                            }
                        }
                    }
                }
            }

            // Video Indicator Badge Pill (Top-Right)
            Rectangle {
                visible: card.hasVideo && Boolean(videoLoader.item && videoLoader.item.hasFirstFrame)
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 8
                height: 18
                width: liveRow.implicitWidth + 12
                radius: 4
                color: Qt.rgba(0, 0, 0, 0.65)
                border.color: Qt.rgba(96, 165, 250, 0.40)
                border.width: 1

                RowLayout {
                    id: liveRow
                    anchors.centerIn: parent
                    spacing: 4
                    RyzoraIcon {
                        name: "play"
                        size: 8
                        color: "#60a5fa"
                    }
                    Text {
                        text: "VIDEO"
                        font.pixelSize: 8
                        font.bold: true
                        font.letterSpacing: 0.5
                        color: "#93c5fd"
                    }
                }
            }

            // Category Badge Pill (Top-Left)
            Rectangle {
                visible: !card.hideCategoryBadge && Boolean(card.itemData && card.itemData.category)
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.margins: 8
                height: 18
                width: catBadgeText.implicitWidth + 12
                radius: 4
                color: Qt.rgba(0, 0, 0, 0.55)
                border.color: Qt.rgba(255, 255, 255, 0.12)
                border.width: 1

                Text {
                    id: catBadgeText
                    anchors.centerIn: parent
                    text: {
                        if (!card.itemData || !card.itemData.category) return "PACKAGE"
                        return String(card.itemData.category).toUpperCase()
                    }
                    font.pixelSize: 9
                    font.letterSpacing: 0.5
                    font.bold: true
                    color: "#FFFFFF"
                }
            }

            // Subtle divider line separating image from metadata
            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 1
                color: theme.borderSubtle
            }


        }

        // ── 2. Minimal Metadata & Contextual Action Section ──
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 58
            Layout.minimumHeight: 58
            Layout.maximumHeight: 58
            color: "transparent"

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                anchors.topMargin: 8
                anchors.bottomMargin: 8
                spacing: 8

                // Title & Subtitle Info (Fills remaining width)
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: card.itemData ? String(card.itemData.name) : ""
                        font.pixelSize: 13
                        font.bold: true
                        color: mouseArea.containsMouse ? theme.accent : theme.text
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Text {
                        text: {
                            if (!card.itemData) return ""
                            var cat = card.itemData.category ? (String(card.itemData.category).charAt(0).toUpperCase() + String(card.itemData.category).slice(1)) : "Customization"
                            var author = card.itemData.author ? (typeof card.itemData.author === "object" ? (card.itemData.author.name || "Community") : String(card.itemData.author)) : "Community"
                            return cat + " · " + author
                        }
                        font.pixelSize: 11
                        color: theme.textSecondary
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                // PureDesk / macOS Contextual Action Pill Beneath Preview
                Rectangle {
                    visible: !card.hideActionButton && (card.actionLabel !== "NONE")
                    property bool isSddm: Boolean(card.itemData && card.itemData.active_targets && card.itemData.active_targets.sddm_login)
                    property bool isLock: Boolean(card.itemData && card.itemData.active_targets && card.itemData.active_targets.session_lock)
                    property bool isAct: Boolean(card.itemData && (card.itemData.is_active || isSddm || isLock))
                    height: 24
                    width: cardActionText.implicitWidth + 18
                    radius: 12
                    color: {
                        if (!card.itemData) return "transparent"
                        if (isSddm) return theme.resolvedTheme === "light" ? "#DCFCE7" : "#064E3B"
                        if (isLock) return theme.resolvedTheme === "light" ? "#EDE9FE" : Qt.rgba(139/255, 92/255, 246/255, 0.25)
                        if (isAct) return theme.resolvedTheme === "light" ? "#DCFCE7" : "#064E3B"
                        if (card.itemData.is_installed) return theme.resolvedTheme === "light" ? "#EFF6FF" : "#1E3A8A"
                        return pillMouse.containsMouse ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#2A2F3A") : (theme.resolvedTheme === "light" ? "#F3F4F6" : "#22262E")
                    }
                    border.color: {
                        if (!card.itemData) return "transparent"
                        if (isSddm) return "#10B981"
                        if (isLock) return "#8B5CF6"
                        if (isAct) return "#10B981"
                        if (card.itemData.is_installed) return "#3B82F6"
                        return pillMouse.containsMouse ? theme.accent : theme.borderSubtle
                    }
                    border.width: 1

                    Text {
                        id: cardActionText
                        anchors.centerIn: parent
                        text: {
                            if (!card.itemData) return ""
                            if (card.actionLabel !== "") return card.actionLabel
                            if (parent.isSddm) return "ACTIVE SDDM"
                            if (parent.isLock) return "ACTIVE LOCK"
                            if (parent.isAct) return "ACTIVE"
                            if (card.itemData.is_installed) return "OPEN"
                            return "GET"
                        }
                        font.pixelSize: 10
                        font.bold: true
                        color: {
                            if (!card.itemData) return theme.textMuted
                            if (parent.isSddm) return theme.resolvedTheme === "light" ? "#047857" : "#34D399"
                            if (parent.isLock) return theme.resolvedTheme === "light" ? "#6D28D9" : "#a78bfa"
                            if (parent.isAct) return theme.resolvedTheme === "light" ? "#047857" : "#34D399"
                            if (card.itemData.is_installed) return theme.resolvedTheme === "light" ? "#1D4ED8" : "#93C5FD"
                            return theme.accent
                        }
                    }

                    MouseArea {
                        id: pillMouse
                        objectName: "storeCardPillMouse"
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        z: 2
                        onClicked: {
                            if (!card.itemData) return;
                            if (!card.itemData.is_installed) {
                                card.actionClicked("install", card.itemData);
                            } else {
                                card.cardClicked(card.itemData);
                            }
                        }
                    }
                }
            }
        }
    }
}
