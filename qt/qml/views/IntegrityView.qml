import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    readonly property string rawDrift: (backend.systemInfo && backend.systemInfo.config_drift) ? backend.systemInfo.config_drift : ""
    readonly property string driftCategory: {
        if (!root.rawDrift || root.rawDrift.length === 0) return "Not scanned"
        if (root.rawDrift.indexOf("In sync") === 0) return "Clean"
        if (root.rawDrift.indexOf("Drift detected") === 0) return "Drift detected"
        if (root.rawDrift.indexOf("Unknown") === 0) return "Unknown"
        return "Not scanned"
    }
    readonly property bool isDriftClean: root.driftCategory === "Clean" 

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

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
                    text: "Configuration drift verification and host restore points"
                    font.pixelSize: 13
                    color: theme.textSecondary
                }
            }

            Button {
                text: "⟳ Probe Configuration Drift"
                onClicked: {
                    backend.loadSystemInfo()
                }
            }
        }

        // ── 2. Live Configuration Drift Status Banner ──
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 96
            radius: theme.radiusLg
            color: root.isDriftClean ? theme.surfaceElevated : "#451a03"
            border.color: root.isDriftClean ? theme.borderSubtle : theme.warning
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 14

                Text {
                    text: root.isDriftClean ? "🛡️" : "⚠️"
                    font.pixelSize: 22
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    RowLayout {
                        spacing: 8
                        Text {
                            text: "Lock-Screen Configuration Drift:"
                            font.pixelSize: 13
                            font.bold: true
                            color: theme.text
                        }
                        Rectangle {
                            height: 18
                            width: catText.implicitWidth + 10
                            radius: 4
                            color: root.driftCategory === "Clean" ? "#1e3a2f" : (root.driftCategory === "Drift detected" ? "#451a03" : theme.surface)
                            border.color: root.driftCategory === "Clean" ? "#10b981" : (root.driftCategory === "Drift detected" ? "#f59e0b" : theme.borderSubtle)
                            border.width: 1

                            Text {
                                id: catText
                                anchors.centerIn: parent
                                text: root.driftCategory.toUpperCase()
                                font.pixelSize: 9
                                font.bold: true
                                font.family: "monospace"
                                color: root.driftCategory === "Clean" ? "#6ee7b7" : (root.driftCategory === "Drift detected" ? "#fde68a" : theme.textMuted)
                            }
                        }
                    }

                    Text {
                        text: root.rawDrift.length > 0 ? root.rawDrift : "No configuration drift probe recorded yet."
                        font.pixelSize: 11
                        font.family: "monospace"
                        color: root.driftCategory === "Clean" ? theme.accent : (root.driftCategory === "Drift detected" ? theme.warning : theme.textMuted)
                    }

                    Text {
                        text: "Probe Scope: Live inspection of ~/.local/share/ryzora/state/active_lockscreen.json and drop-in links. Full filesystem SHA-256 and PGP audits are not performed by this probe."
                        font.pixelSize: 10
                        color: theme.textSecondary
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                }
            }
        }

        // ── 3. Four Honest Metric Cards (Real vs Unscanned) ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            MetricCard {
                label: "State Drift Probe"
                value: root.driftCategory
                status: "Live Target Check"
                accentColor: root.driftCategory === "Clean" ? theme.accent : (root.driftCategory === "Drift detected" ? theme.warning : theme.textMuted)
            }

            MetricCard {
                label: "Host Snapshots"
                value: backend.snapshotsCount.toString()
                status: "Disk Invariant"
                accentColor: theme.accent
            }

            MetricCard {
                label: "Deep SHA-256 Audit"
                value: "Not scanned"
                status: "Unimplemented"
                accentColor: theme.textMuted
            }

            MetricCard {
                label: "PGP Signatures"
                value: "Unscanned"
                status: "Unimplemented"
                accentColor: theme.textMuted
            }
        }

        // ── 4. Monitored System Paths Table ──
        Text {
            text: "SYSTEM PATH INVARIANTS"
            font.pixelSize: 11
            font.bold: true
            font.letterSpacing: 1.0
            color: theme.navSectionLabel
            Layout.topMargin: 6
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 8

            model: [
                { target: "Active lock screen state", path: "~/.local/share/ryzora/state/active_lockscreen.json", probe: "Monitored", note: "Checked against installed package records" },
                { target: "Hypridle drop-in integration", path: "~/.config/hypr/hypridle.conf.d/ryzora.conf", probe: "Monitored", note: "Checked for orphan drop-in drift" },
                { target: "SDDM activation metadata", path: "~/.local/share/ryzora/silentsddm_activation.json", probe: "Monitored", note: "Checked against live theme link" },
                { target: "Package file manifests", path: "~/.local/share/ryzora/installed/*.json", probe: "Unscanned", note: "Deep file hash verification pending Stage F engine" }
            ]

            delegate: Rectangle {
                width: ListView.view.width
                height: 50
                radius: theme.radiusMd
                color: theme.surfaceElevated
                border.color: theme.borderSubtle
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 12

                    Text {
                        text: modelData.probe === "Monitored" ? "•" : "○"
                        font.pixelSize: 14
                        font.bold: true
                        color: modelData.probe === "Monitored" ? theme.accent : theme.textMuted
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        Text {
                            text: modelData.target + " — " + modelData.note
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
                        width: probeTxt.implicitWidth + 12
                        radius: 4
                        color: modelData.probe === "Monitored" ? theme.accentFaint : theme.surface
                        border.color: modelData.probe === "Monitored" ? theme.accent : theme.borderSubtle
                        border.width: 1

                        Text {
                            id: probeTxt
                            anchors.centerIn: parent
                            text: modelData.probe.toUpperCase()
                            font.pixelSize: 9
                            font.bold: true
                            font.family: "monospace"
                            color: modelData.probe === "Monitored" ? theme.accent : theme.textMuted
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
        property color accentColor: theme.accent

        Layout.fillWidth: true
        implicitHeight: 76
        radius: theme.radiusMd
        color: theme.surfaceElevated
        border.color: theme.borderSubtle
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 2

            Text { text: mc.label; font.pixelSize: 11; color: theme.textMuted; elide: Text.ElideRight; Layout.fillWidth: true }
            Text { text: mc.value; font.pixelSize: 16; font.bold: true; font.family: "monospace"; color: theme.text }
            Text { text: mc.status; font.pixelSize: 10; font.bold: true; color: mc.accentColor }
        }
    }
}
