import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        // ── Header Row ──
        RowLayout {
            Layout.fillWidth: true

            ColumnLayout {
                spacing: 2
                Text {
                    text: "Package Updates (" + backend.updatesCount + ")"
                    font.pixelSize: theme.fontTitle
                    font.bold: true
                    color: theme.text
                }
                Text {
                    text: "Synchronized with local and remote package repositories"
                    font.pixelSize: theme.fontCardDesc
                    color: theme.textMuted
                }
            }

            Item { Layout.fillWidth: true }

            RowLayout {
                spacing: 8

                // Update All Button (Paused pending transactional download pipeline)
                Rectangle {
                    width: 150
                    height: 34
                    radius: theme.radiusMd
                    color: theme.surfaceElevated
                    border.color: theme.border
                    border.width: 1
                    opacity: 0.75

                    Text {
                        anchors.centerIn: parent
                        text: backend.updatesCount > 0 ? "⚡ Update All (Paused)" : "Up to Date"
                        font.pixelSize: theme.fontSm
                        font.bold: true
                        color: theme.textSecondary
                    }

                    MouseArea {
                        id: updateAllMouse
                        anchors.fill: parent
                        enabled: backend.updatesCount > 0
                        hoverEnabled: true
                        cursorShape: Qt.ForbiddenCursor
                        onClicked: {
                            backend.applyAllUpdates()
                        }
                    }
                }

                // Check for Updates / Refresh
                Rectangle {
                    width: 100
                    height: 34
                    radius: theme.radiusMd
                    color: backend.isUpdatesLoading ? theme.surfaceElevated : (refreshArea.containsMouse ? theme.surfaceHover : theme.surface)
                    border.color: theme.border
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6
                        Text {
                            text: backend.isUpdatesLoading ? "Checking…" : "↻ Check"
                            font.pixelSize: theme.fontSm
                            font.bold: true
                            color: theme.text
                        }
                    }

                    MouseArea {
                        id: refreshArea
                        anchors.fill: parent
                        enabled: !backend.isUpdatesLoading
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: backend.loadUpdates()
                    }
                }
            }
        }

        // ── Transactional Pipeline Notice ──
        Rectangle {
            Layout.fillWidth: true
            height: 38
            radius: theme.radiusSm
            color: Qt.rgba(245/255, 158/255, 11/255, 0.12)
            border.color: Qt.rgba(245/255, 158/255, 11/255, 0.35)
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 8

                Text {
                    text: "⚠️"
                    font.pixelSize: 13
                }
                Text {
                    text: "Package upgrading is paused until transactional download and staging verification is fully connected. Metadata-only updates are refused for data safety."
                    font.pixelSize: 11
                    color: "#fbbf24"
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                }
            }
        }

        // ── Updates List ──
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.border
            border.width: 1

            // Empty State
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 8
                visible: backend.updatesCount === 0

                Text {
                    text: "✨"
                    font.pixelSize: 32
                    Layout.alignment: Qt.AlignHCenter
                }
                Text {
                    text: "All Packages Up to Date"
                    font.pixelSize: theme.fontBase
                    font.bold: true
                    color: theme.text
                    Layout.alignment: Qt.AlignHCenter
                }
                Text {
                    text: "No updates detected for installed packages."
                    font.pixelSize: theme.fontSm
                    color: theme.textMuted
                    Layout.alignment: Qt.AlignHCenter
                }
            }

            ListView {
                id: updateList
                anchors.fill: parent
                anchors.margins: 12
                spacing: 10
                clip: true
                visible: backend.updatesCount > 0
                model: backend.updatesList

                delegate: Rectangle {
                    width: updateList.width
                    height: 82
                    radius: theme.radiusMd
                    color: itemMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 14

                        Rectangle {
                            width: 36
                            height: 36
                            radius: theme.radiusMd
                            color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.12)
                            border.color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.3)

                            Text {
                                anchors.centerIn: parent
                                text: "⬆️"
                                font.pixelSize: 16
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: modelData.name
                                    font.pixelSize: theme.fontBase
                                    font.bold: true
                                    color: theme.text
                                }

                                // Category Pill
                                Rectangle {
                                    height: 18
                                    width: catTxt.implicitWidth + 12
                                    radius: theme.radiusSm
                                    color: modelData.category === "Security" ? Qt.rgba(239/255, 68/255, 68/255, 0.20) :
                                           modelData.category === "Feature" ? Qt.rgba(59/255, 130/255, 246/255, 0.20) :
                                           Qt.rgba(100/255, 116/255, 139/255, 0.20)
                                    border.color: modelData.category === "Security" ? Qt.rgba(239/255, 68/255, 68/255, 0.50) :
                                                  modelData.category === "Feature" ? Qt.rgba(59/255, 130/255, 246/255, 0.50) :
                                                  Qt.rgba(100/255, 116/255, 139/255, 0.40)
                                    border.width: 1

                                    Text {
                                        id: catTxt
                                        anchors.centerIn: parent
                                        text: (modelData.category || "Feature").toUpperCase()
                                        font.pixelSize: 9
                                        font.bold: true
                                        color: modelData.category === "Security" ? "#f87171" :
                                               modelData.category === "Feature" ? "#60a5fa" : "#94a3b8"
                                    }
                                }
                            }

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: "Version: " + modelData.installed_version + " → " + modelData.available_version
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: theme.accent
                                }
                                Text { text: "•"; font.pixelSize: 10; color: theme.textFaint }
                                Text {
                                    text: modelData.description || "Package update available"
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }
                        }

                        // Apply Update Action Button (Paused pending pipeline)
                        Rectangle {
                            width: 110
                            height: 30
                            radius: theme.radiusSm
                            color: theme.surfaceElevated
                            border.color: theme.border
                            border.width: 1
                            opacity: 0.75

                            Text {
                                anchors.centerIn: parent
                                text: "Update (Paused)"
                                font.pixelSize: 10
                                font.bold: true
                                color: theme.textSecondary
                            }

                            MouseArea {
                                id: updateMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.ForbiddenCursor
                                onClicked: {
                                    backend.applyUpdate(modelData.package_id)
                                }
                            }
                        }
                    }

                    MouseArea {
                        id: itemMouse
                        anchors.fill: parent
                        hoverEnabled: true
                    }
                }

                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            }
        }
    }
}
