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

    // Color Swatches matching React ACCENT_SWATCHES & Extended Palettes
    readonly property var swatches: {
        "blue": "#6488E8",
        "silver": "#94A3B8",
        "purple": "#8b5cf6",
        "green": "#10b981",
        "orange": "#f97316",
        "rose": "#f43f5e",
        "cyan": "#06b6d4",
        "emerald": "#10b981",
        "amber": "#f97316"
    }

    // Swatch Names matching React
    readonly property var swatchNames: {
        "blue": "Ocean Blue",
        "silver": "Deep Silver",
        "purple": "Amethyst",
        "green": "Emerald",
        "orange": "Amber Fire",
        "rose": "Rose",
        "cyan": "Cyan",
        "emerald": "Emerald",
        "amber": "Amber Fire"
    }

    // Dynamic accent resolution matching tokens.css semantic tokens
    readonly property color accent: {
        if (accentName === "silver") {
            return resolvedTheme === "light" ? "#475569" : "#CBD5E1"
        } else if (accentName === "purple") {
            return resolvedTheme === "light" ? "#7c3aed" : "#8b5cf6"
        } else if (accentName === "green" || accentName === "emerald") {
            return resolvedTheme === "light" ? "#059669" : "#10b981"
        } else if (accentName === "orange" || accentName === "amber") {
            return resolvedTheme === "light" ? "#ea580c" : "#f97316"
        } else if (accentName === "rose") {
            return resolvedTheme === "light" ? "#e11d48" : "#f43f5e"
        } else if (accentName === "cyan") {
            return resolvedTheme === "light" ? "#0891b2" : "#06b6d4"
        }
        return resolvedTheme === "light" ? "#2563eb" : "#6488E8"
    }
    readonly property color accentHover: Qt.lighter(accent, 1.15)
    readonly property color accentGlow: Qt.rgba(accent.r, accent.g, accent.b, 0.12)
    readonly property color accentFaint: Qt.rgba(accent.r, accent.g, accent.b, 0.12)

    // Dedicated Switch Component Tokens with High-Contrast OFF/ON States
    readonly property color switchTrackOff: resolvedTheme === "light" ? (increaseContrast ? "#CBD5E1" : "#E2E8F0") : (increaseContrast ? "#333A48" : "#282C35")
    readonly property color switchTrackOffHover: resolvedTheme === "light" ? "#CBD5E1" : "#353B47"
    readonly property color switchTrackOffBorder: resolvedTheme === "light" ? (increaseContrast ? "#64748B" : "#94A3B8") : (increaseContrast ? "#64748B" : "#4A5262")
    readonly property color switchThumbOff: resolvedTheme === "light" ? "#FFFFFF" : "#CBD5E1"
    readonly property color switchThumbOffBorder: resolvedTheme === "light" ? "#94A3B8" : "#475569"
    readonly property color switchThumbOn: "#FFFFFF"

    // Surfaces & Canvas (Monochrome Deep Black Hierarchy matching --rz-* tokens)
    readonly property color bg: resolvedTheme === "light" ? "#F5F6F9" : "#14161A"
    readonly property alias background: root.bg
    readonly property color sidebarBg: resolvedTheme === "light" ? "#EBECEF" : "#101215"
    readonly property color surface: resolvedTheme === "light" ? "#FFFFFF" : "#1C1F24"
    readonly property color surfaceElevated: resolvedTheme === "light" ? "#FFFFFF" : "#242830"
    readonly property color surfaceHover: resolvedTheme === "light" ? "#F2F4F7" : "#262A33"
    readonly property color surfaceActive: resolvedTheme === "light" ? "#E5E7EB" : "#2C313B"

    // Borders (Pure Hex / Valid Colors - Never invalid CSS rgba strings)
    readonly property color border: increaseContrast ? (resolvedTheme === "light" ? "#B0B5C0" : "#4A505E") : (resolvedTheme === "light" ? "#D8DBE2" : "#2E333C")
    readonly property color borderSubtle: increaseContrast ? (resolvedTheme === "light" ? "#C8CCD6" : "#3D4350") : (resolvedTheme === "light" ? "#E5E7EB" : "#252830")
    readonly property color borderStrong: increaseContrast ? (resolvedTheme === "light" ? "#9096A4" : "#5A6172") : (resolvedTheme === "light" ? "#C2C6D1" : "#3D4350")

    // Dedicated Card Tokens
    readonly property color cardBg: surface
    readonly property color cardBorder: borderSubtle
    readonly property color cardBorderHover: Qt.rgba(accent.r, accent.g, accent.b, 0.45)
    readonly property color cardBorderFocused: accent
    readonly property color cardHoverBg: surfaceHover
    readonly property int cardRadius: 14

    // Typography Fonts & Weights
    readonly property string fontFamily: "Inter, Cantarell, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
    readonly property string fontMono: "'Maple Mono NF', 'JetBrains Mono', 'Fira Code', 'DejaVu Sans Mono', monospace"

    readonly property int weightRegular: Font.Normal
    readonly property int weightMedium: Font.Medium
    readonly property int weightSemibold: Font.DemiBold
    readonly property int weightBold: Font.Bold

    // Typography
    readonly property color text: resolvedTheme === "light" ? "#111827" : "#F1F3F7"
    readonly property color textSecondary: increaseContrast ? (resolvedTheme === "light" ? "#1F2937" : "#E5E7EB") : (resolvedTheme === "light" ? "#4B5563" : "#9CA3AF")
    readonly property color textMuted: increaseContrast ? (resolvedTheme === "light" ? "#374151" : "#D1D5DB") : (resolvedTheme === "light" ? "#6B7280" : "#6B7280")
    readonly property color textFaint: resolvedTheme === "light" ? "#9CA3AF" : "#4B5563"

    // Semantic Status Colors
    readonly property color success: "#10b981"
    readonly property color successBg: resolvedTheme === "light" ? "#ECFDF5" : "#064e3b"
    readonly property color warning: "#f59e0b"
    readonly property color warningBg: resolvedTheme === "light" ? "#FFFBEB" : "#451a03"
    readonly property color error: "#ef4444"
    readonly property color errorBg: resolvedTheme === "light" ? "#FEF2F2" : "#7f1d1d"
    readonly property color info: "#3b82f6"
    readonly property color infoBg: resolvedTheme === "light" ? "#EFF6FF" : "#1e3a8a"

    // Nav tokens
    readonly property color navItem: resolvedTheme === "light" ? "#374151" : "#A0A5AF"
    readonly property color navItemHover: resolvedTheme === "light" ? "#111827" : "#F2F3F5"
    readonly property color navItemActiveBg: resolvedTheme === "light" ? "#E2E5EC" : "#202329"
    readonly property color navItemActiveText: resolvedTheme === "light" ? "#111827" : "#F1F3F7"
    readonly property color navSectionLabel: resolvedTheme === "light" ? "#6B7280" : "#737985"

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
    readonly property int fontNav: 14
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
            if (typeof backend !== "undefined" && backend && backend.settingsData) {
                backend.settingsData["appearance_mode"] = appearanceMode
                backend.settingsData["accent_name"] = accentName
            }
        } catch (e) {
            console.log("[Theme] Failed to save appearance:", e)
        }
    }

    function setAccent(name) {
        var key = String(name || "").toLowerCase()
        if (key === "emerald") key = "green"
        if (key === "amber") key = "orange"
        if (swatches[key] !== undefined) {
            accentName = key
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
