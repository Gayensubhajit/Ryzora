import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: btnRoot

    property string text: ""
    property string iconName: ""
    property real iconSize: size === "sm" ? 13 : (size === "lg" ? 18 : 15)
    property bool iconFilled: false
    property string variant: "primary" // "primary" | "secondary" | "tertiary" | "destructive" | "destructive-solid" | "icon-only"
    property string size: "md" // "sm" (28px) | "md" (36px) | "lg" (40px)
    property bool loading: false
    property string loadingText: "Wait..."
    property string tooltip: ""
    property real customRadius: -1

    signal clicked()

    implicitHeight: size === "sm" ? 28 : (size === "lg" ? 40 : 36)
    height: implicitHeight
    implicitWidth: {
        if (variant === "icon-only") return implicitHeight
        return Math.max(size === "sm" ? 72 : (size === "lg" ? 96 : 84), contentRow.implicitWidth + (size === "sm" ? 24 : (size === "lg" ? 32 : 28)))
    }
    width: variant === "icon-only" ? implicitHeight : implicitWidth

    Layout.preferredWidth: implicitWidth
    Layout.minimumWidth: implicitWidth
    Layout.preferredHeight: implicitHeight
    Layout.minimumHeight: implicitHeight

    radius: customRadius >= 0 ? customRadius : (size === "sm" ? 6 : (size === "lg" ? 10 : 8))
    activeFocusOnTab: true
    clip: false

    readonly property bool isHovered: btnMouse.containsMouse && enabled && !loading
    readonly property bool isPressed: btnMouse.pressed && enabled && !loading

    // Background color computation
    color: {
        if (!enabled) return theme ? theme.borderSubtle : "#374151"
        if (loading) return (variant === "primary" ? theme.accentHover : (theme ? theme.surfaceElevated : "#1f2937"))

        if (variant === "primary") {
            if (isPressed) return Qt.darker(theme.accent, 1.15)
            if (isHovered) return theme.accentHover
            return theme.accent
        } else if (variant === "secondary") {
            if (isPressed) return Qt.darker(theme.surfaceElevated, 1.1)
            if (isHovered) return theme.surfaceHover
            return theme.surfaceElevated
        } else if (variant === "tertiary") {
            if (isPressed) return theme.surfaceHover
            if (isHovered) return theme.surfaceHover
            return "transparent"
        } else if (variant === "destructive") {
            if (isPressed) return Qt.rgba(239/255, 68/255, 68/255, 0.30)
            if (isHovered) return Qt.rgba(239/255, 68/255, 68/255, 0.20)
            return Qt.rgba(239/255, 68/255, 68/255, 0.12)
        } else if (variant === "destructive-solid") {
            if (isPressed) return Qt.darker("#ef4444", 1.15)
            if (isHovered) return "#dc2626"
            return "#ef4444"
        } else if (variant === "icon-only") {
            if (isPressed) return theme.surfaceHover
            if (isHovered) return theme.surfaceHover
            return "transparent"
        }
        return theme.accent
    }

    // Border color computation
    border.color: {
        if (btnRoot.activeFocus) return theme.accent
        if (!enabled) return "transparent"

        if (variant === "primary") {
            return isHovered ? theme.accentHover : theme.accent
        } else if (variant === "secondary") {
            return isHovered ? theme.borderStrong : theme.borderSubtle
        } else if (variant === "tertiary") {
            return "transparent"
        } else if (variant === "destructive") {
            return isHovered ? Qt.rgba(239/255, 68/255, 68/255, 0.60) : Qt.rgba(239/255, 68/255, 68/255, 0.35)
        } else if (variant === "destructive-solid") {
            return "#ef4444"
        } else if (variant === "icon-only") {
            return isHovered ? theme.borderSubtle : "transparent"
        }
        return theme.borderSubtle
    }

    border.width: btnRoot.activeFocus ? 2 : (variant === "tertiary" ? 0 : 1)

    Behavior on color { ColorAnimation { duration: 130 } }
    Behavior on border.color { ColorAnimation { duration: 130 } }

    // Foreground / Content Color
    readonly property color contentColor: {
        if (!enabled) return theme ? theme.textMuted : "#9ca3af"
        if (variant === "primary" || variant === "destructive-solid") return "#ffffff"
        if (variant === "destructive") return "#ef4444"
        if (variant === "tertiary") return isHovered ? theme.accent : theme.textSecondary
        if (variant === "icon-only") return isHovered ? theme.text : theme.textSecondary
        return theme.text
    }

    Row {
        id: contentRow
        anchors.centerIn: parent
        spacing: btnRoot.text.length > 0 ? (btnRoot.size === "sm" ? 6 : 8) : 0

        // Loading spinner
        Item {
            visible: btnRoot.loading
            width: spinIcon.size
            height: spinIcon.size
            anchors.verticalCenter: parent.verticalCenter

            RyzoraIcon {
                id: spinIcon
                name: "arrow-clockwise"
                size: btnRoot.iconSize
                color: btnRoot.contentColor
                anchors.centerIn: parent

                RotationAnimator {
                    target: spinIcon
                    from: 0
                    to: 360
                    duration: 900
                    loops: Animation.Infinite
                    running: btnRoot.loading
                }
            }
        }

        // Action Icon
        RyzoraIcon {
            visible: !btnRoot.loading && btnRoot.iconName.length > 0
            name: btnRoot.iconName
            size: btnRoot.iconSize
            filled: btnRoot.iconFilled
            color: btnRoot.contentColor
            anchors.verticalCenter: parent.verticalCenter
        }

        // Text Label
        Text {
            visible: btnRoot.variant !== "icon-only" && (btnRoot.text.length > 0 || btnRoot.loading)
            text: btnRoot.loading ? btnRoot.loadingText : btnRoot.text
            font.family: theme ? theme.fontFamily : "Inter"
            font.pixelSize: btnRoot.size === "sm" ? 11 : (btnRoot.size === "lg" ? 14 : 12)
            font.weight: (btnRoot.variant === "primary" || btnRoot.variant === "destructive-solid") ? (theme ? theme.weightSemibold : Font.DemiBold) : (theme ? theme.weightMedium : Font.Medium)
            color: btnRoot.contentColor
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    // Keyboard navigation
    Keys.onReturnPressed: if (enabled && !loading) btnRoot.clicked()
    Keys.onSpacePressed: if (enabled && !loading) btnRoot.clicked()

    MouseArea {
        id: btnMouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: (btnRoot.enabled && !btnRoot.loading) ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: {
            if (btnRoot.enabled && !btnRoot.loading) {
                btnRoot.clicked()
            }
        }
    }

    // Tooltip
    ToolTip.visible: Boolean(tooltip.length > 0 && btnMouse.containsMouse)
    ToolTip.text: tooltip
    ToolTip.delay: 450
    ToolTip.timeout: 3000
}

