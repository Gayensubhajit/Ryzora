import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    property string selectedCategory: "all"
    property string selectedSubFilter: "All" // "All" | "Qylock" | "SilentSDDM" | "SDDM" | "My Media"
    property string selectedFilterStatus: "all" // "all" | "installed"
    property string sortOrder: "name" // "name" | "category"
    property string searchQuery: ""
    property string selectedPackageId: ""
    property var selectedPackage: null
    property real savedScrollY: 0

    signal packageSelected(var packageData)
    signal actionRequested(string action, var data, var targetOverride)
    signal searchResetRequested()

    function setSelectedPackage(pkg) {
        if (!pkg) {
            root.selectedPackageId = ""
            root.selectedPackage = null
            if (detailView) {
                detailView.packageData = null
                detailView.errorMessage = ""
                detailView.pendingAction = ""
            }
            if (grid) {
                Qt.callLater(function() {
                    if (grid) grid.contentY = root.savedScrollY
                })
            }
            return
        }
        var pkgId = typeof pkg === "string" ? pkg : (pkg && pkg.id ? String(pkg.id) : "")
        if (!pkgId) {
            root.selectedPackageId = ""
            root.selectedPackage = null
            if (detailView) {
                detailView.packageData = null
            }
            return
        }
        root.savedScrollY = grid ? grid.contentY : 0
        root.selectedPackageId = pkgId
        var fresh = root.findPackage(pkgId)
        if (fresh) {
            root.selectedPackage = fresh
        } else if (typeof pkg === "object" && pkg !== null) {
            root.selectedPackage = pkg
        } else {
            root.selectedPackage = null
        }
        if (detailView) {
            detailView.packageData = root.selectedPackage
            detailView.errorMessage = ""
            detailView.pendingAction = ""
        }
    }

    onSelectedPackageChanged: {
        if (root.selectedPackage && root.selectedPackage.id) {
            root.selectedPackageId = String(root.selectedPackage.id)
        } else if (!root.selectedPackage) {
            root.selectedPackageId = ""
        }
        if (detailView) {
            detailView.packageData = root.selectedPackage
        }
    }

    function findPackage(pkgId) {
        if (!pkgId) return null
        var list = backend.catalogList || []
        for (var i = 0; i < list.length; ++i) {
            if (list[i] && String(list[i].id) === String(pkgId)) return list[i]
        }
        var lib = backend.libraryList || []
        for (var j = 0; j < lib.length; ++j) {
            if (lib[j] && String(lib[j].id) === String(pkgId)) return lib[j]
        }
        return null
    }

    function syncSelectedPackage() {
        if (!root.selectedPackageId) return
        var fresh = root.findPackage(root.selectedPackageId)
        if (fresh) {
            root.selectedPackage = fresh
            if (detailView) {
                detailView.packageData = fresh
            }
        }
    }

    readonly property var allLockscreenPackages: {
        var list = backend.catalogList || []
        return list.filter(function(item) {
            return String(item && item.category ? item.category : "").toLowerCase() === "lockscreens"
        })
    }

    readonly property var allFastfetchPackages: {
        var list = backend.catalogList || []
        return list.filter(function(item) {
            var cat = String(item && item.category ? item.category : "").toLowerCase()
            var pt = String(item && item.package_type ? item.package_type : "").toLowerCase()
            return cat === "fastfetch" || pt === "fastfetch"
        })
    }

    readonly property int fastfetchConfigsCount: {
        var list = root.allFastfetchPackages;
        return list.filter(function(item) {
            var isEmb = item && (String(item.subfilter).toLowerCase() === "emblems" || String(item.subfilter).toLowerCase() === "emblem" || String(item.id).indexOf("emblem") !== -1 || (item.tags && item.tags.indexOf("emblem") !== -1));
            return !isEmb;
        }).length;
    }

    readonly property int fastfetchEmblemsCount: {
        var list = root.allFastfetchPackages;
        return list.filter(function(item) {
            var isEmb = item && (String(item.subfilter).toLowerCase() === "emblems" || String(item.subfilter).toLowerCase() === "emblem" || String(item.id).indexOf("emblem") !== -1 || (item.tags && item.tags.indexOf("emblem") !== -1));
            return isEmb;
        }).length;
    }

    readonly property var filteredPackages: {
        var list = backend.catalogList || []

        // Root cause category filtering: strictly enforce category matching
        if (root.selectedCategory && root.selectedCategory !== "all") {
            var cat = root.selectedCategory.toLowerCase()
            list = list.filter(function(item) {
                return String(item && item.category ? item.category : "").toLowerCase() === cat
            })
        }

        // Subfilter filtering when in Fastfetch category
        if (root.selectedCategory === "fastfetch" && root.selectedSubFilter && root.selectedSubFilter !== "All") {
            var ffSub = root.selectedSubFilter
            if (ffSub === "Configs") {
                list = list.filter(function(item) {
                    var isEmb = item && (String(item.subfilter).toLowerCase() === "emblems" || String(item.subfilter).toLowerCase() === "emblem" || String(item.id).indexOf("emblem") !== -1 || (item.tags && item.tags.indexOf("emblem") !== -1))
                    return !isEmb
                })
            } else if (ffSub === "Emblems") {
                list = list.filter(function(item) {
                    var isEmb = item && (String(item.subfilter).toLowerCase() === "emblems" || String(item.subfilter).toLowerCase() === "emblem" || String(item.id).indexOf("emblem") !== -1 || (item.tags && item.tags.indexOf("emblem") !== -1))
                    return isEmb
                })
            }
        }

        // Subfilter filtering when in Lock Screens category
        if (root.selectedCategory === "lockscreens" && root.selectedSubFilter && root.selectedSubFilter !== "All") {
            var sub = root.selectedSubFilter
            if (sub === "Qylock") {
                list = list.filter(function(item) {
                    var isCustom = (item.tags && item.tags.indexOf("custom") !== -1) || String(item.id).indexOf("custom:") === 0 || String(item.subfilter) === "custom_media"
                    var isSilent = !isCustom && (String(item.subfilter) === "silentsddm" || String(item.id).indexOf("silentsddm-") === 0)
                    return !isCustom && !isSilent && ((item.tags && item.tags.indexOf("qylock") !== -1) || String(item.id).indexOf("lockscreen-qylock") === 0 || String(item.subfilter) === "qylock")
                })
            } else if (sub === "SilentSDDM") {
                list = list.filter(function(item) {
                    var isCustom = (item.tags && item.tags.indexOf("custom") !== -1) || String(item.id).indexOf("custom:") === 0 || String(item.subfilter) === "custom_media"
                    return !isCustom && (String(item.subfilter) === "silentsddm" || String(item.id).indexOf("silentsddm-") === 0)
                })
            } else if (sub === "SDDM") {
                list = list.filter(function(item) {
                    return Boolean(item.supports_sddm) || (item.tags && item.tags.indexOf("sddm") !== -1)
                })
            } else if (sub === "My Media") {
                list = list.filter(function(item) {
                    return (item.tags && item.tags.indexOf("custom") !== -1) || String(item.id).indexOf("custom:") === 0 || String(item.subfilter) === "custom_media"
                })
            }
        }

        // Search query filtering
        if (root.searchQuery && root.searchQuery.trim().length > 0) {
            var q = root.searchQuery.trim().toLowerCase()
            list = list.filter(function(item) {
                var nameMatch = String(item.name || "").toLowerCase().indexOf(q) !== -1
                var descMatch = String(item.description || "").toLowerCase().indexOf(q) !== -1
                var authorMatch = String(item.author || "").toLowerCase().indexOf(q) !== -1
                var idMatch = String(item.id || "").toLowerCase().indexOf(q) !== -1
                var tagsMatch = Array.isArray(item.tags) && item.tags.some(function(t) { return String(t).toLowerCase().indexOf(q) !== -1 })
                return nameMatch || descMatch || authorMatch || idMatch || tagsMatch
            })
        }

        // Status filtering
        if (root.selectedFilterStatus === "installed") {
            list = list.filter(function(item) {
                return Boolean(item && item.is_installed)
            })
        }

        // Sorting
        if (root.sortOrder === "category") {
            list = list.slice().sort(function(a, b) {
                var cA = String(a && a.category ? a.category : "").toLowerCase()
                var cB = String(b && b.category ? b.category : "").toLowerCase()
                if (cA !== cB) return cA.localeCompare(cB)
                return String(a && a.name ? a.name : "").localeCompare(String(b && b.name ? b.name : ""))
            })
        } else {
            list = list.slice().sort(function(a, b) {
                return String(a && a.name ? a.name : "").localeCompare(String(b && b.name ? b.name : ""))
            })
        }

        return list
    }

    Connections {
        target: backend
        function onCatalogChanged() { root.syncSelectedPackage() }
        function onLibraryChanged() { root.syncSelectedPackage() }
        function onActiveConfigurationChanged() { root.syncSelectedPackage() }
        function onTargetDeactivated(target, success, message) { root.syncSelectedPackage() }
        function onTargetActivated(packageId, target, success, message) { root.syncSelectedPackage() }
        function onPackageInstalled(packageId, success, message) { root.syncSelectedPackage() }
        function onPackageUninstalled(packageId, success, message) { root.syncSelectedPackage() }
        function onPackageTested(packageId, success, message) { root.syncSelectedPackage() }
    }

    function selectCategory(cat) {
        if (root.selectedCategory !== cat) {
            root.selectedCategory = cat
            root.selectedSubFilter = "All"
            root.updateFilter()
        }
    }

    function updateFilter() {
        backend.filterCatalog(root.selectedCategory, root.searchQuery)
    }

    function resetFilters() {
        root.selectedCategory = "all"
        root.selectedSubFilter = "All"
        root.selectedFilterStatus = "all"
        root.sortOrder = "name"
        root.searchQuery = ""
        root.searchResetRequested()
        root.updateFilter()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 0
        spacing: 12
        visible: root.selectedPackage === null

        // ── Row 1: Title, Subtitle, & Aligned Package Count Badge ──
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 24
            Layout.rightMargin: 24
            Layout.topMargin: 20
            spacing: 16

            ColumnLayout {
                spacing: 3
                Layout.fillWidth: true

                Text {
                    text: "Discover"
                    font.pixelSize: 24
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Explore community desktop customizations."
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            // Right-aligned status pill with package count
            Rectangle {
                height: 28
                width: countRow.implicitWidth + 18
                radius: 6
                color: theme.surfaceActive
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    id: countRow
                    anchors.centerIn: parent
                    spacing: 6
                    RyzoraIcon {
                        name: "package"
                        size: 13
                        color: theme.textSecondary
                    }
                    Text {
                        text: root.filteredPackages.length + " packages"
                        font.pixelSize: 11
                        font.bold: true
                        color: theme.text
                    }
                }
            }
        }

        // ── Row 2: Category Chips on Left + Filters and Clear All on Right ──
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 24
            Layout.rightMargin: 24
            spacing: 10

            // Horizontal Category Chips
            ScrollView {
                Layout.fillWidth: true
                height: 32
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                ScrollBar.vertical.policy: ScrollBar.AlwaysOff

                RowLayout {
                    spacing: 6

                    CategoryChip {
                        label: "All"
                        catId: "all"
                        isSelected: root.selectedCategory === "all"
                        onClicked: root.selectCategory("all")
                    }
                    CategoryChip {
                        label: "Lock Screens"
                        catId: "lockscreens"
                        isSelected: root.selectedCategory === "lockscreens"
                        onClicked: root.selectCategory("lockscreens")
                    }
                    CategoryChip {
                        label: "Fastfetch"
                        catId: "fastfetch"
                        isSelected: root.selectedCategory === "fastfetch"
                        onClicked: root.selectCategory("fastfetch")
                    }
                    CategoryChip {
                        label: "Rices"
                        catId: "rices"
                        isSelected: root.selectedCategory === "rices"
                        onClicked: root.selectCategory("rices")
                    }
                    CategoryChip {
                        label: "Themes"
                        catId: "themes"
                        isSelected: root.selectedCategory === "themes"
                        onClicked: root.selectCategory("themes")
                    }
                    CategoryChip {
                        label: "Wallpapers"
                        catId: "wallpapers"
                        isSelected: root.selectedCategory === "wallpapers"
                        onClicked: root.selectCategory("wallpapers")
                    }
                }
            }

            // Advanced Filters Button
            Rectangle {
                id: filterBtn
                height: 32
                width: filterBtnRow.implicitWidth + 20
                radius: 8
                color: filterMouse.containsMouse || filterPopover.visible ? theme.surfaceHover : (root.selectedFilterStatus !== "all" || root.sortOrder !== "name" ? theme.surfaceActive : theme.surface)
                border.color: filterPopover.visible || root.selectedFilterStatus !== "all" || root.sortOrder !== "name" ? theme.accent : theme.borderSubtle
                border.width: 1

                RowLayout {
                    id: filterBtnRow
                    anchors.centerIn: parent
                    spacing: 6

                    RyzoraIcon {
                        name: "sliders"
                        size: 13
                        color: root.selectedFilterStatus !== "all" || root.sortOrder !== "name" ? theme.accent : theme.textSecondary
                    }

                    Text {
                        text: "Filters"
                        font.pixelSize: 12
                        font.bold: true
                        color: root.selectedFilterStatus !== "all" || root.sortOrder !== "name" ? theme.accent : theme.text
                    }

                    Rectangle {
                        visible: root.selectedFilterStatus !== "all" || root.sortOrder !== "name"
                        width: 6
                        height: 6
                        radius: 3
                        color: theme.accent
                    }

                    RyzoraIcon {
                        name: "chevron-down"
                        size: 10
                        color: theme.textMuted
                    }
                }

                MouseArea {
                    id: filterMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: filterPopover.visible = !filterPopover.visible
                }
            }

            // Clear All Action Button
            Rectangle {
                visible: root.selectedCategory !== "all" || root.selectedFilterStatus !== "all" || root.sortOrder !== "name" || root.searchQuery !== ""
                height: 32
                width: clearTxtRow.implicitWidth + 18
                radius: 8
                color: clearMouse.containsMouse ? theme.surfaceHover : "transparent"
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    id: clearTxtRow
                    anchors.centerIn: parent
                    spacing: 5
                    RyzoraIcon {
                        name: "x"
                        size: 11
                        color: theme.textSecondary
                    }
                    Text {
                        text: "Clear"
                        font.pixelSize: 11
                        font.bold: true
                        color: theme.textSecondary
                    }
                }

                MouseArea {
                    id: clearMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.resetFilters()
                }
            }
        }

        // ── Row 2b: Dedicated Lock Screens Subcategory Row (Visible only when Lock Screens is active) ──
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 24
            Layout.rightMargin: 24
            visible: root.selectedCategory === "lockscreens"
            spacing: 8

            ScrollView {
                Layout.fillWidth: true
                height: 30
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                ScrollBar.vertical.policy: ScrollBar.AlwaysOff

                RowLayout {
                    spacing: 6

                    DiscoverSubFilterChip {
                        label: "All"
                        subId: "All"
                        iconName: "lock"
                        count: root.allLockscreenPackages.length
                        isSelected: root.selectedSubFilter === "All"
                        onClicked: root.selectedSubFilter = "All"
                    }

                    DiscoverSubFilterChip {
                        label: "Qylock"
                        subId: "Qylock"
                        iconName: "shield"
                        count: {
                            var list = root.allLockscreenPackages;
                            return list.filter(function(item) {
                                var isCustom = (item.tags && item.tags.indexOf("custom") !== -1) || String(item.id).indexOf("custom:") === 0 || String(item.subfilter) === "custom_media";
                                var isSilent = !isCustom && (String(item.subfilter) === "silentsddm" || String(item.id).indexOf("silentsddm-") === 0);
                                return !isCustom && !isSilent && ((item.tags && item.tags.indexOf("qylock") !== -1) || String(item.id).indexOf("lockscreen-qylock") === 0 || String(item.subfilter) === "qylock");
                            }).length;
                        }
                        isSelected: root.selectedSubFilter === "Qylock"
                        onClicked: root.selectedSubFilter = "Qylock"
                    }

                    DiscoverSubFilterChip {
                        label: "SilentSDDM"
                        subId: "SilentSDDM"
                        iconName: "monitor"
                        count: {
                            var list = root.allLockscreenPackages;
                            return list.filter(function(item) {
                                var isCustom = (item.tags && item.tags.indexOf("custom") !== -1) || String(item.id).indexOf("custom:") === 0 || String(item.subfilter) === "custom_media";
                                return !isCustom && (String(item.subfilter) === "silentsddm" || String(item.id).indexOf("silentsddm-") === 0);
                            }).length;
                        }
                        isSelected: root.selectedSubFilter === "SilentSDDM"
                        onClicked: root.selectedSubFilter = "SilentSDDM"
                    }

                    DiscoverSubFilterChip {
                        label: "SDDM"
                        subId: "SDDM"
                        iconName: "check-circle"
                        count: {
                            var list = root.allLockscreenPackages;
                            return list.filter(function(item) {
                                return Boolean(item.supports_sddm) || (item.tags && item.tags.indexOf("sddm") !== -1);
                            }).length;
                        }
                        isSelected: root.selectedSubFilter === "SDDM"
                        onClicked: root.selectedSubFilter = "SDDM"
                    }

                    DiscoverSubFilterChip {
                        label: "My Media"
                        subId: "My Media"
                        iconName: "image"
                        count: {
                            var list = root.allLockscreenPackages;
                            return list.filter(function(item) {
                                return (item.tags && item.tags.indexOf("custom") !== -1) || String(item.id).indexOf("custom:") === 0 || String(item.subfilter) === "custom_media";
                            }).length;
                        }
                        isSelected: root.selectedSubFilter === "My Media"
                        onClicked: root.selectedSubFilter = "My Media"
                    }
                }
            }
        }

        // ── Row 2c: Dedicated Fastfetch Subcategory Row (Visible only when Fastfetch is active) ──
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 24
            Layout.rightMargin: 24
            visible: root.selectedCategory === "fastfetch"
            spacing: 8

            ScrollView {
                Layout.fillWidth: true
                height: 30
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                ScrollBar.vertical.policy: ScrollBar.AlwaysOff

                RowLayout {
                    spacing: 6

                    DiscoverSubFilterChip {
                        label: "All"
                        subId: "All"
                        iconName: "terminal"
                        count: root.allFastfetchPackages.length
                        isSelected: root.selectedSubFilter === "All"
                        onClicked: root.selectedSubFilter = "All"
                    }

                    DiscoverSubFilterChip {
                        label: "Configs"
                        subId: "Configs"
                        iconName: "sliders"
                        count: root.fastfetchConfigsCount
                        isSelected: root.selectedSubFilter === "Configs"
                        onClicked: root.selectedSubFilter = "Configs"
                    }

                    DiscoverSubFilterChip {
                        label: "Emblems"
                        subId: "Emblems"
                        iconName: "image"
                        count: root.fastfetchEmblemsCount
                        isSelected: root.selectedSubFilter === "Emblems"
                        onClicked: root.selectedSubFilter = "Emblems"
                    }
                }
            }
        }

        // ── 3. Image-First Package Grid Area ──
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            // Loading Indicator
            Rectangle {
                anchors.fill: parent
                color: Qt.rgba(theme.bg.r, theme.bg.g, theme.bg.b, 0.75)
                visible: backend.isCatalogLoading
                z: 10

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 12
                    BusyIndicator { running: backend.isCatalogLoading }
                    Text {
                        text: "Loading catalog…"
                        font.pixelSize: 13
                        font.bold: true
                        color: theme.accent
                    }
                }
            }

            // Empty State
            EmptyState {
                anchors.fill: parent
                anchors.leftMargin: 24
                anchors.rightMargin: 24
                visible: root.filteredPackages.length === 0 && !backend.isCatalogLoading
                iconName: (root.selectedCategory === "lockscreens" && root.selectedSubFilter === "My Media") ? "image" : "search"
                title: {
                    if (root.selectedCategory === "lockscreens" && root.selectedSubFilter === "My Media") {
                        return "No custom media imported"
                    }
                    if (root.selectedCategory === "lockscreens" && root.selectedSubFilter === "SilentSDDM") {
                        return "No SilentSDDM wallpapers"
                    }
                    return "No packages found"
                }
                message: {
                    if (root.selectedCategory === "lockscreens" && root.selectedSubFilter === "My Media") {
                        return "Import images or videos into SilentSDDM custom media to view them here."
                    }
                    if (root.selectedCategory === "lockscreens" && root.selectedSubFilter === "SilentSDDM") {
                        return "Upstream SilentSDDM catalog wallpapers could not be located."
                    }
                    return "No items match your selected filter or category."
                }
                actionLabel: (root.selectedCategory === "lockscreens" && root.selectedSubFilter !== "All") ? "Show All Lock Screens" : "Clear Filters"
                onActionClicked: {
                    if (root.selectedCategory === "lockscreens" && root.selectedSubFilter !== "All") {
                        root.selectedSubFilter = "All"
                    } else {
                        root.resetFilters()
                    }
                }
            }

            // Responsive Virtualized Image-First GridView
            // Anchored with leftMargin: 24 and rightMargin: 0 so the vertical scrollbar sits at the far-right edge of the viewport
            GridView {
                id: grid
                objectName: "discoverGrid"
                anchors.fill: parent
                anchors.leftMargin: 24
                anchors.rightMargin: 0
                clip: true
                visible: root.filteredPackages.length > 0
                boundsBehavior: Flickable.StopAtBounds
                flickDeceleration: 1800
                maximumFlickVelocity: 4500

                WheelHandler {
                    target: null
                    acceptedDevices: PointerDevice.Mouse
                    blocking: true
                    onWheel: (event) => {
                        var delta = event.angleDelta.y
                        var step = -delta * (80.0 / 120.0) * 1.18
                        grid.contentY = Math.max(0, Math.min(grid.contentHeight - grid.height, grid.contentY + step))
                    }
                }

                // Responsive columns: 4 columns >= 1150px, 3 columns >= 820px, 2 columns >= 520px, 1 column below
                // Width minus 18 ensures right-hand cards do not touch the scrollbar thumb
                property int columnCount: (width - 18) >= 1150 ? 4 : ((width - 18) >= 820 ? 3 : ((width - 18) >= 520 ? 2 : 1))
                cellWidth: Math.floor((width - 18) / columnCount)
                cellHeight: 260

                model: root.filteredPackages
                delegate: Item {
                    width: grid.cellWidth
                    height: grid.cellHeight

                    StoreCard {
                        anchors.fill: parent
                        anchors.margins: 8
                        itemData: modelData
                        isPageVisible: root.visible
                        allowAutoplay: true
                        onCardClicked: function(data) {
                            root.setSelectedPackage(data)
                            root.packageSelected(root.selectedPackage || data)
                        }
                        onActionClicked: function(action, data) {
                            root.actionRequested(action, data, null)
                        }
                    }
                }

                ScrollBar.vertical: ScrollBar {
                    id: vScrollBar
                    policy: ScrollBar.AsNeeded
                    anchors.right: parent.right
                    anchors.rightMargin: 2
                }
            }
        }
    }

    // Dismissal overlay when popover is open
    MouseArea {
        anchors.fill: parent
        visible: filterPopover.visible
        z: 49
        onClicked: filterPopover.visible = false
    }

    // Advanced Filters Popover Menu
    Rectangle {
        id: filterPopover
        visible: false
        width: 240
        height: filterCol.implicitHeight + 24
        x: Math.max(16, Math.min(filterBtn.mapToItem(root, 0, 0).x, root.width - width - 20))
        y: filterBtn.mapToItem(root, 0, 0).y + filterBtn.height + 6
        radius: theme.radiusMd
        color: theme.surfaceElevated
        border.color: theme.borderStrong
        border.width: 1
        z: 50

        ColumnLayout {
            id: filterCol
            anchors.fill: parent
            anchors.margins: 12
            spacing: 10

            // 1. Status Filter Group
            Text {
                text: "STATUS"
                font.pixelSize: 10
                font.bold: true
                color: theme.textMuted
                font.letterSpacing: 0.5
            }

            RowLayout {
                spacing: 6
                FilterPill {
                    label: "All"
                    active: root.selectedFilterStatus === "all"
                    onClicked: { root.selectedFilterStatus = "all"; }
                }
                FilterPill {
                    label: "Installed Only"
                    active: root.selectedFilterStatus === "installed"
                    onClicked: { root.selectedFilterStatus = "installed"; }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            // 2. Sort Order Group
            Text {
                text: "SORT BY"
                font.pixelSize: 10
                font.bold: true
                color: theme.textMuted
                font.letterSpacing: 0.5
            }

            RowLayout {
                spacing: 6
                FilterPill {
                    label: "Name (A–Z)"
                    active: root.sortOrder === "name"
                    onClicked: { root.sortOrder = "name"; }
                }
                FilterPill {
                    label: "Category"
                    active: root.sortOrder === "category"
                    onClicked: { root.sortOrder = "category"; }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            // 3. Reset Action
            Rectangle {
                Layout.fillWidth: true
                height: 28
                radius: theme.radiusSm
                color: resetMouse.containsMouse ? theme.surfaceHover : "transparent"

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 6
                    RyzoraIcon {
                        name: "refresh"
                        size: 11
                        color: theme.accent
                    }
                    Text {
                        text: "Reset All Filters"
                        font.pixelSize: 11
                        font.bold: true
                        color: theme.accent
                    }
                }

                MouseArea {
                    id: resetMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.resetFilters()
                        filterPopover.visible = false
                    }
                }
            }
        }
    }

    // FilterPill Component for Filter Popover
    component FilterPill : Rectangle {
        property string label: ""
        property bool active: false
        signal clicked()

        height: 26
        width: pillText.implicitWidth + 16
        radius: theme.radiusSm
        color: active ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#2C313B") : (pillMouse.containsMouse ? theme.surfaceHover : "transparent")
        border.color: active ? theme.accent : theme.borderSubtle
        border.width: 1

        Text {
            id: pillText
            anchors.centerIn: parent
            text: label
            font.pixelSize: 11
            font.bold: active
            color: active ? theme.text : theme.textSecondary
        }

        MouseArea {
            id: pillMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    // DiscoverSubFilterChip Component for Second-Level Lock Screen Filter
    component DiscoverSubFilterChip : Rectangle {
        property string label: ""
        property string subId: ""
        property string iconName: ""
        property bool isSelected: false
        property int count: -1
        signal clicked()

        height: 28
        width: chipRow.implicitWidth + 20
        radius: 6
        color: isSelected ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#282C35") : (chipArea.containsMouse ? theme.surfaceHover : theme.surface)
        border.color: isSelected ? theme.accent : theme.borderSubtle
        border.width: 1

        Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }
        Behavior on border.color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }

        RowLayout {
            id: chipRow
            anchors.centerIn: parent
            spacing: 5

            RyzoraIcon {
                visible: iconName.length > 0
                name: iconName
                size: 12
                color: isSelected ? theme.accent : (chipArea.containsMouse ? theme.text : theme.textSecondary)
            }

            Text {
                text: label
                font.pixelSize: 11
                font.bold: isSelected
                color: isSelected ? theme.text : (chipArea.containsMouse ? theme.text : theme.textSecondary)
            }

            Text {
                visible: count >= 0
                text: "(" + count + ")"
                font.pixelSize: 10
                color: isSelected ? theme.accent : theme.textMuted
            }
        }

        MouseArea {
            id: chipArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    // CategoryChip Component for Second Row
    component CategoryChip : Rectangle {
        property string label: ""
        property string catId: ""
        property bool isSelected: false
        signal clicked()

        height: 30
        width: chipLabel.implicitWidth + 22
        radius: 8
        color: isSelected ? (theme.resolvedTheme === "light" ? "#E5E7EB" : "#282C35") : (chipArea.containsMouse ? theme.surfaceHover : theme.surface)
        border.color: isSelected ? theme.accent : theme.borderSubtle
        border.width: 1

        Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }
        Behavior on border.color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }

        Text {
            id: chipLabel
            anchors.centerIn: parent
            text: label
            font.pixelSize: 12
            font.bold: isSelected
            color: isSelected ? theme.text : (chipArea.containsMouse ? theme.text : theme.textSecondary)
        }

        MouseArea {
            id: chipArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    // ── Full-Page WalliD-Style Package Detail View ──
    PackageDetailView {
        id: detailView
        objectName: "packageDetailView"
        anchors.fill: parent
        visible: root.visible && root.selectedPackage !== null
        packageData: root.selectedPackage
        fromRoute: "discover"
        onBackRequested: root.setSelectedPackage(null)
        onPackageSelected: function(pkg) {
            root.setSelectedPackage(pkg)
        }
        onActionRequested: function(action, data, targetOverride) {
            root.actionRequested(action, data, targetOverride)
        }
    }
}
