import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    property string selectedCategory: "all"
    property string selectedFilterStatus: "all" // "all" | "installed"
    property string sortOrder: "name" // "name" | "category"
    property string searchQuery: ""
    property string secondaryMode: "packages" // "packages" | "collections" | "creators" | "repositories"

    signal packageSelected(var packageData)
    signal searchResetRequested()

    function updateFilter() {
        backend.filterCatalog(root.selectedCategory, root.searchQuery)
    }

    function resetFilters() {
        root.selectedCategory = "all"
        root.selectedFilterStatus = "all"
        root.sortOrder = "name"
        root.searchQuery = ""
        root.searchResetRequested()
        root.updateFilter()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 14

        // ── 1. Page Header (Restrained macOS App Store Heading) ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            ColumnLayout {
                spacing: 2
                Layout.fillWidth: true

                Text {
                    text: root.secondaryMode === "collections" ? "Featured Collections" :
                          (root.secondaryMode === "creators" ? "Community Creators" :
                          (root.secondaryMode === "repositories" ? "Package Repositories" : "Discover"))
                    font.pixelSize: theme.fontTitle
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: root.secondaryMode === "collections" ? "Curated desktop rices, custom setups, and synchronized themes." :
                          (root.secondaryMode === "creators" ? "Authors and designers publishing custom themes and lock screens." :
                          (root.secondaryMode === "repositories" ? "Upstream package repositories, channels, and integrity indexes." :
                          "Explore community desktop customizations."))
                    font.pixelSize: theme.fontCardDesc
                    color: theme.textSecondary
                }
            }

            // View Mode Switcher Pills: Packages, Collections, Creators, Repositories
            Rectangle {
                height: 32
                width: secPillRow.implicitWidth + 8
                radius: theme.radiusSm
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    id: secPillRow
                    anchors.centerIn: parent
                    spacing: 4

                    Rectangle {
                        height: 24
                        width: secPkgTxt.implicitWidth + 16
                        radius: 4
                        color: root.secondaryMode === "packages" ? theme.surfaceElevated : "transparent"
                        border.color: root.secondaryMode === "packages" ? theme.borderStrong : "transparent"
                        border.width: 1

                        Text {
                            id: secPkgTxt
                            anchors.centerIn: parent
                            text: "Packages"
                            font.pixelSize: 11
                            font.bold: root.secondaryMode === "packages"
                            color: root.secondaryMode === "packages" ? theme.text : theme.textSecondary
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.secondaryMode = "packages"
                        }
                    }

                    Rectangle {
                        height: 24
                        width: secCollTxt.implicitWidth + 16
                        radius: 4
                        color: root.secondaryMode === "collections" ? theme.surfaceElevated : "transparent"
                        border.color: root.secondaryMode === "collections" ? theme.borderStrong : "transparent"
                        border.width: 1

                        Text {
                            id: secCollTxt
                            anchors.centerIn: parent
                            text: "Collections"
                            font.pixelSize: 11
                            font.bold: root.secondaryMode === "collections"
                            color: root.secondaryMode === "collections" ? theme.text : theme.textSecondary
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.secondaryMode = "collections"
                        }
                    }

                    Rectangle {
                        height: 24
                        width: secCreaTxt.implicitWidth + 16
                        radius: 4
                        color: root.secondaryMode === "creators" ? theme.surfaceElevated : "transparent"
                        border.color: root.secondaryMode === "creators" ? theme.borderStrong : "transparent"
                        border.width: 1

                        Text {
                            id: secCreaTxt
                            anchors.centerIn: parent
                            text: "Creators"
                            font.pixelSize: 11
                            font.bold: root.secondaryMode === "creators"
                            color: root.secondaryMode === "creators" ? theme.text : theme.textSecondary
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.secondaryMode = "creators"
                        }
                    }

                    Rectangle {
                        height: 24
                        width: secRepoTxt.implicitWidth + 16
                        radius: 4
                        color: root.secondaryMode === "repositories" ? theme.surfaceElevated : "transparent"
                        border.color: root.secondaryMode === "repositories" ? theme.borderStrong : "transparent"
                        border.width: 1

                        Text {
                            id: secRepoTxt
                            anchors.centerIn: parent
                            text: "Repositories"
                            font.pixelSize: 11
                            font.bold: root.secondaryMode === "repositories"
                            color: root.secondaryMode === "repositories" ? theme.text : theme.textSecondary
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.secondaryMode = "repositories"
                        }
                    }
                }
            }

            // Advanced Filters Popover Trigger & Item Count
            RowLayout {
                visible: root.secondaryMode === "packages"
                spacing: 8

                Text {
                    text: backend.catalogCount + " packages"
                    font.pixelSize: theme.fontMeta
                    color: theme.textMuted
                }

                Rectangle {
                    id: filterBtn
                    height: 28
                    width: filterBtnRow.implicitWidth + 16
                    radius: theme.radiusSm
                    color: filterMouse.containsMouse || filterPopover.visible ? theme.surfaceHover : theme.surface
                    border.color: filterPopover.visible ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: filterBtnRow
                        anchors.centerIn: parent
                        spacing: 6

                        Text {
                            text: "Filters"
                            font.pixelSize: theme.fontSm
                            color: theme.text
                        }
                        Text {
                            text: "▾"
                            font.pixelSize: 9
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
            }
        }

        // ── 2. Compact Category Selector Row ──
        ScrollView {
            visible: root.secondaryMode === "packages"
            Layout.fillWidth: true
            height: 32
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical.policy: ScrollBar.AlwaysOff

            RowLayout {
                spacing: 6

                CategoryChip {
                    label: "All"
                    catId: "all"
                    isSelected: root.selectedCategory === "all"
                    onClicked: { root.selectedCategory = "all"; root.updateFilter(); }
                }
                CategoryChip {
                    label: "Lock Screens"
                    catId: "lockscreens"
                    isSelected: root.selectedCategory === "lockscreens"
                    onClicked: { root.selectedCategory = "lockscreens"; root.updateFilter(); }
                }
                CategoryChip {
                    label: "Fastfetch"
                    catId: "fastfetch"
                    isSelected: root.selectedCategory === "fastfetch"
                    onClicked: { root.selectedCategory = "fastfetch"; root.updateFilter(); }
                }
                CategoryChip {
                    label: "Rices"
                    catId: "rices"
                    isSelected: root.selectedCategory === "rices"
                    onClicked: { root.selectedCategory = "rices"; root.updateFilter(); }
                }
                CategoryChip {
                    label: "Themes"
                    catId: "themes"
                    isSelected: root.selectedCategory === "themes"
                    onClicked: { root.selectedCategory = "themes"; root.updateFilter(); }
                }
                CategoryChip {
                    label: "Wallpapers"
                    catId: "wallpapers"
                    isSelected: root.selectedCategory === "wallpapers"
                    onClicked: { root.selectedCategory = "wallpapers"; root.updateFilter(); }
                }

                Item { Layout.fillWidth: true }

                // Clear Filter Button (when filtered)
                Rectangle {
                    visible: root.selectedCategory !== "all" || root.searchQuery !== ""
                    height: 26
                    width: clearTxt.implicitWidth + 14
                    radius: theme.radiusSm
                    color: clearMouse.containsMouse ? theme.surfaceHover : "transparent"
                    border.color: theme.borderSubtle
                    border.width: 1

                    Text {
                        id: clearTxt
                        anchors.centerIn: parent
                        text: "Clear All"
                        font.pixelSize: 11
                        color: theme.textSecondary
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
        }

        // ── 3. Image-First Package Grid Area ──
        Item {
            visible: root.secondaryMode === "packages"
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
                visible: backend.catalogCount === 0 && !backend.isCatalogLoading
                icon: "🔍"
                title: "No packages found"
                message: "No items match your selected filter or search terms."
                actionLabel: "Clear Filters"
                onActionClicked: root.resetFilters()
            }

            // Responsive Virtualized Image-First GridView
            GridView {
                id: grid
                objectName: "discoverGrid"
                anchors.fill: parent
                clip: true
                visible: backend.catalogCount > 0
                onWidthChanged: positionViewAtBeginning()
                Component.onCompleted: positionViewAtBeginning()

                // Responsive columns: 4 columns >= 1150px, 3 columns >= 820px, 2 columns >= 520px, 1 column below
                property int columnCount: width >= 1150 ? 4 : (width >= 820 ? 3 : (width >= 520 ? 2 : 1))
                cellWidth: Math.floor(width / columnCount)
                cellHeight: 254

                model: backend.catalogList
                delegate: Item {
                    width: grid.cellWidth
                    height: grid.cellHeight

                    StoreCard {
                        anchors.fill: parent
                        anchors.margins: 6
                        itemData: modelData
                        onCardClicked: function(data) {
                            root.packageSelected(data)
                        }
                    }
                }

                ScrollBar.vertical: ScrollBar {
                    policy: ScrollBar.AsNeeded
                }
            }
        }

        // ── 4. Curated Collections Subview ──
        Item {
            visible: root.secondaryMode === "collections"
            Layout.fillWidth: true
            Layout.fillHeight: true

            CollectionsView {
                anchors.fill: parent
                onPackageSelected: function(pkg) {
                    root.packageSelected(pkg)
                }
            }
        }

        // ── 5. Creators Subview ──
        Item {
            visible: root.secondaryMode === "creators"
            Layout.fillWidth: true
            Layout.fillHeight: true

            CreatorsView {
                anchors.fill: parent
                onCreatorSelected: function(author) {
                    root.secondaryMode = "packages"
                    root.selectedCategory = "all"
                    root.searchQuery = author
                    root.updateFilter()
                }
            }
        }

        // ── 6. Repositories Subview ──
        Item {
            visible: root.secondaryMode === "repositories"
            Layout.fillWidth: true
            Layout.fillHeight: true

            RepositoryView {
                anchors.fill: parent
            }
        }
    }

    // Advanced Filters Popover Menu
    Rectangle {
        id: filterPopover
        visible: false
        width: 220
        height: 140
        anchors.top: parent.top
        anchors.topMargin: 46
        anchors.right: parent.right
        anchors.rightMargin: 20
        radius: theme.radiusMd
        color: theme.surfaceElevated
        border.color: theme.borderStrong
        border.width: 1
        z: 50

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            Text {
                text: "Filter By Status"
                font.pixelSize: 11
                font.bold: true
                color: theme.textMuted
            }

            RowLayout {
                spacing: 8
                Rectangle {
                    height: 24
                    width: allStatusTxt.implicitWidth + 14
                    radius: theme.radiusSm
                    color: root.selectedFilterStatus === "all" ? theme.surfaceHover : "transparent"
                    border.color: root.selectedFilterStatus === "all" ? theme.accent : theme.borderSubtle
                    border.width: 1
                    Text {
                        id: allStatusTxt
                        anchors.centerIn: parent
                        text: "All"
                        font.pixelSize: 11
                        color: theme.text
                    }
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: { root.selectedFilterStatus = "all"; filterPopover.visible = false; }
                    }
                }

                Rectangle {
                    height: 24
                    width: instStatusTxt.implicitWidth + 14
                    radius: theme.radiusSm
                    color: root.selectedFilterStatus === "installed" ? theme.surfaceHover : "transparent"
                    border.color: root.selectedFilterStatus === "installed" ? theme.accent : theme.borderSubtle
                    border.width: 1
                    Text {
                        id: instStatusTxt
                        anchors.centerIn: parent
                        text: "Installed Only"
                        font.pixelSize: 11
                        color: theme.text
                    }
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: { root.selectedFilterStatus = "installed"; filterPopover.visible = false; }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            Rectangle {
                Layout.fillWidth: true
                height: 28
                radius: theme.radiusSm
                color: resetMouse.containsMouse ? theme.surfaceHover : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "Reset All Filters"
                    font.pixelSize: 11
                    color: theme.accent
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

    component CategoryChip : Rectangle {
        property string label: ""
        property string catId: ""
        property bool isSelected: false
        signal clicked()

        height: 28
        width: chipLabel.implicitWidth + 20
        radius: theme.radiusSm
        color: isSelected ? theme.surfaceElevated : (chipArea.containsMouse ? theme.surfaceHover : "transparent")
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
}
