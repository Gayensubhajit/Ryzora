import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    signal creatorSelected(string authorName)

    // Dynamically derive creators list from real backend.catalogList
    readonly property var derivedCreators: {
        var list = backend.catalogList || []
        var authorMap = {}

        for (var i = 0; i < list.length; i++) {
            var item = list[i]
            if (!item) continue
            var auth = item.author || "Community"
            if (!authorMap[auth]) {
                authorMap[auth] = {
                    name: auth,
                    packageCount: 0,
                    ratings: [],
                    categories: {},
                    samplePkg: item.name
                }
            }
            authorMap[auth].packageCount++
            if (item.rating && item.rating > 0) {
                authorMap[auth].ratings.push(item.rating)
            }
            if (item.category) {
                authorMap[auth].categories[item.category] = true
            }
        }

        var res = []
        var keys = Object.keys(authorMap)
        // Sort by package count descending
        keys.sort(function(a, b) {
            return authorMap[b].packageCount - authorMap[a].packageCount
        })

        for (var j = 0; j < keys.length; j++) {
            var k = keys[j]
            var data = authorMap[k]
            var avgRating = "Unrated"
            if (data.ratings.length > 0) {
                var sum = 0
                for (var r = 0; r < data.ratings.length; r++) sum += data.ratings[r]
                avgRating = (sum / data.ratings.length).toFixed(1)
            }

            var catList = Object.keys(data.categories).join(", ")
            res.push({
                name: k,
                packageCount: data.packageCount,
                rating: avgRating,
                categories: catList || "general",
                sample: data.samplePkg
            })
        }
        return res
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        // ── 1. Page Header ──
        ColumnLayout {
            spacing: 2
            Layout.fillWidth: true

            Text {
                text: "Community Creators & Authors"
                font.pixelSize: 22
                font.bold: true
                color: theme.text
            }

            Text {
                text: "Derived from actual ingested repository metadata (" + root.derivedCreators.length + " authors across " + backend.catalogCount + " packages)"
                font.pixelSize: 13
                color: theme.textSecondary
            }
        }

        // ── 2. Information Banner on Truthful Signing Metadata ──
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: bannerCol.implicitHeight + 18
            radius: theme.radiusMd
            color: theme.surface
            border.color: theme.border
            border.width: 1

            ColumnLayout {
                id: bannerCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 2

                RowLayout {
                    spacing: 8
                    Text { text: "ℹ️"; font.pixelSize: 13 }
                    Text {
                        text: "Author Metadata & Cryptographic Keys"
                        font.pixelSize: 12
                        font.bold: true
                        color: theme.text
                    }
                }

                Text {
                    text: "Package creators are extracted directly from manifest.json records in the repository. Cryptographic PGP public keys are not currently embedded in the community catalog schema; trust status displays verified manifest authorship."
                    font.pixelSize: 11
                    color: theme.textSecondary
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        // ── 3. Creators Grid ──
        GridView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            cellWidth: Math.floor(width / (width >= 800 ? 2 : 1))
            cellHeight: 140

            model: root.derivedCreators

            delegate: Item {
                width: GridView.view.cellWidth
                height: GridView.view.cellHeight

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 6
                    radius: theme.radiusLg
                    color: crMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                    border.color: crMouse.containsMouse ? theme.accent : theme.borderSubtle
                    border.width: 1

                    MouseArea {
                        id: crMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.creatorSelected(modelData.name)
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 6

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Rectangle {
                                width: 36
                                height: 36
                                radius: 18
                                color: theme.surface
                                border.color: theme.borderSubtle
                                border.width: 1

                                Text {
                                    anchors.centerIn: parent
                                    text: modelData.name.charAt(0).toUpperCase()
                                    font.pixelSize: 16
                                    font.bold: true
                                    color: theme.accent
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1

                                Text {
                                    text: modelData.name
                                    font.pixelSize: 14
                                    font.bold: true
                                    color: theme.text
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }

                                Text {
                                    text: modelData.packageCount + " packages in catalog · Categories: " + modelData.categories
                                    font.pixelSize: 11
                                    color: theme.textMuted
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Text {
                                text: "PGP Key: Unchecked in catalog index"
                                font.pixelSize: 10
                                font.family: "monospace"
                                color: theme.textMuted
                            }

                            Item { Layout.fillWidth: true }

                            RowLayout {
                                visible: modelData.rating !== "Unrated"
                                spacing: 2
                                Text { text: "★"; font.pixelSize: 11; color: "#f59e0b" }
                                Text { text: modelData.rating; font.pixelSize: 11; font.bold: true; color: theme.text }
                            }
                        }
                    }
                }
            }
        }
    }
}
