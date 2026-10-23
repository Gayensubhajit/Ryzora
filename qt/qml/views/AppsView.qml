import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    property string selectedCategory: "All"
    property string searchQuery: ""
    property string sortBy: "name" // "name" | "category" | "status"
    property var selectedPackage: null
    property int featuredIndex: 0

    signal packageSelected(var packageData)
    signal actionRequested(string action, var data, var targetOverride)
    signal clearSearchRequested()

    function setSelectedPackage(pkg) {
        root.selectedPackage = pkg
    }

    // Normalize image URL from backend CatalogItem
    function resolveImageUrl(item) {
        if (!item) return ""
        var raw = item.preview_url || item.hero_image || ""
        var u = String(raw).trim()
        if (!u || u === "file://" || u === "file:///" || u === "http://" || u === "https://") return ""
        if (u.indexOf("file:///") === 0) return u
        if (u.indexOf("file://") === 0) {
            var rest = u.substring(7).trim()
            if (!rest || rest === "/") return ""
            return rest.indexOf("/") === 0 ? ("file://" + rest) : ("file:///" + rest)
        }
        if (u.indexOf("http://") === 0 || u.indexOf("https://") === 0) {
            // Avoid broken github user-attachments URLs that fail in QML Image without auth redirect
            if (u.indexOf("github.com/user-attachments/") !== -1) return ""
            return u
        }
        if (u.indexOf("/") === 0) return "file://" + u
        return ""
    }

    // Determine whether artwork is an isolated logo/emblem (PreserveAspectFit) vs full screenshot/wallpaper (PreserveAspectCrop)
    function isContainedArtwork(item) {
        if (!item) return false
        var sf = String(item.subfilter || "").toLowerCase()
        var pid = String(item.id || "").toLowerCase()
        var u = String(item.preview_url || item.hero_image || "").toLowerCase()
        if (sf === "emblems" || sf === "emblem" || pid.indexOf("emblem") !== -1) return true
        if (u.indexOf("/png/") !== -1 || u.indexOf("/pngs/") !== -1 || u.indexOf("/logo/") !== -1) return true
        if (u.indexOf("tsukikko.png") !== -1 || u.indexOf("cat.png") !== -1 || u.indexOf("hypr.png") !== -1 || u.indexOf("oh-hello-soldier.webp") !== -1) return true
        if (u.indexOf(".svg") !== -1 || u.indexOf("icon.png") !== -1) return true
        return false
    }

    // Helper: Normalize category string for robust matching across catalog
    function getNormalizedCategory(item) {
        if (!item) return "other"
        var cat = (item.category || "").toLowerCase()
        var pid = (item.id || "").toLowerCase()
        if (cat === "rices" || pid.indexOf("rice-") === 0) return "rices"
        if (cat === "lockscreens" || cat === "lock screens" || pid.indexOf("lockscreen-") === 0 || pid.indexOf("silentsddm-") === 0) return "lock screens"
        if (cat === "fastfetch" || pid.indexOf("fastfetch-") === 0) return "fastfetch"
        if (cat === "themes" || pid.indexOf("theme-") === 0) return "themes"
        if (cat === "wallpapers" || pid.indexOf("wallpaper-") === 0) return "wallpapers"
        if (cat === "bars" || pid.indexOf("bar-") === 0) return "bars"
        if (cat === "terminal" || pid.indexOf("terminal-") === 0) return "terminal"
        return "other"
    }

    function getCategoryDisplayName(item) {
        var norm = getNormalizedCategory(item)
        if (norm === "rices") return "Rice & Desktop"
        if (norm === "lock screens") return "Lock Screen"
        if (norm === "fastfetch") {
            var sf = String(item.subfilter || "").toLowerCase()
            return (sf === "emblems" || sf === "emblem") ? "Fastfetch Emblem" : "Fastfetch Preset"
        }
        if (norm === "themes") return "GTK / Qt Theme"
        if (norm === "wallpapers") return "Wallpaper"
        if (norm === "bars") return "Status Bar"
        if (norm === "terminal") return "Terminal"
        return "Package"
    }

    function getPackageIcon(item) {
        var norm = getNormalizedCategory(item)
        if (norm === "rices") return "palette"
        if (norm === "lock screens") return "lock"
        if (norm === "fastfetch") return "terminal"
        if (norm === "wallpapers") return "image"
        if (norm === "themes") return "sliders"
        if (norm === "bars") return "layout-grid"
        return "package"
    }

    // Count items matching a given category pill
    function getCategoryCount(catName) {
        var list = backend.catalogList || []
        var c = catName.toLowerCase()
        if (c === "all") return list.length
        var count = 0
        for (var i = 0; i < list.length; i++) {
            var item = list[i]
            if (!item) continue
            if (c === "installed") {
                if (item.is_installed) count++
            } else if (getNormalizedCategory(item) === c) {
                count++
            }
        }
        return count
    }

    // Filtered applications list based on search, category, and sort
    readonly property var filteredApps: {
        var list = backend.catalogList || []
        var res = []
        var q = root.searchQuery.trim().toLowerCase()
        var cat = root.selectedCategory.toLowerCase()

        for (var i = 0; i < list.length; i++) {
            var item = list[i]
            if (!item) continue

            // 1. Search Query Filter
            if (q.length > 0) {
                var nameMatch = item.name && item.name.toLowerCase().indexOf(q) !== -1
                var descMatch = item.description && item.description.toLowerCase().indexOf(q) !== -1
                var idMatch = item.id && item.id.toLowerCase().indexOf(q) !== -1
                var authorMatch = false
                if (item.author) {
                    var authStr = typeof item.author === "object" ? (item.author.name || "") : String(item.author)
                    authorMatch = authStr.toLowerCase().indexOf(q) !== -1
                }
                var tagsMatch = false
                if (item.tags && Array.isArray(item.tags)) {
                    for (var t = 0; t < item.tags.length; ++t) {
                        if (String(item.tags[t]).toLowerCase().indexOf(q) !== -1) {
                            tagsMatch = true
                            break
                        }
                    }
                }
                if (!nameMatch && !descMatch && !idMatch && !authorMatch && !tagsMatch) continue
            }

            // 2. Category Filter
            if (cat === "all") {
                res.push(item)
            } else if (cat === "installed") {
                if (item.is_installed) res.push(item)
            } else {
                var normCat = getNormalizedCategory(item)
                if (normCat === cat) res.push(item)
            }
        }

        // 3. Sorting
        res.sort(function(a, b) {
            if (root.sortBy === "status") {
                var sa = a.is_active ? 2 : (a.is_installed ? 1 : 0)
                var sb = b.is_active ? 2 : (b.is_installed ? 1 : 0)
                if (sa !== sb) return sb - sa
            } else if (root.sortBy === "category") {
                var ca = getNormalizedCategory(a)
                var cb = getNormalizedCategory(b)
                if (ca !== cb) return ca.localeCompare(cb)
            }
            var nameA = (a.name || "").toLowerCase()
            var nameB = (b.name || "").toLowerCase()
            return nameA.localeCompare(nameB)
        })

        return res
    }

    // Curated Featured items with verified landscape local artwork
    readonly property var featuredItems: {
        var list = backend.catalogList || []
        var res = []
        var preferredIds = [
            "fastfetch-friefetch",
            "lockscreen-qylock-dog-samurai",
            "fastfetch-snoopy-mugiwara",
            "lockscreen-qylock-terraria",
            "fastfetch-taiyo",
            "silentsddm-default"
        ]
        var seen = {}

        for (var p = 0; p < preferredIds.length; p++) {
            var targetId = preferredIds[p]
            for (var i = 0; i < list.length; i++) {
                var item = list[i]
                if (item && item.id === targetId && !seen[item.id]) {
                    var img = resolveImageUrl(item)
                    if (img !== "") {
                        res.push(item)
                        seen[item.id] = true
                    }
                    break
                }
            }
        }

        // Fallback if catalog is custom/minimal: pick up to 4 items with local landscape artwork
        if (res.length === 0) {
            for (var j = 0; j < list.length && res.length < 4; j++) {
                var cand = list[j]
                if (cand && !seen[cand.id]) {
                    var candImg = resolveImageUrl(cand)
                    if (candImg.indexOf("file://") === 0 && !isContainedArtwork(cand)) {
                        res.push(cand)
                        seen[cand.id] = true
                    }
                }
            }
        }
        return res
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 1. MAIN BROWSE VIEW
    // ──────────────────────────────────────────────────────────────────────────
    Flickable {
        id: mainScroll
        anchors.fill: parent
        visible: root.selectedPackage === null
        contentWidth: width
        contentHeight: pageCol.implicitHeight + 36
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ScrollBar.vertical: ScrollBar {
            policy: ScrollBar.AsNeeded
            width: 6
            contentItem: Rectangle {
                implicitWidth: 6
                radius: 3
                color: theme.borderStrong
            }
        }

        ColumnLayout {
            id: pageCol
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.margins: 20
            spacing: 14

            // ── 1. COMPACT PAGE HEADER (Title, Subtitle, Search & Sort on one row) ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 14

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: "Apps"
                        font.pixelSize: 22
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Browse and install desktop rices, lockscreens, Fastfetch presets, and themes."
                        font.pixelSize: 12
                        color: theme.textSecondary
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                // In-page Search Input
                Rectangle {
                    Layout.preferredWidth: Math.min(240, Math.max(160, root.width * 0.24))
                    height: 32
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: searchInp.activeFocus ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 8
                        spacing: 6

                        RyzoraIcon {
                            name: "search"
                            size: 13
                            color: theme.textMuted
                        }

                        TextInput {
                            id: searchInp
                            Layout.fillWidth: true
                            text: root.searchQuery
                            font.pixelSize: 12
                            color: theme.text
                            clip: true
                            selectByMouse: true
                            onTextChanged: root.searchQuery = text

                            Text {
                                visible: !searchInp.text && !searchInp.activeFocus
                                text: "Search packages…"
                                font.pixelSize: 12
                                color: theme.textMuted
                            }
                        }

                        Rectangle {
                            visible: searchInp.text.length > 0
                            width: 16
                            height: 16
                            radius: 8
                            color: "transparent"
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "x"
                                size: 10
                                color: theme.textMuted
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    searchInp.text = ""
                                    root.clearSearchRequested()
                                }
                            }
                        }
                    }
                }

                // Sort Control
                Rectangle {
                    height: 32
                    implicitWidth: sortRow.implicitWidth + 16
                    radius: theme.radiusSm
                    color: theme.surface
                    border.color: sortMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: sortRow
                        anchors.centerIn: parent
                        spacing: 6
                        RyzoraIcon { name: "filter"; size: 11; color: theme.textMuted }
                        Text {
                            text: "Sort: " + (root.sortBy === "status" ? "Status" : (root.sortBy === "category" ? "Category" : "Name"))
                            font.pixelSize: 11
                            font.weight: Font.Medium
                            color: theme.textSecondary
                        }
                    }

                    MouseArea {
                        id: sortMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (root.sortBy === "name") root.sortBy = "category"
                            else if (root.sortBy === "category") root.sortBy = "status"
                            else root.sortBy = "name"
                        }
                    }
                }
            }

            // ── 2. CATEGORY FILTER BAR (Restrained, single horizontal row) ──
            Flickable {
                Layout.fillWidth: true
                height: 32
                contentWidth: categoryRow.implicitWidth
                contentHeight: 32
                clip: true
                boundsBehavior: Flickable.StopAtBounds

                RowLayout {
                    id: categoryRow
                    height: 32
                    spacing: 6

                    Repeater {
                        model: [
                            { label: "All apps", id: "All", icon: "package" },
                            { label: "Rices & Desktops", id: "Rices", icon: "palette" },
                            { label: "Lock Screens", id: "Lock Screens", icon: "lock" },
                            { label: "Fastfetch", id: "Fastfetch", icon: "terminal" },
                            { label: "Themes", id: "Themes", icon: "sliders" },
                            { label: "Wallpapers", id: "Wallpapers", icon: "image" },
                            { label: "Installed", id: "Installed", icon: "check" }
                        ]

                        delegate: Rectangle {
                            readonly property bool isSelected: root.selectedCategory === modelData.id
                            readonly property int count: root.getCategoryCount(modelData.id)
                            height: 30
                            implicitWidth: catInnerRow.implicitWidth + 18
                            radius: theme.radiusSm
                            color: isSelected ? theme.surfaceElevated : (catMouse.containsMouse ? theme.surfaceHover : theme.surface)
                            border.color: isSelected ? theme.accent : (catMouse.containsMouse ? theme.borderStrong : theme.borderSubtle)
                            border.width: 1

                            RowLayout {
                                id: catInnerRow
                                anchors.centerIn: parent
                                spacing: 6

                                RyzoraIcon {
                                    name: modelData.icon
                                    size: 12
                                    color: isSelected ? theme.accent : theme.textSecondary
                                }

                                Text {
                                    text: modelData.label
                                    font.pixelSize: 11
                                    font.weight: isSelected ? Font.DemiBold : Font.Medium
                                    color: isSelected ? theme.text : theme.textSecondary
                                }

                                Text {
                                    text: String(count)
                                    font.pixelSize: 10
                                    font.family: "monospace"
                                    color: isSelected ? theme.accent : theme.textMuted
                                }
                            }

                            MouseArea {
                                id: catMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.selectedCategory = modelData.id
                            }
                        }
                    }
                }
            }

            // ── 3. COMPACT TWO-COLUMN FEATURED BANNER (168px high) ──
            Rectangle {
                id: heroPanel
                visible: root.searchQuery === "" && root.featuredItems.length > 0 && root.selectedCategory === "All"
                Layout.fillWidth: true
                height: 168
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1
                clip: true

                readonly property var currentItem: (root.featuredItems.length > 0 && root.featuredIndex < root.featuredItems.length) ? root.featuredItems[root.featuredIndex] : null
                readonly property string currentArtwork: resolveImageUrl(currentItem)

                RowLayout {
                    anchors.fill: parent
                    spacing: 0

                    // Left Column: App Information & Action
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.margins: 18
                        spacing: 6

                        RowLayout {
                            spacing: 8

                            Rectangle {
                                height: 18
                                radius: 4
                                implicitWidth: featBadgeTxt.implicitWidth + 10
                                color: theme.surfaceElevated
                                border.color: theme.borderStrong
                                border.width: 1

                                Text {
                                    id: featBadgeTxt
                                    anchors.centerIn: parent
                                    text: "FEATURED"
                                    font.pixelSize: 9
                                    font.bold: true
                                    font.letterSpacing: 0.8
                                    color: theme.accent
                                }
                            }

                            Text {
                                text: heroPanel.currentItem ? root.getCategoryDisplayName(heroPanel.currentItem) : ""
                                font.pixelSize: 11
                                color: theme.textMuted
                            }

                            Text {
                                visible: Boolean(heroPanel.currentItem && heroPanel.currentItem.author)
                                text: heroPanel.currentItem && heroPanel.currentItem.author
                                    ? ("· by " + (typeof heroPanel.currentItem.author === "object" ? (heroPanel.currentItem.author.name || "Community") : String(heroPanel.currentItem.author)))
                                    : ""
                                font.pixelSize: 11
                                color: theme.textMuted
                                elide: Text.ElideRight
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: heroPanel.currentItem ? (heroPanel.currentItem.name || "") : ""
                            font.pixelSize: 18
                            font.bold: true
                            color: theme.text
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: heroPanel.currentItem ? (heroPanel.currentItem.description || "") : ""
                            font.pixelSize: 12
                            color: theme.textSecondary
                            maximumLineCount: 2
                            wrapMode: Text.WordWrap
                            elide: Text.ElideRight
                        }

                        Item { Layout.fillHeight: true }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Rectangle {
                                height: 28
                                implicitWidth: exploreRow.implicitWidth + 20
                                radius: theme.radiusSm
                                color: heroBtnMouse.containsMouse ? theme.accentHover : theme.accent

                                RowLayout {
                                    id: exploreRow
                                    anchors.centerIn: parent
                                    spacing: 6
                                    Text {
                                        text: heroPanel.currentItem && heroPanel.currentItem.is_installed ? "View Details" : "Explore Package"
                                        font.pixelSize: 11
                                        font.bold: true
                                        color: "#ffffff"
                                    }
                                    RyzoraIcon {
                                        name: "chevron-right"
                                        size: 11
                                        color: "#ffffff"
                                    }
                                }

                                MouseArea {
                                    id: heroBtnMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (heroPanel.currentItem) {
                                            root.setSelectedPackage(heroPanel.currentItem)
                                            root.packageSelected(heroPanel.currentItem)
                                        }
                                    }
                                }
                            }

                            // Carousel Dots
                            RowLayout {
                                spacing: 5
                                visible: root.featuredItems.length > 1
                                Repeater {
                                    model: root.featuredItems.length
                                    Rectangle {
                                        width: modelData === root.featuredIndex ? 16 : 6
                                        height: 6
                                        radius: 3
                                        color: modelData === root.featuredIndex ? theme.accent : theme.borderStrong
                                        Behavior on width { NumberAnimation { duration: 140 } }
                                        MouseArea {
                                            anchors.fill: parent
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: root.featuredIndex = modelData
                                        }
                                    }
                                }
                            }

                            Item { Layout.fillWidth: true }
                        }
                    }

                    // Subtle Vertical Divider
                    Rectangle {
                        Layout.fillHeight: true
                        width: 1
                        color: theme.borderSubtle
                    }

                    // Right Column: Framed Landscape Artwork Preview
                    Rectangle {
                        Layout.preferredWidth: Math.min(360, Math.max(240, heroPanel.width * 0.40))
                        Layout.fillHeight: true
                        color: theme.resolvedTheme === "light" ? "#F0F2F5" : "#14171D"
                        clip: true

                        Image {
                            id: heroImg
                            anchors.fill: parent
                            source: heroPanel.currentArtwork
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                            cache: true
                            sourceSize.width: 720
                            sourceSize.height: 405
                            smooth: true
                            visible: status === Image.Ready
                        }

                        // Compact Fallback if image is still loading
                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 10
                            visible: heroImg.status !== Image.Ready

                            RyzoraIcon {
                                name: root.getPackageIcon(heroPanel.currentItem)
                                size: 24
                                color: theme.textMuted
                            }
                            Text {
                                text: heroPanel.currentItem ? root.getCategoryDisplayName(heroPanel.currentItem) : ""
                                font.pixelSize: 12
                                color: theme.textMuted
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (heroPanel.currentItem) {
                                    root.setSelectedPackage(heroPanel.currentItem)
                                    root.packageSelected(heroPanel.currentItem)
                                }
                            }
                        }
                    }
                }
            }

            // ── 4. MAIN CATALOGUE GRID (4-5 Columns on Desktop) ──
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        text: root.selectedCategory === "All" ? "All Packages" : root.selectedCategory
                        font.pixelSize: 15
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "(" + root.filteredApps.length + ")"
                        font.pixelSize: 12
                        font.family: "monospace"
                        color: theme.textMuted
                    }

                    Item { Layout.fillWidth: true }
                }

                GridLayout {
                    id: catalogGrid
                    Layout.fillWidth: true
                    columns: root.width < 520 ? 1 : (root.width < 760 ? 2 : (root.width < 1000 ? 3 : (root.width < 1300 ? 4 : 5)))
                    rowSpacing: 14
                    columnSpacing: 14

                    Repeater {
                        model: root.filteredApps

                        delegate: Rectangle {
                            id: appCard
                            Layout.fillWidth: true
                            height: 220
                            radius: theme.radiusMd
                            color: cardMouse.containsMouse ? theme.surfaceHover : theme.surface
                            border.color: cardMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
                            border.width: 1
                            clip: true

                            readonly property string cardImgUrl: root.resolveImageUrl(modelData)
                            readonly property bool useContainFit: root.isContainedArtwork(modelData)

                            MouseArea {
                                id: cardMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.setSelectedPackage(modelData)
                                    root.packageSelected(modelData)
                                }
                            }

                            ColumnLayout {
                                anchors.fill: parent
                                spacing: 0

                                // Top Landscape Preview Area (116px)
                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 116
                                    color: theme.resolvedTheme === "light" ? "#F1F3F6" : "#14171D"
                                    clip: true

                                    Image {
                                        id: cardPreviewImg
                                        anchors.fill: parent
                                        anchors.margins: appCard.useContainFit ? 14 : 0
                                        source: appCard.cardImgUrl
                                        fillMode: appCard.useContainFit ? Image.PreserveAspectFit : Image.PreserveAspectCrop
                                        asynchronous: true
                                        cache: true
                                        sourceSize.width: 480
                                        sourceSize.height: 270
                                        smooth: true
                                        visible: status === Image.Ready
                                    }

                                    // Neutral fallback when artwork is unavailable
                                    ColumnLayout {
                                        anchors.centerIn: parent
                                        spacing: 6
                                        visible: cardPreviewImg.status !== Image.Ready

                                        RyzoraIcon {
                                            Layout.alignment: Qt.AlignHCenter
                                            name: root.getPackageIcon(modelData)
                                            size: 22
                                            color: theme.textMuted
                                            opacity: 0.65
                                        }

                                        Text {
                                            Layout.alignment: Qt.AlignHCenter
                                            text: root.getCategoryDisplayName(modelData)
                                            font.pixelSize: 10
                                            color: theme.textMuted
                                            opacity: 0.75
                                        }
                                    }

                                    // Subtle bottom divider
                                    Rectangle {
                                        anchors.bottom: parent.bottom
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        height: 1
                                        color: theme.borderSubtle
                                    }
                                }

                                // Compact Information Block (104px)
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    Layout.leftMargin: 12
                                    Layout.rightMargin: 12
                                    Layout.topMargin: 10
                                    Layout.bottomMargin: 10
                                    spacing: 4

                                    Text {
                                        Layout.fillWidth: true
                                        text: modelData.name || modelData.id
                                        font.pixelSize: 13
                                        font.bold: true
                                        color: theme.text
                                        elide: Text.ElideRight
                                    }

                                    Text {
                                        Layout.fillWidth: true
                                        text: modelData.description || root.getCategoryDisplayName(modelData)
                                        font.pixelSize: 11
                                        color: theme.textSecondary
                                        maximumLineCount: 2
                                        wrapMode: Text.WordWrap
                                        elide: Text.ElideRight
                                    }

                                    Item { Layout.fillHeight: true }

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 6

                                        Text {
                                            Layout.fillWidth: true
                                            text: root.getCategoryDisplayName(modelData) + (modelData.author ? (" · " + (typeof modelData.author === "object" ? (modelData.author.name || "") : String(modelData.author))) : "")
                                            font.pixelSize: 10
                                            color: theme.textMuted
                                            elide: Text.ElideRight
                                        }

                                        // Contextual Action Pill (Get / Installed / Active)
                                        Rectangle {
                                            height: 22
                                            implicitWidth: cardBtnTxt.implicitWidth + 14
                                            radius: 4
                                            color: modelData.is_active ? Qt.rgba(16/255, 185/255, 129/255, 0.16)
                                                 : (modelData.is_installed ? theme.surfaceElevated : (actionMouse.containsMouse ? theme.accent : theme.surfaceElevated))
                                            border.color: modelData.is_active ? "#10b981"
                                                        : (modelData.is_installed ? theme.borderStrong : (actionMouse.containsMouse ? theme.accent : theme.borderStrong))
                                            border.width: 1

                                            Text {
                                                id: cardBtnTxt
                                                anchors.centerIn: parent
                                                text: modelData.is_active ? "Active" : (modelData.is_installed ? "Installed" : "Get")
                                                font.pixelSize: 10
                                                font.bold: true
                                                color: modelData.is_active ? "#10b981"
                                                     : (modelData.is_installed ? theme.textSecondary : (actionMouse.containsMouse ? "#ffffff" : theme.text))
                                            }

                                            MouseArea {
                                                id: actionMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: {
                                                    if (!modelData.is_installed) {
                                                        root.actionRequested("install", modelData, "")
                                                    } else {
                                                        root.setSelectedPackage(modelData)
                                                        root.packageSelected(modelData)
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

                // Empty State
                EmptyState {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 280
                    visible: root.filteredApps.length === 0
                    iconName: "package"
                    title: "No Matching Packages Found"
                    message: "No catalog packages match category '" + root.selectedCategory + "'" + (root.searchQuery ? " and query '" + root.searchQuery + "'" : "") + "."
                    actionLabel: "Clear Search & Filter"
                    onActionClicked: {
                        root.selectedCategory = "All"
                        root.searchQuery = ""
                        searchInp.text = ""
                        root.clearSearchRequested()
                    }
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 2. EMBEDDED PACKAGE DETAIL VIEW (Overlay for full detail and actions)
    // ──────────────────────────────────────────────────────────────────────────
    PackageDetailView {
        id: detailOverlay
        objectName: "appsPackageDetailView"
        anchors.fill: parent
        visible: root.selectedPackage !== null
        packageData: root.selectedPackage
        fromRoute: "apps"
        onBackRequested: root.setSelectedPackage(null)
        onPackageSelected: function(pkg) {
            root.setSelectedPackage(pkg)
        }
        onActionRequested: function(action, data, targetOverride) {
            root.actionRequested(action, data, targetOverride)
        }
    }
}
