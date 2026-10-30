import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    visible: false
    enabled: visible
    anchors.fill: parent
    color: "#99000000"
    z: 200

    property int currentStep: 0 // 0: Welcome, 1: System, 2: Experience, 3: Complete
    property bool communityContentEnabled: true
    property string customStoragePath: ""
    property string storageValidationMessage: ""
    property bool storagePathValid: true

    property var systemCompat: null
    property var storageInfo: null

    signal completed()
    signal dismissed()

    function open() {
        currentStep = 0
        refreshData()
        visible = true
    }

    function dismiss() {
        visible = false
        dismissed()
    }

    function refreshData() {
        try {
            var rawCompat = backend.getSystemCompatibility()
            systemCompat = JSON.parse(rawCompat)
        } catch (e) {
            console.warn("Failed to parse system compatibility:", e)
            systemCompat = null
        }

        try {
            var rawStorage = backend.getStorageLocations()
            storageInfo = JSON.parse(rawStorage)
            if (storageInfo && storageInfo.custom_storage_path) {
                customStoragePath = storageInfo.custom_storage_path
            }
            if (storageInfo && storageInfo.community_content_enabled !== undefined) {
                communityContentEnabled = storageInfo.community_content_enabled
            }
        } catch (e) {
            console.warn("Failed to parse storage locations:", e)
            storageInfo = null
        }
    }

    function checkCustomPath(candidate) {
        if (!candidate || candidate.trim() === "") {
            storagePathValid = true
            storageValidationMessage = "Using default storage directory (~/.local/share/ryzora)"
            return
        }
        try {
            var raw = backend.validateStoragePath(candidate.trim())
            var res = JSON.parse(raw)
            if (res.valid) {
                storagePathValid = true
                storageValidationMessage = "✓ Valid directory: " + res.path
            } else {
                storagePathValid = false
                storageValidationMessage = "⚠ " + (res.error || "Invalid directory path")
            }
        } catch (e) {
            storagePathValid = false
            storageValidationMessage = "⚠ Failed to validate path"
        }
    }

    function finishSetup() {
        var payload = {
            "setup_version": "0.1.0",
            "community_content_enabled": communityContentEnabled,
            "custom_storage_path": (customStoragePath.trim() !== "" && storagePathValid) ? customStoragePath.trim() : null
        }
        var ok = backend.completeSetupWizard(JSON.stringify(payload))
        if (ok) {
            root.visible = false
            root.completed()
        }
    }

    MouseArea {
        anchors.fill: parent
        // Modal backdrop consumes clicks; wizard must be explicitly navigated
    }

    Rectangle {
        id: dialogCard
        width: Math.min(680, parent.width - 40)
        height: Math.min(590, parent.height - 40)
        anchors.centerIn: parent
        radius: theme.radiusLg
        color: theme.surfaceElevated
        border.color: theme.borderStrong
        border.width: 1

        MouseArea {
            anchors.fill: parent
            // Prevent click-through
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 28
            spacing: 16

            // ── Top Bar: Step Indicator & Close ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                RowLayout {
                    spacing: 8
                    Layout.fillWidth: true

                    Repeater {
                        model: [
                            { idx: 0, label: "Welcome" },
                            { idx: 1, label: "01 System" },
                            { idx: 2, label: "02 Experience" },
                            { idx: 3, label: "03 Ready" }
                        ]

                        Rectangle {
                            height: 24
                            radius: theme.radiusSm
                            color: root.currentStep === modelData.idx ? theme.accent : (root.currentStep > modelData.idx ? Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.20) : theme.surface)
                            border.color: root.currentStep === modelData.idx ? theme.accent : theme.borderSubtle
                            implicitWidth: stepRow.implicitWidth + 16

                            RowLayout {
                                id: stepRow
                                anchors.centerIn: parent
                                spacing: 4

                                Text {
                                    text: modelData.label
                                    font.pixelSize: theme.fontXs
                                    font.bold: root.currentStep === modelData.idx
                                    color: root.currentStep === modelData.idx ? "#ffffff" : (root.currentStep > modelData.idx ? theme.text : theme.textMuted)
                                }
                            }
                        }
                    }
                }

                // Allow skip during first-run or close when reopened
                Button {
                    text: backend.isFirstRun ? "Skip for now" : "✕"
                    flat: true
                    onClicked: root.dismiss()
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: theme.borderSubtle
            }

            // ── Page Content (Stack) ──
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                // ══════════════════════════════════════════
                // Step 0: Welcome to Ryzora
                // ══════════════════════════════════════════
                ColumnLayout {
                    anchors.fill: parent
                    spacing: 16
                    visible: root.currentStep === 0

                    RowLayout {
                        spacing: 16
                        Layout.fillWidth: true

                        Rectangle {
                            width: 56
                            height: 56
                            radius: theme.radiusLg
                            gradient: Gradient {
                                GradientStop { position: 0.0; color: theme.accent }
                                GradientStop { position: 1.0; color: Qt.darker(theme.accent, 1.4) }
                            }

                            Text {
                                anchors.centerIn: parent
                                text: "R"
                                font.pixelSize: 28
                                font.bold: true
                                font.family: "monospace"
                                color: "#ffffff"
                            }
                        }

                        ColumnLayout {
                            spacing: 4
                            Layout.fillWidth: true

                            Text {
                                text: "Welcome to Ryzora"
                                font.pixelSize: 22
                                font.bold: true
                                color: theme.text
                            }

                            Text {
                                text: "Your applications, customizations, and desktop experience in one place."
                                font.pixelSize: theme.fontSm
                                color: theme.textSecondary
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: theme.borderSubtle
                    }

                    Text {
                        text: "Engineered specifically for Arch Linux and CachyOS. Ryzora provides unified application curation, native Wayland theming, and transactional system safety."
                        font.pixelSize: theme.fontSm
                        color: theme.textMuted
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    // Feature highlights
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        Rectangle {
                            Layout.fillWidth: true
                            height: 60
                            radius: theme.radiusMd
                            color: theme.surface
                            border.color: theme.borderSubtle

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 12

                                Text { text: "📦"; font.pixelSize: 20 }
                                ColumnLayout {
                                    spacing: 2
                                    Layout.fillWidth: true
                                    Text { text: "Applications & Multi-Provider Hub"; font.bold: true; font.pixelSize: theme.fontSm; color: theme.text }
                                    Text { text: "Install and manage Flatpak, Arch native (pacman), and AUR software with unified progress."; font.pixelSize: theme.fontXs; color: theme.textSecondary }
                                }
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 60
                            radius: theme.radiusMd
                            color: theme.surface
                            border.color: theme.borderSubtle

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 12

                                Text { text: "🎨"; font.pixelSize: 20 }
                                ColumnLayout {
                                    spacing: 2
                                    Layout.fillWidth: true
                                    Text { text: "Customizations & Studio"; font.bold: true; font.pixelSize: theme.fontSm; color: theme.text }
                                    Text { text: "Curate SilentSDDM lock screens, Wayland wallpapers, and Fastfetch hardware banners."; font.pixelSize: theme.fontXs; color: theme.textSecondary }
                                }
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 60
                            radius: theme.radiusMd
                            color: theme.surface
                            border.color: theme.borderSubtle

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 12

                                Text { text: "🛡️"; font.pixelSize: 20 }
                                ColumnLayout {
                                    spacing: 2
                                    Layout.fillWidth: true
                                    Text { text: "System Integrity & Rollback"; font.bold: true; font.pixelSize: theme.fontSm; color: theme.text }
                                    Text { text: "Automatic pre-install snapshots and cryptographic manifest validation ensure fail-safe operation."; font.pixelSize: theme.fontXs; color: theme.textSecondary }
                                }
                            }
                        }
                    }

                    Item { Layout.fillHeight: true }

                    RowLayout {
                        Layout.fillWidth: true
                        Item { Layout.fillWidth: true }

                        Rectangle {
                            height: 38
                            width: 140
                            radius: theme.radiusMd
                            color: getStartedMouse.containsMouse ? theme.accentHover : theme.accent

                            Text {
                                anchors.centerIn: parent
                                text: "Get Started →"
                                font.pixelSize: theme.fontSm
                                font.bold: true
                                color: "#ffffff"
                            }

                            MouseArea {
                                id: getStartedMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.currentStep = 1
                            }
                        }
                    }
                }

                // ══════════════════════════════════════════
                // Step 1: System Compatibility & Provider Detection
                // ══════════════════════════════════════════
                ColumnLayout {
                    anchors.fill: parent
                    spacing: 14
                    visible: root.currentStep === 1

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true

                        Text {
                            text: "01 — Check your system"
                            font.pixelSize: 18
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            text: "Ryzora inspects available package managers and system capabilities. Zero root password or privileged escalation is required during onboarding."
                            font.pixelSize: theme.fontXs
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: theme.borderSubtle
                    }

                    // Provider Rows
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        // 1. Pacman
                        Rectangle {
                            Layout.fillWidth: true
                            height: 48
                            radius: theme.radiusMd
                            color: theme.surface
                            border.color: theme.borderSubtle

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 10
                                spacing: 12

                                Text { text: "📦"; font.pixelSize: 18 }
                                ColumnLayout {
                                    spacing: 1
                                    Layout.fillWidth: true
                                    Text { text: "Pacman (Arch / CachyOS)"; font.bold: true; font.pixelSize: theme.fontSm; color: theme.text }
                                    Text { text: (root.systemCompat && root.systemCompat.pacman) ? root.systemCompat.pacman.label : "Native package manager"; font.pixelSize: theme.fontXs; color: theme.textMuted }
                                }
                                Rectangle {
                                    height: 22
                                    radius: theme.radiusSm
                                    property bool isAvail: root.systemCompat && root.systemCompat.pacman && root.systemCompat.pacman.available
                                    color: isAvail ? Qt.rgba(16/255, 185/255, 129/255, 0.15) : Qt.rgba(239/255, 68/255, 68/255, 0.15)
                                    border.color: isAvail ? "#10b981" : "#ef4444"
                                    implicitWidth: pacmanStatusText.implicitWidth + 14

                                    Text {
                                        id: pacmanStatusText
                                        anchors.centerIn: parent
                                        text: (root.systemCompat && root.systemCompat.pacman && root.systemCompat.pacman.available) ? "✓ Available" : "✗ Missing"
                                        font.pixelSize: theme.fontXs
                                        font.bold: true
                                        color: (root.systemCompat && root.systemCompat.pacman && root.systemCompat.pacman.available) ? "#34d399" : "#f87171"
                                    }
                                }
                            }
                        }

                        // 2. Flatpak
                        Rectangle {
                            Layout.fillWidth: true
                            height: 48
                            radius: theme.radiusMd
                            color: theme.surface
                            border.color: theme.borderSubtle

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 10
                                spacing: 12

                                Text { text: "🌐"; font.pixelSize: 18 }
                                ColumnLayout {
                                    spacing: 1
                                    Layout.fillWidth: true
                                    Text { text: "Flatpak & Flathub"; font.bold: true; font.pixelSize: theme.fontSm; color: theme.text }
                                    Text { text: (root.systemCompat && root.systemCompat.flatpak) ? root.systemCompat.flatpak.label : "Flathub container runtime"; font.pixelSize: theme.fontXs; color: theme.textMuted }
                                }
                                Rectangle {
                                    height: 22
                                    radius: theme.radiusSm
                                    property bool isAvail: root.systemCompat && root.systemCompat.flatpak && root.systemCompat.flatpak.available
                                    color: isAvail ? Qt.rgba(16/255, 185/255, 129/255, 0.15) : Qt.rgba(107/255, 114/255, 128/255, 0.15)
                                    border.color: isAvail ? "#10b981" : "#6b7280"
                                    implicitWidth: flatpakStatusText.implicitWidth + 14

                                    Text {
                                        id: flatpakStatusText
                                        anchors.centerIn: parent
                                        text: (root.systemCompat && root.systemCompat.flatpak && root.systemCompat.flatpak.available) ? "✓ Available" : "Optional"
                                        font.pixelSize: theme.fontXs
                                        font.bold: true
                                        color: (root.systemCompat && root.systemCompat.flatpak && root.systemCompat.flatpak.available) ? "#34d399" : theme.textMuted
                                    }
                                }
                            }
                        }

                        // 3. AUR Helper
                        Rectangle {
                            Layout.fillWidth: true
                            height: 48
                            radius: theme.radiusMd
                            color: theme.surface
                            border.color: theme.borderSubtle

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 10
                                spacing: 12

                                Text { text: "⚡"; font.pixelSize: 18 }
                                ColumnLayout {
                                    spacing: 1
                                    Layout.fillWidth: true
                                    Text { text: "AUR Helper (paru / yay)"; font.bold: true; font.pixelSize: theme.fontSm; color: theme.text }
                                    Text { text: (root.systemCompat && root.systemCompat.aur) ? root.systemCompat.aur.label : "Arch User Repository build helper"; font.pixelSize: theme.fontXs; color: theme.textMuted }
                                }
                                Rectangle {
                                    height: 22
                                    radius: theme.radiusSm
                                    property bool isAvail: root.systemCompat && root.systemCompat.aur && root.systemCompat.aur.available
                                    color: isAvail ? Qt.rgba(16/255, 185/255, 129/255, 0.15) : Qt.rgba(107/255, 114/255, 128/255, 0.15)
                                    border.color: isAvail ? "#10b981" : "#6b7280"
                                    implicitWidth: aurStatusText.implicitWidth + 14

                                    Text {
                                        id: aurStatusText
                                        anchors.centerIn: parent
                                        text: (root.systemCompat && root.systemCompat.aur && root.systemCompat.aur.available) ? ("✓ " + root.systemCompat.aur.helper) : "Optional"
                                        font.pixelSize: theme.fontXs
                                        font.bold: true
                                        color: (root.systemCompat && root.systemCompat.aur && root.systemCompat.aur.available) ? "#34d399" : theme.textMuted
                                    }
                                }
                            }
                        }

                        // 4. Polkit & Display Server
                        Rectangle {
                            Layout.fillWidth: true
                            height: 48
                            radius: theme.radiusMd
                            color: theme.surface
                            border.color: theme.borderSubtle

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 10
                                spacing: 12

                                Text { text: "🖥️"; font.pixelSize: 18 }
                                ColumnLayout {
                                    spacing: 1
                                    Layout.fillWidth: true
                                    Text { text: "Desktop & Authorization"; font.bold: true; font.pixelSize: theme.fontSm; color: theme.text }
                                    Text { text: (root.systemCompat && root.systemCompat.display_server) ? root.systemCompat.display_server.label : "Wayland / Polkit environment"; font.pixelSize: theme.fontXs; color: theme.textMuted }
                                }
                                Rectangle {
                                    height: 22
                                    radius: theme.radiusSm
                                    color: Qt.rgba(99/255, 102/255, 241/255, 0.15)
                                    border.color: "#6366f1"
                                    implicitWidth: polkitStatusText.implicitWidth + 14

                                    Text {
                                        id: polkitStatusText
                                        anchors.centerIn: parent
                                        text: (root.systemCompat && root.systemCompat.polkit && root.systemCompat.polkit.available) ? "✓ Ready" : "Warning"
                                        font.pixelSize: theme.fontXs
                                        font.bold: true
                                        color: "#818cf8"
                                    }
                                }
                            }
                        }
                    }

                    // Informational callout
                    Rectangle {
                        Layout.fillWidth: true
                        height: 38
                        radius: theme.radiusMd
                        color: Qt.rgba(16/255, 185/255, 129/255, 0.10)
                        border.color: Qt.rgba(16/255, 185/255, 129/255, 0.25)

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 8
                            Text { text: "💡"; font.pixelSize: 14 }
                            Text {
                                text: "Missing optional providers will not block Ryzora. You can use available providers immediately."
                                font.pixelSize: theme.fontXs
                                color: theme.text
                                Layout.fillWidth: true
                            }
                        }
                    }

                    Item { Layout.fillHeight: true }

                    RowLayout {
                        Layout.fillWidth: true

                        Rectangle {
                            height: 36
                            width: 100
                            radius: theme.radiusMd
                            color: backMouse1.containsMouse ? theme.surfaceHover : theme.surface
                            border.color: theme.border

                            Text {
                                anchors.centerIn: parent
                                text: "← Back"
                                font.pixelSize: theme.fontSm
                                color: theme.text
                            }

                            MouseArea {
                                id: backMouse1
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.currentStep = 0
                            }
                        }

                        Item { Layout.fillWidth: true }

                        Rectangle {
                            height: 36
                            width: 130
                            radius: theme.radiusMd
                            color: contMouse1.containsMouse ? theme.accentHover : theme.accent

                            Text {
                                anchors.centerIn: parent
                                text: "Continue →"
                                font.pixelSize: theme.fontSm
                                font.bold: true
                                color: "#ffffff"
                            }

                            MouseArea {
                                id: contMouse1
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.currentStep = 2
                            }
                        }
                    }
                }

                // ══════════════════════════════════════════
                // Step 2: Choose Your Experience & Storage
                // ══════════════════════════════════════════
                ColumnLayout {
                    anchors.fill: parent
                    spacing: 12
                    visible: root.currentStep === 2

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true

                        Text {
                            text: "02 — Choose your experience"
                            font.pixelSize: 18
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            text: "Select what content to configure. Ryzora core remains tiny (30 MiB); large community assets are streamed and cached on-demand."
                            font.pixelSize: theme.fontXs
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: theme.borderSubtle
                    }

                    // Card 1: Essential Ryzora
                    Rectangle {
                        Layout.fillWidth: true
                        height: 64
                        radius: theme.radiusMd
                        color: theme.surface
                        border.color: theme.accent
                        border.width: 1.5

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 12

                            Text { text: "💎"; font.pixelSize: 20 }
                            ColumnLayout {
                                spacing: 2
                                Layout.fillWidth: true
                                RowLayout {
                                    spacing: 8
                                    Text { text: "Essential Ryzora"; font.bold: true; font.pixelSize: theme.fontSm; color: theme.text }
                                    Rectangle {
                                        height: 18
                                        radius: theme.radiusSm
                                        color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.20)
                                        implicitWidth: defaultBadge.implicitWidth + 10
                                        Text {
                                            id: defaultBadge
                                            anchors.centerIn: parent
                                            text: "DEFAULT"
                                            font.pixelSize: 9
                                            font.bold: true
                                            color: theme.accent
                                        }
                                    }
                                }
                                Text { text: "Apps, Library, Updates, and core features. Lightweight and instant."; font.pixelSize: theme.fontXs; color: theme.textSecondary }
                            }

                            Text {
                                text: "✓ Included"
                                font.pixelSize: theme.fontXs
                                font.bold: true
                                color: "#34d399"
                            }
                        }
                    }

                    // Card 2: Community Customizations
                    Rectangle {
                        Layout.fillWidth: true
                        height: 64
                        radius: theme.radiusMd
                        color: theme.surface
                        border.color: root.communityContentEnabled ? theme.borderStrong : theme.borderSubtle

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.communityContentEnabled = !root.communityContentEnabled
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 12

                            Text { text: "🎨"; font.pixelSize: 20 }
                            ColumnLayout {
                                spacing: 2
                                Layout.fillWidth: true
                                RowLayout {
                                    spacing: 8
                                    Text { text: "Community Customizations"; font.bold: true; font.pixelSize: theme.fontSm; color: theme.text }
                                    Rectangle {
                                        height: 18
                                        radius: theme.radiusSm
                                        color: Qt.rgba(16/255, 185/255, 129/255, 0.20)
                                        implicitWidth: optBadge.implicitWidth + 10
                                        Text {
                                            id: optBadge
                                            anchors.centerIn: parent
                                            text: "ON DEMAND"
                                            font.pixelSize: 9
                                            font.bold: true
                                            color: "#10b981"
                                        }
                                    }
                                }
                                Text { text: "Discover wallpapers, lockscreens, and rice styles. Downloaded only when you choose them."; font.pixelSize: theme.fontXs; color: theme.textSecondary }
                            }

                            CheckBox {
                                checked: root.communityContentEnabled
                                onToggled: root.communityContentEnabled = checked
                            }
                        }
                    }

                    // Storage section
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: "Storage Location"
                                font.pixelSize: theme.fontSm
                                font.bold: true
                                color: theme.text
                            }
                            Item { Layout.fillWidth: true }
                            Text {
                                text: (root.storageInfo && root.storageInfo.free_space_human) ? ("Free disk space: " + root.storageInfo.free_space_human) : ""
                                font.pixelSize: theme.fontXs
                                color: theme.textMuted
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 38
                            radius: theme.radiusMd
                            color: theme.surface
                            border.color: root.storagePathValid ? theme.border : "#ef4444"

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 8
                                spacing: 8

                                Text { text: "📁"; font.pixelSize: 14 }
                                TextInput {
                                    id: pathInput
                                    Layout.fillWidth: true
                                    text: root.customStoragePath
                                    color: theme.text
                                    font.pixelSize: theme.fontSm
                                    selectByMouse: true
                                    clip: true

                                    onTextChanged: {
                                        root.customStoragePath = text
                                        root.checkCustomPath(text)
                                    }

                                    Text {
                                        anchors.fill: parent
                                        text: "~/.local/share/ryzora (default)"
                                        color: theme.textMuted
                                        font.pixelSize: theme.fontSm
                                        visible: !pathInput.text && !pathInput.activeFocus
                                    }
                                }

                                Text {
                                    text: root.storagePathValid ? "✓" : "⚠"
                                    font.bold: true
                                    color: root.storagePathValid ? "#34d399" : "#ef4444"
                                }
                            }
                        }

                        Text {
                            text: root.storageValidationMessage !== "" ? root.storageValidationMessage : "Default storage path: ~/.local/share/ryzora"
                            font.pixelSize: theme.fontXs
                            color: root.storagePathValid ? theme.textMuted : "#f87171"
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }

                    Item { Layout.fillHeight: true }

                    RowLayout {
                        Layout.fillWidth: true

                        Rectangle {
                            height: 36
                            width: 100
                            radius: theme.radiusMd
                            color: backMouse2.containsMouse ? theme.surfaceHover : theme.surface
                            border.color: theme.border

                            Text {
                                anchors.centerIn: parent
                                text: "← Back"
                                font.pixelSize: theme.fontSm
                                color: theme.text
                            }

                            MouseArea {
                                id: backMouse2
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.currentStep = 1
                            }
                        }

                        Item { Layout.fillWidth: true }

                        Rectangle {
                            height: 36
                            width: 130
                            radius: theme.radiusMd
                            color: (!root.storagePathValid) ? theme.surface : (contMouse2.containsMouse ? theme.accentHover : theme.accent)
                            opacity: root.storagePathValid ? 1.0 : 0.5

                            Text {
                                anchors.centerIn: parent
                                text: "Continue →"
                                font.pixelSize: theme.fontSm
                                font.bold: true
                                color: root.storagePathValid ? "#ffffff" : theme.textMuted
                            }

                            MouseArea {
                                id: contMouse2
                                anchors.fill: parent
                                enabled: root.storagePathValid
                                hoverEnabled: true
                                cursorShape: root.storagePathValid ? Qt.PointingHandCursor : Qt.ArrowCursor
                                onClicked: root.currentStep = 3
                            }
                        }
                    }
                }

                // ══════════════════════════════════════════
                // Step 3: You're Ready to Explore Ryzora
                // ══════════════════════════════════════════
                ColumnLayout {
                    anchors.fill: parent
                    spacing: 16
                    visible: root.currentStep === 3

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true

                        Text {
                            text: "03 — You're ready to explore Ryzora"
                            font.pixelSize: 18
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            text: "Your preferences have been configured. No unrequested downloads or intrusive system changes occurred."
                            font.pixelSize: theme.fontXs
                            color: theme.textSecondary
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: theme.borderSubtle
                    }

                    // Success Hero Banner
                    Rectangle {
                        Layout.fillWidth: true
                        height: 100
                        radius: theme.radiusLg
                        color: Qt.rgba(16/255, 185/255, 129/255, 0.08)
                        border.color: Qt.rgba(16/255, 185/255, 129/255, 0.30)

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 16
                            spacing: 16

                            Rectangle {
                                width: 56
                                height: 56
                                radius: theme.radiusFull
                                color: Qt.rgba(16/255, 185/255, 129/255, 0.20)
                                border.color: "#10b981"

                                Text {
                                    anchors.centerIn: parent
                                    text: "✓"
                                    font.pixelSize: 28
                                    font.bold: true
                                    color: "#34d399"
                                }
                            }

                            ColumnLayout {
                                spacing: 4
                                Layout.fillWidth: true

                                Text {
                                    text: "Setup Complete"
                                    font.pixelSize: theme.fontBase
                                    font.bold: true
                                    color: theme.text
                                }

                                Text {
                                    text: "Ryzora is configured and ready. You can reopen this wizard or adjust settings anytime in Settings → Maintenance."
                                    font.pixelSize: theme.fontXs
                                    color: theme.textSecondary
                                    wrapMode: Text.WordWrap
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }

                    // Summary details card
                    Rectangle {
                        Layout.fillWidth: true
                        height: 110
                        radius: theme.radiusMd
                        color: theme.surface
                        border.color: theme.borderSubtle

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 6

                            RowLayout {
                                Text { text: "Experience Mode:"; font.pixelSize: theme.fontXs; color: theme.textMuted }
                                Text {
                                    text: root.communityContentEnabled ? "Essential + Community On-Demand" : "Essential Ryzora (Minimal)"
                                    font.pixelSize: theme.fontXs
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            RowLayout {
                                Text { text: "Storage Location:"; font.pixelSize: theme.fontXs; color: theme.textMuted }
                                Text {
                                    text: (root.customStoragePath.trim() !== "") ? root.customStoragePath : "~/.local/share/ryzora"
                                    font.pixelSize: theme.fontXs
                                    font.bold: true
                                    color: theme.text
                                }
                            }

                            RowLayout {
                                Text { text: "System Providers:"; font.pixelSize: theme.fontXs; color: theme.textMuted }
                                Text {
                                    text: (root.systemCompat && root.systemCompat.pacman && root.systemCompat.pacman.available) ? "Arch / CachyOS (Pacman) Ready" : "Configured"
                                    font.pixelSize: theme.fontXs
                                    font.bold: true
                                    color: "#34d399"
                                }
                            }

                            RowLayout {
                                Text { text: "Setup Version:"; font.pixelSize: theme.fontXs; color: theme.textMuted }
                                Text { text: "0.1.0"; font.pixelSize: theme.fontXs; font.bold: true; color: theme.text }
                            }
                        }
                    }

                    Item { Layout.fillHeight: true }

                    RowLayout {
                        Layout.fillWidth: true

                        Rectangle {
                            height: 38
                            width: 100
                            radius: theme.radiusMd
                            color: backMouse3.containsMouse ? theme.surfaceHover : theme.surface
                            border.color: theme.border

                            Text {
                                anchors.centerIn: parent
                                text: "← Back"
                                font.pixelSize: theme.fontSm
                                color: theme.text
                            }

                            MouseArea {
                                id: backMouse3
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.currentStep = 2
                            }
                        }

                        Item { Layout.fillWidth: true }

                        Rectangle {
                            height: 40
                            width: 160
                            radius: theme.radiusMd
                            gradient: Gradient {
                                GradientStop { position: 0.0; color: enterMouse.containsMouse ? Qt.lighter("#10b981", 1.1) : "#10b981" }
                                GradientStop { position: 1.0; color: enterMouse.containsMouse ? Qt.darker("#059669", 1.05) : "#059669" }
                            }

                            RowLayout {
                                anchors.centerIn: parent
                                spacing: 8
                                Text {
                                    text: "Enter Ryzora ✓"
                                    font.pixelSize: theme.fontSm
                                    font.bold: true
                                    color: "#ffffff"
                                }
                            }

                            MouseArea {
                                id: enterMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.finishSetup()
                            }
                        }
                    }
                }
            }
        }
    }
}

