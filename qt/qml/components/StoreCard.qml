import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia

Rectangle {
    id: card
    radius: theme.cardRadius
    clip: true
    implicitWidth: 320
    implicitHeight: 330
    color: mouseArea.containsMouse ? theme.surfaceHover : theme.surface
    border.color: card.activeFocus ? theme.accent : (mouseArea.containsMouse ? theme.cardBorderHover : theme.cardBorder)
    border.width: card.activeFocus ? 2 : 1
    activeFocusOnTab: true

    Keys.onReturnPressed: card.cardClicked(card.itemData)
    Keys.onSpacePressed: card.cardClicked(card.itemData)

    Behavior on color { ColorAnimation { duration: 120 } }
    Behavior on border.color { ColorAnimation { duration: 120 } }

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
        return "file://" + (typeof backend !== "undefined" && backend ? backend.resolveHomePath() : "") + "/" + vu
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
                sourceSize.width: 480
                sourceSize.height: 300
                smooth: true
                scale: mouseArea.containsMouse ? 1.03 : 1.0
                Behavior on scale { NumberAnimation { duration: 180; easing.type: Easing.OutQuad } }
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

        // ── 2. Metadata & Contextual Action Section ──
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 115
            Layout.minimumHeight: 110
            color: "transparent"

            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                anchors.topMargin: 8
                anchors.bottomMargin: 10
                spacing: 3

                // Title & Author Info
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: card.itemData ? String(card.itemData.name) : ""
                        font.family: theme.fontFamily
                        font.pixelSize: 15
                        font.weight: theme.weightSemibold
                        color: mouseArea.containsMouse ? theme.accent : theme.text
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                Text {
                    text: {
                        if (!card.itemData) return ""
                        var cat = card.itemData.category ? (String(card.itemData.category).charAt(0).toUpperCase() + String(card.itemData.category).slice(1)) : "Customization"
                        var author = card.itemData.author ? (typeof card.itemData.author === "object" ? (card.itemData.author.name || "Community") : String(card.itemData.author)) : "Community"
                        return cat + " · " + author
                    }
                    font.family: theme.fontFamily
                    font.pixelSize: 12
                    font.weight: theme.weightRegular
                    color: theme.textSecondary
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                // Short Description
                Text {
                    text: {
                        if (!card.itemData) return ""
                        if (card.itemData.description) return String(card.itemData.description)
                        if (card.itemData.summary) return String(card.itemData.summary)
                        return "Community desktop customization for your Ryzora workspace."
                    }
                    font.family: theme.fontFamily
                    font.pixelSize: 11
                    font.weight: theme.weightRegular
                    color: theme.textMuted
                    elide: Text.ElideRight
                    maximumLineCount: 2
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                Item { Layout.fillHeight: true }

                // Footer Row: Status + Action Button
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    // Active / Status Badge
                    Rectangle {
                        property bool isSddm: Boolean(card.itemData && card.itemData.active_targets && card.itemData.active_targets.sddm_login)
                        property bool isLock: Boolean(card.itemData && card.itemData.active_targets && card.itemData.active_targets.session_lock)
                        property bool isAct: Boolean(card.itemData && (card.itemData.is_active || isSddm || isLock))
                        property bool isInst: Boolean(card.itemData && card.itemData.is_installed)
                        visible: isAct || isInst
                        height: 20
                        width: statusBadgeText.implicitWidth + 12
                        radius: 10
                        color: isAct ? (theme.resolvedTheme === "light" ? "#DCFCE7" : "#064E3B") : (theme.resolvedTheme === "light" ? "#EFF6FF" : "#1E3A8A")
                        border.color: isAct ? "#10B981" : "#3B82F6"
                        border.width: 1

                        Text {
                            id: statusBadgeText
                            anchors.centerIn: parent
                            text: parent.isSddm ? "ACTIVE SDDM" : (parent.isLock ? "ACTIVE LOCK" : (parent.isAct ? "ACTIVE" : "INSTALLED"))
                            font.pixelSize: 9
                            font.bold: true
                            color: parent.isAct ? (theme.resolvedTheme === "light" ? "#047857" : "#34D399") : (theme.resolvedTheme === "light" ? "#1D4ED8" : "#93C5FD")
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Contextual Action Button
                    RyzoraButton {
                        visible: !card.hideActionButton && (card.actionLabel !== "NONE")
                        size: "sm"
                        variant: (card.itemData && card.itemData.is_installed) ? "secondary" : "primary"
                        text: {
                            if (!card.itemData) return ""
                            if (card.actionLabel !== "") return card.actionLabel
                            if (card.itemData.is_installed) return "Open"
                            return "Get"
                        }
                        iconName: {
                            if (!card.itemData) return ""
                            if (card.itemData.is_installed) return "play"
                            return "arrow-download"
                        }
                        iconFilled: Boolean(card.itemData && card.itemData.is_installed)
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
