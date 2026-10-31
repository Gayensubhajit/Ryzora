import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    signal backRequested()

    property var storageDetails: ({})
    property bool isRefreshing: false

    // Cleanup Confirmation Modal State
    property bool confirmVisible: false
    property string confirmActionType: ""
    property string confirmTitle: ""
    property string confirmDescription: ""
    property string confirmTarget: ""
    property string confirmReclaimable: ""

    // Collapsible Security Section
    property bool securityExpanded: true

    function refreshStorageDetails() {
        if (!backend) return;
        isRefreshing = true;
        try {
            var raw = backend.getCommunityStorageDetails();
            if (raw) {
                storageDetails = JSON.parse(raw);
            }
        } catch (e) {
            console.warn("Failed to parse community storage details:", e);
        }
        isRefreshing = false;
    }

    Component.onCompleted: {
        root.refreshStorageDetails();
    }

    Connections {
        target: backend
        function onCatalogChanged() {
            if (root.visible) root.refreshStorageDetails();
        }
        function onLibraryChanged() {
            if (root.visible) root.refreshStorageDetails();
        }
    }

    function openConfirm(type, title, desc, target, reclaimable) {
        confirmActionType = type;
        confirmTitle = title;
        confirmDescription = desc;
        confirmTarget = target;
        confirmReclaimable = reclaimable;
        confirmVisible = true;
    }

    function executeConfirmedCleanup() {
        confirmVisible = false;
        var action = confirmActionType;
        if (!backend) return;

        if (action === "unused_downloads") {
            backend.cleanUnusedDownloads();
        } else if (action === "previews") {
            backend.cleanPreviews();
        } else if (action === "temp_data") {
            backend.cleanTemporaryData();
        } else if (action === "clear_cache") {
            backend.clearContentCache();
        }
        root.refreshStorageDetails();
    }

    ScrollView {
        id: mainScroll
        anchors.fill: parent
        anchors.margins: 0
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ScrollBar.vertical.policy: ScrollBar.AsNeeded

        ColumnLayout {
            width: mainScroll.width
            spacing: 24

            // ── Top Header with Back Navigation & Overall Metrics ──
            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 28
                Layout.rightMargin: 28
                Layout.topMargin: 20
                spacing: 12

                // Back Button to Settings
                Rectangle {
                    id: backBtn
                    height: 28
                    width: backRow.implicitWidth + 16
                    radius: 6
                    color: backMouse.containsMouse ? theme.surfaceHover : "transparent"
                    border.color: backMouse.containsMouse ? theme.borderSubtle : "transparent"
                    border.width: 1

                    RowLayout {
                        id: backRow
                        anchors.centerIn: parent
                        spacing: 6
                        Text {
                            text: "←"
                            font.pixelSize: 13
                            font.bold: true
                            color: theme.accent
                        }
                        Text {
                            text: "Back to Settings"
                            font.pixelSize: 12
                            font.bold: true
                            color: theme.accent
                        }
                    }

                    MouseArea {
                        id: backMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.backRequested()
                    }
                }

                // Title Row
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 16

                    ColumnLayout {
                        spacing: 4
                        Layout.fillWidth: true

                        Text {
                            text: "Community Storage Manager"
                            font.pixelSize: 24
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            text: "Inspect local disk consumption, prune disposable preview caches, and clean unreferenced downloads."
                            font.pixelSize: 13
                            color: theme.textSecondary
                        }
                    }

                    // Refresh Button
                    Rectangle {
                        height: 32
                        width: refreshRow.implicitWidth + 18
                        radius: 6
                        color: refreshMouse.containsMouse ? theme.surfaceHover : theme.surfaceActive
                        border.color: theme.borderSubtle
                        border.width: 1

                        RowLayout {
                            id: refreshRow
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon {
                                name: "refresh"
                                size: 13
                                color: theme.text
                            }
                            Text {
                                text: "Refresh"
                                font.pixelSize: 11
                                font.bold: true
                                color: theme.text
                            }
                        }

                        MouseArea {
                            id: refreshMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.refreshStorageDetails()
                        }
                    }

                    // Right Pill: Total Usage
                    Rectangle {
                        height: 32
                        width: totalRow.implicitWidth + 20
                        radius: 6
                        color: theme.surfaceActive
                        border.color: "#10B981"
                        border.width: 1

                        RowLayout {
                            id: totalRow
                            anchors.centerIn: parent
                            spacing: 6
                            RyzoraIcon {
                                name: "shield-check"
                                size: 14
                                color: "#10B981"
                            }
                            Text {
                                text: "Total: " + (root.storageDetails.total_human || "0 B")
                                font.pixelSize: 12
                                font.bold: true
                                color: "#10B981"
                            }
                        }
                    }
                }
            }

            // ── Section 1: Responsive Overview Grid (5 Metric Cards) ──
            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 28
                Layout.rightMargin: 28
                spacing: 10

                Text {
                    text: "STORAGE OVERVIEW"
                    font.pixelSize: 11
                    font.bold: true
                    font.letterSpacing: 0.8
                    color: theme.textMuted
                }

                GridLayout {
                    id: metricsGrid
                    Layout.fillWidth: true
                    columns: {
                        var w = mainScroll.width - 56;
                        if (w >= 1150) return 5;
                        if (w >= 740) return 3;
                        if (w >= 480) return 2;
                        return 1;
                    }
                    rowSpacing: 12
                    columnSpacing: 12

                    // Card 1: Installed Content
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 160
                        implicitHeight: 88
                        radius: 8
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 4

                            RowLayout {
                                spacing: 6
                                RyzoraIcon { name: "package"; size: 14; color: theme.accent }
                                Text {
                                    text: "INSTALLED PACKAGES"
                                    font.pixelSize: 10
                                    font.bold: true
                                    font.letterSpacing: 0.5
                                    color: theme.textMuted
                                }
                            }

                            Text {
                                text: root.storageDetails.installed_customizations_human || "0 B"
                                font.pixelSize: 18
                                font.bold: true
                                color: theme.text
                            }

                            Text {
                                text: (root.storageDetails.installed_customizations_count || 0) + " packages active"
                                font.pixelSize: 11
                                color: theme.textSecondary
                            }
                        }
                    }

                    // Card 2: Download Cache
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 160
                        implicitHeight: 88
                        radius: 8
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 4

                            RowLayout {
                                spacing: 6
                                RyzoraIcon { name: "download"; size: 14; color: "#3B82F6" }
                                Text {
                                    text: "DOWNLOADED ASSETS"
                                    font.pixelSize: 10
                                    font.bold: true
                                    font.letterSpacing: 0.5
                                    color: theme.textMuted
                                }
                            }

                            Text {
                                text: root.storageDetails.downloaded_cache_human || "0 B"
                                font.pixelSize: 18
                                font.bold: true
                                color: theme.text
                            }

                            Text {
                                text: (root.storageDetails.unused_downloads_human || "0 B") + " unreferenced"
                                font.pixelSize: 11
                                color: theme.textSecondary
                            }
                        }
                    }

                    // Card 3: Preview Cache
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 160
                        implicitHeight: 88
                        radius: 8
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 4

                            RowLayout {
                                spacing: 6
                                RyzoraIcon { name: "image"; size: 14; color: "#8B5CF6" }
                                Text {
                                    text: "PREVIEW CACHE"
                                    font.pixelSize: 10
                                    font.bold: true
                                    font.letterSpacing: 0.5
                                    color: theme.textMuted
                                }
                            }

                            Text {
                                text: root.storageDetails.preview_cache_human || "0 B"
                                font.pixelSize: 18
                                font.bold: true
                                color: theme.text
                            }

                            Text {
                                text: "Posters, GIFs & video clips"
                                font.pixelSize: 11
                                color: theme.textSecondary
                            }
                        }
                    }

                    // Card 4: Temporary & Staging
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 160
                        implicitHeight: 88
                        radius: 8
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 4

                            RowLayout {
                                spacing: 6
                                RyzoraIcon { name: "archive"; size: 14; color: "#F59E0B" }
                                Text {
                                    text: "TEMPORARY DATA"
                                    font.pixelSize: 10
                                    font.bold: true
                                    font.letterSpacing: 0.5
                                    color: theme.textMuted
                                }
                            }

                            Text {
                                text: root.storageDetails.temporary_data_human || "0 B"
                                font.pixelSize: 18
                                font.bold: true
                                color: theme.text
                            }

                            Text {
                                text: "Staging & build artifacts"
                                font.pixelSize: 11
                                color: theme.textSecondary
                            }
                        }
                    }

                    // Card 5: Total Community Storage
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 160
                        implicitHeight: 88
                        radius: 8
                        color: theme.surfaceElevated
                        border.color: "#10B981"
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 4

                            RowLayout {
                                spacing: 6
                                RyzoraIcon { name: "shield-check"; size: 14; color: "#10B981" }
                                Text {
                                    text: "TOTAL FOOTPRINT"
                                    font.pixelSize: 10
                                    font.bold: true
                                    font.letterSpacing: 0.5
                                    color: theme.textMuted
                                }
                            }

                            Text {
                                text: root.storageDetails.total_human || "0 B"
                                font.pixelSize: 18
                                font.bold: true
                                color: "#10B981"
                            }

                            Text {
                                text: "~/.local/share/ryzora/"
                                font.pixelSize: 11
                                color: theme.textSecondary
                                elide: Text.ElideMiddle
                            }
                        }
                    }
                }
            }

            // ── Section 2: Storage Cleanup (4 Full-Width Consistently Aligned Rows) ──
            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 28
                Layout.rightMargin: 28
                spacing: 12

                ColumnLayout {
                    spacing: 2
                    Text {
                        text: "STORAGE CLEANUP"
                        font.pixelSize: 11
                        font.bold: true
                        font.letterSpacing: 0.8
                        color: theme.textMuted
                    }
                    Text {
                        text: "Selectively reclaim disk space without corrupting installed customizations or active sessions."
                        font.pixelSize: 12
                        color: theme.textSecondary
                    }
                }

                // Cleanup Row 1: Clean Unused Downloads
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 74
                    radius: 8
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 16
                        anchors.rightMargin: 16
                        spacing: 16

                        Rectangle {
                            width: 38
                            height: 38
                            radius: 8
                            color: Qt.rgba(59/255, 130/255, 246/255, 0.12)
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "download"
                                size: 18
                                color: "#3B82F6"
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: "Clean Unused Downloads"
                                    font.pixelSize: 13
                                    font.bold: true
                                    color: theme.text
                                }
                                Rectangle {
                                    height: 18
                                    width: reclaimDlTxt.implicitWidth + 10
                                    radius: 4
                                    color: theme.surfaceActive
                                    Text {
                                        id: reclaimDlTxt
                                        anchors.centerIn: parent
                                        text: "Reclaimable: " + (root.storageDetails.unused_downloads_human || "0 B")
                                        font.pixelSize: 10
                                        font.bold: true
                                        color: "#3B82F6"
                                    }
                                }
                            }

                            Text {
                                text: "Remove cached package files and assets that are not referenced by any currently installed customization."
                                font.pixelSize: 11
                                color: theme.textSecondary
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        Rectangle {
                            id: cleanUnusedBtn
                            objectName: "cleanUnusedDownloadsButton"
                            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                            height: 34
                            width: 154
                            radius: 6
                            color: cleanUnusedMouse.containsMouse ? theme.surfaceHover : theme.surfaceActive
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 6
                                Text {
                                    text: "🧹 Clean Downloads"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            MouseArea {
                                id: cleanUnusedMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.openConfirm(
                                        "unused_downloads",
                                        "Clean Unused Downloads",
                                        "safely remove orphaned cached assets from ~/.local/share/ryzora/cache/assets/",
                                        "Cached package archives not tied to installed items",
                                        root.storageDetails.unused_downloads_human || "0 B"
                                    );
                                }
                            }
                        }
                    }
                }

                // Cleanup Row 2: Clear Preview Cache
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 74
                    radius: 8
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 16
                        anchors.rightMargin: 16
                        spacing: 16

                        Rectangle {
                            width: 38
                            height: 38
                            radius: 8
                            color: Qt.rgba(139/255, 92/255, 246/255, 0.12)
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "image"
                                size: 18
                                color: "#8B5CF6"
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: "Clear Preview & Poster Cache"
                                    font.pixelSize: 13
                                    font.bold: true
                                    color: theme.text
                                }
                                Rectangle {
                                    height: 18
                                    width: reclaimPrevTxt.implicitWidth + 10
                                    radius: 4
                                    color: theme.surfaceActive
                                    Text {
                                        id: reclaimPrevTxt
                                        anchors.centerIn: parent
                                        text: "Reclaimable: " + (root.storageDetails.preview_cache_human || "0 B")
                                        font.pixelSize: 10
                                        font.bold: true
                                        color: "#8B5CF6"
                                    }
                                }
                            }

                            Text {
                                text: "Remove disposable cached preview posters, thumbnails, and preview clips. Previews stream on-demand when browsing."
                                font.pixelSize: 11
                                color: theme.textSecondary
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        Rectangle {
                            id: cleanPrevBtn
                            objectName: "cleanPreviewsButton"
                            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                            height: 34
                            width: 154
                            radius: 6
                            color: cleanPrevMouse.containsMouse ? theme.surfaceHover : theme.surfaceActive
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 6
                                Text {
                                    text: "🖼️ Clear Previews"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            MouseArea {
                                id: cleanPrevMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.openConfirm(
                                        "previews",
                                        "Clear Preview & Poster Cache",
                                        "delete cached posters and video thumbnails in ~/.local/share/ryzora/cache/previews/",
                                        "Disposable preview posters and thumbnails",
                                        root.storageDetails.preview_cache_human || "0 B"
                                    );
                                }
                            }
                        }
                    }
                }

                // Cleanup Row 3: Clean Temporary & Staging Data
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 74
                    radius: 8
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 16
                        anchors.rightMargin: 16
                        spacing: 16

                        Rectangle {
                            width: 38
                            height: 38
                            radius: 8
                            color: Qt.rgba(245/255, 158/255, 11/255, 0.12)
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "archive"
                                size: 18
                                color: "#F59E0B"
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: "Clean Temporary & Staging Data"
                                    font.pixelSize: 13
                                    font.bold: true
                                    color: theme.text
                                }
                                Rectangle {
                                    height: 18
                                    width: reclaimTempTxt.implicitWidth + 10
                                    radius: 4
                                    color: theme.surfaceActive
                                    Text {
                                        id: reclaimTempTxt
                                        anchors.centerIn: parent
                                        text: "Reclaimable: " + (root.storageDetails.temporary_data_human || "0 B")
                                        font.pixelSize: 10
                                        font.bold: true
                                        color: "#F59E0B"
                                    }
                                }
                            }

                            Text {
                                text: "Remove safely identifiable incomplete downloads, extraction directories, and abandoned staging transaction data."
                                font.pixelSize: 11
                                color: theme.textSecondary
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        Rectangle {
                            id: cleanTempBtn
                            objectName: "cleanTemporaryDataButton"
                            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                            height: 34
                            width: 154
                            radius: 6
                            color: cleanTempMouse.containsMouse ? theme.surfaceHover : theme.surfaceActive
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 6
                                Text {
                                    text: "🗑️ Clean Staging"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            MouseArea {
                                id: cleanTempMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.openConfirm(
                                        "temp_data",
                                        "Clean Temporary & Staging Data",
                                        "purge temporary unpack files and incomplete staging directories in ~/.local/share/ryzora/staging/",
                                        "Abandoned staging files and temporary transaction buffers",
                                        root.storageDetails.temporary_data_human || "0 B"
                                    );
                                }
                            }
                        }
                    }
                }

                // Cleanup Row 4: Prune Content Cache
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 74
                    radius: 8
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 16
                        anchors.rightMargin: 16
                        spacing: 16

                        Rectangle {
                            width: 38
                            height: 38
                            radius: 8
                            color: Qt.rgba(16/255, 185/255, 129/255, 0.12)
                            RyzoraIcon {
                                anchors.centerIn: parent
                                name: "refresh"
                                size: 18
                                color: "#10B981"
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3

                            RowLayout {
                                spacing: 8
                                Text {
                                    text: "Prune Content Cache"
                                    font.pixelSize: 13
                                    font.bold: true
                                    color: theme.text
                                }
                                Rectangle {
                                    height: 18
                                    width: reclaimAllTxt.implicitWidth + 10
                                    radius: 4
                                    color: theme.surfaceActive
                                    Text {
                                        id: reclaimAllTxt
                                        anchors.centerIn: parent
                                        text: "Safe Disposable Cache"
                                        font.pixelSize: 10
                                        font.bold: true
                                        color: "#10B981"
                                    }
                                }
                            }

                            Text {
                                text: "Remove verified disposable cache files across assets, previews, and temp data while safeguarding installed packages."
                                font.pixelSize: 11
                                color: theme.textSecondary
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        Rectangle {
                            id: clearCacheBtn
                            objectName: "clearContentCacheButton"
                            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                            height: 34
                            width: 154
                            radius: 6
                            color: clearCacheMouse.containsMouse ? theme.surfaceHover : theme.surfaceActive
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 6
                                Text {
                                    text: "♻️ Prune Cache"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            MouseArea {
                                id: clearCacheMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.openConfirm(
                                        "clear_cache",
                                        "Prune Entire Content Cache",
                                        "clear disposable cached content in ~/.local/share/ryzora/cache/",
                                        "Disposable cached downloads, previews, and staging files",
                                        "Disposable cache contents"
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // ── Section 3: Compact Collapsible Security Section ──
            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 28
                Layout.rightMargin: 28
                spacing: 8

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: secCol.implicitHeight + 20
                    radius: 8
                    color: theme.surfaceElevated
                    border.color: theme.borderSubtle
                    border.width: 1

                    ColumnLayout {
                        id: secCol
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 8

                        // Clickable header to toggle
                        Item {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 24

                            RowLayout {
                                anchors.fill: parent
                                spacing: 8

                                RyzoraIcon { name: "shield-check"; size: 15; color: "#10B981" }

                                Text {
                                    text: "CONTENT SAFETY, SHA-256 INTEGRITY & OFFLINE GUARANTEES"
                                    font.pixelSize: 11
                                    font.bold: true
                                    font.letterSpacing: 0.5
                                    color: theme.text
                                    Layout.fillWidth: true
                                }

                                Text {
                                    text: root.securityExpanded ? "▲ Collapse" : "▼ Expand"
                                    font.pixelSize: 11
                                    font.bold: true
                                    color: theme.accent
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.securityExpanded = !root.securityExpanded
                            }
                        }

                        // Collapsible details
                        ColumnLayout {
                            Layout.fillWidth: true
                            visible: root.securityExpanded
                            spacing: 6

                            Text {
                                text: "• Rootless User-Space Isolation: Community packages and cached content reside entirely within your user home directory (~/.local/share/ryzora/). No root access or /usr mutations are ever performed for community content.\n• Cryptographic SHA-256 Verification: Assets are integrity-checked against expected checksums prior to final deployment.\n• Offline Availability: Installed desktop themes, lockscreens, and rices are cached locally and remain 100% operational without network connectivity.\n• Selective Cleanup Protection: Shared files and active lockscreen resources referenced by installed package manifests are never removed by cleanup routines."
                                font.pixelSize: 11
                                color: theme.textSecondary
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                                lineHeight: 1.35
                            }
                        }
                    }
                }
            }

            Item { height: 28 }
        }
    }

    // ── Cleanup Confirmation Modal Dialog ──
    Rectangle {
        id: modalScrim
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, 0.60)
        visible: root.confirmVisible
        z: 99

        MouseArea {
            anchors.fill: parent
            onClicked: {} // prevent click-through
        }

        Rectangle {
            id: confirmModal
            objectName: "storageCleanupConfirmModal"
            anchors.centerIn: parent
            width: Math.min(520, parent.width - 40)
            implicitHeight: modalCol.implicitHeight + 36
            radius: 12
            color: theme.surfaceElevated
            border.color: theme.borderStrong
            border.width: 1

            ColumnLayout {
                id: modalCol
                anchors.fill: parent
                anchors.margins: 20
                spacing: 16

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    RyzoraIcon {
                        name: "alert"
                        size: 20
                        color: "#F59E0B"
                    }

                    Text {
                        text: "Confirm Cleanup"
                        font.pixelSize: 16
                        font.bold: true
                        color: theme.text
                        Layout.fillWidth: true
                    }
                }

                Text {
                    text: "Are you sure you want to " + root.confirmDescription + "?"
                    font.pixelSize: 13
                    color: theme.text
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: detailsCol.implicitHeight + 16
                    radius: 8
                    color: theme.surfaceActive
                    border.color: theme.borderSubtle
                    border.width: 1

                    ColumnLayout {
                        id: detailsCol
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 4

                        Text {
                            text: "Target: " + root.confirmTarget
                            font.pixelSize: 11
                            color: theme.textSecondary
                        }
                        Text {
                            text: "Estimated space to reclaim: " + root.confirmReclaimable
                            font.pixelSize: 11
                            font.bold: true
                            color: "#10B981"
                        }
                        Text {
                            text: "Installed packages and active desktop configurations will not be affected."
                            font.pixelSize: 10
                            color: theme.textMuted
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    Item { Layout.fillWidth: true }

                    // Cancel Button
                    Rectangle {
                        height: 34
                        width: 90
                        radius: 6
                        color: cancelModalMouse.containsMouse ? theme.surfaceHover : "transparent"
                        border.color: theme.borderSubtle
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: "Cancel"
                            font.pixelSize: 12
                            font.bold: true
                            color: theme.text
                        }

                        MouseArea {
                            id: cancelModalMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.confirmVisible = false
                        }
                    }

                    // Confirm & Clean Button
                    Rectangle {
                        id: confirmActionBtn
                        objectName: "confirmCleanupActionButton"
                        height: 34
                        width: 140
                        radius: 6
                        color: confirmModalMouse.containsMouse ? "#DC2626" : "#EF4444"

                        Text {
                            anchors.centerIn: parent
                            text: "Confirm & Clean"
                            font.pixelSize: 12
                            font.bold: true
                            color: "#ffffff"
                        }

                        MouseArea {
                            id: confirmModalMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.executeConfirmedCleanup()
                        }
                    }
                }
            }
        }
    }
}
