import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: root

    width: 360
    implicitHeight: Math.min(480, contentCol.implicitHeight + 24)
    padding: 0
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    background: Rectangle {
        radius: 10
        color: theme.cardBg
        border.color: theme.borderStrong
        border.width: 1
    }

    contentItem: Flickable {
        id: popoverFlick
        contentWidth: width
        contentHeight: contentCol.implicitHeight + 16
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ColumnLayout {
            id: contentCol
            width: popoverFlick.width
            spacing: 12

            // Header
            Rectangle {
                Layout.fillWidth: true
                height: 44
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 14
                    anchors.rightMargin: 12
                    spacing: 8

                    RyzoraIcon {
                        name: "download"
                        size: 16
                        color: theme.accent
                    }

                    Text {
                        text: "Installation Manager"
                        font.pixelSize: 13
                        font.weight: Font.Bold
                        color: theme.text
                    }

                    Rectangle {
                        radius: 10
                        height: 18
                        implicitWidth: activeBadge.implicitWidth + 10
                        color: theme.accent
                        visible: backend && backend.activeAppOperationsCount > 0

                        Text {
                            id: activeBadge
                            anchors.centerIn: parent
                            text: (backend ? backend.activeAppOperationsCount : 0) + " active"
                            font.pixelSize: 10
                            font.weight: Font.Bold
                            color: "#ffffff"
                        }
                    }

                    Item { Layout.fillWidth: true }

                    Rectangle {
                        width: 22
                        height: 22
                        radius: 11
                        color: closeMouse.containsMouse ? theme.surfaceHover : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: "✕"
                            font.pixelSize: 11
                            color: theme.textMuted
                        }

                        MouseArea {
                            id: closeMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.close()
                        }
                    }
                }
            }

            // Section 1: Active Transactions
            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 14
                Layout.rightMargin: 14
                spacing: 8
                visible: backend && backend.activeTransactions && backend.activeTransactions.length > 0

                Text {
                    text: "ACTIVE OPERATIONS"
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                    color: theme.textMuted
                }

                Repeater {
                    model: backend ? backend.activeTransactions : []
                    delegate: Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: itemCol.implicitHeight + 16
                        radius: 8
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            id: itemCol
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 6

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 8

                                RyzoraIcon {
                                    name: "download"
                                    size: 16
                                    color: theme.accent
                                }

                                Text {
                                    text: modelData.app_name || modelData.app_id || "Application"
                                    font.pixelSize: 13
                                    font.weight: Font.DemiBold
                                    color: theme.text
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }

                                Text {
                                    text: (modelData.percentage >= 0 ? modelData.percentage + "%" : "")
                                    font.pixelSize: 12
                                    font.weight: Font.Bold
                                    color: theme.accent
                                    visible: text !== ""
                                }

                                Rectangle {
                                    width: 54
                                    height: 22
                                    radius: 4
                                    color: cancelOpMouse.containsMouse ? (theme.resolvedTheme === "light" ? "#FEE2E2" : "#451A1A") : theme.surfaceHover
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    Text {
                                        anchors.centerIn: parent
                                        text: modelData.stage === "cancelling" ? "..." : "Cancel"
                                        font.pixelSize: 11
                                        color: cancelOpMouse.containsMouse ? "#EF4444" : theme.textSecondary
                                    }

                                    MouseArea {
                                        id: cancelOpMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            if (backend) backend.cancelAppOperation(modelData.app_id)
                                        }
                                    }
                                }
                            }

                            // Progress bar
                            Rectangle {
                                Layout.fillWidth: true
                                height: 4
                                radius: 2
                                color: theme.surfaceHover
                                clip: true

                                Rectangle {
                                    height: parent.height
                                    width: parent.width * Math.max(0, Math.min(100, modelData.percentage >= 0 ? modelData.percentage : 30)) / 100
                                    radius: 2
                                    color: theme.accent
                                }
                            }

                            Text {
                                text: {
                                    if (modelData.bytes_downloaded && modelData.bytes_total) {
                                        var s = modelData.bytes_downloaded + " / " + modelData.bytes_total
                                        if (modelData.download_speed) s += " (" + modelData.download_speed + ")"
                                        return s
                                    }
                                    return modelData.message || (modelData.stage ? modelData.stage.toUpperCase() : "In progress")
                                }
                                font.pixelSize: 11
                                color: theme.textMuted
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                        }
                    }
                }
            }

            // Section 2: Recent Transactions
            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 14
                Layout.rightMargin: 14
                spacing: 8
                visible: backend && backend.recentTransactions && backend.recentTransactions.length > 0

                Text {
                    text: "RECENT OPERATIONS"
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                    color: theme.textMuted
                }

                Repeater {
                    model: backend ? backend.recentTransactions.slice(0, 5) : []
                    delegate: Rectangle {
                        Layout.fillWidth: true
                        height: 38
                        radius: 6
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            spacing: 8

                            RyzoraIcon {
                                name: modelData.stage === "succeeded" ? "check" : (modelData.stage === "cancelled" ? "close" : "alert")
                                size: 14
                                color: modelData.stage === "succeeded" ? "#10B981" : (modelData.stage === "cancelled" ? theme.textMuted : "#EF4444")
                            }

                            Text {
                                text: modelData.app_name || modelData.app_id || "Application"
                                font.pixelSize: 12
                                font.weight: Font.Medium
                                color: theme.text
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Rectangle {
                                radius: 4
                                height: 18
                                implicitWidth: statusLabel.implicitWidth + 8
                                color: modelData.stage === "succeeded"
                                    ? (theme.resolvedTheme === "light" ? "#DCFCE7" : "#064E3B")
                                    : (modelData.stage === "cancelled" ? theme.surfaceHover : (theme.resolvedTheme === "light" ? "#FEE2E2" : "#451A1A"))

                                Text {
                                    id: statusLabel
                                    anchors.centerIn: parent
                                    text: modelData.stage === "succeeded" ? "Completed" : (modelData.stage === "cancelled" ? "Cancelled" : "Failed")
                                    font.pixelSize: 10
                                    font.weight: Font.Bold
                                    color: modelData.stage === "succeeded"
                                        ? "#059669"
                                        : (modelData.stage === "cancelled" ? theme.textMuted : "#EF4444")
                                }
                            }
                        }
                    }
                }
            }

            // Empty State
            Item {
                Layout.fillWidth: true
                height: 80
                visible: (!backend || (!backend.activeTransactions.length && !backend.recentTransactions.length))

                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 4

                    RyzoraIcon {
                        Layout.alignment: Qt.AlignHCenter
                        name: "download"
                        size: 22
                        color: theme.textMuted
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: "No active or recent operations"
                        font.pixelSize: 12
                        color: theme.textMuted
                    }
                }
            }
        }
    }
}
