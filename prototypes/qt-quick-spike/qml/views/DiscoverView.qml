import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    property string selectedCategory: "all"
    property string selectedSubfilter: "all"
    property string searchQuery: ""

    signal packageSelected(var packageData)

    function updateFilter() {
        backend.filterCatalog(root.selectedCategory, root.searchQuery)
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 14

        // ── 1. Discover Header ──
        ColumnLayout {
            spacing: 2
            Layout.fillWidth: true

            Text {
                text: "Discover"
                font.pixelSize: 24
                font.bold: true
                color: theme.text
            }

            Text {
                text: "Explore themes, wallpapers, lock screens and more"
                font.pixelSize: 13
                color: theme.textSecondary
            }
        }

        // ── 2. Category Tabs Bar ──
        ScrollView {
            Layout.fillWidth: true
            height: 38
            ScrollBar.horizontal.policy: ScrollBar.AsNeeded
            ScrollBar.vertical.policy: ScrollBar.AlwaysOff

            RowLayout {
                spacing: 8

                CategoryTab {
                    label: "All"
                    catId: "all"
                    isSelected: root.selectedCategory === "all"
                    onClicked: { root.selectedCategory = "all"; root.selectedSubfilter = "all"; root.updateFilter(); }
                }
                CategoryTab {
                    label: "Lock Screens"
                    catId: "lockscreens"
                    isSelected: root.selectedCategory === "lockscreens"
                    onClicked: { root.selectedCategory = "lockscreens"; root.selectedSubfilter = "all"; root.updateFilter(); }
                }
                CategoryTab {
                    label: "Fastfetch"
                    catId: "fastfetch"
                    isSelected: root.selectedCategory === "fastfetch"
                    onClicked: { root.selectedCategory = "fastfetch"; root.selectedSubfilter = "all"; root.updateFilter(); }
                }
                CategoryTab {
                    label: "Rices"
                    catId: "rices"
                    isSelected: root.selectedCategory === "rices"
                    onClicked: { root.selectedCategory = "rices"; root.selectedSubfilter = "all"; root.updateFilter(); }
                }
                CategoryTab {
                    label: "Themes"
                    catId: "themes"
                    isSelected: root.selectedCategory === "themes"
                    onClicked: { root.selectedCategory = "themes"; root.selectedSubfilter = "all"; root.updateFilter(); }
                }
                CategoryTab {
                    label: "Wallpapers"
                    catId: "wallpapers"
                    isSelected: root.selectedCategory === "wallpapers"
                    onClicked: { root.selectedCategory = "wallpapers"; root.selectedSubfilter = "all"; root.updateFilter(); }
                }
            }
        }

        // ── 3. Subcategory Filter Pills (when applicable) ──
        RowLayout {
            visible: root.selectedCategory === "lockscreens" || root.selectedCategory === "fastfetch"
            spacing: 6
            Layout.fillWidth: true

            SubFilterPill {
                label: "All"
                subId: "all"
                isSelected: root.selectedSubfilter === "all"
                onClicked: { root.selectedSubfilter = "all"; root.updateFilter(); }
            }

            // Lock Screens subfilters
            SubFilterPill {
                visible: root.selectedCategory === "lockscreens"
                label: "Qylock"
                subId: "qylock"
                isSelected: root.selectedSubfilter === "qylock"
                onClicked: { root.selectedSubfilter = "qylock"; root.updateFilter(); }
            }
            SubFilterPill {
                visible: root.selectedCategory === "lockscreens"
                label: "SilentSDDM"
                subId: "silentsddm"
                isSelected: root.selectedSubfilter === "silentsddm"
                onClicked: { root.selectedSubfilter = "silentsddm"; root.updateFilter(); }
            }
            SubFilterPill {
                visible: root.selectedCategory === "lockscreens"
                label: "SDDM"
                subId: "sddm"
                isSelected: root.selectedSubfilter === "sddm"
                onClicked: { root.selectedSubfilter = "sddm"; root.updateFilter(); }
            }

            // Fastfetch subfilters
            SubFilterPill {
                visible: root.selectedCategory === "fastfetch"
                label: "Configs"
                subId: "configs"
                isSelected: root.selectedSubfilter === "configs"
                onClicked: { root.selectedSubfilter = "configs"; root.updateFilter(); }
            }
            SubFilterPill {
                visible: root.selectedCategory === "fastfetch"
                label: "Emblems"
                subId: "emblems"
                isSelected: root.selectedSubfilter === "emblems"
                onClicked: { root.selectedSubfilter = "emblems"; root.updateFilter(); }
            }

            Item { Layout.fillWidth: true }

            Text {
                text: backend.catalogCount + " packages available"
                font.pixelSize: 11
                color: theme.textMuted
            }
        }

        // ── 4. Main View Area: Responsive Grid of StoreCard items ──
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
                        text: "Querying package catalog…"
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
                message: "No items match your selected filter or search terms. Try selecting 'All' or clearing the search."
                actionLabel: "Reset Filters"
                onActionClicked: {
                    root.selectedCategory = "all"
                    root.selectedSubfilter = "all"
                    root.searchQuery = ""
                    root.updateFilter()
                }
            }

            // Responsive Virtualized GridView
            GridView {
                id: grid
                anchors.fill: parent
                clip: true
                visible: backend.catalogCount > 0

                // Responsive Columns matching .store-grid-fluid (1 col, 2 cols, 3-4 cols)
                property int columnCount: width >= 1120 ? 4 : (width >= 820 ? 3 : (width >= 560 ? 2 : 1))
                cellWidth: Math.floor(width / columnCount)
                cellHeight: 304

                model: backend.catalogList
                delegate: Item {
                    width: grid.cellWidth
                    height: grid.cellHeight

                    StoreCard {
                        anchors.centerIn: parent
                        width: grid.cellWidth - 14
                        height: 290
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
    }

    component CategoryTab : Rectangle {
        property string label: ""
        property string catId: ""
        property bool isSelected: false
        signal clicked()

        height: 32
        width: tabLabel.width + 24
        radius: theme.radiusSm
        color: isSelected ? theme.accent : (tabArea.containsMouse ? theme.surfaceHover : "transparent")
        border.color: isSelected ? theme.accent : theme.borderSubtle
        border.width: 1

        Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 80 } }

        Text {
            id: tabLabel
            anchors.centerIn: parent
            text: label
            font.pixelSize: 12
            font.bold: isSelected
            color: isSelected ? "#ffffff" : (tabArea.containsMouse ? theme.text : theme.textSecondary)
        }

        MouseArea {
            id: tabArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    component SubFilterPill : Rectangle {
        property string label: ""
        property string subId: ""
        property bool isSelected: false
        signal clicked()

        height: 24
        width: pillText.width + 16
        radius: 12
        color: isSelected ? theme.surfaceElevated : "transparent"
        border.color: isSelected ? theme.accent : theme.borderSubtle
        border.width: 1

        Text {
            id: pillText
            anchors.centerIn: parent
            text: label
            font.pixelSize: 10
            font.bold: isSelected
            color: isSelected ? theme.accent : theme.textMuted
        }

        MouseArea {
            id: pillArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }
}
