import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    signal packageSelected(var packageData)
    signal navigateRequested(string tabName)

    ScrollView {
        anchors.fill: parent
        clip: true
        contentWidth: availableWidth
        ScrollBar.vertical.policy: ScrollBar.AsNeeded

        ColumnLayout {
            width: parent.width
            anchors.margins: 20
            spacing: 16

            // ── 1. Top Header Banner Card ──
            Rectangle {
                id: bannerCard
                Layout.fillWidth: true
                height: Math.max(76, bannerRow.implicitHeight + 28)
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    id: bannerRow
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 16

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true
                        Layout.minimumWidth: 220

                        Text {
                            text: "Installed Packages"
                            font.pixelSize: theme.fontTitle
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            text: backend.libraryCount + " installed configuration" + (backend.libraryCount === 1 ? "" : "s") + " · " + backend.activeCount + " active on system."
                            font.pixelSize: theme.fontCardDesc
                            color: theme.textMuted
                        }
                    }

                    // Collision-Free Explicit Header Actions
                    RowLayout {
                        spacing: 8
                        Layout.alignment: Qt.AlignRight | Qt.AlignVCenter

                        // Check for Updates Button
                        ActionButton {
                            id: btnCheckUpdates
                            iconText: backend.isUpdatesLoading ? "⌛" : "↻"
                            label: backend.isUpdatesLoading ? "Checking…" : "Check for Updates"
                            onClicked: backend.loadUpdates()
                        }

                        // Updates Button
                        ActionButton {
                            id: btnUpdatesNav
                            label: "Updates"
                            onClicked: root.navigateRequested("updates")
                        }

                        // View Backups Button
                        ActionButton {
                            id: btnBackupsNav
                            label: "View Backups"
                            onClicked: root.navigateRequested("backups")
                        }
                    }
                }
            }

            // ── 2. Available Updates Alert (if any updates detected) ──
            Rectangle {
                visible: backend.updatesCount > 0
                Layout.fillWidth: true
                height: 44
                radius: theme.radiusMd
                color: "#1e293b"
                border.color: "#38bdf8"
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 10

                    Text { text: "⬆️"; font.pixelSize: 14 }
                    Text {
                        text: backend.updatesCount + " package update" + (backend.updatesCount === 1 ? "" : "s") + " available. Click to review."
                        font.pixelSize: 12
                        font.bold: true
                        color: "#7dd3fc"
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        height: 26
                        width: 90
                        radius: 4
                        color: "#0369a1"
                        Text {
                            anchors.centerIn: parent
                            text: "Open Updates"
                            font.pixelSize: 11
                            font.bold: true
                            color: "#ffffff"
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.navigateRequested("updates")
                        }
                    }
                }
            }

            // ── 3. Current Configuration Section: Real Host State Driven ──
            // Case A: Real Active Lock Screen Exists
            Rectangle {
                visible: Boolean(backend.activeConfiguration && backend.activeConfiguration.has_active)
                Layout.fillWidth: true
                height: 84
                radius: theme.radiusXl
                color: theme.surfaceElevated
                border.color: theme.borderStrong
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 14

                    Rectangle {
                        width: 36
                        height: 36
                        radius: theme.radiusLg
                        color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.15)
                        border.color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.3)

                        Text {
                            anchors.centerIn: parent
                            text: "✨"
                            font.pixelSize: 16
                        }
                    }

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true

                        RowLayout {
                            spacing: 8
                            Text {
                                text: {
                                    if (!backend.activeConfiguration) return "ACTIVE CONFIGURATION"
                                    var t = backend.activeConfiguration.active_type
                                    return t === "theme" ? "ACTIVE LOCK SCREEN THEME" :
                                           t === "wallpaper" ? "ACTIVE WALLPAPER" : "ACTIVE LOCK SCREEN"
                                }
                                font.pixelSize: 12
                                font.bold: true
                                color: theme.text
                            }

                            // Active Indicator Pill
                            Rectangle {
                                height: 18
                                width: 56
                                radius: 9
                                color: "#10b981"
                                Text {
                                    anchors.centerIn: parent
                                    text: "ACTIVE"
                                    font.pixelSize: 9
                                    font.bold: true
                                    color: "#ffffff"
                                }
                            }

                            // Target Pills
                            Rectangle {
                                visible: Boolean(backend.activeConfiguration && backend.activeConfiguration.session_lock_active)
                                height: 18
                                width: 76
                                radius: 4
                                color: "#312e81"
                                Text {
                                    anchors.centerIn: parent
                                    text: "Session Lock"
                                    font.pixelSize: 9
                                    font.bold: true
                                    color: "#c7d2fe"
                                }
                            }
                            Rectangle {
                                visible: Boolean(backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                height: 18
                                width: 76
                                radius: 4
                                color: "#451a03"
                                Text {
                                    anchors.centerIn: parent
                                    text: "SDDM Login"
                                    font.pixelSize: 9
                                    font.bold: true
                                    color: "#fde68a"
                                }
                            }
                        }

                        Text {
                            text: {
                                if (!backend.activeConfiguration) return ""
                                return backend.activeConfiguration.active_title + " · " + backend.activeConfiguration.description
                            }
                            font.pixelSize: 11
                            color: theme.textMuted
                        }
                    }

                    // Verified Safe Deactivate Action (NEVER UNINSTALLS)
                    Rectangle {
                        height: 30
                        width: deactTxt.implicitWidth + 20
                        radius: theme.radiusSm
                        color: deactArea.containsMouse ? "#3b1219" : theme.surface
                        border.color: deactArea.containsMouse ? "#ef4444" : theme.borderStrong
                        border.width: 1

                        Text {
                            id: deactTxt
                            anchors.centerIn: parent
                            text: "Deactivate"
                            font.pixelSize: 11
                            font.bold: true
                            color: deactArea.containsMouse ? "#f87171" : theme.textMuted
                        }

                        MouseArea {
                            id: deactArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: backend.deactivateTarget("all")
                        }
                    }
                }
            }

            // Case B: Honest Inactive Status (When no target is currently active)
            Rectangle {
                visible: !Boolean(backend.activeConfiguration && backend.activeConfiguration.has_active)
                Layout.fillWidth: true
                height: 52
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 10

                    Text { text: "🔒"; font.pixelSize: 14 }
                    Text {
                        text: "No active lock screen or login theme configured. System default greeter is currently active."
                        font.pixelSize: 11
                        color: theme.textMuted
                        Layout.fillWidth: true
                    }
                    ActionButton {
                        label: "Browse Themes"
                        onClicked: root.navigateRequested("discover")
                    }
                }
            }

            // ── 4. Installed Packages Header & Grid ──
            RowLayout {
                Layout.fillWidth: true
                visible: backend.libraryCount > 0

                Text {
                    text: backend.libraryCount + " installed package" + (backend.libraryCount === 1 ? "" : "s") + (backend.activeCount > 0 ? (" · " + backend.activeCount + " active") : "")
                    font.pixelSize: 12
                    color: theme.textFaint
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: "Click 'History' on any package for historical snapshots & rollback."
                    font.pixelSize: 11
                    color: theme.textMuted
                }
            }

            // Empty State
            Rectangle {
                visible: backend.libraryCount === 0 && !backend.isLibraryLoading
                Layout.fillWidth: true
                height: 220
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1

                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 12

                    Text {
                        text: "📦"
                        font.pixelSize: 32
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Text {
                        text: "No packages currently installed."
                        font.pixelSize: 14
                        font.bold: true
                        color: theme.textMuted
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Rectangle {
                        height: 34
                        width: 160
                        radius: theme.radiusSm
                        color: theme.accent
                        Layout.alignment: Qt.AlignHCenter

                        Text {
                            anchors.centerIn: parent
                            text: "Browse Marketplace"
                            font.pixelSize: 12
                            font.bold: true
                            color: "#ffffff"
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.navigateRequested("discover")
                        }
                    }
                }
            }

            // Flow Grid of Rich PackageCard items matching React grid-cols
            Flow {
                id: cardsGrid
                Layout.fillWidth: true
                spacing: 14
                visible: backend.libraryCount > 0

                Repeater {
                    model: backend.libraryList

                    PackageCard {
                        width: {
                            var avail = cardsGrid.width
                            var cols = avail >= 1100 ? 4 : (avail >= 800 ? 3 : (avail >= 460 ? 2 : 1))
                            return Math.floor((avail - (cols - 1) * 14) / cols)
                        }
                        itemData: modelData
                        onCardClicked: root.packageSelected(modelData)
                        onHistoryClicked: root.navigateRequested("backups")
                    }
                }
            }
        }
    }

    component ActionButton : Rectangle {
        id: actBtn
        property string label: ""
        property string iconText: ""
        signal clicked()

        implicitHeight: 32
        implicitWidth: btnRow.implicitWidth + 24
        Layout.preferredHeight: 32
        Layout.preferredWidth: implicitWidth
        radius: theme.radiusSm
        color: btnMouse.containsMouse ? theme.surfaceElevated : "transparent"
        border.color: theme.borderSubtle
        border.width: 1

        RowLayout {
            id: btnRow
            anchors.centerIn: parent
            spacing: 6

            Text {
                visible: actBtn.iconText !== ""
                text: actBtn.iconText
                font.pixelSize: 12
                color: theme.textSecondary
            }

            Text {
                text: actBtn.label
                font.pixelSize: theme.fontNav
                font.bold: true
                color: btnMouse.containsMouse ? theme.text : theme.textSecondary
            }
        }

        MouseArea {
            id: btnMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: actBtn.clicked()
        }
    }
}
