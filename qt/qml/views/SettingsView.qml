import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    signal launchLabRequested()
    signal launchCreatorRequested()
    signal launchStorageRequested()

    // Canonical settings state (12 fields tracked with backend)
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

    property string currentSection: "preferences" // "preferences" | "about"

    // Storage summary strings
    property string storageCacheStr: "0 B"
    property string storageInstalledStr: "0 B"
    property string storageTotalStr: "0 B"

    // Responsive breakpoint based on actual Settings viewport width (not fixed window size)
    readonly property bool isWide: width >= 900

    // Unsaved changes tracker
    readonly property bool hasUnsavedChanges: {
        var s = backend ? backend.settingsData : null
        if (!s) return false
        if (s["default_release_channel"] !== undefined && s["default_release_channel"] !== defaultReleaseChannel) return true
        if (s["auto_refresh_enabled"] !== undefined && s["auto_refresh_enabled"] !== autoRefreshEnabled) return true
        if (s["auto_refresh_interval_minutes"] !== undefined && s["auto_refresh_interval_minutes"] !== autoRefreshIntervalMinutes) return true
        if (s["notification_level"] !== undefined && s["notification_level"] !== notificationLevel) return true
        if (s["update_notification_policy"] !== undefined && s["update_notification_policy"] !== updateNotificationPolicy) return true
        if (s["integrity_scan_on_startup"] !== undefined && s["integrity_scan_on_startup"] !== integrityScanOnStartup) return true
        if (s["show_unverified_packages"] !== undefined && s["show_unverified_packages"] !== showUnverifiedPackages) return true
        if (s["show_nightly_packages"] !== undefined && s["show_nightly_packages"] !== showNightlyPackages) return true
        if (s["compact_ui"] !== undefined && s["compact_ui"] !== compactUi) return true
        if (s["snapshot_policy"] !== undefined && s["snapshot_policy"] !== snapshotPolicy) return true
        if (s["snapshot_retention_count"] !== undefined && s["snapshot_retention_count"] !== snapshotRetentionCount) return true
        if (s["snapshot_auto_cleanup"] !== undefined && s["snapshot_auto_cleanup"] !== snapshotAutoCleanup) return true
        return false
    }

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
        backend.postNotification("info", "Settings Saved", "Preferences updated and safely persisted.")
    }

    function resetToDefaults() {
        backend.resetSettings()
        backend.postNotification("info", "Settings Reset", "Restored factory preferences.")
    }

    function showAbout() {
        currentSection = "about"
        if (settingsScroll) {
            settingsScroll.contentY = 0
        }
    }

    function showPreferences() {
        currentSection = "preferences"
    }

    function scrollToY(yPos) {
        if (settingsScroll) {
            settingsScroll.contentY = yPos
        }
    }

    // Main Scroll Area (full-width native desktop preferences viewport)
    Flickable {
        id: settingsScroll
        objectName: "settingsScroll"
        anchors.fill: parent
        contentWidth: width
        contentHeight: mainPageCol.implicitHeight + 52
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ScrollBar.vertical: ScrollBar {
            policy: ScrollBar.AsNeeded
            width: 6
            contentItem: Rectangle {
                implicitWidth: 6
                radius: 3
                color: theme.borderStrong
            }
        }

        // Full-Width Content Column with 28px side margins (no artificial max-width cap)
        ColumnLayout {
            id: mainPageCol
            objectName: "settingsMainPageCol"
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.topMargin: 24
            anchors.leftMargin: root.width < 600 ? 16 : 28
            anchors.rightMargin: root.width < 600 ? 16 : 28
            spacing: 24

            // ══════════════════════════════════════════════════════════════════════
            // 1. PAGE HEADER (Title, Subtitle, Actions, Unsaved State)
            // ══════════════════════════════════════════════════════════════════════
            RowLayout {
                Layout.fillWidth: true
                spacing: 16

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    Text {
                        text: "Settings"
                        font.family: theme.fontFamily
                        font.pixelSize: 26
                        font.weight: theme.weightBold
                        color: theme.text
                    }

                    Text {
                        text: "Personalize your Ryzora experience and desktop system preferences."
                        font.family: theme.fontFamily
                        font.pixelSize: 13
                        color: theme.textSecondary
                        Layout.fillWidth: true
                    }
                }

                // Unsaved Changes Indicator Badge
                Rectangle {
                    visible: root.hasUnsavedChanges && root.currentSection === "preferences"
                    height: 28
                    implicitWidth: unsavedRow.implicitWidth + 16
                    radius: 14
                    color: Qt.rgba(245/255, 158/255, 11/255, 0.15)
                    border.color: Qt.rgba(245/255, 158/255, 11/255, 0.40)
                    border.width: 1

                    RowLayout {
                        id: unsavedRow
                        anchors.centerIn: parent
                        spacing: 6
                        Rectangle { width: 6; height: 6; radius: 3; color: "#f59e0b" }
                        Text {
                            text: "Unsaved Changes"
                            font.family: theme.fontFamily
                            font.pixelSize: 11
                            font.weight: theme.weightMedium
                            color: "#f59e0b"
                        }
                    }
                }

                // Top Header Action Buttons
                RowLayout {
                    spacing: 10
                    visible: root.currentSection === "preferences"

                    RyzoraButton {
                        text: "Restore Defaults"
                        iconName: "arrow-undo"
                        variant: "secondary"
                        size: "md"
                        onClicked: root.resetToDefaults()
                    }

                    RyzoraButton {
                        text: "Save Changes"
                        iconName: "checkmark"
                        variant: "primary"
                        size: "md"
                        onClicked: root.saveAll()
                    }
                }
            }

            // ══════════════════════════════════════════════════════════════════════
            // 2. SUB-NAVIGATION BAR (Preferences vs Dedicated About)
            // ══════════════════════════════════════════════════════════════════════
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                // Preferences Segment Button
                Rectangle {
                    id: prefNavBtn
                    objectName: "prefNavBtn"
                    height: 36
                    implicitWidth: prefRow.implicitWidth + 24
                    radius: theme.radiusMd
                    color: root.currentSection === "preferences" ? theme.accent : (prefMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                    border.color: root.currentSection === "preferences" ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: prefRow
                        anchors.centerIn: parent
                        spacing: 8

                        RyzoraIcon {
                            name: "settings"
                            size: 14
                            filled: root.currentSection === "preferences"
                            color: root.currentSection === "preferences" ? "#ffffff" : theme.textSecondary
                        }

                        Text {
                            text: "Preferences"
                            font.family: theme.fontFamily
                            font.pixelSize: 13
                            font.weight: root.currentSection === "preferences" ? theme.weightSemibold : theme.weightMedium
                            color: root.currentSection === "preferences" ? "#ffffff" : theme.text
                        }
                    }

                    MouseArea {
                        id: prefMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.showPreferences()
                    }
                }

                // About Ryzora Dedicated Destination Button
                Rectangle {
                    id: aboutNavBtn
                    objectName: "aboutNavBtn"
                    height: 36
                    implicitWidth: aboutRow.implicitWidth + 24
                    radius: theme.radiusMd
                    color: root.currentSection === "about" ? theme.accent : (aboutMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                    border.color: root.currentSection === "about" ? theme.accent : theme.borderSubtle
                    border.width: 1

                    RowLayout {
                        id: aboutRow
                        anchors.centerIn: parent
                        spacing: 8

                        RyzoraIcon {
                            name: "info"
                            size: 14
                            filled: root.currentSection === "about"
                            color: root.currentSection === "about" ? "#ffffff" : theme.textSecondary
                        }

                        Text {
                            text: "About Ryzora"
                            font.family: theme.fontFamily
                            font.pixelSize: 13
                            font.weight: root.currentSection === "about" ? theme.weightSemibold : theme.weightMedium
                            color: root.currentSection === "about" ? "#ffffff" : theme.text
                        }
                    }

                    MouseArea {
                        id: aboutMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.showAbout()
                    }
                }

                Item { Layout.fillWidth: true }
            }

            // ══════════════════════════════════════════════════════════════════════
            // 3. PREFERENCES CONTENT (macOS 2-Column Responsive Layout)
            // ══════════════════════════════════════════════════════════════════════
            RowLayout {
                id: prefColumnsLayout
                objectName: "preferencesSection"
                Layout.fillWidth: true
                spacing: 20
                visible: root.currentSection === "preferences"

                readonly property real halfColWidth: Math.max(280, Math.floor((mainPageCol.width - spacing) / 2))

                // ── LEFT COLUMN (Balanced 50% width on wide viewports, 100% on narrow) ──
                ColumnLayout {
                    id: leftCol
                    objectName: "settingsLeftCol"
                    Layout.fillWidth: true
                    Layout.preferredWidth: root.isWide ? prefColumnsLayout.halfColWidth : mainPageCol.width
                    Layout.minimumWidth: root.isWide ? prefColumnsLayout.halfColWidth : mainPageCol.width
                    Layout.maximumWidth: root.isWide ? prefColumnsLayout.halfColWidth : mainPageCol.width
                    Layout.alignment: Qt.AlignTop
                    spacing: 20

                    // 1. Appearance Card
                    SettingsCard {
                        title: "Appearance"
                        description: "Visual theme, window frame styling, and desktop system accent."

                        // Theme Mode Segmented Picker
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Text {
                                text: "Theme Mode"
                                font.family: theme.fontFamily
                                font.pixelSize: 14
                                font.weight: theme.weightSemibold
                                color: theme.text
                            }

                            Text {
                                text: "Choose your interface theme. Independent from installed lock screens."
                                font.family: theme.fontFamily
                                font.pixelSize: 12
                                color: theme.textSecondary
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 8

                                ThemeModeOption {
                                    modeId: "system"
                                    label: "System"
                                    iconName: "desktop"
                                    isSelected: theme.appearanceMode === "system"
                                    onClicked: theme.setAppearanceMode("system")
                                }

                                ThemeModeOption {
                                    modeId: "light"
                                    label: "Light"
                                    iconName: "weather-sunny"
                                    isSelected: theme.appearanceMode === "light"
                                    onClicked: theme.setAppearanceMode("light")
                                }

                                ThemeModeOption {
                                    modeId: "dark"
                                    label: "Dark"
                                    iconName: "weather-moon"
                                    isSelected: theme.appearanceMode === "dark"
                                    onClicked: theme.setAppearanceMode("dark")
                                }
                            }
                        }

                        CardDivider {}

                        // Accent Color Swatches
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Text {
                                text: "Accent Color"
                                font.family: theme.fontFamily
                                font.pixelSize: 14
                                font.weight: theme.weightSemibold
                                color: theme.text
                            }

                            Text {
                                text: "Highlight color for primary buttons, active tabs, and focus rings."
                                font.family: theme.fontFamily
                                font.pixelSize: 12
                                color: theme.textSecondary
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10

                                ColorSwatch { swatchKey: "blue"; swatchColor: "#3b82f6"; swatchName: "Blue"; isSelected: theme.accentName === "blue" }
                                ColorSwatch { swatchKey: "purple"; swatchColor: "#a855f7"; swatchName: "Purple"; isSelected: theme.accentName === "purple" }
                                ColorSwatch { swatchKey: "emerald"; swatchColor: "#10b981"; swatchName: "Emerald"; isSelected: theme.accentName === "emerald" }
                                ColorSwatch { swatchKey: "amber"; swatchColor: "#f59e0b"; swatchName: "Amber"; isSelected: theme.accentName === "amber" }
                                ColorSwatch { swatchKey: "rose"; swatchColor: "#f43f5e"; swatchName: "Rose"; isSelected: theme.accentName === "rose" }
                                ColorSwatch { swatchKey: "cyan"; swatchColor: "#06b6d4"; swatchName: "Cyan"; isSelected: theme.accentName === "cyan" }
                            }
                        }
                    }

                    // 2. Downloads & Storage Card
                    SettingsCard {
                        id: storageContentSettingsCard
                        objectName: "storageContentSettingsCard"
                        title: "Downloads & Storage"
                        description: "Local disk space consumption, cache retention, and snapshot backups."

                        SettingRow {
                            label: "Disk Space Consumption"
                            description: "Total local footprint: " + root.storageTotalStr + " (Cache: " + root.storageCacheStr + " · Installed: " + root.storageInstalledStr + ")"

                            RyzoraButton {
                                text: "Open Storage Manager"
                                iconName: "archive"
                                variant: "secondary"
                                size: "sm"
                                onClicked: root.launchStorageRequested()
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Snapshot Restore Points"
                            description: "Automatic btrfs/timeshift snapshots created before transactions."

                            SegmentedControl {
                                options: ["Always Ask", "Auto", "Disabled"]
                                selectedIndex: root.snapshotPolicy === "always" ? 0 : (root.snapshotPolicy === "auto" ? 1 : (root.snapshotPolicy === "disabled" ? 2 : 0))
                                onOptionSelected: function(idx) {
                                    if (idx === 0) root.snapshotPolicy = "always"
                                    else if (idx === 1) root.snapshotPolicy = "auto"
                                    else root.snapshotPolicy = "disabled"
                                }
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Automatic Snapshot Cleanup"
                            description: "Prune oldest snapshot backups when retention count exceeds " + root.snapshotRetentionCount + "."

                            SettingsSwitch {
                                checked: root.snapshotAutoCleanup
                                onToggled: function(c) { root.snapshotAutoCleanup = c }
                            }
                        }
                    }

                    // 3. Display & Behavior Card
                    SettingsCard {
                        title: "Display & Behavior"
                        description: "Window density, startup integrity checks, and notification alerts."

                        SettingRow {
                            label: "Compact Layout Density"
                            description: "Tighter spacing and compact paddings for low-resolution displays."

                            SettingsSwitch {
                                checked: root.compactUi
                                onToggled: function(c) { root.compactUi = c }
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Startup Integrity Scan"
                            description: "Verify SHA-256 package checksums and detect local file drift on launch."

                            SettingsSwitch {
                                checked: root.integrityScanOnStartup
                                onToggled: function(c) { root.integrityScanOnStartup = c }
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Notification Alerts"
                            description: "Desktop toast notifications for package transactions and updates."

                            SegmentedControl {
                                options: ["All", "Warnings Only", "Silent"]
                                selectedIndex: root.notificationLevel === "all" ? 0 : (root.notificationLevel === "warning" ? 1 : 2)
                                onOptionSelected: function(idx) {
                                    if (idx === 0) root.notificationLevel = "all"
                                    else if (idx === 1) root.notificationLevel = "warning"
                                    else root.notificationLevel = "silent"
                                }
                            }
                        }
                    }

                    // 4. Developer Tools & Diagnostics Card
                    SettingsCard {
                        title: "Developer Tools"
                        description: "Simulated desktop environments and package manifest authoring tools."

                        SettingRow {
                            label: "Compatibility Lab"
                            description: "Test lock screens against simulated Ubuntu, Fedora, and Arch environments."

                            RyzoraButton {
                                text: "Launch Lab"
                                iconName: "pulse"
                                variant: "secondary"
                                size: "sm"
                                onClicked: root.launchLabRequested()
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Package Creator"
                            description: "Generate and export verified Ryzora package manifests for custom themes."

                            RyzoraButton {
                                text: "Open Creator"
                                iconName: "wrench"
                                variant: "secondary"
                                size: "sm"
                                onClicked: root.launchCreatorRequested()
                            }
                        }
                    }
                }

                // ── RIGHT COLUMN (Balanced 50% width side-by-side on wide viewports) ──
                ColumnLayout {
                    id: rightCol
                    objectName: "settingsRightCol"
                    Layout.fillWidth: true
                    Layout.preferredWidth: prefColumnsLayout.halfColWidth
                    Layout.minimumWidth: prefColumnsLayout.halfColWidth
                    Layout.maximumWidth: prefColumnsLayout.halfColWidth
                    Layout.alignment: Qt.AlignTop
                    spacing: 20
                    visible: root.isWide

                    // 1. Accessibility Card
                    SettingsCard {
                        title: "Accessibility"
                        description: "Visual comfort, motion intensity, and high-contrast outlines."

                        SettingRow {
                            label: "Reduce Transparency"
                            description: "Replace translucent acrylic overlays with solid opaque surfaces."

                            SettingsSwitch {
                                checked: theme.reduceTransparency
                                onToggled: theme.toggleReduceTransparency()
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Reduce Motion"
                            description: "Disable window animations, carousel transitions, and pulse glows."

                            SettingsSwitch {
                                checked: theme.reduceMotion
                                onToggled: theme.toggleReduceMotion()
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Increase Contrast"
                            description: "Enhance text contrast and add sharper borders around cards and inputs."

                            SettingsSwitch {
                                checked: theme.increaseContrast
                                onToggled: theme.toggleIncreaseContrast()
                            }
                        }
                    }

                    // 2. Repositories & Updates Card
                    SettingsCard {
                        title: "Repositories & Updates"
                        description: "Release channels, automated package sync, and refresh frequency."

                        SettingRow {
                            label: "Default Release Channel"
                            description: "Software release tier for newly subscribed packages and catalogs."

                            SegmentedControl {
                                options: ["Stable", "Beta", "Nightly"]
                                selectedIndex: root.defaultReleaseChannel === "stable" ? 0 : (root.defaultReleaseChannel === "beta" ? 1 : 2)
                                onOptionSelected: function(idx) {
                                    if (idx === 0) root.defaultReleaseChannel = "stable"
                                    else if (idx === 1) root.defaultReleaseChannel = "beta"
                                    else root.defaultReleaseChannel = "nightly"
                                }
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Automatic Repository Refresh"
                            description: "Periodically sync Arch repositories and Flathub indexes in background."

                            SettingsSwitch {
                                checked: root.autoRefreshEnabled
                                onToggled: function(c) { root.autoRefreshEnabled = c }
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Refresh Interval"
                            description: "Background check frequency when automatic sync is enabled."

                            SegmentedControl {
                                options: ["15m", "30m", "1 hour", "2 hours"]
                                selectedIndex: root.autoRefreshIntervalMinutes <= 15 ? 0 : (root.autoRefreshIntervalMinutes <= 30 ? 1 : (root.autoRefreshIntervalMinutes <= 60 ? 2 : 3))
                                onOptionSelected: function(idx) {
                                    if (idx === 0) root.autoRefreshIntervalMinutes = 15
                                    else if (idx === 1) root.autoRefreshIntervalMinutes = 30
                                    else if (idx === 2) root.autoRefreshIntervalMinutes = 60
                                    else root.autoRefreshIntervalMinutes = 120
                                }
                            }
                        }
                    }

                    // 3. Package Providers & Sources Card
                    SettingsCard {
                        title: "Package Sources & Trust"
                        description: "Control community AUR packages, Flathub sandbox sources, and unverified repositories."

                        SettingRow {
                            label: "Show Unverified Packages"
                            description: "Display community-contributed packages that lack an upstream maintainer key."

                            SettingsSwitch {
                                checked: root.showUnverifiedPackages
                                onToggled: function(c) { root.showUnverifiedPackages = c }
                            }
                        }

                        CardDivider {}

                        SettingRow {
                            label: "Show Nightly Builds"
                            description: "Expose bleeding-edge development builds and prerelease testing versions."

                            SettingsSwitch {
                                checked: root.showNightlyPackages
                                onToggled: function(c) { root.showNightlyPackages = c }
                            }
                        }
                    }

                    // 4. Setup & System Configuration Card
                    SettingsCard {
                        title: "Setup & System Configuration"
                        description: "First-run onboarding wizard, package manager checks, and configuration paths."

                        SettingRow {
                            label: "First-Run Experience Wizard"
                            description: "Status: " + ((backend.settingsData && backend.settingsData.setup_completed) ? ("✓ Completed (v" + (backend.settingsData.setup_version || "0.1.0") + ")") : "Not completed") + " · Re-run system checks."

                            RyzoraButton {
                                text: "Reopen Setup Wizard"
                                iconName: "arrow-clockwise"
                                variant: "secondary"
                                size: "sm"
                                onClicked: backend.reopenSetupWizard()
                            }
                        }
                    }
                }
            }

            // ── FALLBACK FOR NARROW SCREENS: Render Right Column Cards Below Left Column Cards ──
            ColumnLayout {
                id: narrowFallbackCol
                Layout.fillWidth: true
                spacing: 20
                visible: !root.isWide && root.currentSection === "preferences"

                // 1. Accessibility Card
                SettingsCard {
                    title: "Accessibility"
                    description: "Visual comfort, motion intensity, and high-contrast outlines."

                    SettingRow {
                        label: "Reduce Transparency"
                        description: "Replace translucent acrylic overlays with solid opaque surfaces."
                        SettingsSwitch { checked: theme.reduceTransparency; onToggled: theme.toggleReduceTransparency() }
                    }
                    CardDivider {}
                    SettingRow {
                        label: "Reduce Motion"
                        description: "Disable window animations, carousel transitions, and pulse glows."
                        SettingsSwitch { checked: theme.reduceMotion; onToggled: theme.toggleReduceMotion() }
                    }
                    CardDivider {}
                    SettingRow {
                        label: "Increase Contrast"
                        description: "Enhance text contrast and add sharper borders around cards and inputs."
                        SettingsSwitch { checked: theme.increaseContrast; onToggled: theme.toggleIncreaseContrast() }
                    }
                }

                // 2. Repositories & Updates Card
                SettingsCard {
                    title: "Repositories & Updates"
                    description: "Release channels, automated package sync, and refresh frequency."

                    SettingRow {
                        label: "Default Release Channel"
                        description: "Software release tier for newly subscribed packages and catalogs."
                        SegmentedControl {
                            options: ["Stable", "Beta", "Nightly"]
                            selectedIndex: root.defaultReleaseChannel === "stable" ? 0 : (root.defaultReleaseChannel === "beta" ? 1 : 2)
                            onOptionSelected: function(idx) {
                                if (idx === 0) root.defaultReleaseChannel = "stable"
                                else if (idx === 1) root.defaultReleaseChannel = "beta"
                                else root.defaultReleaseChannel = "nightly"
                            }
                        }
                    }
                    CardDivider {}
                    SettingRow {
                        label: "Automatic Repository Refresh"
                        description: "Periodically sync Arch repositories and Flathub indexes in background."
                        SettingsSwitch { checked: root.autoRefreshEnabled; onToggled: function(c) { root.autoRefreshEnabled = c } }
                    }
                    CardDivider {}
                    SettingRow {
                        label: "Refresh Interval"
                        description: "Background check frequency when automatic sync is enabled."
                        SegmentedControl {
                            options: ["15m", "30m", "1 hour", "2 hours"]
                            selectedIndex: root.autoRefreshIntervalMinutes <= 15 ? 0 : (root.autoRefreshIntervalMinutes <= 30 ? 1 : (root.autoRefreshIntervalMinutes <= 60 ? 2 : 3))
                            onOptionSelected: function(idx) {
                                if (idx === 0) root.autoRefreshIntervalMinutes = 15
                                else if (idx === 1) root.autoRefreshIntervalMinutes = 30
                                else if (idx === 2) root.autoRefreshIntervalMinutes = 60
                                else root.autoRefreshIntervalMinutes = 120
                            }
                        }
                    }
                }

                // 3. Package Providers & Sources Card
                SettingsCard {
                    title: "Package Sources & Trust"
                    description: "Control community AUR packages, Flathub sandbox sources, and unverified repositories."

                    SettingRow {
                        label: "Show Unverified Packages"
                        description: "Display community-contributed packages that lack an upstream maintainer key."
                        SettingsSwitch { checked: root.showUnverifiedPackages; onToggled: function(c) { root.showUnverifiedPackages = c } }
                    }
                    CardDivider {}
                    SettingRow {
                        label: "Show Nightly Builds"
                        description: "Expose bleeding-edge development builds and prerelease testing versions."
                        SettingsSwitch { checked: root.showNightlyPackages; onToggled: function(c) { root.showNightlyPackages = c } }
                    }
                }

                // 4. Setup & System Configuration Card
                SettingsCard {
                    title: "Setup & System Configuration"
                    description: "First-run onboarding wizard, package manager checks, and configuration paths."

                    SettingRow {
                        label: "First-Run Experience Wizard"
                        description: "Status: " + ((backend.settingsData && backend.settingsData.setup_completed) ? ("✓ Completed (v" + (backend.settingsData.setup_version || "0.1.0") + ")") : "Not completed") + " · Re-run system checks."

                        RyzoraButton {
                            text: "Reopen Setup Wizard"
                            iconName: "arrow-clockwise"
                            variant: "secondary"
                            size: "sm"
                            onClicked: backend.reopenSetupWizard()
                        }
                    }
                }
            }

            // ══════════════════════════════════════════════════════════════════════
            // 4. DEDICATED ABOUT RYZORA DESTINATION
            // ══════════════════════════════════════════════════════════════════════
            ColumnLayout {
                id: aboutSection
                objectName: "aboutSection"
                Layout.fillWidth: true
                spacing: 20
                visible: root.currentSection === "about"

                // 1. Hero Brand Identity Card
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: aboutHeroCol.implicitHeight + 48
                    radius: theme.radiusLg
                    color: theme.surfaceElevated
                    border.color: theme.borderStrong
                    border.width: 1

                    ColumnLayout {
                        id: aboutHeroCol
                        anchors.fill: parent
                        anchors.margins: 24
                        spacing: 20

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 20

                            Image {
                                source: "qrc:/resources/icons/ryzora_logo.svg"
                                width: 64
                                height: 64
                                Layout.preferredWidth: 64
                                Layout.preferredHeight: 64
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 4

                                RowLayout {
                                    spacing: 10

                                    Text {
                                        text: "Ryzora"
                                        font.family: theme.fontFamily
                                        font.pixelSize: 24
                                        font.weight: theme.weightBold
                                        color: theme.text
                                    }

                                    Rectangle {
                                        height: 22
                                        implicitWidth: verChipText.implicitWidth + 14
                                        radius: 11
                                        color: theme.accent
                                        Text {
                                            id: verChipText
                                            anchors.centerIn: parent
                                            text: "v" + (backend && backend.appVersion ? backend.appVersion : "0.1.0")
                                            font.family: theme.fontMono
                                            font.pixelSize: 11
                                            font.bold: true
                                            color: "#ffffff"
                                        }
                                    }

                                    Rectangle {
                                        height: 22
                                        implicitWidth: licChipText.implicitWidth + 14
                                        radius: 11
                                        color: theme.surface
                                        border.color: theme.borderSubtle
                                        border.width: 1
                                        Text {
                                            id: licChipText
                                            anchors.centerIn: parent
                                            text: "MIT License"
                                            font.family: theme.fontFamily
                                            font.pixelSize: 11
                                            font.weight: theme.weightMedium
                                            color: theme.textSecondary
                                        }
                                    }
                                }

                                Text {
                                    text: "Unified Linux Desktop Software Store & System Experience Suite"
                                    font.family: theme.fontFamily
                                    font.pixelSize: 14
                                    font.weight: theme.weightMedium
                                    color: theme.text
                                }

                                Text {
                                    text: "Engineered for Garuda Linux, Arch Linux, and modern Wayland desktops."
                                    font.family: theme.fontFamily
                                    font.pixelSize: 12
                                    color: theme.textMuted
                                }
                            }
                        }

                        Rectangle { Layout.fillWidth: true; height: 1; color: theme.borderSubtle }

                        Text {
                            text: "Ryzora brings an Apple/Microsoft Store discovery experience to Linux. It provides transactional package management across Arch Pacman, AUR, and Flatpak with cryptographic manifest verification, snapshot restore points, and native system customization."
                            font.family: theme.fontFamily
                            font.pixelSize: 13
                            lineHeight: 1.4
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }
                }

                // 2. Specifications & Runtime Environment Card
                SettingsCard {
                    title: "System & Build Specifications"
                    description: "Runtime environment, architecture, and core engine components."

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 14

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Text { text: "CORE ENGINE"; font.family: theme.fontMono; font.pixelSize: 10; font.bold: true; color: theme.textMuted }
                                Text { text: "ryzora-core · ryzora-qt-backend (Rust)"; font.family: theme.fontFamily; font.pixelSize: 13; font.weight: theme.weightMedium; color: theme.text }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Text { text: "UI FRAMEWORK"; font.family: theme.fontMono; font.pixelSize: 10; font.bold: true; color: theme.textMuted }
                                Text { text: "Qt 6 Quick / QML · Fluent Design System"; font.family: theme.fontFamily; font.pixelSize: 13; font.weight: theme.weightMedium; color: theme.text }
                            }
                        }

                        CardDivider {}

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Text { text: "DESKTOP SESSION"; font.family: theme.fontMono; font.pixelSize: 10; font.bold: true; color: theme.textMuted }
                                Text {
                                    text: (backend && backend.systemInfo && backend.systemInfo.desktop) ? backend.systemInfo.desktop : "Desktop Session"
                                    font.family: theme.fontFamily
                                    font.pixelSize: 13
                                    font.weight: theme.weightMedium
                                    color: theme.text
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Text { text: "DISPLAY PROTOCOL"; font.family: theme.fontMono; font.pixelSize: 10; font.bold: true; color: theme.textMuted }
                                Text {
                                    text: ((backend && backend.systemInfo && backend.systemInfo.session_type) ? backend.systemInfo.session_type.toUpperCase() : "WAYLAND") + " (ext-session-lock-v1)"
                                    font.family: theme.fontFamily
                                    font.pixelSize: 13
                                    font.weight: theme.weightMedium
                                    color: theme.text
                                }
                            }
                        }

                        CardDivider {}

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Text { text: "APPLICATION IDENTIFIERS"; font.family: theme.fontMono; font.pixelSize: 10; font.bold: true; color: theme.textMuted }
                                Text { text: "org.ryzora.desktop · com.ryzora.Ryzora"; font.family: theme.fontMono; font.pixelSize: 12; color: theme.textSecondary }
                            }
                        }
                    }
                }

                // 3. Project Links & Community Card
                SettingsCard {
                    title: "Project Links & Source Code"
                    description: "Authoritative repository, issue tracking, and community resources."

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 14

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            RyzoraButton {
                                text: "GitHub Repository"
                                iconName: "open"
                                variant: "primary"
                                size: "md"
                                onClicked: Qt.openUrlExternally("https://github.com/Gayensubhajit/Ryzora")
                            }

                            RyzoraButton {
                                text: "Report an Issue"
                                iconName: "warning"
                                variant: "secondary"
                                size: "md"
                                onClicked: Qt.openUrlExternally("https://github.com/Gayensubhajit/Ryzora/issues")
                            }

                            RyzoraButton {
                                text: "Back to Preferences"
                                iconName: "chevron-left"
                                variant: "tertiary"
                                size: "md"
                                onClicked: root.showPreferences()
                            }

                            Item { Layout.fillWidth: true }
                        }

                        Text {
                            text: "Source code repository: https://github.com/Gayensubhajit/Ryzora"
                            font.family: theme.fontMono
                            font.pixelSize: 11
                            color: theme.textMuted
                        }
                    }
                }
            }

            Item { height: 28 } // Bottom padding
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    // REUSABLE macOS PREFERENCES DESIGN SYSTEM COMPONENTS
    // ══════════════════════════════════════════════════════════════════════

    // 1. SettingsCard: 22px padding, 14px radius, clean title/description header
    component SettingsCard : Rectangle {
        property string title: ""
        property string description: ""
        default property alias content: cardLayout.children

        Layout.fillWidth: true
        implicitHeight: cardCol.implicitHeight + 44
        radius: 14
        color: theme.surfaceElevated
        border.color: theme.borderSubtle
        border.width: 1

        ColumnLayout {
            id: cardCol
            anchors.fill: parent
            anchors.margins: 22
            spacing: 14

            ColumnLayout {
                spacing: 4
                Layout.fillWidth: true
                Text {
                    text: title
                    font.family: theme.fontFamily
                    font.pixelSize: 16
                    font.weight: theme.weightSemibold
                    color: theme.text
                }
                Text {
                    text: description
                    font.family: theme.fontFamily
                    font.pixelSize: 12
                    color: theme.textSecondary
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                    visible: description.length > 0
                }
            }

            ColumnLayout {
                id: cardLayout
                Layout.fillWidth: true
                spacing: 12
            }
        }
    }

    // 2. SettingRow: Left title + subLabel, Right aligned interactive control
    component SettingRow : RowLayout {
        property string label: ""
        property string description: ""
        default property alias rightControl: controlSlot.children

        Layout.fillWidth: true
        spacing: 16

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3
            Text {
                text: label
                font.family: theme.fontFamily
                font.pixelSize: 14
                font.weight: theme.weightMedium
                color: theme.text
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            Text {
                text: description
                font.family: theme.fontFamily
                font.pixelSize: 12
                color: theme.textSecondary
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
                visible: description.length > 0
            }
        }

        Item {
            id: controlSlot
            implicitWidth: childrenRect.width
            implicitHeight: childrenRect.height
            Layout.preferredWidth: childrenRect.width
            Layout.minimumWidth: childrenRect.width
            Layout.fillWidth: false
            Layout.alignment: Qt.AlignVCenter | Qt.AlignRight
        }
    }

    // 3. CardDivider: Fine subtle line between settings rows
    component CardDivider : Rectangle {
        Layout.fillWidth: true
        height: 1
        color: theme.borderSubtle
        opacity: 0.7
    }

    // 4. SettingsSwitch: Native-style smooth sliding switch (42x24)
    component SettingsSwitch : Rectangle {
        id: switchRoot
        property bool checked: false
        signal toggled(bool checked)

        width: 42
        height: 24
        radius: 12
        color: checked ? theme.accent : (switchMouse.containsMouse ? theme.surfaceActive : theme.surfaceElevated)
        border.color: checked ? theme.accent : theme.borderStrong
        border.width: 1

        Behavior on color { ColorAnimation { duration: theme.reduceMotion ? 0 : 120 } }
        Behavior on border.color { ColorAnimation { duration: theme.reduceMotion ? 0 : 120 } }

        Rectangle {
            id: thumb
            width: 18
            height: 18
            radius: 9
            color: "#ffffff"
            anchors.verticalCenter: parent.verticalCenter
            x: switchRoot.checked ? parent.width - width - 3 : 3

            Behavior on x { NumberAnimation { duration: theme.reduceMotion ? 0 : 120; easing.type: Easing.InOutQuad } }
        }

        MouseArea {
            id: switchMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: switchRoot.toggled(!switchRoot.checked)
        }
    }

    // 5. SegmentedControl: macOS-style mutually exclusive segmented button pill
    component SegmentedControl : Rectangle {
        id: segRoot
        property var options: []
        property int selectedIndex: 0
        signal optionSelected(int index)

        height: 32
        implicitWidth: segRow.implicitWidth + 4
        radius: 8
        color: theme.surface
        border.color: theme.borderSubtle
        border.width: 1

        RowLayout {
            id: segRow
            anchors.centerIn: parent
            spacing: 2

            Repeater {
                model: segRoot.options
                delegate: Rectangle {
                    readonly property bool isCur: index === segRoot.selectedIndex
                    height: 28
                    implicitWidth: segItemText.implicitWidth + 16
                    radius: 6
                    color: isCur ? theme.surfaceElevated : (segItemMouse.containsMouse ? theme.surfaceHover : "transparent")
                    border.color: isCur ? theme.borderSubtle : "transparent"
                    border.width: 1

                    Text {
                        id: segItemText
                        anchors.centerIn: parent
                        text: modelData
                        font.family: theme.fontFamily
                        font.pixelSize: 12
                        font.weight: isCur ? theme.weightSemibold : theme.weightMedium
                        color: isCur ? theme.text : theme.textSecondary
                    }

                    MouseArea {
                        id: segItemMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: segRoot.optionSelected(index)
                    }
                }
            }
        }
    }

    // 6. ThemeModeOption: macOS large segmented cards for System / Light / Dark
    component ThemeModeOption : Rectangle {
        property string modeId: ""
        property string label: ""
        property string iconName: ""
        property bool isSelected: false
        signal clicked()

        Layout.fillWidth: true
        height: 52
        radius: 10
        color: isSelected ? Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.12) : (themeMouse.containsMouse ? theme.surfaceHover : theme.surface)
        border.color: isSelected ? theme.accent : (themeMouse.containsMouse ? theme.borderStrong : theme.borderSubtle)
        border.width: isSelected ? 2 : 1

        RowLayout {
            anchors.centerIn: parent
            spacing: 8

            RyzoraIcon {
                name: iconName
                size: 16
                color: isSelected ? theme.accent : theme.textSecondary
            }

            Text {
                text: label
                font.family: theme.fontFamily
                font.pixelSize: 13
                font.weight: isSelected ? theme.weightSemibold : theme.weightMedium
                color: isSelected ? theme.accent : theme.text
            }
        }

        MouseArea {
            id: themeMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }
    }

    // 7. ColorSwatch: 32x32 color circle with checkmark
    component ColorSwatch : Rectangle {
        property string swatchKey: ""
        property string swatchName: ""
        property color swatchColor: "#ffffff"
        property bool isSelected: false

        width: 32
        height: 32
        radius: 16
        color: swatchColor
        border.color: isSelected ? "#ffffff" : "transparent"
        border.width: isSelected ? 2 : 0

        ToolTip.visible: swatchMouse.containsMouse
        ToolTip.text: swatchName
        ToolTip.delay: 300

        RyzoraIcon {
            anchors.centerIn: parent
            name: "checkmark"
            size: 12
            color: "#ffffff"
            visible: parent.isSelected
        }

        MouseArea {
            id: swatchMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: theme.setAccent(swatchKey)
        }
    }
}
