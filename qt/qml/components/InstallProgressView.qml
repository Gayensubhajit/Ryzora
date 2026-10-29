import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    property string appName: "Application"
    property string iconUrl: ""
    property string stage: ""
    property int percentage: -1
    property bool isDeterminate: false
    property string message: ""
    property string bytesDownloaded: ""
    property string bytesTotal: ""
    property string downloadSpeed: ""
    property int currentStep: 0
    property int totalSteps: 0
    property string currentItem: ""
    property string etaStr: ""
    property string diagnosticDetails: ""
    property bool showActivity: false

    signal cancelRequested()

    implicitHeight: mainLayout.implicitHeight + 36
    radius: 12
    color: theme.cardBg
    border.color: theme.borderStrong
    border.width: 1

    ColumnLayout {
        id: mainLayout
        anchors.fill: parent
        anchors.margins: 18
        spacing: 14

        // 1. Header: App Icon + Titles | Percentage + Cancel button
        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            // App Icon thumbnail
            Rectangle {
                width: 44
                height: 44
                radius: 8
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                Image {
                    id: wsIconImg
                    anchors.fill: parent
                    anchors.margins: 4
                    source: {
                        if (!root.iconUrl) return ""
                        var u = root.iconUrl.trim()
                        if (u.indexOf(":/") !== -1 || u.indexOf("qrc:/") === 0 || u.indexOf("data:") === 0) return u
                        if (u.indexOf("/") === 0) return "file://" + u
                        return u
                    }
                    fillMode: Image.PreserveAspectFit
                    smooth: true
                    visible: status === Image.Ready
                }

                RyzoraIcon {
                    anchors.centerIn: parent
                    name: "download"
                    size: 22
                    color: theme.accent
                    visible: wsIconImg.status !== Image.Ready
                }
            }

            // Titles Column
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "Installing " + root.appName
                    font.pixelSize: 16
                    font.weight: Font.Bold
                    color: theme.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                Text {
                    text: {
                        if (root.stage === "cancelling") return "Cancelling installation..."
                        if (root.currentItem !== "" && root.currentItem.indexOf(".") !== -1) {
                            return "Downloading application and required runtime (" + root.currentItem + ")"
                        }
                        if (root.message !== "") return root.message
                        if (root.stage === "resolving") return "Resolving packages and dependencies..."
                        if (root.stage === "downloading") return "Downloading application and dependencies..."
                        if (root.stage === "building") return "Compiling AUR package from source..."
                        if (root.stage === "installing") return "Deploying files and configuring sandbox..."
                        if (root.stage === "finalizing") return "Running post-installation hooks..."
                        return "In progress"
                    }
                    font.pixelSize: 13
                    color: theme.textSecondary
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }

            // Large Percentage Indicator
            Text {
                text: root.isDeterminate && root.percentage >= 0 ? (root.percentage + "%") : ""
                font.pixelSize: 24
                font.weight: Font.Bold
                color: theme.accent
                visible: text !== ""
            }

            // Prominent Cancel Button
            Rectangle {
                id: cancelBtn
                width: 84
                height: 32
                radius: 6
                enabled: root.stage !== "cancelling"
                color: cancelMouse.containsMouse && enabled ? (theme.resolvedTheme === "light" ? "#FEE2E2" : "#451A1A") : theme.surfaceElevated
                border.color: cancelMouse.containsMouse && enabled ? "#EF4444" : theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 5
                    RyzoraIcon {
                        name: "close"
                        size: 13
                        color: cancelMouse.containsMouse && cancelBtn.enabled ? "#EF4444" : theme.textSecondary
                    }
                    Text {
                        text: root.stage === "cancelling" ? "Cancelling..." : "Cancel"
                        font.pixelSize: 12
                        font.weight: Font.Medium
                        color: cancelMouse.containsMouse && cancelBtn.enabled ? "#EF4444" : theme.text
                    }
                }

                MouseArea {
                    id: cancelMouse
                    anchors.fill: parent
                    hoverEnabled: parent.enabled
                    cursorShape: parent.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                    onClicked: root.cancelRequested()
                }
            }
        }

        // 2. Wide Progress Bar (12px tall, full-width)
        Rectangle {
            Layout.fillWidth: true
            height: 12
            radius: 6
            color: theme.surfaceElevated
            clip: true

            // Determinate Progress Bar
            Rectangle {
                visible: root.isDeterminate && root.percentage >= 0
                height: parent.height
                width: parent.width * Math.max(0, Math.min(100, root.percentage)) / 100
                radius: 6
                color: theme.accent

                Behavior on width {
                    NumberAnimation { duration: 200; easing.type: Easing.OutQuad }
                }
            }

            // Indeterminate Smooth Sliding Glow
            Rectangle {
                id: indeterminateBar
                visible: !root.isDeterminate || root.percentage < 0
                height: parent.height
                width: Math.max(60, parent.width * 0.28)
                radius: 6
                color: theme.accent

                SequentialAnimation on x {
                    loops: Animation.Infinite
                    running: root.visible && (!root.isDeterminate || root.percentage < 0)
                    PropertyAnimation {
                        from: -indeterminateBar.width
                        to: root.width
                        duration: 1200
                        easing.type: Easing.InOutQuad
                    }
                }
            }
        }

        // 3. Live Transfer Metrics Row (4 distinct metric columns)
        InstallMetricsRow {
            Layout.fillWidth: true
            downloadedValue: {
                if (root.bytesDownloaded !== "" && root.bytesTotal !== "") {
                    return root.bytesDownloaded + " / " + root.bytesTotal
                }
                if (root.stage === "resolving") return "Checking deps"
                if (root.stage === "building") return "Compiling source"
                if (root.stage === "authenticating") return "Authorizing"
                if (root.stage === "installing") return "Deploying files"
                if (root.stage === "downloading") return root.isDeterminate ? "Calculating…" : "Retrieving…"
                return "—"
            }
            downloadedHelper: {
                if (root.bytesTotal !== "") return "Total package & runtime size"
                if (root.stage === "resolving") return "Package dependency resolution"
                if (root.stage === "building") return "makepkg compilation phase"
                if (root.stage === "authenticating") return "Administrator privilege check"
                if (root.stage === "installing") return "Pacman package deployment"
                return "Current phase status"
            }
            speedValue: {
                if (root.downloadSpeed !== "") return root.downloadSpeed
                if (root.stage === "downloading") return "Measuring…"
                if (root.stage === "building") return "Active build"
                if (root.stage === "resolving") return "Resolving"
                return "—"
            }
            speedHelper: {
                if (root.downloadSpeed !== "") return "Based on real byte transfer"
                if (root.stage === "building") return "Compiling binary objects"
                if (root.stage === "resolving") return "Repository metadata query"
                return "Transfer rate"
            }
            etaValue: {
                if (root.etaStr !== "") return root.etaStr
                if (root.stage === "downloading" && root.downloadSpeed !== "") return "Estimating…"
                return "—"
            }
            etaHelper: root.etaStr !== "" ? "Estimated remaining time" : (root.stage === "downloading" ? "Updated during download" : "Indeterminate phase")
            stepValue: {
                if (root.currentStep > 0 && root.totalSteps > 0) {
                    return "Step " + root.currentStep + " of " + root.totalSteps
                }
                if (root.stage === "resolving") return "Step 1 of 4"
                if (root.stage === "downloading") return "Step 2 of 4"
                if (root.stage === "building") return "Step 3 of 4"
                if (root.stage === "installing") return "Step 4 of 4"
                if (root.stage === "finalizing") return "Finalizing"
                return "In progress"
            }
            stepHelper: {
                if (root.currentItem !== "") return root.currentItem
                if (root.stage === "building") return "Source compilation"
                if (root.stage === "downloading") return "Downloading application"
                if (root.stage === "installing") return "Deploying files"
                return "Current operation phase"
            }
        }

        // 4. Expandable Diagnostics / Activity View
        InstallActivityView {
            Layout.fillWidth: true
            stage: root.stage
            currentItem: root.currentItem
            diagnosticDetails: root.diagnosticDetails
            rawMessage: root.message
            isExpanded: root.showActivity
        }
    }
}
