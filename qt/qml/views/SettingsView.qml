import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root
    signal launchLabRequested()
    signal launchCreatorRequested()
    signal launchStorageRequested()

    // Local copy of canonical settings (12 fields)
    property string defaultReleaseChannel: "stable"
    property bool autoRefreshEnabled: true
    property int autoRefreshIntervalMinutes: 60
    property string notificationLevel: "all"
    property string updateNotificationPolicy: "notify"
    property bool integrityScanOnStartup: false
    property bool showUnverifiedPackages: true
    property bool showNightlyPackages: false
    property bool compactUi: false
    property string snapshotPolicy: "ask"
    property int snapshotRetentionCount: 5
    property bool snapshotAutoCleanup: true

    property string storageCacheStr: "0 B"
    property string storageInstalledStr: "0 B"
    property string storageTotalStr: "0 B"

    function refreshStorageInfo() {
        try {
            var raw = backend.getStorageUsage()
            var data = JSON.parse(raw)
            if (data["cache_human"]) storageCacheStr = data["cache_human"]
            if (data["installed_human"]) storageInstalledStr = data["installed_human"]
            if (data["total_human"]) storageTotalStr = data["total_human"]
        } catch (e) {
            console.warn("Failed to parse storage usage:", e)
        }
    }

    function syncFromBackend() {
        var s = backend.settingsData
        if (!s || Object.keys(s).length === 0) return

        if (s["default_release_channel"] !== undefined) defaultReleaseChannel = s["default_release_channel"]
        if (s["auto_refresh_enabled"] !== undefined) autoRefreshEnabled = s["auto_refresh_enabled"]
        if (s["auto_refresh_interval_minutes"] !== undefined) autoRefreshIntervalMinutes = s["auto_refresh_interval_minutes"]
        if (s["notification_level"] !== undefined) notificationLevel = s["notification_level"]
        if (s["update_notification_policy"] !== undefined) updateNotificationPolicy = s["update_notification_policy"]
        if (s["integrity_scan_on_startup"] !== undefined) integrityScanOnStartup = s["integrity_scan_on_startup"]
        if (s["show_unverified_packages"] !== undefined) showUnverifiedPackages = s["show_unverified_packages"]
        if (s["show_nightly_packages"] !== undefined) showNightlyPackages = s["show_nightly_packages"]
        if (s["compact_ui"] !== undefined) compactUi = s["compact_ui"]
        if (s["snapshot_policy"] !== undefined) snapshotPolicy = s["snapshot_policy"]
        if (s["snapshot_retention_count"] !== undefined) snapshotRetentionCount = s["snapshot_retention_count"]
        if (s["snapshot_auto_cleanup"] !== undefined) snapshotAutoCleanup = s["snapshot_auto_cleanup"]
    }

    Connections {
        target: backend
        function onSettingsChanged() {
            root.syncFromBackend()
        }
    }

    Component.onCompleted: {
        backend.loadSettings()
        root.syncFromBackend()
        root.refreshStorageInfo()
    }

    function saveAll() {
        var base = Object.assign({}, backend.settingsData || {})
        var payload = Object.assign(base, {
            "default_release_channel": defaultReleaseChannel,
            "auto_refresh_enabled": autoRefreshEnabled,
            "auto_refresh_interval_minutes": autoRefreshIntervalMinutes,
            "notification_level": notificationLevel,
            "update_notification_policy": updateNotificationPolicy,
            "integrity_scan_on_startup": integrityScanOnStartup,
            "show_unverified_packages": showUnverifiedPackages,
            "show_nightly_packages": showNightlyPackages,
            "compact_ui": compactUi,
            "snapshot_policy": snapshotPolicy,
            "snapshot_retention_count": snapshotRetentionCount,
            "snapshot_auto_cleanup": snapshotAutoCleanup,
            "sidebar_width": base.sidebar_width || 240,
            "sidebar_collapsed": base.sidebar_collapsed || false
        })
        backend.saveSettings(payload)
    }

    function resetToDefaults() {
        backend.resetSettings()
    }

    ScrollView {
        id: settingsScroll
        anchors.fill: parent
        Component.onCompleted: {
            if (Qt.application.arguments.indexOf("--scroll-bottom") !== -1) {
                ScrollBar.vertical.position = 0.55
            }
        }
        contentWidth: availableWidth
        clip: true

        ColumnLayout {
            width: parent.width
            spacing: 24

            // Header Row
            RowLayout {
                Layout.fillWidth: true
                spacing: 16

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    Text {
                        text: "Settings"
                        font.pixelSize: theme.font2xl
                        font.bold: true
                        color: theme.text
                    }

                    Text {
                        text: "Preferences for Ryzora. Security invariants are enforced by the engine and cannot be overridden here."
                        font.pixelSize: theme.fontSm
                        color: theme.textMuted
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                    }
                }

                // Top Action Buttons
                RowLayout {
                    spacing: 8

                    Rectangle {
                        height: 34
                        width: 100
                        radius: theme.radiusMd
                        color: defaultsMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                        border.color: theme.border

                        Text {
                            anchors.centerIn: parent
                            text: "↺ Defaults"
                            font.pixelSize: theme.fontSm
                            font.bold: true
                            color: theme.text
                        }

                        MouseArea {
                            id: defaultsMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.resetToDefaults()
                        }
                    }

                    Rectangle {
                        height: 34
                        width: 90
                        radius: theme.radiusMd
                        color: saveMouse.containsMouse ? theme.accentHover : theme.accent

                        Text {
                            anchors.centerIn: parent
                            text: "✓ Save"
                            font.pixelSize: theme.fontSm
                            font.bold: true
                            color: "#ffffff"
                        }

                        MouseArea {
                            id: saveMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.saveAll()
                        }
                    }
                }
            }

            // ── Section 1: Appearance ──
            SettingsCard {
                title: "Appearance"
                description: "Visual preferences, color palettes, and desktop scheme integration."

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 16

                    // Theme Mode
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text {
                            text: "Theme Mode"
                            font.pixelSize: theme.fontBase
                            font.bold: true
                            color: theme.text
                        }
                        Text {
                            text: "Separated from content packages: Ryzora UI stays readable regardless of installed desktop themes."
                            font.pixelSize: theme.fontSm
                            color: theme.textMuted
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            ThemeCard {
                                Layout.fillWidth: true
                                modeId: "system"
                                label: "System"
                                description: "Follows Linux desktop scheme (" + theme.systemScheme + ")"
                                isSelected: theme.appearanceMode === "system"
                                onClicked: theme.setAppearanceMode("system")
                            }

                            ThemeCard {
                                Layout.fillWidth: true
                                modeId: "light"
                                label: "Light"
                                description: "Clean & luminous translucent Ryzora Glass"
                                isSelected: theme.appearanceMode === "light"
                                onClicked: theme.setAppearanceMode("light")
                            }

                            ThemeCard {
                                Layout.fillWidth: true
                                modeId: "dark"
                                label: "Dark"
                                description: "Midnight glass & deep dark surfaces"
                                isSelected: theme.appearanceMode === "dark"
                                onClicked: theme.setAppearanceMode("dark")
                            }
                        }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    // Accent Color
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: "Accent Color"
                            font.pixelSize: theme.fontBase
                            font.bold: true
                            color: theme.text
                        }
                        Text {
                            text: "Functional focus color for primary buttons, focus rings, links, and active navigation."
                            font.pixelSize: theme.fontSm
                            color: theme.textMuted
                        }

                        Flow {
                            Layout.fillWidth: true
                            spacing: 10

                            AccentButton {
                                swatchKey: "blue"
                                swatchName: "Ocean Blue"
                                swatchColor: "#3478e5"
                                isSelected: theme.accentName === "blue"
                                onClicked: theme.setAccent("blue")
                            }

                            AccentButton {
                                swatchKey: "silver"
                                swatchName: "Deep Silver"
                                swatchColor: "#D9DCE2"
                                isSelected: theme.accentName === "silver"
                                onClicked: theme.setAccent("silver")
                            }

                            AccentButton {
                                swatchKey: "purple"
                                swatchName: "Amethyst"
                                swatchColor: "#8b5cf6"
                                isSelected: theme.accentName === "purple"
                                onClicked: theme.setAccent("purple")
                            }

                            AccentButton {
                                swatchKey: "green"
                                swatchName: "Emerald"
                                swatchColor: "#10b981"
                                isSelected: theme.accentName === "green"
                                onClicked: theme.setAccent("green")
                            }

                            AccentButton {
                                swatchKey: "orange"
                                swatchName: "Amber Fire"
                                swatchColor: "#f97316"
                                isSelected: theme.accentName === "orange"
                                onClicked: theme.setAccent("orange")
                            }
                        }
                    }
                }
            }

            // ── Section 2: Accessibility ──
            SettingsCard {
                title: "Accessibility"
                description: "Display adaptations and ergonomic controls for readability and motion sensitivity."

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    ToggleRow {
                        label: "Reduce Transparency"
                        subLabel: "Replaces frosted translucent glass with solid high-contrast surfaces."
                        checked: theme.reduceTransparency
                        onToggled: theme.toggleReduceTransparency()
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    ToggleRow {
                        label: "Reduce Motion"
                        subLabel: "Disables fluid animations and transitions for instant layout response."
                        checked: theme.reduceMotion
                        onToggled: theme.toggleReduceMotion()
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    ToggleRow {
                        label: "Increase Contrast"
                        subLabel: "Sharpens border outlines and strengthens typographic contrast across all surfaces."
                        checked: theme.increaseContrast
                        onToggled: theme.toggleIncreaseContrast()
                    }
                }
            }

            // ── Section 3: Repositories ──
            SettingsCard {
                title: "Repositories"
                description: "Distribution channels, index synchronization, and polling intervals."

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 14

                    // Release Channel
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text { text: "Default Release Channel"; font.pixelSize: theme.fontBase; font.bold: true; color: theme.text }
                        Text { text: "Applied to newly added repositories."; font.pixelSize: theme.fontSm; color: theme.textMuted }

                        RowLayout {
                            spacing: 8

                            RadioPill {
                                label: "Stable"
                                isSelected: root.defaultReleaseChannel === "stable"
                                onClicked: root.defaultReleaseChannel = "stable"
                            }

                            RadioPill {
                                label: "Beta"
                                isSelected: root.defaultReleaseChannel === "beta"
                                onClicked: root.defaultReleaseChannel = "beta"
                            }

                            RadioPill {
                                label: "Nightly"
                                isSelected: root.defaultReleaseChannel === "nightly"
                                onClicked: root.defaultReleaseChannel = "nightly"
                            }
                        }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    // Automatic Refresh Toggle
                    ToggleRow {
                        label: "Automatic Repository Refresh"
                        subLabel: "Periodically refresh repository indexes in the background."
                        checked: root.autoRefreshEnabled
                        onToggled: function(c) { root.autoRefreshEnabled = c }
                    }

                    // Interval Slider
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6
                        visible: root.autoRefreshEnabled

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "Refresh Interval"; font.pixelSize: theme.fontBase; font.bold: true; color: theme.text }
                            Item { Layout.fillWidth: true }
                            Rectangle {
                                height: 22
                                width: badgeText.implicitWidth + 14
                                radius: theme.radiusSm
                                color: theme.surfaceElevated
                                border.color: theme.border
                                Text {
                                    id: badgeText
                                    anchors.centerIn: parent
                                    text: root.autoRefreshIntervalMinutes + " min"
                                    font.pixelSize: theme.fontXs
                                    font.bold: true
                                    color: theme.accent
                                }
                            }
                        }

                        Text { text: "Minutes between automatic refreshes (15–1440)."; font.pixelSize: theme.fontSm; color: theme.textMuted }

                        Slider {
                            Layout.fillWidth: true
                            from: 15
                            to: 1440
                            stepSize: 15
                            value: root.autoRefreshIntervalMinutes
                            onMoved: root.autoRefreshIntervalMinutes = value
                        }
                    }
                }
            }

            // ── Section 4: Display ──
            SettingsCard {
                title: "Display"
                description: "Interface density and catalog visibility preferences."

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    ToggleRow {
                        label: "Compact UI"
                        subLabel: "Denser layout with smaller spacing."
                        checked: root.compactUi
                        onToggled: function(c) { root.compactUi = c }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    ToggleRow {
                        label: "Show Unverified Packages"
                        subLabel: "Show Community/Unvetted packages in Discover."
                        checked: root.showUnverifiedPackages
                        onToggled: function(c) { root.showUnverifiedPackages = c }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    ToggleRow {
                        label: "Show Nightly Packages"
                        subLabel: "Show packages on the nightly release channel."
                        checked: root.showNightlyPackages
                        onToggled: function(c) { root.showNightlyPackages = c }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    ToggleRow {
                        label: "Compact Sidebar (Icons Only)"
                        subLabel: "Collapse the navigation sidebar by default to maximize content space."
                        checked: (backend.settingsData && backend.settingsData.sidebar_collapsed) || false
                        onToggled: function(c) {
                            var base = Object.assign({}, backend.settingsData || {}, { "sidebar_collapsed": c })
                            backend.saveSettings(base)
                        }
                    }
                }
            }

            // ── Section 5: Notifications ──
            SettingsCard {
                title: "Notifications"
                description: "System alerts, update advisories, and status toasts."

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 14

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text { text: "Notification Level"; font.pixelSize: theme.fontBase; font.bold: true; color: theme.text }
                        Text { text: "Security alerts are always shown regardless of this setting."; font.pixelSize: theme.fontSm; color: theme.textMuted }

                        RowLayout {
                            spacing: 8

                            RadioPill {
                                label: "All"
                                isSelected: root.notificationLevel === "all"
                                onClicked: root.notificationLevel = "all"
                            }

                            RadioPill {
                                label: "Security only"
                                isSelected: root.notificationLevel === "security_only"
                                onClicked: root.notificationLevel = "security_only"
                            }

                            RadioPill {
                                label: "None"
                                isSelected: root.notificationLevel === "none"
                                onClicked: root.notificationLevel = "none"
                            }
                        }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text { text: "Update Notifications"; font.pixelSize: theme.fontBase; font.bold: true; color: theme.text }

                        RowLayout {
                            spacing: 8

                            RadioPill {
                                label: "Show in activity log"
                                isSelected: root.updateNotificationPolicy === "notify"
                                onClicked: root.updateNotificationPolicy = "notify"
                            }

                            RadioPill {
                                label: "Silent"
                                isSelected: root.updateNotificationPolicy === "silent"
                                onClicked: root.updateNotificationPolicy = "silent"
                            }
                        }
                    }
                }
            }

            // ── Section 6: Safety & Recovery ──
            SettingsCard {
                title: "Safety & Recovery"
                description: "Transactional snapshots, state rollback, and startup integrity verification."

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 14

                    // Snapshot Policy
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text { text: "Snapshot Policy"; font.pixelSize: theme.fontBase; font.bold: true; color: theme.text }
                        Text { text: "Control when full system recovery snapshots are created before applying changes."; font.pixelSize: theme.fontSm; color: theme.textMuted }

                        RowLayout {
                            spacing: 8

                            RadioPill {
                                label: "Never"
                                isSelected: root.snapshotPolicy === "never"
                                onClicked: root.snapshotPolicy = "never"
                            }

                            RadioPill {
                                label: "Ask every time (Default)"
                                isSelected: root.snapshotPolicy === "ask"
                                onClicked: root.snapshotPolicy = "ask"
                            }

                            RadioPill {
                                label: "Always"
                                isSelected: root.snapshotPolicy === "always"
                                onClicked: root.snapshotPolicy = "always"
                            }
                        }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    // Snapshot Retention Limit
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "Snapshot Retention Limit"; font.pixelSize: theme.fontBase; font.bold: true; color: theme.text }
                            Item { Layout.fillWidth: true }
                            Rectangle {
                                height: 22
                                width: snapBadge.implicitWidth + 14
                                radius: theme.radiusSm
                                color: theme.surfaceElevated
                                border.color: theme.border
                                Text {
                                    id: snapBadge
                                    anchors.centerIn: parent
                                    text: root.snapshotRetentionCount + " snapshots"
                                    font.pixelSize: theme.fontXs
                                    font.bold: true
                                    color: theme.accent
                                }
                            }
                        }

                        Text { text: "Maximum number of recovery snapshots to retain (1–20)."; font.pixelSize: theme.fontSm; color: theme.textMuted }

                        Slider {
                            Layout.fillWidth: true
                            from: 1
                            to: 20
                            stepSize: 1
                            value: root.snapshotRetentionCount
                            onMoved: root.snapshotRetentionCount = value
                        }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    ToggleRow {
                        label: "Automatic Snapshot Cleanup"
                        subLabel: "Automatically prune older snapshots exceeding the retention limit."
                        checked: root.snapshotAutoCleanup
                        onToggled: function(c) { root.snapshotAutoCleanup = c }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    ToggleRow {
                        label: "Startup Integrity Scan"
                        subLabel: "Run a read-only integrity scan on application start. Off by default to avoid startup latency."
                        checked: root.integrityScanOnStartup
                        onToggled: function(c) { root.integrityScanOnStartup = c }
                    }

                    // Security invariant notice
                    Rectangle {
                        Layout.fillWidth: true
                        height: noteCol.implicitHeight + 20
                        radius: theme.radiusMd
                        color: theme.surfaceElevated
                        border.color: theme.borderSubtle

                        RowLayout {
                            id: noteCol
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Text {
                                text: "🛡️"
                                font.pixelSize: 16
                            }

                            Text {
                                Layout.fillWidth: true
                                text: "Transactional backups of directly modified files are always created to guarantee safe rollback, even when full system snapshots are disabled. Signature verification and trust enforcement remain strictly mandatory."
                                font.pixelSize: theme.fontSm
                                color: theme.textSecondary
                                wrapMode: Text.WordWrap
                            }
                        }
                    }
                }
            }

            // ── Section 7: Developer Diagnostics ──
            SettingsCard {
                title: "Developer Diagnostics"
                description: "Hardware adapter simulation and desktop environment verification."

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 16

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 16

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Text {
                                text: "Universal Linux Compatibility Lab"
                                font.pixelSize: theme.fontBase
                                font.bold: true
                                color: theme.text
                            }

                            Text {
                                text: "Inspect live hardware capabilities and simulate reference Linux environments (Ubuntu GNOME/GDM, Fedora KDE/SDDM, Arch, Debian) against concrete runtime adapters."
                                font.pixelSize: theme.fontSm
                                color: theme.textMuted
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        Rectangle {
                            height: 34
                            width: 120
                            radius: theme.radiusMd
                            color: labMouse.containsMouse ? Qt.rgba(139/255, 92/255, 246/255, 0.25) : theme.surfaceElevated
                            border.color: Qt.rgba(139/255, 92/255, 246/255, 0.40)

                            Text {
                                anchors.centerIn: parent
                                text: "⚗ Launch Lab"
                                font.pixelSize: theme.fontSm
                                font.bold: true
                                color: "#a78bfa"
                            }

                            MouseArea {
                                id: labMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.launchLabRequested()
                                }
                            }
                        }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 16

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Text {
                                text: "Developer Package Creator"
                                font.pixelSize: theme.fontBase
                                font.bold: true
                                color: theme.text
                            }

                            Text {
                                text: "Generate, validate, and export Ryzora package manifests for lock screens, fastfetch configs, and desktop customization suites."
                                font.pixelSize: theme.fontSm
                                color: theme.textMuted
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        Rectangle {
                            height: 34
                            width: 130
                            radius: theme.radiusMd
                            color: creatorMouse.containsMouse ? Qt.rgba(245/255, 158/255, 11/255, 0.25) : theme.surfaceElevated
                            border.color: Qt.rgba(245/255, 158/255, 11/255, 0.40)

                            Text {
                                anchors.centerIn: parent
                                text: "📦 Open Creator"
                                font.pixelSize: theme.fontSm
                                font.bold: true
                                color: "#fbbf24"
                            }

                            MouseArea {
                                id: creatorMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.launchCreatorRequested()
                                }
                            }
                        }
                    }
                }
            }

            // ── Card: Storage & Content ──
            SettingsCard {
                id: storageContentSettingsCard
                objectName: "storageContentSettingsCard"
                title: "Storage & Content"
                description: "Inspect local disk consumption, clean unreferenced downloads, and manage disposable preview cache."

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 16

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            Text {
                                text: "Community Storage & Cache Management"
                                font.pixelSize: theme.fontBase
                                font.bold: true
                                color: theme.text
                            }

                            Text {
                                text: "Local storage footprint: " + root.storageTotalStr + " (Cache: " + root.storageCacheStr + " · Installed: " + root.storageInstalledStr + ")"
                                font.pixelSize: theme.fontSm
                                color: theme.textSecondary
                            }

                            Text {
                                text: "On-demand wallpapers, lockscreen videos, and previews are cached locally in user storage (~/.local/share/ryzora/). Inspect storage breakdown and execute granular cleanups in the dedicated Storage Manager."
                                font.pixelSize: theme.fontXs
                                color: theme.textMuted
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        Rectangle {
                            id: openStorageManagerBtn
                            objectName: "openStorageManagerButton"
                            height: 36
                            width: 176
                            radius: theme.radiusMd
                            color: openStorageMouse.containsMouse ? theme.accentHover : theme.accent

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 6
                                RyzoraIcon {
                                    name: "archive"
                                    size: 14
                                    color: "#ffffff"
                                }
                                Text {
                                    text: "Open Storage Manager"
                                    font.pixelSize: theme.fontSm
                                    font.bold: true
                                    color: "#ffffff"
                                }
                            }

                            MouseArea {
                                id: openStorageMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.launchStorageRequested()
                            }
                        }
                    }
                }
            }

            // ── Card: Setup & First-Run Wizard ──
            SettingsCard {
                title: "Setup & System Configuration"
                description: "Recheck package managers, customize on-demand community content, or review storage directories."

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 16

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            Text {
                                text: "First-Run Experience & Setup Wizard"
                                font.pixelSize: theme.fontBase
                                font.bold: true
                                color: theme.text
                            }

                            Text {
                                text: "Status: " + ((backend.settingsData && backend.settingsData.setup_completed) ? ("✓ Completed (v" + (backend.settingsData.setup_version || "0.1.0") + ")") : "Not completed") + " · Community Content: " + ((backend.settingsData && backend.settingsData.community_content_enabled !== false) ? "On-demand enabled" : "Disabled")
                                font.pixelSize: theme.fontSm
                                color: theme.textSecondary
                            }

                            Text {
                                text: "Storage: " + ((backend.settingsData && backend.settingsData.custom_storage_path) ? backend.settingsData.custom_storage_path : "~/.local/share/ryzora (default)")
                                font.pixelSize: theme.fontXs
                                color: theme.textMuted
                            }
                        }

                        Rectangle {
                            height: 34
                            width: 170
                            radius: theme.radiusMd
                            color: reopenWizardMouse.containsMouse ? Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.25) : theme.surfaceElevated
                            border.color: theme.accent

                            Text {
                                anchors.centerIn: parent
                                text: "↺ Reopen Setup Wizard"
                                font.pixelSize: theme.fontSm
                                font.bold: true
                                color: theme.accent
                            }

                            MouseArea {
                                id: reopenWizardMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    backend.reopenSetupWizard()
                                }
                            }
                        }
                    }
                }
            }

            Item { height: 20 }
        }
    }

    // ── Reusable Component Definitions ──

    component SettingsCard : Rectangle {
        property string title: ""
        property string description: ""
        default property alias content: cardLayout.children

        Layout.fillWidth: true
        height: cardCol.implicitHeight + 28
        radius: theme.radiusMd
        color: theme.surface
        border.color: theme.border

        ColumnLayout {
            id: cardCol
            anchors.fill: parent
            anchors.margins: 16
            spacing: 14

            ColumnLayout {
                spacing: 2
                Text { text: title; font.pixelSize: theme.fontBase; font.bold: true; color: theme.text }
                Text { text: description; font.pixelSize: theme.fontSm; color: theme.textMuted }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

            Item {
                id: cardLayout
                Layout.fillWidth: true
                implicitHeight: childrenRect.height
            }
        }
    }

    component ThemeCard : Rectangle {
        property string modeId: ""
        property string label: ""
        property string description: ""
        property bool isSelected: false
        signal clicked()

        height: 72
        radius: theme.radiusMd
        color: isSelected ? theme.surfaceElevated : theme.surface
        border.color: isSelected ? theme.accent : theme.borderSubtle
        border.width: isSelected ? 1.5 : 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 10
            spacing: 2

            RowLayout {
                Layout.fillWidth: true
                Text {
                    text: label
                    font.pixelSize: theme.fontBase
                    font.bold: true
                    color: isSelected ? theme.accent : theme.text
                }
                Item { Layout.fillWidth: true }
                Rectangle {
                    visible: isSelected
                    width: 16
                    height: 16
                    radius: 8
                    color: theme.accent
                    Text {
                        anchors.centerIn: parent
                        text: "✓"
                        font.pixelSize: 10
                        font.bold: true
                        color: "#ffffff"
                    }
                }
            }

            Text {
                text: description
                font.pixelSize: theme.fontXs
                color: theme.textMuted
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    component AccentButton : Rectangle {
        property string swatchKey: ""
        property string swatchName: ""
        property color swatchColor: "#ffffff"
        property bool isSelected: false
        signal clicked()

        width: 130
        height: 36
        radius: theme.radiusMd
        color: isSelected ? theme.surfaceElevated : theme.surface
        border.color: isSelected ? theme.accent : theme.borderSubtle
        border.width: isSelected ? 1.5 : 1

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            spacing: 8

            Rectangle {
                width: 14
                height: 14
                radius: 7
                color: swatchColor
                border.color: Qt.rgba(255, 255, 255, 0.2)

                Text {
                    anchors.centerIn: parent
                    visible: isSelected
                    text: "✓"
                    font.pixelSize: 9
                    font.bold: true
                    color: (swatchKey === "silver" && theme.resolvedTheme === "dark") ? "#050608" : "#ffffff"
                }
            }

            Text {
                text: swatchName
                font.pixelSize: theme.fontSm
                font.bold: isSelected
                color: isSelected ? theme.text : theme.textSecondary
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    component RadioPill : Rectangle {
        property string label: ""
        property bool isSelected: false
        signal clicked()

        height: 32
        width: pillText.implicitWidth + 24
        radius: theme.radiusSm
        color: isSelected ? theme.accent : theme.surfaceElevated
        border.color: isSelected ? theme.accent : theme.border

        Text {
            id: pillText
            anchors.centerIn: parent
            text: label
            font.pixelSize: theme.fontSm
            font.bold: isSelected
            color: isSelected ? "#ffffff" : theme.text
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    component ToggleRow : RowLayout {
        property string label: ""
        property string subLabel: ""
        property bool checked: false
        signal toggled(bool checked)

        Layout.fillWidth: true
        spacing: 16

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text { text: label; font.pixelSize: theme.fontBase; font.bold: true; color: theme.text }
            Text { text: subLabel; font.pixelSize: theme.fontSm; color: theme.textMuted }
        }

        Rectangle {
            width: 44
            height: 24
            radius: 12
            color: checked ? theme.accent : theme.surfaceElevated
            border.color: checked ? theme.accent : theme.borderStrong

            Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 120 } }

            Rectangle {
                width: 18
                height: 18
                radius: 9
                color: checked ? "#ffffff" : theme.textSecondary
                anchors.verticalCenter: parent.verticalCenter
                x: checked ? parent.width - width - 3 : 3

                Behavior on x { NumberAnimation { duration: theme.reduceMotion ? 0 : 120 } }
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: toggled(!checked)
            }
        }
    }
}
