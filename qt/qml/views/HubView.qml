import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Flickable {
    id: root
    contentWidth: width
    contentHeight: contentCol.implicitHeight + 40
    clip: true
    boundsBehavior: Flickable.StopAtBounds

    signal navigateRequested(string tabName)
    signal packageSelected(var packageData)

    // First real catalog item as featured
    readonly property var featuredPackage: (backend.catalogList && backend.catalogList.length > 0) ? backend.catalogList[0] : null

    ColumnLayout {
        id: contentCol
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: 20
        spacing: 18

        // ── 1. Hub Welcome Banner ──
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: bannerLayout.implicitHeight + 32
            radius: theme.radiusLg
            color: theme.surface
            border.color: theme.border
            border.width: 1

            RowLayout {
                id: bannerLayout
                anchors.fill: parent
                anchors.margins: 20
                spacing: 16

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    RowLayout {
                        spacing: 8
                        Text {
                            text: "🌐"
                            font.pixelSize: 18
                        }
                        Text {
                            text: "Ryzora Hub"
                            font.pixelSize: 20
                            font.bold: true
                            color: theme.text
                        }
                        Rectangle {
                            height: 20
                            width: ecoText.implicitWidth + 12
                            radius: theme.radiusSm
                            color: theme.accentFaint
                            border.color: theme.accent
                            border.width: 1

                            Text {
                                id: ecoText
                                anchors.centerIn: parent
                                text: "Host: " + ((backend.systemInfo && backend.systemInfo.desktop) ? backend.systemInfo.desktop : "Desktop")
                                font.pixelSize: 10
                                font.bold: true
                                color: theme.accent
                            }
                        }
                    }

                    Text {
                        text: "Host state: " + backend.libraryCount + " packages installed, " + backend.snapshotsCount + " restore points recorded, and " + backend.catalogCount + " community catalog items indexed."
                        font.pixelSize: 13
                        color: theme.textSecondary
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                }

                Button {
                    text: "⟳ Refresh Ecosystem"
                    onClicked: {
                        backend.filterCatalog("all", "")
                        backend.loadLibrary(true)
                        backend.loadUpdates()
                        backend.loadSnapshots()
                        backend.loadSystemInfo()
                    }
                }
            }
        }

        // ── 2. Real-time Metric Stat Cards (Data-Bound) ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            StatCard {
                iconSymbol: "📚"
                label: "Installed Packages"
                value: backend.libraryCount.toString()
                subtext: backend.activeCount > 0 ? (backend.activeCount + " active on host") : "Ready in library"
                onClicked: root.navigateRequested("library")
            }

            StatCard {
                iconSymbol: "⬆️"
                label: "Available Updates"
                value: backend.updatesCount.toString()
                subtext: backend.updatesCount > 0 ? "Pending upstream updates" : "All packages up to date"
                accentColor: backend.updatesCount > 0 ? theme.warning : theme.accent
                onClicked: root.navigateRequested("updates")
            }

            StatCard {
                iconSymbol: "🗄️"
                label: "Catalog Items"
                value: backend.catalogCount.toString()
                subtext: "Ingested from local repository"
                onClicked: root.navigateRequested("discover")
            }

            StatCard {
                iconSymbol: "🛡️"
                label: "System Snapshots"
                value: backend.snapshotsCount.toString()
                subtext: "Verified restore points"
                onClicked: root.navigateRequested("backups")
            }
        }

        // ── 3. Quick Action Navigation Row ──
        Text {
            text: "QUICK ACTIONS"
            font.pixelSize: 11
            font.bold: true
            font.letterSpacing: 1.0
            color: theme.navSectionLabel
            Layout.topMargin: 4
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            ActionCard {
                iconSymbol: "🧭"
                title: "Browse Catalog"
                description: "Explore " + backend.catalogCount + " lock screens, Fastfetch presets and themes."
                buttonText: "Open Discover →"
                onClicked: root.navigateRequested("discover")
            }

            ActionCard {
                iconSymbol: "💻"
                title: "Host Diagnostics"
                description: "Inspect Wayland socket, Hypridle service, and SDDM helper."
                buttonText: "View Diagnostics →"
                onClicked: root.navigateRequested("system")
            }

            ActionCard {
                iconSymbol: "🧪"
                title: "Compatibility Lab"
                description: "Simulate multi-distro configurations (Garuda, Arch, Ubuntu, Fedora)."
                buttonText: "Launch Lab →"
                onClicked: root.navigateRequested("compat_lab")
            }
        }

        // ── 4. Real Featured Package Showcase ──
        Text {
            text: "CATALOG HIGHLIGHT"
            font.pixelSize: 11
            font.bold: true
            font.letterSpacing: 1.0
            color: theme.navSectionLabel
            Layout.topMargin: 4
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 110
            radius: theme.radiusLg
            color: theme.surfaceElevated
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 16

                Rectangle {
                    width: 78
                    height: 78
                    radius: theme.radiusMd
                    color: "#0a0c10"
                    border.color: theme.borderSubtle
                    border.width: 1

                    Text {
                        anchors.centerIn: parent
                        text: (root.featuredPackage && root.featuredPackage.category === "lockscreens") ? "🔒" :
                              (root.featuredPackage && root.featuredPackage.category === "fastfetch") ? "⚡" : "📦"
                        font.pixelSize: 28
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    RowLayout {
                        spacing: 8
                        Text {
                            text: root.featuredPackage ? root.featuredPackage.name : "Loading catalog..."
                            font.pixelSize: 15
                            font.bold: true
                            color: theme.text
                            elide: Text.ElideRight
                        }
                        Rectangle {
                            visible: root.featuredPackage && (root.featuredPackage.author === "Darkkal44" || root.featuredPackage.author === "Ryzora Community")
                            height: 18
                            width: 60
                            radius: 4
                            color: "#33f59e0b"
                            border.color: "#66f59e0b"
                            Text { anchors.centerIn: parent; text: "OFFICIAL"; font.pixelSize: 9; font.bold: true; color: "#fde68a" }
                        }
                    }

                    Text {
                        text: root.featuredPackage ? (root.featuredPackage.description || "Linux desktop customization configuration.") : "Loading package description..."
                        font.pixelSize: 12
                        color: theme.textSecondary
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Text {
                        text: root.featuredPackage ? ("Author: " + (root.featuredPackage.author || "Community") + " · Category: " + root.featuredPackage.category) : ""
                        font.pixelSize: 11
                        color: theme.textMuted
                    }
                }

                Button {
                    text: "Inspect Package Details →"
                    enabled: root.featuredPackage !== null
                    onClicked: {
                        if (root.featuredPackage) {
                            root.packageSelected(root.featuredPackage)
                        }
                    }
                }
            }
        }
    }

    component StatCard : Rectangle {
        id: sc
        property string iconSymbol: ""
        property string label: ""
        property string value: ""
        property string subtext: ""
        property color accentColor: theme.accent
        signal clicked()

        Layout.fillWidth: true
        implicitHeight: 88
        radius: theme.radiusLg
        color: scMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
        border.color: scMouse.containsMouse ? sc.accentColor : theme.borderSubtle
        border.width: 1

        Behavior on border.color { ColorAnimation { duration: 120 } }
        Behavior on color { ColorAnimation { duration: 120 } }

        MouseArea {
            id: scMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: sc.clicked()
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 2

            RowLayout {
                Layout.fillWidth: true
                spacing: 6
                Text { text: sc.iconSymbol; font.pixelSize: 13 }
                Text { text: sc.label; font.pixelSize: 11; font.bold: true; color: theme.textSecondary; Layout.fillWidth: true; elide: Text.ElideRight }
            }

            Text {
                text: sc.value
                font.pixelSize: 18
                font.bold: true
                font.family: "monospace"
                color: theme.text
            }

            Text {
                text: sc.subtext
                font.pixelSize: 10
                color: theme.textMuted
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
        }
    }

    component ActionCard : Rectangle {
        id: ac
        property string iconSymbol: ""
        property string title: ""
        property string description: ""
        property string buttonText: ""
        signal clicked()

        Layout.fillWidth: true
        implicitHeight: 120
        radius: theme.radiusLg
        color: acMouse.containsMouse ? theme.surfaceHover : theme.surface
        border.color: acMouse.containsMouse ? theme.accent : theme.borderSubtle
        border.width: 1

        Behavior on border.color { ColorAnimation { duration: 120 } }

        MouseArea {
            id: acMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: ac.clicked()
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 6

            RowLayout {
                spacing: 8
                Text { text: ac.iconSymbol; font.pixelSize: 16 }
                Text { text: ac.title; font.pixelSize: 13; font.bold: true; color: theme.text }
            }

            Text {
                text: ac.description
                font.pixelSize: 11
                color: theme.textSecondary
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            Text {
                text: ac.buttonText
                font.pixelSize: 11
                font.bold: true
                color: theme.accent
            }
        }
    }
}
