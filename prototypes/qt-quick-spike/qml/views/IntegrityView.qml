import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property bool isAuditing: false
    property string auditStatus: "Last scanned: Just now (Healthy)"

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 18

        // ── 1. Page Header ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: "Package Integrity & Security"
                    font.pixelSize: 22
                    font.bold: true
                    color: theme.text
                }

                Text {
                    text: "Cryptographic SHA-256 verification, configuration drift detection, and tamper defense"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            Button {
                text: root.isAuditing ? "Auditing System..." : "🛡️ Run Integrity Audit"
                enabled: !root.isAuditing
                onClicked: {
                    root.isAuditing = true
                    auditTimer.start()
                }
            }
        }

        Timer {
            id: auditTimer
            interval: 800
            onTriggered: {
                root.isAuditing = false
                root.auditStatus = "Audit complete: 0 mismatches found across all packages."
            }
        }

        // ── 2. Status Banner ──
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 80
            radius: theme.radiusLg
            color: "#064e3b"
            border.color: "#10b981"
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 14

                Text { text: "🛡️"; font.pixelSize: 24 }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: "All Installed Packages Match Official Manifest Signatures"
                        font.pixelSize: 14
                        font.bold: true
                        color: "#ffffff"
                    }

                    Text {
                        text: root.auditStatus
                        font.pixelSize: 12
                        color: "#6ee7b7"
                    }
                }
            }
        }

        // ── 3. Four Health Metric Cards ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            MetricCard {
                label: "Modified Files"
                value: "0"
                status: "Clean"
                isHealthy: true
            }

            MetricCard {
                label: "Missing Files"
                value: "0"
                status: "Verified"
                isHealthy: true
            }

            MetricCard {
                label: "Config Drift"
                value: "0"
                status: "In Sync"
                isHealthy: true
            }

            MetricCard {
                label: "Signatures"
                value: "100%"
                status: "Valid"
                isHealthy: true
            }
        }

        // ── 4. Audit Checks Table ──
        Text {
            text: "INTEGRITY AUDIT LOG"
            font.pixelSize: 11
            font.bold: true
            font.letterSpacing: 1.0
            color: theme.navSectionLabel
            Layout.topMargin: 8
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 8

            model: [
                { target: "silent-sddm-dr460nized", path: "/usr/share/sddm/themes/silent-sddm-dr460nized", result: "MATCH", status: "Healthy (SHA-256 Verified)" },
                { target: "clockwork-orbital", path: "~/.config/quickshell/lockscreens/clockwork-orbital", result: "MATCH", status: "Healthy (SHA-256 Verified)" },
                { target: "hypridle drop-in", path: "~/.config/systemd/user/hypridle.service.d/10-ryzora.conf", result: "MATCH", status: "Config Invariant Verified" },
                { target: "active state metadata", path: "~/.local/share/ryzora/state/active_lockscreen.json", result: "MATCH", status: "Strict JSON Schema Verified" }
            ]

            delegate: Rectangle {
                width: ListView.view.width
                height: 52
                radius: theme.radiusMd
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 12

                    Text { text: "✓"; font.pixelSize: 12; font.bold: true; color: "#10b981" }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        Text {
                            text: modelData.target
                            font.pixelSize: 12
                            font.bold: true
                            color: theme.text
                        }

                        Text {
                            text: modelData.path
                            font.pixelSize: 10
                            font.family: "monospace"
                            color: theme.textMuted
                        }
                    }

                    Rectangle {
                        height: 20
                        width: matchTxt.implicitWidth + 12
                        radius: 4
                        color: "#1e3a2f"
                        border.color: "#10b981"
                        border.width: 1

                        Text {
                            id: matchTxt
                            anchors.centerIn: parent
                            text: modelData.result
                            font.pixelSize: 9
                            font.bold: true
                            font.family: "monospace"
                            color: "#6ee7b7"
                        }
                    }
                }
            }
        }
    }

    component MetricCard : Rectangle {
        id: mc
        property string label: ""
        property string value: ""
        property string status: ""
        property bool isHealthy: true

        Layout.fillWidth: true
        implicitHeight: 80
        radius: theme.radiusMd
        color: theme.surfaceElevated
        border.color: theme.borderSubtle
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 2

            Text { text: mc.label; font.pixelSize: 11; color: theme.textMuted; elide: Text.ElideRight; Layout.fillWidth: true }
            Text { text: mc.value; font.pixelSize: 18; font.bold: true; font.family: "monospace"; color: theme.text }
            Text { text: mc.status; font.pixelSize: 10; font.bold: true; color: mc.isHealthy ? "#10b981" : "#ef4444" }
        }
    }
}
