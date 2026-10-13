import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    property string selectedCategory: "All"
    property string searchQuery: ""

    signal packageSelected(var packageData)
    signal clearSearchRequested()

    // Filter packages from real backend.catalogList
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
                if (!nameMatch && !descMatch && !idMatch) continue
            }

            // 2. Category Filter
            if (cat === "all") {
                res.push(item)
            } else if (cat === "installed") {
                if (item.is_installed) res.push(item)
            } else if (cat === "lock screens") {
                if (item.category === "lockscreens") res.push(item)
            } else if (cat === "fastfetch") {
                if (item.category === "fastfetch") res.push(item)
            } else if (cat === "rices") {
                if (item.category === "rices") res.push(item)
            } else if (cat === "themes") {
                if (item.category === "themes") res.push(item)
            } else if (cat === "wallpapers") {
                if (item.category === "wallpapers") res.push(item)
            }
        }
        return res
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 14

        // ── 1. Page Header ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "Desktop Applications & Packages"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Ingested Ryzora catalog packages (" + root.filteredApps.length + " matching of " + backend.catalogCount + " indexed)"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            Rectangle {
                height: 28
                width: alpmStatusTxt.implicitWidth + 16
                radius: theme.radiusSm
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1

                Text {
                    id: alpmStatusTxt
                    anchors.centerIn: parent
                    text: "ALPM Integration: In Progress (Stage F)"
                    font.pixelSize: 11
                    font.bold: true
                    color: theme.textMuted
                }
            }
        }

        // ── 2. Information Banner Explaining Stage Gap Truthfully ──
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: bannerCol.implicitHeight + 20
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.border
            border.width: 1

            ColumnLayout {
                id: bannerCol
                anchors.fill: parent
                anchors.margins: 14
                spacing: 4

                RowLayout {
                    spacing: 8
                    Text { text: "ℹ️"; font.pixelSize: 14 }
                    Text {
                        text: "Native Pacman / ALPM System Packages Pipeline"
                        font.pixelSize: 13
                        font.bold: true
                        color: theme.text
                    }
                }

                Text {
                    text: "Direct pacman / ALPM system execution from /var/lib/pacman/local/ is scheduled for Stage F system integration. Currently browsing community catalog applications and active desktop configurations with live category and search filtering."
                    font.pixelSize: 12
                    color: theme.textSecondary
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        // ── 3. Category Filter Chips ──
        ScrollView {
            Layout.fillWidth: true
            height: 38
            ScrollBar.horizontal.policy: ScrollBar.AsNeeded
            ScrollBar.vertical.policy: ScrollBar.AlwaysOff

            RowLayout {
                spacing: 8

                Repeater {
                    model: ["All", "Lock Screens", "Fastfetch", "Rices", "Themes", "Wallpapers", "Installed"]
                    delegate: Rectangle {
                        property bool isSelected: root.selectedCategory === modelData
                        implicitWidth: catLabel.implicitWidth + 24
                        implicitHeight: 32
                        radius: theme.radiusMd
                        color: isSelected ? theme.navItemActiveBg : (catMouse.containsMouse ? theme.surfaceHover : theme.surface)
                        border.color: isSelected ? theme.accent : theme.borderSubtle
                        border.width: 1

                        Text {
                            id: catLabel
                            anchors.centerIn: parent
                            text: modelData
                            font.pixelSize: 12
                            font.bold: isSelected
                            color: isSelected ? theme.navItemActiveText : theme.textSecondary
                        }

                        MouseArea {
                            id: catMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.selectedCategory = modelData
                        }
                    }
                }
            }
        }

        // ── 4. Live Applications List ──
        ListView {
            id: appsList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 8
            visible: root.filteredApps.length > 0

            model: root.filteredApps

            delegate: Rectangle {
                width: ListView.view.width
                height: 64
                radius: theme.radiusMd
                color: appMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                border.color: appMouse.containsMouse ? theme.accent : theme.borderSubtle
                border.width: 1

                MouseArea {
                    id: appMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.packageSelected(modelData)
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 14

                    Rectangle {
                        width: 40
                        height: 40
                        radius: theme.radiusSm
                        color: theme.surface
                        border.color: theme.borderSubtle
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: modelData.category === "lockscreens" ? "🔒" :
                                  modelData.category === "fastfetch" ? "⚡" :
                                  modelData.category === "rices" ? "🎨" : "📦"
                            font.pixelSize: 18
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            spacing: 8
                            Text {
                                text: modelData.name || modelData.id
                                font.pixelSize: 13
                                font.bold: true
                                color: theme.text
                            }
                            Rectangle {
                                height: 16
                                width: catTxt.implicitWidth + 8
                                radius: 4
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1
                                Text { id: catTxt; anchors.centerIn: parent; text: (modelData.category || "package").toUpperCase(); font.pixelSize: 9; font.family: "monospace"; color: theme.textMuted }
                            }
                            Text {
                                text: modelData.version ? ("v" + modelData.version) : ""
                                font.pixelSize: 10
                                font.family: "monospace"
                                color: theme.textMuted
                            }
                        }

                        Text {
                            text: modelData.description || "Linux desktop customization configuration."
                            font.pixelSize: 11
                            color: theme.textSecondary
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }

                    Rectangle {
                        height: 24
                        width: instBadge.implicitWidth + 14
                        radius: 12
                        color: modelData.is_active ? "#10b981" : (modelData.is_installed ? "#1e3a2f" : theme.surface)
                        border.color: modelData.is_active ? "#34d399" : (modelData.is_installed ? "#10b981" : theme.borderSubtle)
                        border.width: 1

                        Text {
                            id: instBadge
                            anchors.centerIn: parent
                            text: modelData.is_active ? "✓ Active" : (modelData.is_installed ? "✓ Installed" : "Available")
                            font.pixelSize: 10
                            font.bold: true
                            color: modelData.is_active ? "#ffffff" : (modelData.is_installed ? "#6ee7b7" : theme.textMuted)
                        }
                    }
                }
            }
        }

        // Empty State
        EmptyState {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.filteredApps.length === 0
            title: "No Matching Packages Found"
            message: "No catalog packages match category '" + root.selectedCategory + "'" + (root.searchQuery ? " and search '" + root.searchQuery + "'" : "") + "."
            actionLabel: "Clear Search & Filter"
            onActionClicked: {
                root.selectedCategory = "All"
                root.searchQuery = ""
                root.clearSearchRequested()
            }
        }
    }
}
