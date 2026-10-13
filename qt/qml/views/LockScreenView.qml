import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        ColumnLayout {
            spacing: 2
            Text {
                text: "Lock Screen & Display Integration"
                font.pixelSize: theme.fontTitle
                font.bold: true
                color: theme.text
            }
            Text {
                text: "Target-aware display manager and session lock configuration"
                font.pixelSize: theme.fontCardDesc
                color: theme.textMuted
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            // ── SDDM Login Screen Card ──
            Rectangle {
                Layout.fillWidth: true
                height: 200
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.border
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 8

                    RowLayout {
                        spacing: 8
                        Text { text: "🔒"; font.pixelSize: 16 }
                        Text {
                            text: "Login Screen (SDDM)"
                            font.pixelSize: theme.fontSection
                            font.bold: true
                            color: theme.text
                        }
                        Item { Layout.fillWidth: true }
                        // Status Badge
                        Rectangle {
                            height: 20
                            width: sddmActiveTxt.implicitWidth + 16
                            radius: 4
                            color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                ? Qt.rgba(16/255, 185/255, 129/255, 0.20)
                                : Qt.rgba(100/255, 116/255, 139/255, 0.20)
                            border.color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                ? Qt.rgba(16/255, 185/255, 129/255, 0.50)
                                : Qt.rgba(100/255, 116/255, 139/255, 0.30)
                            border.width: 1

                            Text {
                                id: sddmActiveTxt
                                anchors.centerIn: parent
                                text: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                                    ? "ACTIVE THEME"
                                    : "DEFAULT SYSTEM GREETER"
                                font.pixelSize: 9
                                font.bold: true
                                color: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active) ? "#34d399" : "#94a3b8"
                            }
                        }
                    }

                    Text {
                        text: (backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                            ? ("Theme: " + backend.activeConfiguration.active_title + " · /etc/sddm.conf.d/ryzora.conf")
                            : "No Ryzora theme currently active for SDDM. System default greeter is running."
                        font.pixelSize: theme.fontSm
                        color: theme.textSecondary
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    Text {
                        text: "Requires Polkit authorization for /etc/sddm.conf.d drop-ins. All changes hash-verified with automated fail-closed recovery intact."
                        font.pixelSize: 11
                        color: theme.textMuted
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    Item { Layout.fillHeight: true }

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "Config drift: " + (backend.systemInfo ? backend.systemInfo.config_drift : "Unknown")
                            font.pixelSize: 11
                            color: theme.textMuted
                        }
                        Item { Layout.fillWidth: true }

                        // Deactivate SDDM Button
                        Rectangle {
                            visible: Boolean(backend.activeConfiguration && backend.activeConfiguration.sddm_login_active)
                            height: 28
                            width: deactSddmTxt.implicitWidth + 20
                            radius: theme.radiusSm
                            color: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                ? (deactSddmMouse.containsMouse ? "#3b1219" : theme.surfaceElevated)
                                : theme.surfaceElevated
                            border.color: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                ? (deactSddmMouse.containsMouse ? "#ef4444" : theme.borderStrong)
                                : theme.border
                            border.width: 1
                            opacity: (backend.systemInfo && backend.systemInfo.polkit_helper_installed) ? 1.0 : 0.65

                            Text {
                                id: deactSddmTxt
                                anchors.centerIn: parent
                                text: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                    ? "Deactivate SDDM"
                                    : "Deactivate SDDM (Elevation Required)"
                                font.pixelSize: 11
                                font.bold: true
                                color: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                    ? (deactSddmMouse.containsMouse ? "#f87171" : theme.textMuted)
                                    : theme.textFaint
                            }
                            MouseArea {
                                id: deactSddmMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: (backend.systemInfo && backend.systemInfo.polkit_helper_installed)
                                    ? Qt.PointingHandCursor
                                    : Qt.ForbiddenCursor
                                onClicked: {
                                    if (backend.systemInfo && !backend.systemInfo.polkit_helper_installed) {
                                        backend.postNotification("warning", "Elevation Helper Required", "SDDM deactivation requires privileged Polkit helper. Please set up the helper in System Integration.");
                                        return;
                                    }
                                    backend.deactivateTarget("login_screen")
                                }
                            }
                        }
                    }
                }
            }

            // ── Session Lock Card ──
            Rectangle {
                Layout.fillWidth: true
                height: 200
                radius: theme.radiusMd
                color: theme.surface
                border.color: theme.border
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 8

                    RowLayout {
                        spacing: 8
                        Text { text: "🛡️"; font.pixelSize: 16 }
                        Text {
                            text: "Session Lock (Quickshell / Hyprlock)"
                            font.pixelSize: theme.fontSection
                            font.bold: true
                            color: theme.text
                        }
                        Item { Layout.fillWidth: true }
                        // Status Badge
                        Rectangle {
                            height: 20
                            width: sessionActiveTxt.implicitWidth + 16
                            radius: 4
                            color: (backend.activeConfiguration && backend.activeConfiguration.session_lock_active)
                                ? Qt.rgba(99/255, 102/255, 241/255, 0.20)
                                : Qt.rgba(100/255, 116/255, 139/255, 0.20)
                            border.color: (backend.activeConfiguration && backend.activeConfiguration.session_lock_active)
                                ? Qt.rgba(99/255, 102/255, 241/255, 0.50)
                                : Qt.rgba(100/255, 116/255, 139/255, 0.30)
                            border.width: 1

                            Text {
                                id: sessionActiveTxt
                                anchors.centerIn: parent
                                text: (backend.activeConfiguration && backend.activeConfiguration.session_lock_active)
                                    ? "ACTIVE SESSION LOCK"
                                    : "DEFAULT LOCK"
                                font.pixelSize: 9
                                font.bold: true
                                color: (backend.activeConfiguration && backend.activeConfiguration.session_lock_active) ? "#818cf8" : "#94a3b8"
                            }
                        }
                    }

                    Text {
                        text: (backend.activeConfiguration && backend.activeConfiguration.session_lock_active)
                            ? ("Theme: " + backend.activeConfiguration.active_title + " · ext-session-lock-v1 protocol")
                            : "No Ryzora theme currently active for session lock. Native compositor lock active."
                        font.pixelSize: theme.fontSm
                        color: theme.textSecondary
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    Text {
                        text: "Wayland ext-session-lock-v1 protocol. Governed by hypridle.service lifecycle on Garuda Linux. Zero elevation required."
                        font.pixelSize: 11
                        color: theme.textMuted
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    Item { Layout.fillHeight: true }

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "Protocol: ext-session-lock-v1"
                            font.pixelSize: 11
                            color: theme.textMuted
                        }
                        Item { Layout.fillWidth: true }

                        // Deactivate Session Lock Button
                        Rectangle {
                            visible: Boolean(backend.activeConfiguration && backend.activeConfiguration.session_lock_active)
                            height: 28
                            width: deactSessionTxt.implicitWidth + 20
                            radius: theme.radiusSm
                            color: deactSessionMouse.containsMouse ? "#3b1219" : theme.surfaceElevated
                            border.color: deactSessionMouse.containsMouse ? "#ef4444" : theme.borderStrong
                            border.width: 1

                            Text {
                                id: deactSessionTxt
                                anchors.centerIn: parent
                                text: "Deactivate Session Lock"
                                font.pixelSize: 11
                                font.bold: true
                                color: deactSessionMouse.containsMouse ? "#f87171" : theme.textMuted
                            }
                            MouseArea {
                                id: deactSessionMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    backend.deactivateTarget("session_lock")
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Security & Transactional Recovery Guarantees Card ──
        Rectangle {
            Layout.fillWidth: true
            height: 90
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.border
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 14
                spacing: 8

                RowLayout {
                    spacing: 8
                    Text { text: "🛡️"; font.pixelSize: 14 }
                    Text {
                        text: "Security Invariants & Non-Destructive Mutations"
                        font.pixelSize: theme.fontBase
                        font.bold: true
                        color: theme.text
                    }
                }

                Text {
                    text: "Deactivating any target alters only ~/.local/share/ryzora/active_lockscreen.json or SDDM drop-in configurations. Installed package files in ~/.local/share/ryzora/installed remain completely untouched."
                    font.pixelSize: 11
                    color: theme.textMuted
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        Item { Layout.fillHeight: true }
    }
}
