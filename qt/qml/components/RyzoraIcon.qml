import QtQuick

Item {
    id: root

    property string name: "lock"
    property color color: theme ? theme.text : "#ffffff"
    property real size: 16
    property bool filled: false
    property real strokeWidth: 1.8 // Kept for backwards compatibility

    implicitWidth: size
    implicitHeight: size
    width: size
    height: size

    readonly property string colorHex: {
        var c = root.color
        return encodeURIComponent(c.toString())
    }

    readonly property string canonicalName: (root.name || "").toLowerCase().trim()

    Image {
        id: iconImg
        anchors.centerIn: parent
        width: root.size
        height: root.size
        sourceSize.width: Math.max(16, Math.round(root.size * (typeof Screen !== "undefined" && Screen.devicePixelRatio ? Screen.devicePixelRatio : 1.0)))
        sourceSize.height: Math.max(16, Math.round(root.size * (typeof Screen !== "undefined" && Screen.devicePixelRatio ? Screen.devicePixelRatio : 1.0)))
        fillMode: Image.PreserveAspectFit
        smooth: true
        asynchronous: false
        source: {
            if (!root.canonicalName || root.canonicalName.length === 0) return ""
            var variant = root.filled ? "filled" : "regular"
            return "image://fluent/" + root.canonicalName + "_" + variant + "?color=" + root.colorHex
        }
    }
}
