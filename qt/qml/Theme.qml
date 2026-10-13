import QtQuick

QtObject {
    id: root

    // Active appearance mode: "dark" | "light" | "system"
    property string appearanceMode: "dark"

    // Detected system appearance from host desktop: "dark" | "light"
    property string systemScheme: "dark"

    // Resolved active theme
    readonly property string resolvedTheme: appearanceMode === "system" ? systemScheme : appearanceMode

    // Active accent color name: "blue" | "silver" | "purple" | "green" | "orange"
    property string accentName: "blue"

    // Accessibility settings
    property bool reduceTransparency: false
    property bool reduceMotion: false
    property bool increaseContrast: false

    // Color Swatches matching React ACCENT_SWATCHES
    readonly property var swatches: {
        "blue": "#6488E8",
        "silver": "#D9DCE2",
        "purple": "#8b5cf6",
        "green": "#10b981",
        "orange": "#f97316"
    }

    // Swatch Names matching React
    readonly property var swatchNames: {
        "blue": "Ocean Blue",
        "silver": "Deep Silver",
        "purple": "Amethyst",
        "green": "Emerald",
        "orange": "Amber Fire"
    }

    // Dynamic accent resolution matching tokens.css semantic tokens
    readonly property color accent: {
        if (accentName === "silver") {
            return resolvedTheme === "light" ? "#4b5563" : "#D9DCE2"
        } else if (accentName === "purple") {
            return resolvedTheme === "light" ? "#7c3aed" : "#8b5cf6"
        } else if (accentName === "green") {
            return resolvedTheme === "light" ? "#059669" : "#10b981"
        } else if (accentName === "orange") {
            return resolvedTheme === "light" ? "#ea580c" : "#f97316"
        }
        return resolvedTheme === "light" ? "#3b6fe8" : "#6488E8"
    }
    readonly property color accentHover: Qt.lighter(accent, 1.15)
    readonly property color accentGlow: Qt.rgba(accent.r, accent.g, accent.b, 0.12)
    readonly property color accentFaint: Qt.rgba(accent.r, accent.g, accent.b, 0.12)

    // Surfaces & Canvas (Monochrome Deep Black Hierarchy matching --rz-* tokens)
    readonly property color bg: resolvedTheme === "light" ? "#F5F6F8" : "#17191D"
    readonly property color sidebarBg: resolvedTheme === "light" ? "#ECEEF2" : "#131519"
    readonly property color surface: resolvedTheme === "light" ? "#FFFFFF" : "#202329"
    readonly property color surfaceElevated: resolvedTheme === "light" ? "#FFFFFF" : "#282C34"
    readonly property color surfaceHover: resolvedTheme === "light" ? "#F0F2F5" : "#282C34"
    readonly property color surfaceActive: resolvedTheme === "light" ? "#E4E7EB" : "#30343C"

    // Borders
    readonly property color border: increaseContrast ? (resolvedTheme === "light" ? "rgba(0, 0, 0, 0.25)" : "#3D424D") : (resolvedTheme === "light" ? "rgba(20, 24, 30, 0.08)" : "#30343C")
    readonly property color borderSubtle: increaseContrast ? (resolvedTheme === "light" ? "rgba(0, 0, 0, 0.20)" : "#30343C") : (resolvedTheme === "light" ? "rgba(20, 24, 30, 0.05)" : "#282B33")
    readonly property color borderStrong: increaseContrast ? (resolvedTheme === "light" ? "rgba(0, 0, 0, 0.35)" : "#475569") : (resolvedTheme === "light" ? "rgba(20, 24, 30, 0.16)" : "#3D424D")

    // Typography
    readonly property color text: resolvedTheme === "light" ? "#1A1C20" : "#E8EAF0"
    readonly property color textSecondary: increaseContrast ? (resolvedTheme === "light" ? "#272a30" : "#d1d5db") : (resolvedTheme === "light" ? "#5f6368" : "#969BA6")
    readonly property color textMuted: increaseContrast ? (resolvedTheme === "light" ? "#3a3f4a" : "#9ca3af") : (resolvedTheme === "light" ? "#80868b" : "#7A808C")
    readonly property color textFaint: resolvedTheme === "light" ? "#9aa0a6" : "#555A66"

    // Semantic Status Colors
    readonly property color success: "#10b981"
    readonly property color successBg: resolvedTheme === "light" ? "rgba(16, 185, 129, 0.10)" : "#064e3b"
    readonly property color warning: "#f59e0b"
    readonly property color warningBg: resolvedTheme === "light" ? "rgba(245, 158, 11, 0.10)" : "#451a03"
    readonly property color error: "#ef4444"
    readonly property color errorBg: resolvedTheme === "light" ? "rgba(239, 68, 68, 0.10)" : "#7f1d1d"
    readonly property color info: "#3b82f6"
    readonly property color infoBg: resolvedTheme === "light" ? "rgba(59, 130, 246, 0.10)" : "#1e3a8a"

    // Nav tokens
    readonly property color navItem: resolvedTheme === "light" ? "#35373c" : "#A0A5AF"
    readonly property color navItemHover: resolvedTheme === "light" ? "#17181b" : "#F2F3F5"
    readonly property color navItemActiveBg: resolvedTheme === "light" ? "#E2E5EC" : "#202329"
    readonly property color navItemActiveText: resolvedTheme === "light" ? "#1A1C20" : "#E8EAF0"
    readonly property color navSectionLabel: resolvedTheme === "light" ? "#7c828f" : "#737985"

    // Geometry & Radii
    readonly property int radiusSm: 6
    readonly property int radiusMd: 8
    readonly property int radiusLg: 12
    readonly property int radiusXl: 16
    readonly property int radiusFull: 9999

    // Typography scale (Coherent Hierarchy)
    readonly property int fontBadge: 10
    readonly property int fontXs: 10
    readonly property int fontMeta: 11
    readonly property int fontSm: 11
    readonly property int fontNav: 13
    readonly property int fontBase: 13
    readonly property int fontCardDesc: 13
    readonly property int fontCardTitle: 15
    readonly property int fontMd: 15
    readonly property int fontLg: 16
    readonly property int fontSection: 18
    readonly property int fontXl: 19
    readonly property int fontTitle: 24
    readonly property int font2xl: 24
    readonly property int font3xl: 28

    // Initialization and Persistence
    property bool initialized: false

    Component.onCompleted: {
        try {
            var sys = backend.detectSystemAppearance()
            if (sys === "light" || sys === "dark") {
                systemScheme = sys
            }
            var saved = backend.loadAppearance()
            if (saved && saved["mode"]) {
                appearanceMode = saved["mode"]
            }
            if (saved && saved["accent"]) {
                accentName = saved["accent"]
            }
            if (saved && saved["reduce_transparency"] !== undefined) {
                reduceTransparency = saved["reduce_transparency"]
            }
            if (saved && saved["reduce_motion"] !== undefined) {
                reduceMotion = saved["reduce_motion"]
            }
            if (saved && saved["increase_contrast"] !== undefined) {
                increaseContrast = saved["increase_contrast"]
            }
        } catch (e) {
            console.log("[Theme] Failed to load appearance:", e)
        }
        initialized = true
    }

    function persistAppearance() {
        if (!initialized) return
        try {
            backend.saveAppearance({
                "mode": appearanceMode,
                "accent": accentName,
                "reduce_transparency": reduceTransparency,
                "reduce_motion": reduceMotion,
                "increase_contrast": increaseContrast
            })
        } catch (e) {
            console.log("[Theme] Failed to save appearance:", e)
        }
    }

    function setAccent(name) {
        if (swatches[name]) {
            accentName = name
            persistAppearance()
        }
    }

    function setAppearanceMode(mode) {
        if (mode === "dark" || mode === "light" || mode === "system") {
            appearanceMode = mode
            persistAppearance()
        }
    }

    function toggleReduceTransparency() {
        reduceTransparency = !reduceTransparency
        persistAppearance()
    }

    function toggleReduceMotion() {
        reduceMotion = !reduceMotion
        persistAppearance()
    }

    function toggleIncreaseContrast() {
        increaseContrast = !increaseContrast
        persistAppearance()
    }
}
