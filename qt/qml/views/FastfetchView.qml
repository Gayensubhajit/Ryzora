import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import "../components"

Item {
    id: root

    signal backRequested()

    // ── Configuration & Preset State ──
    property string selectedPresetId: ""
    property string selectedPresetTitle: "Default"
    property string baseConfigJsonc: ""
    property string accentColor: "#6488E8"
    onAccentColorChanged: {
        if (accentHexInput && accentHexInput.text.toLowerCase() !== root.accentColor.toLowerCase()) {
            accentHexInput.text = root.accentColor
        }
    }
    property string displaySeparator: "  "
    property bool colorModified: false
    property bool separatorModified: false
    property string emblemType: "none" // "image" | "ascii" | "builtin" | "none"
    property string selectedEmblemId: ""
    property string customEmblemPath: ""
    property int widthCols: 36
    property int heightLines: 18
    property int paddingCols: 3
    property int paddingTop: 2
    property bool dither: false
    property bool modulesModified: false
    property bool logoModified: false
    property string styleDropdownSearch: ""

    // UI State
    property bool isModified: false
    property bool isSaved: true
    property string presetSearch: ""
    property string presetFilter: "all" // "all" | "installed"
    property string inspectorSection: "appearance" // "appearance" | "layout" | "colors" | "modules"
    property bool showAddModuleBar: false
    property bool showRawConfigModal: false
    property string actionFeedback: ""
    property string actionFeedbackType: "info" // "info" | "success" | "error"

    function defaultModuleRows() {
        return [
            { id: "r-title", rowType: "telemetry", key: "title", label: "Host / User Title", customText: "", enabled: true },
            { id: "r-tagline", rowType: "tagline", key: "custom", label: "Tagline", customText: "Ryzora — hand-crafted Linux desktop", enabled: true },
            { id: "r-sp1", rowType: "spacer", key: "break", label: "Spacer", customText: "", enabled: true },
            { id: "r-h-vitals", rowType: "section", key: "custom", label: "Section Header", customText: "VITALS", enabled: true },
            { id: "r-cpu", rowType: "telemetry", key: "cpu", label: "CPU", customText: "", enabled: true },
            { id: "r-gpu", rowType: "telemetry", key: "gpu", label: "GPU", customText: "", enabled: true },
            { id: "r-mem", rowType: "telemetry", key: "memory", label: "MEMORY", customText: "", enabled: true },
            { id: "r-disk", rowType: "telemetry", key: "disk", label: "DISK", customText: "", enabled: true },
            { id: "r-sp2", rowType: "spacer", key: "break", label: "Spacer", customText: "", enabled: true },
            { id: "r-h-sys", rowType: "section", key: "custom", label: "Section Header", customText: "SYSTEM", enabled: true },
            { id: "r-os", rowType: "telemetry", key: "os", label: "OS", customText: "", enabled: true },
            { id: "r-kernel", rowType: "telemetry", key: "kernel", label: "KERNEL", customText: "", enabled: true },
            { id: "r-wm", rowType: "telemetry", key: "wm", label: "WM", customText: "", enabled: true },
            { id: "r-shell", rowType: "telemetry", key: "shell", label: "SHELL", customText: "", enabled: true },
            { id: "r-pkgs", rowType: "telemetry", key: "packages", label: "PKGS", customText: "", enabled: true },
            { id: "r-sp3", rowType: "spacer", key: "break", label: "Spacer", customText: "", enabled: true },
            { id: "r-h-sess", rowType: "section", key: "custom", label: "Section Header", customText: "SESSION", enabled: true },
            { id: "r-uptime", rowType: "telemetry", key: "uptime", label: "UPTIME", customText: "", enabled: true },
            { id: "r-term", rowType: "telemetry", key: "terminal", label: "TERM", customText: "", enabled: true },
            { id: "r-colors", rowType: "telemetry", key: "colors", label: "Color Dots", customText: "", enabled: true }
        ]
    }

    property var moduleList: defaultModuleRows()

    property var availableNewModules: [
        { rowType: "spacer", key: "break", label: "Spacer", customText: "" },
        { rowType: "section", key: "custom", label: "Section Header", customText: "SECTION" },
        { rowType: "tagline", key: "custom", label: "Tagline", customText: "Ryzora — hand-crafted Linux desktop" },
        { rowType: "telemetry", key: "cpu", label: "CPU", customText: "" },
        { rowType: "telemetry", key: "gpu", label: "GPU", customText: "" },
        { rowType: "telemetry", key: "memory", label: "MEMORY", customText: "" },
        { rowType: "telemetry", key: "disk", label: "DISK", customText: "" },
        { rowType: "telemetry", key: "os", label: "OS", customText: "" },
        { rowType: "telemetry", key: "host", label: "HOST", customText: "" },
        { rowType: "telemetry", key: "kernel", label: "KERNEL", customText: "" },
        { rowType: "telemetry", key: "wm", label: "WM", customText: "" },
        { rowType: "telemetry", key: "de", label: "DE", customText: "" },
        { rowType: "telemetry", key: "shell", label: "SHELL", customText: "" },
        { rowType: "telemetry", key: "packages", label: "PKGS", customText: "" },
        { rowType: "telemetry", key: "uptime", label: "UPTIME", customText: "" },
        { rowType: "telemetry", key: "terminal", label: "TERM", customText: "" },
        { rowType: "telemetry", key: "display", label: "DISPLAY", customText: "" },
        { rowType: "telemetry", key: "theme", label: "THEME", customText: "" },
        { rowType: "telemetry", key: "icons", label: "ICONS", customText: "" },
        { rowType: "telemetry", key: "battery", label: "BATTERY", customText: "" },
        { rowType: "telemetry", key: "localip", label: "LOCAL IP", customText: "" },
        { rowType: "telemetry", key: "colors", label: "Color Dots", customText: "" }
    ]

    function stripJsonc(raw) {
        if (!raw) return ""
        var s = String(raw)
        var out = ""
        var inStr = false
        var esc = false
        var i = 0
        var n = s.length
        while (i < n) {
            var ch = s[i]
            var next = (i + 1 < n) ? s[i + 1] : ""
            if (inStr) {
                out += ch
                if (esc) {
                    esc = false
                } else if (ch === "\\") {
                    esc = true
                } else if (ch === "\"") {
                    inStr = false
                }
                i++
            } else {
                if (ch === "\"") {
                    inStr = true
                    out += ch
                    i++
                } else if (ch === "/" && next === "/") {
                    i += 2
                    while (i < n && s[i] !== "\n") i++
                } else if (ch === "/" && next === "*") {
                    i += 2
                    while (i + 1 < n && !(s[i] === "*" && s[i + 1] === "/")) i++
                    i += 2
                } else {
                    out += ch
                    i++
                }
            }
        }
        return out.replace(/,(\s*[\]}])/g, "$1")
    }

    function resolvePresetThumbnail(presetId) {
        if (!presetId || presetId === "default") {
            var embs = backend.fastfetchLocalEmblems || []
            for (var e = 0; e < embs.length; ++e) {
                if (embs[e].id === "katana" && embs[e].path) {
                    return String(embs[e].path).indexOf("file://") === 0 ? String(embs[e].path) : ("file://" + String(embs[e].path))
                }
            }
            return ""
        }
        try {
            var cfg = backend.getFastfetchPresetConfig(presetId)
            var parsed = JSON.parse(stripJsonc(cfg))
            if (parsed && parsed.logo && parsed.logo.source) {
                var src = String(parsed.logo.source)
                var sl = src.toLowerCase()
                if (sl.endsWith(".png") || sl.endsWith(".jpg") || sl.endsWith(".jpeg") || sl.endsWith(".webp") || sl.endsWith(".svg")) {
                    if (src.indexOf("/") === 0) return "file://" + src
                    if (src.indexOf("file://") === 0) return src
                }
            }
        } catch (err) {}
        var catList = backend.catalogList || []
        for (var i = 0; i < catList.length; ++i) {
            var c = catList[i]
            if (c && (c.id === presetId || c.id === "fastfetch-" + presetId)) {
                var img = String(c.image || c.preview_url || c.thumbnail || "")
                if (img.length > 0 && !img.endsWith(".mp4")) {
                    return img
                }
            }
        }
        return ""
    }

    readonly property var installedStyles: {
        var _libDep = backend.libraryList
        var _instDep = backend.fastfetchInstalledPresets
        var _actDep = root.activeFastfetchId
        var _embDep = backend.fastfetchLocalEmblems
        return getInstalledStyles()
    }

    readonly property var filteredInstalledStyles: {
        var all = root.installedStyles || []
        var q = String(root.styleDropdownSearch || "").trim().toLowerCase()
        if (q.length === 0) return all
        var res = []
        for (var i = 0; i < all.length; ++i) {
            var it = all[i]
            if (String(it.title || "").toLowerCase().indexOf(q) !== -1 ||
                String(it.id || "").toLowerCase().indexOf(q) !== -1 ||
                String(it.author || "").toLowerCase().indexOf(q) !== -1) {
                res.push(it)
            }
        }
        return res
    }

    function getInstalledStyles() {
        var res = []
        var seen = {}

        // 1. Default system fastfetch configuration
        var isDefActive = !root.activeFastfetchId || root.activeFastfetchId === "" || root.activeFastfetchId === "default"
        res.push({
            id: "default",
            title: "Default Fastfetch",
            author: "Fastfetch Team",
            thumbnail: resolvePresetThumbnail("default"),
            isActive: isDefActive
        })
        seen["default"] = true

        // 2. Discover genuinely installed packages from backend.libraryList
        var lib = backend.libraryList || []
        for (var i = 0; i < lib.length; ++i) {
            var item = lib[i]
            if (!item || !item.id) continue
            var id = String(item.id)
            var cat = String(item.category || item.package_type || "").toLowerCase()
            var isFf = cat === "fastfetch" || id.indexOf("fastfetch-") === 0
            var isEmblem = id.indexOf("emblem") !== -1 || cat === "emblems" || cat === "emblem"
            if (isFf && !isEmblem) {
                if (!seen[id]) {
                    seen[id] = true
                    var pInfo = root.getPresetItem(id)
                    res.push({
                        id: id,
                        title: pInfo.title || item.name || id,
                        author: pInfo.author || item.author || "",
                        thumbnail: resolvePresetThumbnail(id),
                        isActive: root.isPresetActive(id)
                    })
                }
            }
        }

        // 3. Inspect presets directory matching library installations
        var rawInstalled = backend.fastfetchInstalledPresets || []
        for (var j = 0; j < rawInstalled.length; ++j) {
            var pId = String(rawInstalled[j])
            if (pId.indexOf("emblem") !== -1) continue
            if (!seen[pId] && root.isPresetInstalledInLibrary(pId)) {
                seen[pId] = true
                var pInfo2 = root.getPresetItem(pId)
                res.push({
                    id: pId,
                    title: pInfo2.title || pId,
                    author: pInfo2.author || "",
                    thumbnail: resolvePresetThumbnail(pId),
                    isActive: root.isPresetActive(pId)
                })
            }
        }

        return res
    }

    readonly property string activeFastfetchId: {
        if (backend.activeConfiguration && backend.activeConfiguration.active_fastfetch_id) {
            return String(backend.activeConfiguration.active_fastfetch_id)
        }
        var lib = backend.libraryList || []
        for (var i = 0; i < lib.length; ++i) {
            var item = lib[i]
            if (item && (item.category === "fastfetch" || String(item.id || "").indexOf("fastfetch-") === 0) && item.is_active) {
                return String(item.id)
            }
        }
        return ""
    }

    function isPresetInstalledInLibrary(id) {
        if (!id) return false
        var clean = String(id).replace(/^fastfetch-/, "")
        var full = "fastfetch-" + clean
        var lib = backend.libraryList || []
        for (var i = 0; i < lib.length; ++i) {
            var item = lib[i]
            if (!item || !item.id) continue
            if (item.id === id || item.id === full || item.id === clean) {
                return true
            }
        }
        return false
    }

    function isPresetActive(id) {
        if (!id) return false
        var active = root.activeFastfetchId
        if (!active || active === "" || active === "default") {
            return id === "default"
        }
        var cleanA = String(id).replace(/^fastfetch-/, "")
        var cleanB = String(active).replace(/^fastfetch-/, "")
        return cleanA === cleanB || id === active
    }

    function getPresetItem(item) {
        if (!item) return { id: "", title: "", isActive: false, isInstalled: false }
        var idStr = typeof item === "string" ? item : String(item.id || "")
        var cleanTitle = typeof item === "string"
            ? idStr.replace(/^fastfetch-/, "").replace(/[-_]/g, " ")
            : String(item.title || item.name || idStr)
        if (cleanTitle.length > 0 && typeof item === "string") {
            cleanTitle = cleanTitle.split(" ").map(function(w) {
                return w.length > 0 ? (w.charAt(0).toUpperCase() + w.slice(1)) : ""
            }).join(" ")
        }
        var catList = backend.catalogList || []
        var author = ""
        for (var i = 0; i < catList.length; ++i) {
            var c = catList[i]
            if (c && (c.id === idStr || c.id === "fastfetch-" + idStr)) {
                if (c.name) cleanTitle = String(c.name)
                if (c.author) author = String(c.author)
                break
            }
        }
        return {
            id: idStr,
            title: cleanTitle,
            author: author,
            isActive: root.isPresetActive(idStr),
            isInstalled: root.isPresetInstalledInLibrary(idStr)
        }
    }

    readonly property var filteredPresets: {
        var all = backend.fastfetchInstalledPresets || []
        var q = root.presetSearch.trim().toLowerCase()
        var out = []
        for (var i = 0; i < all.length; ++i) {
            var pInfo = root.getPresetItem(all[i])
            if (!pInfo.id || pInfo.id.indexOf("emblem") !== -1) continue
            if (root.presetFilter === "installed" && !pInfo.isInstalled && !pInfo.isActive) {
                continue
            }
            if (q.length > 0) {
                if (pInfo.title.toLowerCase().indexOf(q) === -1 && pInfo.id.toLowerCase().indexOf(q) === -1) {
                    continue
                }
            }
            out.push(pInfo)
        }
        return out
    }

    Timer {
        id: previewDebounceTimer
        interval: 180
        repeat: false
        onTriggered: root.updatePreview()
    }

    Timer {
        id: feedbackClearTimer
        interval: 4500
        repeat: false
        onTriggered: root.actionFeedback = ""
    }

    function showFeedback(type, msg) {
        root.actionFeedbackType = type
        root.actionFeedback = msg
        feedbackClearTimer.restart()
    }

    Connections {
        target: backend
        function onFastfetchLocalEmblemsChanged() {
            if (root.emblemType === "image" && (!root.customEmblemPath || root.customEmblemPath.length === 0)) {
                root.selectedEmblemId = root.selectedEmblemId
            }
        }
        function onFastfetchInstalledPresetsChanged() {
            if (root.selectedPresetId) {
                var pInfo = root.getPresetItem(root.selectedPresetId)
                if (pInfo.title) root.selectedPresetTitle = pInfo.title
            }
        }
        function onFastfetchChanged() {
            // Live terminal output updated
        }
        function onLibraryChanged() {
            // Installed styles cache updated
        }
    }

    function markModified() {
        root.isModified = true
        root.isSaved = false
        previewDebounceTimer.restart()
    }

    function resolveCurrentEmblemPath() {
        if (root.customEmblemPath && root.customEmblemPath.trim().length > 0) {
            return root.customEmblemPath.trim()
        }
        var list = backend.fastfetchLocalEmblems || []
        var cleanTarget = root.selectedEmblemId.replace(/^fastfetch-emblem-/, "").replace(/^emblem-/, "").toLowerCase()
        if (cleanTarget.length > 0) {
            for (var i = 0; i < list.length; ++i) {
                var item = list[i]
                var itemId = String(item.id || "").replace(/^fastfetch-emblem-/, "").toLowerCase()
                if (itemId === cleanTarget) {
                    return String(item.path || "")
                }
            }
        }
        if (list.length > 0 && root.emblemType === "image") {
            return String(list[0].path || "")
        }
        return ""
    }

    readonly property string resolvedEmblemPath: {
        var _typeDep = root.emblemType
        var _selDep = root.selectedEmblemId
        var _custDep = root.customEmblemPath
        var _listDep = backend.fastfetchLocalEmblems
        return resolveCurrentEmblemPath()
    }

    readonly property string resolvedEmblemUrl: {
        var p = root.resolvedEmblemPath
        if (!p || p.length === 0) return ""
        return p.indexOf("file://") === 0 ? p : ("file://" + p)
    }

    function isPresetLocalEmblem(p) {
        if (!p) return false
        var clean = String(p).replace(/^file:\/\//, "")
        var list = backend.fastfetchLocalEmblems || []
        for (var i = 0; i < list.length; ++i) {
            var itemPath = String(list[i].path || "").replace(/^file:\/\//, "")
            if (itemPath === clean) return true
        }
        return false
    }

    function colorToHex(col) {
        if (!col) return ""
        var s = String(col).trim()
        if (s.indexOf("#") === 0) {
            if (s.length === 7) return s.toUpperCase()
            if (s.length === 4) return ("#" + s[1] + s[1] + s[2] + s[2] + s[3] + s[3]).toUpperCase()
            return s.toUpperCase()
        }
        // Match 38;2;R;G;B (with or without escape codes or prefix)
        var rgbMatch = s.match(/(?:38;2;)?(\d{1,3});(\d{1,3});(\d{1,3})/)
        if (rgbMatch) {
            var r = parseInt(rgbMatch[1], 10)
            var g = parseInt(rgbMatch[2], 10)
            var b = parseInt(rgbMatch[3], 10)
            if (r <= 255 && g <= 255 && b <= 255) {
                var rh = (r < 16 ? "0" : "") + r.toString(16)
                var gh = (g < 16 ? "0" : "") + g.toString(16)
                var bh = (b < 16 ? "0" : "") + b.toString(16)
                return ("#" + rh + gh + bh).toUpperCase()
            }
        }
        var map = {
            "black": "#1E293B",
            "red": "#EF4444",
            "green": "#10B981",
            "yellow": "#F59E0B",
            "blue": "#3B82F6",
            "magenta": "#EC4899",
            "purple": "#A855F7",
            "cyan": "#06B6D4",
            "white": "#F1F5F9",
            "bright_black": "#64748B",
            "bright_red": "#F87171",
            "bright_green": "#34D399",
            "bright_yellow": "#FBBF24",
            "bright_blue": "#60A5FA",
            "bright_magenta": "#F472B6",
            "bright_cyan": "#22D3EE",
            "bright_white": "#FFFFFF",
            "default": "#6488E8",
            "1": "#EF4444", "31": "#EF4444", "91": "#F87171",
            "2": "#10B981", "32": "#10B981", "92": "#34D399",
            "3": "#F59E0B", "33": "#F59E0B", "93": "#FBBF24",
            "4": "#3B82F6", "34": "#3B82F6", "94": "#60A5FA",
            "5": "#EC4899", "35": "#EC4899", "95": "#F472B6",
            "6": "#06B6D4", "36": "#06B6D4", "96": "#22D3EE",
            "7": "#F1F5F9", "37": "#F1F5F9", "97": "#FFFFFF",
            "8": "#1E293B", "30": "#1E293B", "90": "#64748B",
            "9": "#6488E8"
        }
        var lower = s.toLowerCase()
        if (map[lower]) return map[lower]
        return ""
    }

    function rgbFromHex(hexStr) {
        var h = String(hexStr || "#6488E8").replace("#", "").trim()
        if (h.length === 6) {
            var r = parseInt(h.substring(0, 2), 16)
            var g = parseInt(h.substring(2, 4), 16)
            var b = parseInt(h.substring(4, 6), 16)
            if (!isNaN(r) && !isNaN(g) && !isNaN(b)) {
                return { r: r, g: g, b: b }
            }
        }
        return { r: 100, g: 136, b: 232 }
    }

    function buildDraftJson() {
        var config = {
            "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json"
        }
        if (root.baseConfigJsonc && root.baseConfigJsonc.trim().length > 0) {
            try {
                var stripped = root.stripJsonc(root.baseConfigJsonc)
                var parsed = JSON.parse(stripped)
                if (parsed && typeof parsed === "object") {
                    config = parsed
                }
            } catch (e) {}
        }

        // 1. Logo
        if (root.emblemType === "none") {
            config["logo"] = { "type": "none" }
        } else if (root.emblemType === "builtin") {
            config["logo"] = {
                "type": "builtin",
                "padding": { "top": root.paddingTop, "left": 1, "right": root.paddingCols }
            }
        } else if (root.emblemType === "ascii") {
            var path = root.resolvedEmblemPath
            if (path && path.length > 0 && !path.toLowerCase().endsWith(".png") && !path.toLowerCase().endsWith(".jpg") && !path.toLowerCase().endsWith(".jpeg") && !path.toLowerCase().endsWith(".webp") && !path.toLowerCase().endsWith(".svg")) {
                config["logo"] = { "type": "file", "source": path, "padding": { "top": root.paddingTop, "left": 1, "right": root.paddingCols } }
            } else {
                config["logo"] = { "type": "auto", "padding": { "top": root.paddingTop, "left": 1, "right": root.paddingCols } }
            }
        } else if (root.emblemType === "image") {
            var imgPath = root.resolvedEmblemPath
            config["logo"] = {
                "type": "kitty-direct",
                "source": imgPath,
                "width": root.widthCols,
                "height": root.heightLines,
                "padding": { "top": root.paddingTop, "left": 1, "right": root.paddingCols }
            }
        }

        // 2. Display, Separator & Accent Color
        if (!config["display"] || typeof config["display"] !== "object") config["display"] = {}
        if (root.separatorModified || typeof config["display"]["separator"] !== "string") {
            config["display"]["separator"] = root.displaySeparator
        }
        var rgb = rgbFromHex(root.accentColor)
        var ansiRgb = "38;2;" + rgb.r + ";" + rgb.g + ";" + rgb.b
        if (root.colorModified) {
            config["display"]["color"] = {
                "keys": ansiRgb,
                "title": ansiRgb
            }
        }

        // 3. Modules (Generate from moduleList when modified or missing, preserving rawModule details)
        var needModuleGen = root.modulesModified || !config["modules"] || !Array.isArray(config["modules"]) || config["modules"].length === 0
        if (needModuleGen) {
            var modsList = (root.moduleList && root.moduleList.length > 0) ? root.moduleList : root.defaultModuleRows()
            var mods = []
            for (var i = 0; i < modsList.length; ++i) {
                var m = modsList[i]
                if (!m.enabled) continue
                if (m.rawModule && !m.customModified) {
                    var cloned = JSON.parse(JSON.stringify(m.rawModule))
                    if (typeof cloned === "object" && cloned !== null && m.labelModified && m.label) {
                        cloned["key"] = m.label
                    }
                    mods.push(cloned)
                } else if (m.rowType === "spacer" || m.key === "break" || m.key === "separator") {
                    mods.push("break")
                } else if (m.rowType === "tagline") {
                    var tagTxt = m.customText || "Ryzora — hand-crafted Linux desktop"
                    mods.push({
                        "type": "custom",
                        "format": "\u001b[38;2;" + rgb.r + ";" + rgb.g + ";" + rgb.b + "m■\u001b[0m \u001b[2m" + tagTxt + "\u001b[0m"
                    })
                } else if (m.rowType === "section") {
                    var secTitle = (m.customText || "SECTION").toUpperCase()
                    var dashesLen = Math.max(6, 42 - secTitle.length)
                    var dashes = ""
                    for (var d = 0; d < dashesLen; ++d) dashes += "─"
                    mods.push({
                        "type": "custom",
                        "format": "\u001b[2m── " + secTitle + " " + dashes + "\u001b[0m"
                    })
                } else if (m.key === "title") {
                    mods.push("title")
                } else if (m.key === "colors") {
                    mods.push({ "type": "colors", "symbol": "circle" })
                } else {
                    mods.push({
                        "type": m.key,
                        "key": (m.label || m.key).toUpperCase()
                    })
                }
            }
            config["modules"] = mods
        }

        // If user explicitly changed accent color, propagate to per-module keyColor so presets with hardcoded keyColors update live
        if (root.colorModified && Array.isArray(config["modules"])) {
            for (var kIdx = 0; kIdx < config["modules"].length; ++kIdx) {
                var modEntry = config["modules"][kIdx]
                if (modEntry && typeof modEntry === "object") {
                    if (modEntry.type !== "custom" && modEntry.type !== "colors") {
                        modEntry["keyColor"] = ansiRgb
                        if (modEntry["keycolor"]) delete modEntry["keycolor"]
                        if (modEntry["key_color"]) delete modEntry["key_color"]
                    } else if (modEntry.type === "custom" && modEntry.format) {
                        modEntry.format = String(modEntry.format).replace(/(?:\\u001b|\x1b)\[38;2;\d+;\d+;\d+m/g, "\u001b[" + ansiRgb + "m")
                    }
                }
            }
        }

        return JSON.stringify(config, null, 2)
    }

    property string _lastPreviewDraft: ""

    function updatePreview(force) {
        if (!root.visible) return
        var draft = buildDraftJson()
        if (!force && draft === root._lastPreviewDraft && backend.fastfetchHtml && backend.fastfetchHtml.length > 0) {
            return
        }
        root._lastPreviewDraft = draft
        backend.requestDraftPreview(draft)
    }

    onVisibleChanged: {
        if (visible) {
            updatePreview(false)
        }
    }

    function parseLoadedConfig(jsonc) {
        if (!jsonc || jsonc.trim().length === 0) return
        try {
            var stripped = root.stripJsonc(jsonc)
            var parsed = JSON.parse(stripped)

            if (parsed.logo && typeof parsed.logo === "object") {
                var ltype = String(parsed.logo.type || "").toLowerCase()
                var src = String(parsed.logo.source || "")
                var srcLower = src.toLowerCase()
                var isImageExt = srcLower.endsWith(".png") || srcLower.endsWith(".jpg") || srcLower.endsWith(".jpeg") || srcLower.endsWith(".webp") || srcLower.endsWith(".svg")
                var isImageType = ltype.indexOf("kitty") !== -1 || ltype.indexOf("image") !== -1 || ltype.indexOf("chafa") !== -1 || ltype === "iterm" || ltype === "sixel"

                if (ltype === "none") {
                    root.emblemType = "none"
                    root.customEmblemPath = ""
                } else if (ltype === "builtin") {
                    root.emblemType = "builtin"
                    root.customEmblemPath = ""
                } else if (isImageType || isImageExt) {
                    root.emblemType = "image"
                    var list = backend.fastfetchLocalEmblems || []
                    var matched = false
                    for (var i = 0; i < list.length; ++i) {
                        var embId = String(list[i].id || "").toLowerCase()
                        if (list[i].path === src || embId === srcLower || srcLower.endsWith("/" + embId + ".png")) {
                            root.selectedEmblemId = list[i].id
                            root.customEmblemPath = String(list[i].path || "")
                            matched = true
                            break
                        }
                    }
                    if (!matched && src.length > 0) {
                        root.customEmblemPath = src
                    }
                } else if (ltype === "file" || ltype.indexOf("ascii") !== -1 || ltype === "auto" || src.length > 0) {
                    root.emblemType = "ascii"
                    root.customEmblemPath = src
                } else {
                    root.emblemType = "builtin"
                    root.customEmblemPath = ""
                }

                if (typeof parsed.logo.width === "number" && parsed.logo.width > 0) {
                    root.widthCols = parsed.logo.width
                } else {
                    root.widthCols = 36
                }
                if (typeof parsed.logo.height === "number" && parsed.logo.height > 0) {
                    root.heightLines = parsed.logo.height
                } else {
                    root.heightLines = 18
                }
                if (parsed.logo.padding && typeof parsed.logo.padding === "object") {
                    if (typeof parsed.logo.padding.top === "number") {
                        root.paddingTop = Math.max(0, Math.min(10, parsed.logo.padding.top))
                    } else {
                        root.paddingTop = (root.emblemType === "image" ? 2 : 1)
                    }
                    if (typeof parsed.logo.padding.right === "number") {
                        root.paddingCols = parsed.logo.padding.right
                    } else if (typeof parsed.logo.padding.left === "number") {
                        root.paddingCols = parsed.logo.padding.left
                    } else {
                        root.paddingCols = 3
                    }
                } else {
                    root.paddingTop = (root.emblemType === "image" ? 2 : 1)
                    root.paddingCols = 3
                }
            } else {
                root.emblemType = "builtin"
                root.customEmblemPath = ""
                root.widthCols = 36
                root.heightLines = 18
                root.paddingTop = 2
                root.paddingCols = 3
            }

            if (parsed.display && typeof parsed.display === "object") {
                if (typeof parsed.display.separator === "string") {
                    root.displaySeparator = parsed.display.separator
                } else {
                    root.displaySeparator = "  "
                }
            } else {
                root.displaySeparator = "  "
            }

            var extractedColor = ""
            if (parsed.display && typeof parsed.display === "object" && parsed.display.color) {
                if (typeof parsed.display.color === "string") {
                    extractedColor = colorToHex(parsed.display.color)
                } else if (typeof parsed.display.color === "object") {
                    if (parsed.display.color.keys) extractedColor = colorToHex(parsed.display.color.keys)
                    else if (parsed.display.color.title) extractedColor = colorToHex(parsed.display.color.title)
                }
            }
            if (!extractedColor && Array.isArray(parsed.modules)) {
                for (var mi = 0; mi < parsed.modules.length; ++mi) {
                    var mObj = parsed.modules[mi]
                    if (mObj && typeof mObj === "object") {
                        var kc = mObj.keyColor || mObj.keycolor || mObj.key_color || mObj.color
                        if (kc) {
                            extractedColor = colorToHex(kc)
                            if (extractedColor) break
                        }
                        if (mObj.format && typeof mObj.format === "string") {
                            extractedColor = colorToHex(mObj.format)
                            if (extractedColor) break
                        }
                    }
                }
            }
            if (extractedColor) {
                root.accentColor = extractedColor
            }

            if (Array.isArray(parsed.modules) && parsed.modules.length > 0) {
                var newMods = []
                for (var mIdx = 0; mIdx < parsed.modules.length; ++mIdx) {
                    var item = parsed.modules[mIdx]
                    var rawCopy = JSON.parse(JSON.stringify(item))
                    if (typeof item === "string") {
                        var sKey = item.toLowerCase()
                        if (sKey === "break" || sKey === "separator") {
                            newMods.push({ id: "mod-" + mIdx, rowType: "spacer", key: sKey, label: "Spacer", customText: "", enabled: true, rawModule: rawCopy })
                        } else {
                            newMods.push({ id: "mod-" + mIdx, rowType: "telemetry", key: sKey, label: sKey.toUpperCase(), customText: "", enabled: true, rawModule: rawCopy })
                        }
                    } else if (item && typeof item === "object") {
                        var oType = String(item.type || "").toLowerCase()
                        var fmt = String(item.format || "")
                        if (oType === "break" || oType === "separator") {
                            newMods.push({ id: "mod-" + mIdx, rowType: "spacer", key: "break", label: "Spacer", customText: "", enabled: true, rawModule: rawCopy })
                        } else if (oType === "custom") {
                            var cleanFmt = fmt.replace(/\x1b\[[0-9;]*m/g, "").trim()
                            if (cleanFmt.indexOf("──") === 0) {
                                var secName = cleanFmt.replace(/^──\s*/, "").replace(/\s*─+$/, "").trim()
                                newMods.push({ id: "mod-" + mIdx, rowType: "section", key: "custom", label: "Section Header", customText: secName || "SECTION", enabled: true, rawModule: rawCopy })
                            } else {
                                var tagClean = cleanFmt.replace(/^■\s*/, "").trim()
                                newMods.push({ id: "mod-" + mIdx, rowType: "tagline", key: "custom", label: "Tagline", customText: tagClean || cleanFmt, enabled: true, rawModule: rawCopy })
                            }
                        } else {
                            var lbl = item.key ? String(item.key).replace(/\x1b\[[0-9;]*m/g, "").trim() : oType.toUpperCase()
                            if (!lbl) lbl = oType.toUpperCase()
                            newMods.push({ id: "mod-" + mIdx, rowType: "telemetry", key: oType, label: lbl, customText: "", enabled: true, rawModule: rawCopy })
                        }
                    }
                }
                if (newMods.length > 0) {
                    root.moduleList = newMods
                } else {
                    root.moduleList = root.defaultModuleRows()
                }
            } else {
                root.moduleList = root.defaultModuleRows()
            }
        } catch (e) {
            console.warn("[FastfetchView] Failed to parse config:", e)
        }
    }

    function loadPreset(presetId) {
        if (!presetId) return
        root.selectedPresetId = presetId
        if (presetId === "default") {
            root.selectedPresetTitle = "Default Fastfetch"
            var saved = backend.getFastfetchSavedConfig()
            if (saved && saved.trim().length > 0) {
                root.baseConfigJsonc = saved
                parseLoadedConfig(saved)
            } else {
                root.moduleList = root.defaultModuleRows()
                root.emblemType = "image"
                root.selectedEmblemId = "katana"
                root.customEmblemPath = ""
                root.widthCols = 36
                root.heightLines = 18
                root.paddingCols = 3
                root.paddingTop = 2
            }
            root.colorModified = false
            root.separatorModified = false
            root.modulesModified = false
            root.logoModified = false
            root.isModified = false
            root.isSaved = true
            updatePreview()
            return
        }
        var pInfo = root.getPresetItem(presetId)
        root.selectedPresetTitle = pInfo.title || presetId
        var cfg = backend.getFastfetchPresetConfig(presetId)
        if (cfg && cfg.trim().length > 0) {
            root.baseConfigJsonc = cfg
            parseLoadedConfig(cfg)
            root.colorModified = false
            root.separatorModified = false
            root.modulesModified = false
            root.logoModified = false
        }
        root.isModified = false
        root.isSaved = true
        updatePreview()
    }

    function loadPackage(packageId) {
        if (!packageId) return
        var pid = String(packageId)
        if (pid.indexOf("emblem") !== -1) {
            var cleanEmb = pid.replace(/^fastfetch-emblem-/, "").replace(/^emblem-/, "").toLowerCase()
            root.selectedEmblemId = cleanEmb
            root.customEmblemPath = ""
            root.emblemType = "image"
            root.inspectorSection = "appearance"
            root.markModified()
            return
        }
        loadPreset(pid)
    }

    function triggerApplySelectedStyle() {
        var targetId = root.selectedPresetId || "default"
        if (targetId === "default") {
            var draft = root.buildDraftJson()
            if (backend.saveFastfetchConfiguration(draft, "default")) {
                backend.applyFastfetchPreset("default")
                root.baseConfigJsonc = draft
                root.isSaved = true
                root.isModified = false
                root.showFeedback("success", "Applied default style to ~/.config/fastfetch/config.jsonc.")
                root.updatePreview()
            } else {
                root.showFeedback("error", "Failed to apply default configuration.")
            }
            return
        }

        if (root.isModified) {
            var draftMod = root.buildDraftJson()
            if (backend.saveFastfetchConfiguration(draftMod, targetId)) {
                backend.applyFastfetchPreset(targetId)
                root.baseConfigJsonc = draftMod
                root.isSaved = true
                root.isModified = false
                root.showFeedback("success", "Saved & applied style '" + root.selectedPresetTitle + "'.")
                root.updatePreview()
            } else {
                root.showFeedback("error", "Failed to validate or apply style.")
            }
            return
        }

        if (backend.applyFastfetchPreset(targetId)) {
            var saved = backend.getFastfetchSavedConfig()
            if (saved && saved.trim().length > 0) {
                root.baseConfigJsonc = saved
                parseLoadedConfig(saved)
            }
            root.isSaved = true
            root.isModified = false
            root.showFeedback("success", "Applied style '" + root.selectedPresetTitle + "' to ~/.config/fastfetch/config.jsonc.")
            root.updatePreview()
        } else {
            root.showFeedback("error", "Failed to apply preset '" + root.selectedPresetTitle + "'.")
        }
    }

    function triggerSaveDraft() {
        var draft = root.buildDraftJson()
        if (backend.saveFastfetchConfiguration(draft, root.selectedPresetId || "custom")) {
            root.baseConfigJsonc = draft
            root.isSaved = true
            root.isModified = false
            root.showFeedback("success", "Saved configuration to ~/.config/fastfetch/config.jsonc.")
        } else {
            root.showFeedback("error", "Failed to save configuration.")
        }
    }

    function triggerRevertDraft() {
        if (root.baseConfigJsonc && root.baseConfigJsonc.trim().length > 0) {
            root.parseLoadedConfig(root.baseConfigJsonc)
            root.colorModified = false
            root.separatorModified = false
            root.modulesModified = false
        } else {
            root.initStudio()
            return
        }
        root.isModified = false
        root.isSaved = true
        root.showFeedback("info", "Reverted unsaved changes.")
        root.updatePreview()
    }

    function triggerResetDefault() {
        if (backend.resetFastfetchDefault()) {
            root.selectedPresetId = ""
            root.selectedPresetTitle = "Default"
            var saved = backend.getFastfetchSavedConfig()
            if (saved && saved.trim().length > 0) {
                root.baseConfigJsonc = saved
                root.parseLoadedConfig(saved)
            } else {
                root.moduleList = root.defaultModuleRows()
                root.emblemType = "none"
                root.selectedEmblemId = ""
                root.customEmblemPath = ""
                root.widthCols = 30
                root.heightLines = 14
                root.paddingCols = 3
                root.paddingTop = 2
                root.dither = false
                root.accentColor = "#6488E8"
            }
            root.colorModified = false
            root.separatorModified = false
            root.modulesModified = false
            root.isModified = false
            root.isSaved = true
            root.showFeedback("info", "Restored default Fastfetch configuration.")
            root.updatePreview()
        }
    }

    function triggerRemoveInstalledStyle() {
        if (backend.removeInstalledFastfetchStyle(root.selectedPresetId)) {
            root.selectedPresetId = ""
            root.selectedPresetTitle = "Default"
            var saved = backend.getFastfetchSavedConfig()
            if (saved && saved.trim().length > 0) {
                root.baseConfigJsonc = saved
                root.parseLoadedConfig(saved)
            }
            root.isModified = false
            root.isSaved = true
            root.showFeedback("info", "Removed active style and restored previous configuration.")
            root.updatePreview()
        }
    }

    function initStudio() {
        if (!backend.fastfetchInstalledPresets || backend.fastfetchInstalledPresets.length === 0) {
            backend.loadFastfetchInstalledPresets()
        }
        if (!backend.fastfetchLocalEmblems || backend.fastfetchLocalEmblems.length === 0) {
            backend.loadFastfetchLocalEmblems()
        }
        var saved = backend.fastfetchSavedConfig || backend.getFastfetchSavedConfig()
        if (saved && saved.trim().length > 0) {
            root.baseConfigJsonc = saved
            parseLoadedConfig(saved)
            if (root.activeFastfetchId) {
                root.selectedPresetId = root.activeFastfetchId
                var pInfo = root.getPresetItem(root.activeFastfetchId)
                if (pInfo.title) root.selectedPresetTitle = pInfo.title
            }
            root.isModified = false
            root.isSaved = true
            updatePreview()
        } else {
            root.moduleList = root.defaultModuleRows()
            root.emblemType = "image"
            root.selectedEmblemId = "katana"
            root.widthCols = 30
            root.heightLines = 14
            root.paddingCols = 3
            root.paddingTop = 2
            root.accentColor = "#6488E8"
            root.colorModified = true
            root.modulesModified = true
            root.isModified = false
            root.isSaved = true
            updatePreview()
        }
    }

    Component.onCompleted: {
        initStudio()
    }

    FileDialog {
        id: imageFileDialog
        title: "Select Emblem File"
        fileMode: FileDialog.OpenFile
        nameFilters: ["Images & ASCII (*.png *.jpg *.jpeg *.webp *.svg *.txt *.ascii)", "All Files (*)"]
        onAccepted: {
            var path = selectedFile.toString().replace(/^file:\/\//, "")
            root.customEmblemPath = path
            if (path.toLowerCase().endsWith(".txt") || path.toLowerCase().endsWith(".ascii")) {
                root.emblemType = "ascii"
            } else {
                root.emblemType = "image"
            }
            root.markModified()
        }
    }

    // ── Reusable Ryzora Action Button ──
    component StudioButton: Rectangle {
        id: btnRoot
        property string label: ""
        property string iconName: ""
        property bool primary: false
        property bool danger: false
        property bool enabledBtn: true
        signal clicked()

        height: 32
        implicitWidth: btnRow.implicitWidth + 20
        radius: theme.radiusSm
        opacity: enabledBtn ? 1.0 : 0.45
        color: {
            if (!enabledBtn) return theme.surface
            if (primary) return btnMouse.containsMouse ? theme.accentHover : theme.accent
            if (danger) return btnMouse.containsMouse ? Qt.rgba(239/255, 68/255, 68/255, 0.16) : theme.surfaceElevated
            return btnMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated
        }
        border.color: {
            if (primary) return theme.accent
            if (danger && btnMouse.containsMouse) return theme.error
            return btnMouse.containsMouse ? theme.borderStrong : theme.borderSubtle
        }
        border.width: 1

        RowLayout {
            id: btnRow
            anchors.centerIn: parent
            spacing: 6

            RyzoraIcon {
                visible: btnRoot.iconName !== ""
                name: btnRoot.iconName
                size: 13
                color: btnRoot.primary ? "#FFFFFF" : (btnRoot.danger ? theme.error : theme.text)
            }

            Text {
                text: btnRoot.label
                font.pixelSize: 12
                font.weight: btnRoot.primary ? Font.DemiBold : Font.Medium
                color: btnRoot.primary ? "#FFFFFF" : (btnRoot.danger ? theme.error : theme.text)
            }
        }

        MouseArea {
            id: btnMouse
            anchors.fill: parent
            enabled: btnRoot.enabledBtn
            hoverEnabled: true
            cursorShape: btnRoot.enabledBtn ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: btnRoot.clicked()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ─────────────────────────────────────────────────────────────────
        // 1. COMPACT RYZORA STUDIO HEADER (With Installed Styles Dropdown)
        // ─────────────────────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: headerCol.implicitHeight + 22
            color: theme.bg

            ColumnLayout {
                id: headerCol
                anchors.fill: parent
                anchors.leftMargin: 24
                anchors.rightMargin: 24
                anchors.topMargin: 12
                anchors.bottomMargin: 10
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    ColumnLayout {
                        spacing: 2
                        Text {
                            text: "Fastfetch Studio"
                            font.pixelSize: 19
                            font.bold: true
                            color: theme.text
                        }
                        Text {
                            text: "Customize your terminal system readout, emblem, modules, and layout."
                            font.pixelSize: 11
                            color: theme.textSecondary
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Inline Action Feedback
                    Text {
                        visible: root.actionFeedback !== ""
                        text: root.actionFeedback
                        font.pixelSize: 11
                        font.weight: Font.Medium
                        color: root.actionFeedbackType === "error"
                               ? theme.error
                               : (root.actionFeedbackType === "success" ? theme.success : theme.accent)
                        elide: Text.ElideRight
                        Layout.maximumWidth: 220
                    }

                    // ── Compact Searchable Installed Styles Dropdown ──
                    Rectangle {
                        id: styleDropdownTrigger
                        objectName: "installedStylesDropdownTrigger"
                        height: 34
                        implicitWidth: Math.max(220, Math.min(310, triggerRow.implicitWidth + 24))
                        radius: theme.radiusSm
                        color: styleDropdownPopup.opened
                               ? theme.surfaceHover
                               : (triggerMouse.containsMouse ? theme.surfaceHover : theme.surfaceElevated)
                        border.color: styleDropdownPopup.opened ? theme.accent : (triggerMouse.containsMouse ? theme.borderStrong : theme.borderSubtle)
                        border.width: 1

                        RowLayout {
                            id: triggerRow
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            spacing: 8

                            // Selected style emblem thumbnail or terminal icon
                            Rectangle {
                                width: 20; height: 20; radius: 4
                                color: Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.12)
                                clip: true

                                Image {
                                    id: triggerThumb
                                    anchors.fill: parent
                                    anchors.margins: 2
                                    source: root.resolvePresetThumbnail(root.selectedPresetId || "default")
                                    sourceSize.width: 40
                                    sourceSize.height: 40
                                    fillMode: Image.PreserveAspectFit
                                    visible: status === Image.Ready && String(source).length > 0
                                    smooth: true
                                    asynchronous: true
                                }

                                RyzoraIcon {
                                    anchors.centerIn: parent
                                    visible: !triggerThumb.visible
                                    name: "terminal"
                                    size: 11
                                    color: theme.accent
                                }
                            }

                            Text {
                                text: root.selectedPresetTitle || "Default Fastfetch"
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                                color: theme.text
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            // Active / Draft / Installed Status Badge
                            Rectangle {
                                height: 18
                                radius: 9
                                implicitWidth: stateBadgeText.implicitWidth + 12
                                property bool isActiveNow: root.isPresetActive(root.selectedPresetId || "default")
                                color: root.isModified
                                       ? Qt.rgba(245/255, 158/255, 11/255, 0.16)
                                       : (isActiveNow
                                          ? Qt.rgba(16/255, 185/255, 129/255, 0.16)
                                          : Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.14))
                                border.color: root.isModified ? theme.warning : (isActiveNow ? theme.success : theme.accent)
                                border.width: 1

                                Text {
                                    id: stateBadgeText
                                    anchors.centerIn: parent
                                    text: root.isModified
                                          ? "Draft"
                                          : (parent.isActiveNow ? "Active" : "Installed")
                                    font.pixelSize: 9
                                    font.bold: true
                                    color: root.isModified ? theme.warning : (parent.isActiveNow ? theme.success : theme.accent)
                                }
                            }

                            RyzoraIcon {
                                name: styleDropdownPopup.opened ? "chevron-up" : "chevron-down"
                                size: 12
                                color: theme.textSecondary
                            }
                        }

                        MouseArea {
                            id: triggerMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (styleDropdownPopup.opened) {
                                    styleDropdownPopup.close()
                                } else {
                                    backend.loadFastfetchInstalledPresets()
                                    root.styleDropdownSearch = ""
                                    styleDropdownPopup.open()
                                    styleSearchField.forceActiveFocus()
                                }
                            }
                        }

                        Popup {
                            id: styleDropdownPopup
                            objectName: "installedStylesDropdownPopup"
                            x: 0
                            y: styleDropdownTrigger.height + 6
                            width: 310
                            height: Math.min(340, popupCol.implicitHeight + 20)
                            padding: 10
                            modal: false
                            focus: true
                            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

                            background: Rectangle {
                                radius: theme.radiusMd
                                color: theme.surfaceElevated
                                border.color: theme.borderStrong
                                border.width: 1
                            }

                            ColumnLayout {
                                id: popupCol
                                width: parent.width
                                spacing: 8

                                // Search box inside dropdown
                                TextField {
                                    id: styleSearchField
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 30
                                    placeholderText: "Filter installed styles…"
                                    placeholderTextColor: theme.textMuted
                                    font.pixelSize: 11
                                    color: theme.text
                                    text: root.styleDropdownSearch
                                    leftPadding: 10
                                    rightPadding: 10
                                    onTextEdited: root.styleDropdownSearch = text
                                    background: Rectangle {
                                        radius: theme.radiusSm
                                        color: theme.bg
                                        border.color: styleSearchField.activeFocus ? theme.accent : theme.borderSubtle
                                        border.width: 1
                                    }
                                }

                                ListView {
                                    id: dropdownStylesList
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: Math.min(240, Math.max(44, contentHeight))
                                    clip: true
                                    spacing: 4
                                    boundsBehavior: Flickable.StopAtBounds
                                    model: root.filteredInstalledStyles

                                    delegate: Rectangle {
                                        id: optRow
                                        width: dropdownStylesList.width
                                        height: 40
                                        radius: theme.radiusSm
                                        property bool isSel: (root.selectedPresetId === modelData.id) || (!root.selectedPresetId && modelData.id === "default")
                                        property bool isAct: Boolean(modelData.isActive)

                                        color: isSel
                                               ? Qt.rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.16)
                                               : (optMouse.containsMouse ? theme.surfaceHover : "transparent")
                                        border.color: isSel ? theme.accent : "transparent"
                                        border.width: 1

                                        RowLayout {
                                            anchors.fill: parent
                                            anchors.leftMargin: 8
                                            anchors.rightMargin: 8
                                            spacing: 8

                                            Rectangle {
                                                width: 24; height: 24; radius: 4
                                                color: theme.bg
                                                border.color: theme.borderSubtle
                                                border.width: 1
                                                clip: true

                                                Image {
                                                    id: optThumb
                                                    anchors.fill: parent
                                                    anchors.margins: 2
                                                    source: modelData.thumbnail || ""
                                                    sourceSize.width: 48
                                                    sourceSize.height: 48
                                                    fillMode: Image.PreserveAspectFit
                                                    visible: status === Image.Ready && String(source).length > 0
                                                    smooth: true
                                                    asynchronous: true
                                                }

                                                RyzoraIcon {
                                                    anchors.centerIn: parent
                                                    visible: !optThumb.visible
                                                    name: "terminal"
                                                    size: 12
                                                    color: optRow.isSel ? theme.accent : theme.textSecondary
                                                }
                                            }

                                            ColumnLayout {
                                                Layout.fillWidth: true
                                                spacing: 1
                                                Text {
                                                    text: modelData.title
                                                    font.pixelSize: 12
                                                    font.weight: optRow.isSel ? Font.DemiBold : Font.Medium
                                                    color: theme.text
                                                    elide: Text.ElideRight
                                                    Layout.fillWidth: true
                                                }
                                                Text {
                                                    visible: Boolean(modelData.author && modelData.author.length > 0)
                                                    text: modelData.author || ""
                                                    font.pixelSize: 10
                                                    color: theme.textMuted
                                                    elide: Text.ElideRight
                                                    Layout.fillWidth: true
                                                }
                                            }

                                            Rectangle {
                                                visible: optRow.isAct
                                                height: 16
                                                radius: 8
                                                implicitWidth: actOptText.implicitWidth + 10
                                                color: Qt.rgba(16/255, 185/255, 129/255, 0.16)
                                                border.color: theme.success
                                                border.width: 1

                                                Text {
                                                    id: actOptText
                                                    anchors.centerIn: parent
                                                    text: "Active"
                                                    font.pixelSize: 9
                                                    font.bold: true
                                                    color: theme.success
                                                }
                                            }

                                            RyzoraIcon {
                                                visible: optRow.isSel
                                                name: "check"
                                                size: 13
                                                color: theme.accent
                                            }
                                        }

                                        MouseArea {
                                            id: optMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                root.loadPreset(modelData.id)
                                                styleDropdownPopup.close()
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    StudioButton {
                        iconName: "compass"
                        label: "Browse Packages"
                        onClicked: {
                            if (typeof discoverView !== "undefined" && discoverView) {
                                discoverView.activeCategory = "fastfetch"
                                discoverView.activeFastfetchSubfilter = "All"
                                discoverView.updateFilter()
                            }
                            if (typeof window !== "undefined" && typeof window.navigateTo === "function") {
                                window.navigateTo("discover")
                            }
                        }
                    }

                    StudioButton {
                        iconName: "file-text"
                        label: "Edit Config"
                        onClicked: {
                            rawConfigEditor.text = root.buildDraftJson()
                            root.showRawConfigModal = true
                        }
                    }

                    StudioButton {
                        iconName: "rotate-ccw"
                        label: "Reset to Default"
                        onClicked: root.triggerResetDefault()
                    }
                }
            }
        }

        // ─────────────────────────────────────────────────────────────────
        // 2. TWO-COLUMN WORKSPACE (Left: Terminal 58%, Right: Inspector 42%)
        // ─────────────────────────────────────────────────────────────────
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 24
            Layout.rightMargin: 24
            Layout.topMargin: 4
            Layout.bottomMargin: 12
            spacing: 14

            // ═════════════════════════════════════════════════════════════
            // LEFT COLUMN: MINIMAL TERMINAL PREVIEW (~60% width)
            // ═════════════════════════════════════════════════════════════
            Rectangle {
                Layout.fillHeight: true
                Layout.fillWidth: true
                Layout.preferredWidth: 60
                Layout.minimumWidth: 460
                radius: theme.radiusLg
                color: "#090B0F"
                border.color: theme.borderSubtle
                border.width: 1
                clip: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 0

                    // Minimal Terminal Header Bar (No fake macOS window dots)
                    Rectangle {
                        Layout.fillWidth: true
                        height: 36
                        color: "#0E1117"

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 14
                            anchors.rightMargin: 12
                            spacing: 8

                            RyzoraIcon {
                                name: "terminal"
                                size: 13
                                color: theme.accent
                            }

                            Text {
                                text: "Fastfetch Preview"
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                                color: "#E2E8F0"
                            }

                            Text {
                                text: "· " + (root.selectedPresetTitle || "Default Fastfetch")
                                font.pixelSize: 11
                                font.family: "JetBrains Mono, monospace"
                                color: "#94A3B8"
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            // Honest Live Status Indicator
                            RowLayout {
                                spacing: 6
                                property bool hasValidOutput: Boolean(backend.fastfetchHtml && backend.fastfetchHtml.trim().length > 0 && (!backend.fastfetchError || backend.fastfetchError.length === 0))
                                Rectangle {
                                    width: 7; height: 7; radius: 3.5
                                    color: backend.isFastfetchLoading
                                           ? "#F59E0B"
                                           : ((backend.fastfetchError && backend.fastfetchError.length > 0)
                                              ? "#EF4444"
                                              : (parent.hasValidOutput ? "#10B981" : "#64748B"))
                                }
                                Text {
                                    text: backend.isFastfetchLoading
                                          ? "Updating preview…"
                                          : ((backend.fastfetchError && backend.fastfetchError.length > 0)
                                             ? "Preview failed"
                                             : (parent.hasValidOutput ? "Live Output" : "Waiting for output…"))
                                    font.pixelSize: 11
                                    font.family: "JetBrains Mono, monospace"
                                    color: "#94A3B8"
                                }
                            }

                            // Preview in Terminal Button
                            Rectangle {
                                height: 26
                                radius: theme.radiusSm
                                implicitWidth: termLaunchRow.implicitWidth + 16
                                color: termLaunchMouse.containsMouse ? "#252B39" : "#171B24"
                                border.color: termLaunchMouse.containsMouse ? theme.accent : "#262C3A"
                                border.width: 1

                                RowLayout {
                                    id: termLaunchRow
                                    anchors.centerIn: parent
                                    spacing: 6
                                    RyzoraIcon { name: "terminal"; size: 11; color: theme.accent }
                                    Text {
                                        text: "Preview in Terminal"
                                        font.pixelSize: 11
                                        font.weight: Font.DemiBold
                                        color: "#F1F5F9"
                                    }
                                }

                                MouseArea {
                                    id: termLaunchMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        var draft = root.buildDraftJson()
                                        backend.launchTerminalWithDraft(draft)
                                    }
                                }
                            }
                        }

                        Rectangle {
                            anchors.bottom: parent.bottom
                            anchors.left: parent.left
                            anchors.right: parent.right
                            height: 1
                            color: "#1A202C"
                        }
                    }

                    // Error Banner inside Terminal (if Fastfetch subprocess reported an error)
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.margins: 10
                        height: 30
                        radius: theme.radiusSm
                        visible: Boolean(backend.fastfetchError && backend.fastfetchError.length > 0)
                        color: Qt.rgba(239/255, 68/255, 68/255, 0.16)
                        border.color: theme.error
                        border.width: 1

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            spacing: 8
                            RyzoraIcon { name: "alert"; size: 13; color: "#F87171" }
                            Text {
                                text: backend.fastfetchError
                                font.pixelSize: 11
                                font.family: "JetBrains Mono, monospace"
                                color: "#FCA5A5"
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                        }
                    }

                    // Main Non-Scrolling Auto-Fitted Terminal Surface
                    Item {
                        id: termViewport
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.margins: 12
                        clip: true

                        Row {
                            id: termContentRow
                            spacing: (root.emblemType === "image" && root.resolvedEmblemUrl !== "")
                                     ? Math.max(12, Math.min(28, root.paddingCols * 6.0))
                                     : 0
                            padding: 4

                            readonly property real rawContentW: (emblemContainer.visible ? (emblemContainer.width + spacing) : 0) + Math.max(200, termOut.implicitWidth) + 16
                            readonly property real rawContentH: Math.max(emblemContainer.visible ? (emblemContainer.height + emblemContainer.y) : 0, Math.max(120, termOut.implicitHeight)) + 24
                            readonly property real scaleX: (rawContentW > 0 && termViewport.width > 24) ? ((termViewport.width - 24) / rawContentW) : 1.0
                            readonly property real scaleY: (rawContentH > 0 && termViewport.height > 36) ? ((termViewport.height - 36) / rawContentH) : 1.0
                            readonly property real autoScale: Math.max(0.55, Math.min(1.0, Math.min(scaleX, scaleY)))

                            transformOrigin: Item.TopLeft
                            scale: autoScale
                            x: 12
                            y: 16

                            // Left Emblem Image (Sized to user widthCols/heightLines and auto-fitted with readout on one screen)
                            Item {
                                id: emblemContainer
                                visible: root.emblemType === "image" && root.resolvedEmblemUrl !== "" && previewEmblemImage.status !== Image.Error

                                width: visible ? Math.max(120, Math.min(360, root.widthCols * 7.6)) : 0
                                height: visible ? Math.max(140, Math.min(400, Math.max(root.heightLines * 18.5, Math.min(340, termOut.implicitHeight * 0.9)))) : 0
                                y: Math.max(0, root.paddingTop * 18.0)

                                Image {
                                    id: previewEmblemImage
                                    objectName: "fastfetchPreviewEmblemImage"
                                    anchors.fill: parent
                                    source: root.resolvedEmblemUrl
                                    fillMode: Image.PreserveAspectFit
                                    smooth: true
                                    mipmap: false
                                    // Decode off the UI thread at display resolution (source PNGs reach 2526x4096).
                                    asynchronous: true
                                    cache: true
                                    sourceSize.height: 800
                                    opacity: root.dither ? 0.86 : 1.0
                                }
                            }

                            // Right Readout Text (Live Fastfetch Output)
                            TextEdit {
                                id: termOut
                                objectName: "fastfetchTerminalReadout"
                                readOnly: true
                                selectByMouse: true
                                textFormat: TextEdit.RichText
                                text: {
                                    if (backend.isFastfetchLoading && (!backend.fastfetchHtml || backend.fastfetchHtml.length === 0)) {
                                        return "<pre style='color:#64748B; font-size: 13px;'>Rendering Fastfetch preview…</pre>"
                                    }
                                    if (backend.fastfetchHtml && backend.fastfetchHtml.trim().length > 0) {
                                        return backend.fastfetchHtml
                                    }
                                    return "<pre style='color:#64748B; font-size: 13px;'>Running live Fastfetch preview…</pre>"
                                }
                                font.family: "JetBrainsMono Nerd Font, JetBrains Mono, monospace"
                                font.pixelSize: 13
                                color: "#F1F5F9"
                                wrapMode: TextEdit.NoWrap
                            }
                        }
                    }
                }
            }

            // ═════════════════════════════════════════════════════════════
            // RIGHT COLUMN: COMPACT CONFIGURATION INSPECTOR (~40% width)
            // ═════════════════════════════════════════════════════════════
            Rectangle {
                Layout.fillHeight: true
                Layout.fillWidth: true
                Layout.preferredWidth: 40
                Layout.minimumWidth: 360
                Layout.maximumWidth: 540
                radius: theme.radiusLg
                color: theme.surface
                border.color: theme.borderSubtle
                border.width: 1
                clip: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 0

                    // Inspector Header + Segmented Section Picker
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.margins: 12
                        spacing: 10

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 6

                            RyzoraIcon {
                                name: "sliders"
                                size: 14
                                color: theme.accent
                            }

                            Text {
                                text: "Configuration"
                                font.pixelSize: 13
                                font.bold: true
                                color: theme.text
                            }

                            Item { Layout.fillWidth: true }
                        }

                        // Compact Segmented Tabs: Emblem | Layout | Colors | Modules
                        Rectangle {
                            Layout.fillWidth: true
                            height: 32
                            radius: theme.radiusSm
                            color: theme.bg
                            border.color: theme.borderSubtle
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 3
                                spacing: 2

                                Repeater {
                                    model: [
                                        { id: "appearance", label: "Emblem", icon: "image" },
                                        { id: "layout", label: "Layout", icon: "sliders" },
                                        { id: "colors", label: "Colors", icon: "palette" },
                                        { id: "modules", label: "Modules", icon: "info" }
                                    ]

                                    Rectangle {
                                        property bool isTabSel: root.inspectorSection === modelData.id
                                        Layout.fillWidth: true
                                        Layout.fillHeight: true
                                        radius: 4
                                        color: isTabSel ? theme.surfaceElevated : (tabM.containsMouse ? theme.surfaceHover : "transparent")
                                        border.color: isTabSel ? theme.accent : "transparent"
                                        border.width: 1

                                        RowLayout {
                                            anchors.centerIn: parent
                                            spacing: 4
                                            RyzoraIcon {
                                                name: modelData.icon
                                                size: 11
                                                color: parent.parent.isTabSel ? theme.accent : theme.textSecondary
                                            }
                                            Text {
                                                text: modelData.label
                                                font.pixelSize: 11
                                                font.weight: parent.parent.isTabSel ? Font.DemiBold : Font.Medium
                                                color: parent.parent.isTabSel ? theme.accent : theme.textSecondary
                                            }
                                        }

                                        MouseArea {
                                            id: tabM
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: root.inspectorSection = modelData.id
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: theme.borderSubtle
                    }

                    // Scrollable Inspector Body
                    ScrollView {
                        id: inspectorScroll
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        ScrollBar.vertical.policy: ScrollBar.AsNeeded
                        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                        contentWidth: availableWidth

                        ColumnLayout {
                            width: Math.max(240, inspectorScroll.availableWidth - 24)
                            x: 12
                            spacing: 14

                            Item { width: 1; height: 4 }

                            // ─────────────────────────────────────────────
                            // TAB 1: EMBLEM (Mode + Compact Picker + Custom File)
                            // ─────────────────────────────────────────────
                            ColumnLayout {
                                visible: root.inspectorSection === "appearance"
                                Layout.fillWidth: true
                                spacing: 12

                                Text {
                                    text: "Emblem Mode"
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    color: theme.text
                                }

                                // Mode Pills: Image | ASCII | Built-in | None
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Repeater {
                                        model: [
                                            { id: "image", label: "Image" },
                                            { id: "ascii", label: "ASCII" },
                                            { id: "builtin", label: "Built-in" },
                                            { id: "none", label: "None" }
                                        ]

                                        Rectangle {
                                            property bool isSel: root.emblemType === modelData.id
                                            Layout.fillWidth: true
                                            height: 28
                                            radius: theme.radiusSm
                                            color: isSel ? theme.accentFaint : (modeM.containsMouse ? theme.surfaceHover : theme.bg)
                                            border.color: isSel ? theme.accent : theme.borderSubtle
                                            border.width: 1

                                            Text {
                                                anchors.centerIn: parent
                                                text: modelData.label
                                                font.pixelSize: 11
                                                font.weight: parent.isSel ? Font.DemiBold : Font.Medium
                                                color: parent.isSel ? theme.accent : theme.textSecondary
                                            }

                                            MouseArea {
                                                id: modeM
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: {
                                                    root.emblemType = modelData.id
                                                    if (modelData.id === "image" && root.selectedEmblemId === "" && root.customEmblemPath === "") {
                                                        root.selectedEmblemId = "katana"
                                                    }
                                                    root.logoModified = true
                                                    root.markModified()
                                                }
                                            }
                                        }
                                    }
                                }

                                // Custom Emblem / ASCII Path Selector
                                ColumnLayout {
                                    visible: root.emblemType === "image" || root.emblemType === "ascii"
                                    Layout.fillWidth: true
                                    spacing: 6

                                    RowLayout {
                                        Layout.fillWidth: true
                                        Text {
                                            text: root.emblemType === "ascii" ? "ASCII Source File" : "Emblem Source"
                                            font.pixelSize: 11
                                            font.weight: Font.Medium
                                            color: theme.textSecondary
                                        }
                                        Item { Layout.fillWidth: true }
                                        StudioButton {
                                            iconName: "upload"
                                            label: "Choose File"
                                            height: 26
                                            onClicked: imageFileDialog.open()
                                        }
                                    }

                                    TextField {
                                        id: pathInput
                                        Layout.fillWidth: true
                                        Layout.preferredHeight: 30
                                        font.pixelSize: 11
                                        font.family: "JetBrains Mono, monospace"
                                        text: root.customEmblemPath !== "" ? root.customEmblemPath : root.resolvedEmblemPath
                                        placeholderText: "Select or enter file path…"
                                        placeholderTextColor: theme.textMuted
                                        color: theme.textSecondary
                                        leftPadding: 8
                                        rightPadding: 8
                                        background: Rectangle {
                                            radius: theme.radiusSm
                                            color: theme.bg
                                            border.color: pathInput.activeFocus ? theme.accent : theme.borderSubtle
                                            border.width: 1
                                        }
                                        onEditingFinished: {
                                            var trimmed = text.trim()
                                            if (trimmed.length > 0 && trimmed !== root.resolvedEmblemPath) {
                                                root.customEmblemPath = trimmed
                                                root.markModified()
                                            }
                                        }
                                    }
                                }

                                // Active Emblem Compact Preview Card
                                Rectangle {
                                    visible: root.emblemType === "image" && root.resolvedEmblemUrl !== ""
                                    Layout.fillWidth: true
                                    height: 94
                                    radius: theme.radiusSm
                                    color: theme.bg
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.margins: 8
                                        spacing: 12

                                        // Emblem preview thumbnail
                                        Rectangle {
                                            width: 76; height: 76
                                            radius: theme.radiusSm
                                            color: "#07090D"
                                            border.color: theme.borderStrong
                                            border.width: 1
                                            clip: true

                                            Image {
                                                anchors.fill: parent
                                                anchors.margins: 4
                                                source: root.resolvedEmblemUrl
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                mipmap: true
                                            }
                                        }

                                        // Details & live resolution readout
                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            Layout.fillHeight: true
                                            spacing: 2

                                            RowLayout {
                                                Layout.fillWidth: true
                                                spacing: 6
                                                Text {
                                                    text: "Active Emblem"
                                                    font.pixelSize: 12
                                                    font.weight: Font.DemiBold
                                                    color: theme.text
                                                }
                                                Rectangle {
                                                    width: 6; height: 6; radius: 3
                                                    color: theme.accent
                                                }
                                                Item { Layout.fillWidth: true }
                                                Text {
                                                    text: root.widthCols + " × " + root.heightLines
                                                    font.pixelSize: 10
                                                    font.family: "JetBrains Mono, monospace"
                                                    color: theme.accent
                                                }
                                            }

                                            Text {
                                                text: {
                                                    if (root.customEmblemPath) {
                                                        var parts = root.customEmblemPath.split("/")
                                                        return parts[parts.length - 1] || "Custom Emblem"
                                                    }
                                                    return root.selectedEmblemId || "katana"
                                                }
                                                font.pixelSize: 11
                                                font.family: "JetBrains Mono, monospace"
                                                color: theme.textSecondary
                                                elide: Text.ElideMiddle
                                                Layout.fillWidth: true
                                            }

                                            Item { Layout.fillHeight: true }

                                            RowLayout {
                                                spacing: 6
                                                StudioButton {
                                                    label: "Small"
                                                    height: 22
                                                    onClicked: {
                                                        root.widthCols = 24
                                                        root.heightLines = 12
                                                        root.logoModified = true
                                                        root.markModified()
                                                    }
                                                }
                                                StudioButton {
                                                    label: "Medium"
                                                    height: 22
                                                    onClicked: {
                                                        root.widthCols = 34
                                                        root.heightLines = 17
                                                        root.logoModified = true
                                                        root.markModified()
                                                    }
                                                }
                                                StudioButton {
                                                    label: "Large"
                                                    height: 22
                                                    onClicked: {
                                                        root.widthCols = 44
                                                        root.heightLines = 22
                                                        root.logoModified = true
                                                        root.markModified()
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Compact Emblem Picker Grid (Always visible in Emblem tab; clicking any tile switches to Image mode)
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 8

                                    RowLayout {
                                        Layout.fillWidth: true
                                        Text {
                                            text: "Installed Emblems"
                                            font.pixelSize: 12
                                            font.weight: Font.DemiBold
                                            color: theme.text
                                        }
                                        Item { Layout.fillWidth: true }
                                        Text {
                                            text: (backend.fastfetchLocalEmblems ? backend.fastfetchLocalEmblems.length : 0) + " available"
                                            font.pixelSize: 11
                                            color: theme.textMuted
                                        }
                                    }

                                    Grid {
                                        id: emblemGrid
                                        Layout.fillWidth: true
                                        width: parent ? parent.width : 340
                                        readonly property real targetTileWidth: 104
                                        readonly property real gridW: width > 0 ? width : 340
                                        columns: Math.max(2, Math.floor((gridW + spacing) / (targetTileWidth + spacing)))
                                        spacing: 8
                                        readonly property real calculatedTileWidth: Math.max(90, Math.floor((gridW - (columns - 1) * spacing) / Math.max(1, columns)))

                                        // Custom Upload Tile
                                        Rectangle {
                                            width: emblemGrid.calculatedTileWidth
                                            height: 106
                                            radius: theme.radiusSm
                                            property bool isCustomSel: root.emblemType === "image" && root.customEmblemPath.length > 0 && !root.isPresetLocalEmblem(root.customEmblemPath)
                                            color: isCustomSel ? theme.accentFaint : (upTileM.containsMouse ? theme.surfaceHover : theme.bg)
                                            border.color: isCustomSel ? theme.accent : (upTileM.containsMouse ? theme.borderStrong : theme.borderSubtle)
                                            border.width: isCustomSel ? 2 : 1

                                            Item {
                                                id: upIconBox
                                                width: parent.width - 12
                                                height: 64
                                                anchors.top: parent.top
                                                anchors.topMargin: 8
                                                anchors.horizontalCenter: parent.horizontalCenter

                                                RyzoraIcon {
                                                    anchors.centerIn: parent
                                                    name: "upload"
                                                    size: 24
                                                    color: parent.parent.isCustomSel ? theme.accent : theme.textSecondary
                                                }
                                            }

                                            Text {
                                                anchors.top: upIconBox.bottom
                                                anchors.topMargin: 4
                                                anchors.horizontalCenter: parent.horizontalCenter
                                                width: parent.width - 8
                                                text: "Custom"
                                                font.pixelSize: 11
                                                font.weight: parent.isCustomSel ? Font.DemiBold : Font.Normal
                                                color: parent.isCustomSel ? theme.accent : theme.text
                                                elide: Text.ElideRight
                                                horizontalAlignment: Text.AlignHCenter
                                            }

                                            MouseArea {
                                                id: upTileM
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: imageFileDialog.open()
                                            }
                                        }

                                        // Discovered Local Emblems
                                        Repeater {
                                            model: backend.fastfetchLocalEmblems

                                            Rectangle {
                                                width: emblemGrid.calculatedTileWidth
                                                height: 106
                                                radius: theme.radiusSm
                                                property bool isSel: root.emblemType === "image" && (
                                                    (root.selectedEmblemId.length > 0 && root.selectedEmblemId.toLowerCase() === String(modelData.id || "").toLowerCase()) ||
                                                    (root.customEmblemPath.length > 0 && (root.customEmblemPath === String(modelData.path || "") || root.customEmblemPath === ("file://" + String(modelData.path || ""))))
                                                )
                                                color: isSel ? theme.accentFaint : (embM.containsMouse ? theme.surfaceHover : theme.bg)
                                                border.color: isSel ? theme.accent : (embM.containsMouse ? theme.borderStrong : theme.borderSubtle)
                                                border.width: isSel ? 2 : 1

                                                Item {
                                                    id: embImgBox
                                                    width: parent.width - 12
                                                    height: 64
                                                    anchors.top: parent.top
                                                    anchors.topMargin: 8
                                                    anchors.horizontalCenter: parent.horizontalCenter

                                                    Image {
                                                        anchors.fill: parent
                                                        source: modelData.path.indexOf("file://") === 0 ? modelData.path : ("file://" + modelData.path)
                                                        sourceSize.width: 128
                                                        sourceSize.height: 128
                                                        fillMode: Image.PreserveAspectFit
                                                        smooth: true
                                                        mipmap: true
                                                        asynchronous: true
                                                    }
                                                }

                                                Text {
                                                    anchors.top: embImgBox.bottom
                                                    anchors.topMargin: 4
                                                    anchors.horizontalCenter: parent.horizontalCenter
                                                    width: parent.width - 8
                                                    text: String(modelData.name || modelData.id || "")
                                                    font.pixelSize: 11
                                                    font.weight: parent.isSel ? Font.DemiBold : Font.Normal
                                                    color: parent.isSel ? theme.accent : theme.text
                                                    elide: Text.ElideRight
                                                    horizontalAlignment: Text.AlignHCenter
                                                }

                                                MouseArea {
                                                    id: embM
                                                    anchors.fill: parent
                                                    hoverEnabled: true
                                                    cursorShape: Qt.PointingHandCursor
                                                    onClicked: {
                                                        root.selectedEmblemId = String(modelData.id || "")
                                                        root.customEmblemPath = String(modelData.path || "")
                                                        root.emblemType = "image"
                                                        root.logoModified = true
                                                        root.markModified()
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // ─────────────────────────────────────────────
                            // TAB 2: LAYOUT (Width, Height, Padding, Separator, Dither)
                            // ─────────────────────────────────────────────
                            ColumnLayout {
                                visible: root.inspectorSection === "layout"
                                Layout.fillWidth: true
                                spacing: 10

                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        text: "Emblem Geometry"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        color: theme.text
                                    }
                                    Item { Layout.fillWidth: true }
                                    StudioButton {
                                        label: "Auto Fit"
                                        height: 26
                                        onClicked: {
                                            root.widthCols = 36
                                            root.heightLines = 18
                                            root.paddingCols = 3
                                            root.paddingTop = 2
                                            root.logoModified = true
                                            root.markModified()
                                        }
                                    }
                                }

                                // Width Stepper Card
                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 44
                                    radius: theme.radiusSm
                                    color: theme.bg
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 10
                                        anchors.rightMargin: 10
                                        spacing: 8

                                        ColumnLayout {
                                            spacing: 0
                                            Text { text: "Width"; font.pixelSize: 12; font.weight: Font.Medium; color: theme.text }
                                            Text { text: "Terminal columns"; font.pixelSize: 10; color: theme.textMuted }
                                        }
                                        Item { Layout.fillWidth: true }
                                        Rectangle {
                                            width: 26; height: 26; radius: 5
                                            color: wDecM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            Text { anchors.centerIn: parent; text: "−"; font.pixelSize: 13; color: theme.text }
                                            MouseArea {
                                                id: wDecM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                onClicked: { if (root.widthCols > 10) { root.widthCols -= 2; root.logoModified = true; root.markModified(); } }
                                            }
                                        }
                                        Text {
                                            text: root.widthCols + " ch"
                                            font.pixelSize: 12
                                            font.family: "JetBrains Mono, monospace"
                                            font.bold: true
                                            color: theme.text
                                            Layout.preferredWidth: 42
                                            horizontalAlignment: Text.AlignHCenter
                                        }
                                        Rectangle {
                                            width: 26; height: 26; radius: 5
                                            color: wIncM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            Text { anchors.centerIn: parent; text: "+"; font.pixelSize: 13; color: theme.text }
                                            MouseArea {
                                                id: wIncM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                onClicked: { if (root.widthCols < 80) { root.widthCols += 2; root.logoModified = true; root.markModified(); } }
                                            }
                                        }
                                    }
                                }

                                // Height Stepper Card
                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 44
                                    radius: theme.radiusSm
                                    color: theme.bg
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 10
                                        anchors.rightMargin: 10
                                        spacing: 8

                                        ColumnLayout {
                                            spacing: 0
                                            Text { text: "Height"; font.pixelSize: 12; font.weight: Font.Medium; color: theme.text }
                                            Text { text: "Terminal rows"; font.pixelSize: 10; color: theme.textMuted }
                                        }
                                        Item { Layout.fillWidth: true }
                                        Rectangle {
                                            width: 26; height: 26; radius: 5
                                            color: hDecM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            Text { anchors.centerIn: parent; text: "−"; font.pixelSize: 13; color: theme.text }
                                            MouseArea {
                                                id: hDecM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                onClicked: { if (root.heightLines > 6) { root.heightLines -= 1; root.logoModified = true; root.markModified(); } }
                                            }
                                        }
                                        Text {
                                            text: root.heightLines + " ln"
                                            font.pixelSize: 12
                                            font.family: "JetBrains Mono, monospace"
                                            font.bold: true
                                            color: theme.text
                                            Layout.preferredWidth: 42
                                            horizontalAlignment: Text.AlignHCenter
                                        }
                                        Rectangle {
                                            width: 26; height: 26; radius: 5
                                            color: hIncM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            Text { anchors.centerIn: parent; text: "+"; font.pixelSize: 13; color: theme.text }
                                            MouseArea {
                                                id: hIncM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                onClicked: { if (root.heightLines < 50) { root.heightLines += 1; root.logoModified = true; root.markModified(); } }
                                            }
                                        }
                                    }
                                }

                                // Padding Stepper Card
                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 44
                                    radius: theme.radiusSm
                                    color: theme.bg
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 10
                                        anchors.rightMargin: 10
                                        spacing: 8

                                        ColumnLayout {
                                            spacing: 0
                                            Text { text: "Right Padding"; font.pixelSize: 12; font.weight: Font.Medium; color: theme.text }
                                            Text { text: "Gap before readout"; font.pixelSize: 10; color: theme.textMuted }
                                        }
                                        Item { Layout.fillWidth: true }
                                        Rectangle {
                                            width: 26; height: 26; radius: 5
                                            color: pDecM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            Text { anchors.centerIn: parent; text: "−"; font.pixelSize: 13; color: theme.text }
                                            MouseArea {
                                                id: pDecM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                onClicked: { if (root.paddingCols > 0) { root.paddingCols -= 1; root.logoModified = true; root.markModified(); } }
                                            }
                                        }
                                        Text {
                                            text: root.paddingCols + " ch"
                                            font.pixelSize: 12
                                            font.family: "JetBrains Mono, monospace"
                                            font.bold: true
                                            color: theme.text
                                            Layout.preferredWidth: 42
                                            horizontalAlignment: Text.AlignHCenter
                                        }
                                        Rectangle {
                                            width: 26; height: 26; radius: 5
                                            color: pIncM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            Text { anchors.centerIn: parent; text: "+"; font.pixelSize: 13; color: theme.text }
                                            MouseArea {
                                                id: pIncM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                onClicked: { if (root.paddingCols < 20) { root.paddingCols += 1; root.logoModified = true; root.markModified(); } }
                                            }
                                        }
                                    }
                                }

                                // Top Padding Stepper Card
                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 44
                                    radius: theme.radiusSm
                                    color: theme.bg
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 10
                                        anchors.rightMargin: 10
                                        spacing: 8

                                        ColumnLayout {
                                            spacing: 0
                                            Text { text: "Top Padding"; font.pixelSize: 12; font.weight: Font.Medium; color: theme.text }
                                            Text { text: "Logo breathing room"; font.pixelSize: 10; color: theme.textMuted }
                                        }
                                        Item { Layout.fillWidth: true }
                                        Rectangle {
                                            width: 26; height: 26; radius: 5
                                            color: ptDecM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            Text { anchors.centerIn: parent; text: "−"; font.pixelSize: 13; color: theme.text }
                                            MouseArea {
                                                id: ptDecM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                onClicked: { if (root.paddingTop > 0) { root.paddingTop -= 1; root.logoModified = true; root.markModified(); } }
                                            }
                                        }
                                        Text {
                                            text: root.paddingTop + " lines"
                                            font.pixelSize: 12
                                            font.family: "JetBrains Mono, monospace"
                                            font.bold: true
                                            color: theme.text
                                            Layout.preferredWidth: 48
                                            horizontalAlignment: Text.AlignHCenter
                                        }
                                        Rectangle {
                                            width: 26; height: 26; radius: 5
                                            color: ptIncM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                            border.color: theme.borderSubtle
                                            Text { anchors.centerIn: parent; text: "+"; font.pixelSize: 13; color: theme.text }
                                            MouseArea {
                                                id: ptIncM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                onClicked: { if (root.paddingTop < 10) { root.paddingTop += 1; root.logoModified = true; root.markModified(); } }
                                            }
                                        }
                                    }
                                }

                                // Display Separator Picker
                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 44
                                    radius: theme.radiusSm
                                    color: theme.bg
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 10
                                        anchors.rightMargin: 10
                                        spacing: 8

                                        ColumnLayout {
                                            spacing: 0
                                            Text { text: "Key Separator"; font.pixelSize: 12; font.weight: Font.Medium; color: theme.text }
                                            Text { text: "Between label and value"; font.pixelSize: 10; color: theme.textMuted }
                                        }
                                        Item { Layout.fillWidth: true }

                                        Repeater {
                                            model: [
                                                { val: "  ", label: "Space" },
                                                { val: ": ", label: ":" },
                                                { val: " ➜ ", label: "➜" },
                                                { val: " │ ", label: "│" }
                                            ]
                                            Rectangle {
                                                property bool isSep: root.displaySeparator === modelData.val
                                                height: 24
                                                width: sepLbl.implicitWidth + 14
                                                radius: 4
                                                color: isSep ? theme.accentFaint : theme.surfaceElevated
                                                border.color: isSep ? theme.accent : theme.borderSubtle
                                                border.width: 1
                                                Text {
                                                    id: sepLbl
                                                    anchors.centerIn: parent
                                                    text: modelData.label
                                                    font.pixelSize: 10
                                                    font.bold: parent.isSep
                                                    color: parent.isSep ? theme.accent : theme.textSecondary
                                                }
                                                MouseArea {
                                                    anchors.fill: parent
                                                    cursorShape: Qt.PointingHandCursor
                                                    onClicked: {
                                                        root.displaySeparator = modelData.val
                                                        root.separatorModified = true
                                                        root.markModified()
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Dither Toggle Card
                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 44
                                    radius: theme.radiusSm
                                    color: theme.bg
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 10
                                        anchors.rightMargin: 10
                                        spacing: 8

                                        ColumnLayout {
                                            spacing: 0
                                            Text { text: "Dither Preview"; font.pixelSize: 12; font.weight: Font.Medium; color: theme.text }
                                            Text { text: "Subtle contrast softening"; font.pixelSize: 10; color: theme.textMuted }
                                        }
                                        Item { Layout.fillWidth: true }

                                        Rectangle {
                                            width: 40; height: 22; radius: 11
                                            color: root.dither ? theme.accent : theme.surfaceElevated
                                            border.color: root.dither ? theme.accent : theme.borderStrong
                                            border.width: 1

                                            Rectangle {
                                                width: 16; height: 16; radius: 8
                                                anchors.verticalCenter: parent.verticalCenter
                                                x: root.dither ? parent.width - width - 3 : 3
                                                color: "#FFFFFF"
                                                Behavior on x { NumberAnimation { duration: 120 } }
                                            }

                                            MouseArea {
                                                anchors.fill: parent
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: {
                                                    root.dither = !root.dither
                                                    root.markModified()
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // ─────────────────────────────────────────────
                            // TAB 3: COLORS (Readout Key Accent Color)
                            // ─────────────────────────────────────────────
                            ColumnLayout {
                                visible: root.inspectorSection === "colors"
                                Layout.fillWidth: true
                                spacing: 12

                                Text {
                                    text: "Readout Accent Color"
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    color: theme.text
                                }

                                // Live Color Preview Chip
                                Rectangle {
                                    Layout.fillWidth: true
                                    height: 48
                                    radius: theme.radiusSm
                                    color: "#080A0F"
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 12
                                        anchors.rightMargin: 12
                                        spacing: 10

                                        Text {
                                            text: "OS"
                                            font.pixelSize: 12
                                            font.bold: true
                                            font.family: "JetBrains Mono, monospace"
                                            color: root.accentColor
                                        }
                                        Text {
                                            text: root.displaySeparator
                                            font.pixelSize: 12
                                            font.family: "JetBrains Mono, monospace"
                                            color: theme.textMuted
                                        }
                                        Text {
                                            text: "Linux (x86_64)"
                                            font.pixelSize: 12
                                            font.family: "JetBrains Mono, monospace"
                                            color: "#F1F5F9"
                                        }
                                        Item { Layout.fillWidth: true }
                                        Rectangle {
                                            width: 18; height: 18; radius: 9
                                            color: root.accentColor
                                            border.color: "#FFFFFF"
                                            border.width: 1.5
                                        }
                                        Text {
                                            text: root.accentColor.toUpperCase()
                                            font.pixelSize: 11
                                            font.bold: true
                                            font.family: "JetBrains Mono, monospace"
                                            color: theme.textSecondary
                                        }
                                    }
                                }

                                Rectangle {
                                    Layout.fillWidth: true
                                    implicitHeight: colorCol.implicitHeight + 20
                                    radius: theme.radiusSm
                                    color: theme.bg
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    ColumnLayout {
                                        id: colorCol
                                        anchors.fill: parent
                                        anchors.margins: 10
                                        spacing: 12

                                        RowLayout {
                                            Layout.fillWidth: true
                                            spacing: 10

                                            Rectangle {
                                                width: 32; height: 32; radius: 6
                                                color: root.accentColor
                                                border.color: theme.borderStrong
                                                border.width: 1
                                            }

                                            ColumnLayout {
                                                Layout.fillWidth: true
                                                spacing: 1
                                                Text {
                                                    text: "Key & Tagline Color"
                                                    font.pixelSize: 12
                                                    font.weight: Font.Medium
                                                    color: theme.text
                                                }
                                                Text {
                                                    text: "Applied to Fastfetch module keys"
                                                    font.pixelSize: 10
                                                    color: theme.textMuted
                                                }
                                            }

                                            TextField {
                                                id: accentHexInput
                                                Layout.preferredWidth: 86
                                                Layout.preferredHeight: 28
                                                font.pixelSize: 11
                                                font.family: "JetBrains Mono, monospace"
                                                text: root.accentColor
                                                color: theme.text
                                                leftPadding: 8
                                                rightPadding: 8
                                                background: Rectangle {
                                                    radius: theme.radiusSm
                                                    color: theme.surface
                                                    border.color: accentHexInput.activeFocus ? theme.accent : theme.borderSubtle
                                                    border.width: 1
                                                }
                                                onTextEdited: {
                                                    var v = text.trim()
                                                    if (/^#[0-9a-fA-F]{6}$/.test(v)) {
                                                        root.accentColor = v
                                                        root.colorModified = true
                                                        root.markModified()
                                                    }
                                                }
                                            }
                                        }

                                        Flow {
                                            Layout.fillWidth: true
                                            spacing: 8

                                            Repeater {
                                                model: [
                                                    "#6488E8", "#3B82F6", "#38BDF8", "#06B6D4",
                                                    "#10B981", "#22C55E", "#84CC16", "#EAB308",
                                                    "#F59E0B", "#F97316", "#EF4444", "#F43F5E",
                                                    "#EC4899", "#D946EF", "#A855F7", "#8B5CF6",
                                                    "#6366F1", "#94A3B8"
                                                ]
                                                Rectangle {
                                                    width: 30; height: 30; radius: 7
                                                    color: modelData
                                                    property bool isSwatchSel: root.accentColor.toLowerCase() === modelData.toLowerCase()
                                                    border.color: isSwatchSel ? "#FFFFFF" : "transparent"
                                                    border.width: isSwatchSel ? 2 : 0

                                                    // Active Selection Outer Ring
                                                    Rectangle {
                                                        visible: parent.isSwatchSel
                                                        anchors.fill: parent
                                                        anchors.margins: -3
                                                        radius: 10
                                                        color: "transparent"
                                                        border.color: modelData
                                                        border.width: 1.5
                                                    }

                                                    MouseArea {
                                                        anchors.fill: parent
                                                        cursorShape: Qt.PointingHandCursor
                                                        onClicked: {
                                                            root.accentColor = modelData
                                                            accentHexInput.text = modelData
                                                            root.colorModified = true
                                                            root.markModified()
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // ─────────────────────────────────────────────
                            // TAB 4: MODULES (Order, Visibility, Add/Remove)
                            // ─────────────────────────────────────────────
                            ColumnLayout {
                                visible: root.inspectorSection === "modules"
                                Layout.fillWidth: true
                                spacing: 10

                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        text: "Readout Modules"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        color: theme.text
                                    }
                                    Item { Layout.fillWidth: true }
                                    StudioButton {
                                        iconName: "plus"
                                        label: root.showAddModuleBar ? "Done" : "Add Module"
                                        height: 26
                                        onClicked: root.showAddModuleBar = !root.showAddModuleBar
                                    }
                                }

                                // Expandable Quick-Add Module Pills
                                Rectangle {
                                    visible: root.showAddModuleBar
                                    Layout.fillWidth: true
                                    implicitHeight: addFlow.implicitHeight + 16
                                    radius: theme.radiusSm
                                    color: theme.bg
                                    border.color: theme.borderSubtle
                                    border.width: 1

                                    Flow {
                                        id: addFlow
                                        anchors.fill: parent
                                        anchors.margins: 8
                                        spacing: 6

                                        Repeater {
                                            model: root.availableNewModules
                                            Rectangle {
                                                height: 24
                                                width: addPillTxt.implicitWidth + 16
                                                radius: 4
                                                color: addPillM.containsMouse ? theme.surfaceHover : theme.surfaceElevated
                                                border.color: addPillM.containsMouse ? theme.accent : theme.borderSubtle
                                                border.width: 1

                                                Text {
                                                    id: addPillTxt
                                                    anchors.centerIn: parent
                                                    text: "+ " + modelData.label
                                                    font.pixelSize: 10
                                                    font.weight: Font.Medium
                                                    color: theme.text
                                                }

                                                MouseArea {
                                                    id: addPillM
                                                    anchors.fill: parent
                                                    hoverEnabled: true
                                                    cursorShape: Qt.PointingHandCursor
                                                    onClicked: {
                                                        var next = root.moduleList.slice()
                                                        next.push({
                                                            id: "r-" + Date.now() + "-" + modelData.key,
                                                            rowType: modelData.rowType,
                                                            key: modelData.key,
                                                            label: modelData.label,
                                                            customText: modelData.customText || "",
                                                            enabled: true
                                                        })
                                                        root.moduleList = next
                                                        root.modulesModified = true
                                                        root.markModified()
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Module Rows List
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 5

                                    Repeater {
                                        model: root.moduleList
                                        Rectangle {
                                            Layout.fillWidth: true
                                            height: 36
                                            radius: theme.radiusSm
                                            color: theme.bg
                                            border.color: theme.borderSubtle
                                            border.width: 1
                                            opacity: modelData.enabled ? 1.0 : 0.52

                                            RowLayout {
                                                anchors.fill: parent
                                                anchors.leftMargin: 6
                                                anchors.rightMargin: 6
                                                spacing: 4

                                                // Move Up
                                                Rectangle {
                                                    width: 22; height: 22; radius: 4
                                                    color: upM.containsMouse ? theme.surfaceHover : "transparent"
                                                    RyzoraIcon { anchors.centerIn: parent; name: "chevron-up"; size: 12; color: theme.textSecondary }
                                                    MouseArea {
                                                        id: upM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                        onClicked: {
                                                            if (index > 0) {
                                                                var next = root.moduleList.slice()
                                                                var item = next.splice(index, 1)[0]
                                                                next.splice(index - 1, 0, item)
                                                                root.moduleList = next
                                                                root.modulesModified = true
                                                                root.markModified()
                                                            }
                                                        }
                                                    }
                                                }

                                                // Move Down
                                                Rectangle {
                                                    width: 22; height: 22; radius: 4
                                                    color: dnM.containsMouse ? theme.surfaceHover : "transparent"
                                                    RyzoraIcon { anchors.centerIn: parent; name: "chevron-down"; size: 12; color: theme.textSecondary }
                                                    MouseArea {
                                                        id: dnM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                        onClicked: {
                                                            if (index < root.moduleList.length - 1) {
                                                                var next = root.moduleList.slice()
                                                                var item = next.splice(index, 1)[0]
                                                                next.splice(index + 1, 0, item)
                                                                root.moduleList = next
                                                                root.modulesModified = true
                                                                root.markModified()
                                                            }
                                                        }
                                                    }
                                                }

                                                // Spacer Row
                                                Text {
                                                    visible: modelData.rowType === "spacer"
                                                    text: "Spacer Line"
                                                    font.pixelSize: 11
                                                    color: theme.textSecondary
                                                    Layout.fillWidth: true
                                                    Layout.leftMargin: 4
                                                }

                                                // Editable Section / Tagline Row
                                                RowLayout {
                                                    visible: modelData.rowType === "section" || modelData.rowType === "tagline"
                                                    Layout.fillWidth: true
                                                    spacing: 6

                                                    Text {
                                                        text: modelData.rowType === "section" ? "Header" : "Tagline"
                                                        font.pixelSize: 10
                                                        font.weight: Font.Medium
                                                        color: theme.textMuted
                                                    }

                                                    TextField {
                                                        Layout.fillWidth: true
                                                        Layout.preferredHeight: 24
                                                        font.pixelSize: 11
                                                        text: modelData.customText || ""
                                                        color: theme.text
                                                        leftPadding: 6
                                                        rightPadding: 6
                                                        background: Rectangle {
                                                            radius: 4
                                                            color: theme.surface
                                                            border.color: theme.borderSubtle
                                                        }
                                                        onTextEdited: {
                                                            var next = root.moduleList.slice()
                                                            var updated = Object.assign({}, next[index])
                                                            updated.customText = text
                                                            updated.customModified = true
                                                            next[index] = updated
                                                            root.moduleList = next
                                                            root.modulesModified = true
                                                            root.markModified()
                                                        }
                                                    }
                                                }

                                                // Standard Telemetry Module Row
                                                RowLayout {
                                                    visible: modelData.rowType === "telemetry"
                                                    Layout.fillWidth: true
                                                    spacing: 6
                                                    Layout.leftMargin: 4

                                                    Text {
                                                        text: modelData.label
                                                        font.pixelSize: 11
                                                        font.weight: Font.DemiBold
                                                        color: theme.text
                                                        elide: Text.ElideRight
                                                    }

                                                    Text {
                                                        text: modelData.key
                                                        font.pixelSize: 10
                                                        font.family: "JetBrains Mono, monospace"
                                                        color: theme.textMuted
                                                        elide: Text.ElideRight
                                                        Layout.fillWidth: true
                                                    }
                                                }

                                                // Visibility Toggle (Eye Icon)
                                                Rectangle {
                                                    width: 24; height: 24; radius: 4
                                                    color: visM.containsMouse ? theme.surfaceHover : "transparent"
                                                    RyzoraIcon {
                                                        anchors.centerIn: parent
                                                        name: modelData.enabled ? "eye" : "eye-off"
                                                        size: 13
                                                        color: modelData.enabled ? theme.accent : theme.textMuted
                                                    }
                                                    MouseArea {
                                                        id: visM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                        onClicked: {
                                                            var next = root.moduleList.slice()
                                                            var updated = Object.assign({}, next[index])
                                                            updated.enabled = !updated.enabled
                                                            next[index] = updated
                                                            root.moduleList = next
                                                            root.modulesModified = true
                                                            root.markModified()
                                                        }
                                                    }
                                                }

                                                // Remove Row (Trash Icon)
                                                Rectangle {
                                                    width: 24; height: 24; radius: 4
                                                    color: delM.containsMouse ? Qt.rgba(239/255, 68/255, 68/255, 0.14) : "transparent"
                                                    RyzoraIcon {
                                                        anchors.centerIn: parent
                                                        name: "trash"
                                                        size: 12
                                                        color: delM.containsMouse ? theme.error : theme.textMuted
                                                    }
                                                    MouseArea {
                                                        id: delM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                                        onClicked: {
                                                            var next = root.moduleList.slice()
                                                            next.splice(index, 1)
                                                            root.moduleList = next
                                                            root.modulesModified = true
                                                            root.markModified()
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            Item { width: 1; height: 24 }
                        }
                    }
                }
            }
        }

        // ─────────────────────────────────────────────────────────────────
        // 3. BOTTOM ACTION BAR (Status + Revert + Save)
        // ─────────────────────────────────────────────────────────────────
        Rectangle {
            Layout.fillWidth: true
            height: 52
            color: theme.surface
            border.color: theme.borderSubtle
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 24
                anchors.rightMargin: 24
                spacing: 12

                RowLayout {
                    spacing: 8
                    Rectangle {
                        width: 8; height: 8; radius: 4
                        color: root.isModified ? theme.warning : theme.success
                    }
                    Text {
                        text: root.isModified
                              ? "Unsaved changes in draft · ~/.config/fastfetch/config.jsonc"
                              : "Saved · ~/.config/fastfetch/config.jsonc"
                        font.pixelSize: 12
                        font.weight: Font.Medium
                        color: root.isModified ? theme.text : theme.textSecondary
                    }
                }

                Item { Layout.fillWidth: true }

                StudioButton {
                    iconName: "rotate-ccw"
                    label: "Revert"
                    enabledBtn: root.isModified
                    onClicked: root.triggerRevertDraft()
                }

                StudioButton {
                    iconName: "terminal"
                    label: "Preview in Terminal"
                    onClicked: {
                        var draft = root.buildDraftJson()
                        backend.launchTerminalWithDraft(draft)
                    }
                }

                StudioButton {
                    iconName: "upload"
                    label: "Apply Installed Style"
                    onClicked: root.triggerApplySelectedStyle()
                }

                StudioButton {
                    iconName: "save"
                    label: "Save Configuration"
                    primary: true
                    onClicked: root.triggerSaveDraft()
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // MODAL: EDIT CONFIG (In-App JSONC Editor + External Editor)
    // ─────────────────────────────────────────────────────────────────
    Rectangle {
        visible: root.showRawConfigModal
        anchors.fill: parent
        color: "#B3000000"
        z: 100

        MouseArea {
            anchors.fill: parent
            onClicked: root.showRawConfigModal = false
        }

        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width - 60, 740)
            height: Math.min(parent.height - 60, 560)
            radius: theme.radiusLg
            color: theme.surface
            border.color: theme.borderStrong
            border.width: 1

            MouseArea { anchors.fill: parent }

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 12

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    RyzoraIcon { name: "file-text"; size: 16; color: theme.accent }
                    Text {
                        text: "~/.config/fastfetch/config.jsonc"
                        font.pixelSize: 13
                        font.bold: true
                        font.family: "JetBrains Mono, monospace"
                        color: theme.text
                    }
                    Item { Layout.fillWidth: true }
                    StudioButton {
                        iconName: "terminal"
                        label: "Open in External Editor"
                        onClicked: backend.openFastfetchConfigInEditor()
                    }
                    StudioButton {
                        iconName: "x"
                        label: "Close"
                        onClicked: root.showRawConfigModal = false
                    }
                }

                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true

                    TextArea {
                        id: rawConfigEditor
                        selectByMouse: true
                        text: root.buildDraftJson()
                        font.family: "JetBrains Mono, monospace"
                        font.pixelSize: 12
                        color: "#F1F5F9"
                        background: Rectangle {
                            color: "#090B0F"
                            radius: theme.radiusSm
                            border.color: theme.borderSubtle
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Item { Layout.fillWidth: true }
                    StudioButton {
                        iconName: "save"
                        label: "Apply & Save Raw JSONC"
                        primary: true
                        onClicked: {
                            var edited = rawConfigEditor.text
                            if (backend.saveFastfetchConfiguration(edited, root.selectedPresetId || "custom")) {
                                root.baseConfigJsonc = edited
                                root.parseLoadedConfig(edited)
                                root.isSaved = true
                                root.isModified = false
                                root.showRawConfigModal = false
                                root.showFeedback("success", "Saved raw JSONC configuration.")
                                root.updatePreview()
                            } else {
                                root.showFeedback("error", "Invalid JSON/JSONC configuration.")
                            }
                        }
                    }
                }
            }
        }
    }
}
