// SPDX-License-Identifier: MIT
pragma Singleton
import QtQuick

// The Control Center module registry (T235).
//
// One pure, QML-only source of truth for the measured Control Center grid and
// the Edit Controls catalogue. It owns:
//
//   * the 22-category Edit Controls inventory (`categories`),
//   * every module's label, owner, glyph, category and default span
//     (`modules`),
//   * the measured default panel inventory (`defaultModules`),
//   * the 4-column 64 pt grid with 12 pt gaps (`layout`, `spanSize`),
//   * the three-page model (`pageModules`) and the span overrides
//     (`spanFor`), and
//   * the menu-bar-only mapping (`menuBarKey`).
//
// The panel and the Edit Controls catalogue consume the same registry, so the
// live grid and the management surface cannot drift apart. No service or bus
// access happens here.
QtObject {
    id: root

    // ── Measured geometry (docs/design/23-macos-structural-surface-review.md
    //    §6, S-CC thresholds) ─────────────────────────────────────────────
    readonly property int columns: 4
    readonly property int cellSize: 64
    readonly property int gap: 12
    // 4×64 + 3×12 = 292 pt, the measured module field.
    readonly property int moduleFieldWidth: columns * cellSize + (columns - 1) * gap
    // The measured expanded Wi-Fi sheet: 292 pt field + 16 pt inset.
    readonly property int expandedSheetWidth: 308
    readonly property int expandedOuterInset: 16
    // The measured Edit Controls window and category sidebar.
    readonly property int editWindowWidth: 736
    readonly property int editWindowHeight: 900
    readonly property int editSidebarWidth: 245

    // ── The 22-category Edit Controls inventory, in reference order ────────
    readonly property var categories: [
        { id: "all-controls", label: qsTr("All Controls") },
        { id: "battery", label: qsTr("Battery") },
        { id: "clock", label: qsTr("Clock") },
        { id: "connectivity", label: qsTr("Connectivity") },
        { id: "desktop-finder", label: qsTr("Desktop & Files") },
        { id: "display-brightness", label: qsTr("Display & Brightness") },
        { id: "fast-user-switching", label: qsTr("Fast User Switching") },
        { id: "focus", label: qsTr("Focus") },
        { id: "home", label: qsTr("Home") },
        { id: "keyboards", label: qsTr("Keyboards") },
        { id: "lock-screen", label: qsTr("Lock Screen") },
        { id: "notes", label: qsTr("Notes") },
        { id: "now-playing", label: qsTr("Now Playing") },
        { id: "screenshot", label: qsTr("Screenshot") },
        { id: "shortcuts", label: qsTr("Shortcuts") },
        { id: "sounds", label: qsTr("Sounds") },
        { id: "utilities", label: qsTr("Utilities") },
        { id: "voice-memos", label: qsTr("Voice Memos") },
        { id: "accessibility", label: qsTr("Accessibility") },
        { id: "hearing-accessibility", label: qsTr("Hearing Accessibility") },
        { id: "motor-accessibility", label: qsTr("Motor Accessibility") },
        { id: "vision-accessibility", label: qsTr("Vision Accessibility") }
    ]

    // ── The module catalogue ──────────────────────────────────────────────
    // `kind` drives the capsule's control: toggle, slider, info, action or
    // storage. `span` is the measured default. `expandable` modules open a
    // sheet; every module has a sheet so absence and state stay inspectable.
    // `owner` names the real adapter/owner; an empty owner is an explicit
    // unavailable module (AirDrop, Stage Manager, media, ...).
    readonly property var modules: [
        // Connectivity
        { id: "wifi", label: qsTr("Wi-Fi"), category: "connectivity",
          glyph: "wifi", span: "2x1", kind: "toggle", expandable: true,
          owner: "NetworkManager" },
        { id: "bluetooth", label: qsTr("Bluetooth"), category: "connectivity",
          glyph: "bluetooth", span: "2x1", kind: "toggle", expandable: true,
          owner: "BlueZ" },
        { id: "airdrop", label: qsTr("Nearby Transfer"), category: "connectivity",
          glyph: "airdrop", span: "2x1", kind: "action", expandable: true,
          owner: "" },
        { id: "vpn", label: qsTr("VPN"), category: "connectivity",
          glyph: "vpn", span: "2x1", kind: "info", expandable: true,
          owner: "NetworkManager" },
        // Now Playing
        { id: "now-playing", label: qsTr("Now Playing"),
          category: "now-playing", glyph: "now-playing", span: "2x2",
          kind: "media", expandable: true, owner: "" },
        // Desktop & Finder
        { id: "stage-manager", label: qsTr("Stage"),
          category: "desktop-finder", glyph: "stage-manager", span: "1x1",
          kind: "action", expandable: true, owner: "" },
        { id: "edit-controls", label: qsTr("Edit Controls"),
          category: "all-controls", glyph: "edit-controls", span: "footer",
          kind: "edit", expandable: false, owner: "shell" },
        // Display & Brightness
        { id: "brightness", label: qsTr("Display"),
          category: "display-brightness", glyph: "brightness", span: "4x1",
          kind: "slider", expandable: true, owner: "compositor" },
        { id: "dark", label: qsTr("Dark Mode"),
          category: "display-brightness", glyph: "appearance", span: "1x1",
          kind: "toggle", expandable: true, owner: "settingsd" },
        { id: "keyboard-brightness", label: qsTr("Keyboard Brightness"),
          category: "display-brightness", glyph: "keyboard-brightness",
          span: "2x1", kind: "info", expandable: true, owner: "" },
        // Focus
        { id: "focus", label: qsTr("Focus"), category: "focus",
          glyph: "focus", span: "2x1", kind: "toggle", expandable: true,
          owner: "notification service" },
        // Sounds
        { id: "volume", label: qsTr("Sound"), category: "sounds",
          glyph: "volume", span: "4x1", kind: "slider", expandable: true,
          owner: "PipeWire / WirePlumber" },
        { id: "airplay-audio", label: qsTr("Audio Casting"),
          category: "sounds", glyph: "airplay-audio", span: "1x1",
          kind: "info", expandable: true, owner: "" },
        // Battery
        { id: "battery", label: qsTr("Battery"), category: "battery",
          glyph: "battery", span: "2x1", kind: "info", expandable: true,
          owner: "UPower" },
        // Clock
        { id: "clock", label: qsTr("Clock"), category: "clock",
          glyph: "clock", span: "2x1", kind: "info", expandable: true,
          owner: "shell" },
        // Lock Screen
        { id: "lock-screen", label: qsTr("Lock Screen"),
          category: "lock-screen", glyph: "lock", span: "2x1",
          kind: "info", expandable: true, owner: "session idle engine" },
        // Keyboards
        { id: "keyboard", label: qsTr("Keyboard"), category: "keyboards",
          glyph: "keyboard", span: "2x1", kind: "info", expandable: true,
          owner: "libinput" },
        { id: "shortcuts", label: qsTr("Shortcuts"), category: "shortcuts",
          glyph: "shortcuts", span: "2x1", kind: "info", expandable: true,
          owner: "" },
        // Screenshot
        { id: "capture-screen", label: qsTr("Capture Screen"),
          category: "screenshot", glyph: "capture-screen", span: "1x1",
          kind: "action", expandable: true, owner: "screenshot portal" },
        { id: "screenshot", label: qsTr("Screenshot"), category: "screenshot",
          glyph: "screenshot", span: "2x1", kind: "action", expandable: true,
          owner: "screenshot portal" },
        // Utilities
        { id: "storage", label: qsTr("Storage"), category: "utilities",
          glyph: "storage", span: "4x1", kind: "storage", expandable: true,
          owner: "UDisks2" },
        { id: "mission-control", label: qsTr("Overview"),
          category: "utilities", glyph: "overview", span: "2x1",
          kind: "info", expandable: true, owner: "compositor" },
        { id: "menu-bar", label: qsTr("Menu Bar"), category: "utilities",
          glyph: "menu-bar", span: "2x1", kind: "info", expandable: true,
          owner: "shell" },
        { id: "software-update", label: qsTr("Software Update"),
          category: "utilities", glyph: "software-update", span: "2x1",
          kind: "info", expandable: true, owner: "update provider" },
        { id: "users", label: qsTr("Users"), category: "utilities",
          glyph: "users", span: "2x1", kind: "info", expandable: true,
          owner: "AccountsService" },
        { id: "printers", label: qsTr("Printers"), category: "utilities",
          glyph: "printer", span: "2x1", kind: "info", expandable: true,
          owner: "CUPS / SANE" },
        { id: "privacy", label: qsTr("Privacy"), category: "utilities",
          glyph: "privacy", span: "2x1", kind: "info", expandable: true,
          owner: "portal PermissionStore" },
        { id: "utilities", label: qsTr("Utilities"), category: "utilities",
          glyph: "utilities", span: "2x1", kind: "info", expandable: true,
          owner: "" },
        // Home / Notes / Voice Memos
        { id: "home-control", label: qsTr("Home"), category: "home",
          glyph: "home-control", span: "2x1", kind: "info", expandable: true,
          owner: "" },
        { id: "notes", label: qsTr("Notes"), category: "notes",
          glyph: "notes", span: "2x1", kind: "info", expandable: true,
          owner: "" },
        { id: "voice-memos", label: qsTr("Voice Memos"),
          category: "voice-memos", glyph: "voice-memos", span: "2x1",
          kind: "info", expandable: true, owner: "" },
        // Fast User Switching
        { id: "fast-user-switching", label: qsTr("Fast User Switching"),
          category: "fast-user-switching", glyph: "fast-user-switching",
          span: "2x1", kind: "info", expandable: true, owner: "" },
        // Accessibility
        { id: "accessibility", label: qsTr("Accessibility"),
          category: "accessibility", glyph: "accessibility", span: "2x1",
          kind: "info", expandable: true, owner: "AT-SPI" },
        { id: "hearing-accessibility", label: qsTr("Hearing"),
          category: "hearing-accessibility", glyph: "hearing",
          span: "2x1", kind: "info", expandable: true, owner: "" },
        { id: "motor-accessibility", label: qsTr("Motor"),
          category: "motor-accessibility", glyph: "motor",
          span: "2x1", kind: "info", expandable: true, owner: "" },
        { id: "vision-accessibility", label: qsTr("Vision"),
          category: "vision-accessibility", glyph: "vision",
          span: "2x1", kind: "info", expandable: true, owner: "" }
    ]

    // The measured default panel inventory (13 modules, in reference order).
    readonly property var defaultModules: [
        "wifi", "bluetooth", "now-playing", "airdrop", "stage-manager",
        "screen-mirroring", "dark", "capture-screen", "focus", "brightness",
        "volume", "airplay-audio", "edit-controls"
    ]

    // The pre-T235 18-module inventory. The Settings pane's existing
    // customization keeps these ids valid, so a persisted older list still
    // renders; the default is the measured 13-module panel above.
    readonly property var legacyModules: [
        "wifi", "bluetooth", "storage", "focus", "volume", "brightness",
        "dark", "keyboard", "mission-control", "battery", "lock-screen",
        "menu-bar", "software-update", "users", "printers", "privacy",
        "accessibility", "vpn"
    ]

    // `screen-mirroring` is part of the default inventory but its owner is the
    // ScreenCast portal, so it is declared separately to keep the catalogue
    // order readable.
    readonly property var screenMirroringModule: ({
        id: "screen-mirroring", label: qsTr("Screen Casting"),
        category: "connectivity", glyph: "screen-mirroring", span: "1x1",
        kind: "action", expandable: true, owner: "ScreenCast portal"
    })

    // ── Registry lookups ──────────────────────────────────────────────────
    function module(id) {
        if (id === "screen-mirroring")
            return root.screenMirroringModule;
        for (var i = 0; i < root.modules.length; ++i) {
            if (root.modules[i].id === id)
                return root.modules[i];
        }
        return null;
    }

    function moduleLabel(id) {
        var entry = root.module(id);
        return entry !== null ? entry.label : id;
    }

    function moduleCategory(id) {
        var entry = root.module(id);
        return entry !== null ? entry.category : "all-controls";
    }

    function moduleGlyph(id) {
        var entry = root.module(id);
        return entry !== null ? entry.glyph : "info";
    }

    function moduleKind(id) {
        var entry = root.module(id);
        return entry !== null ? entry.kind : "info";
    }

    function defaultSpan(id) {
        var entry = root.module(id);
        return entry !== null ? entry.span : "2x1";
    }

    function menuBarKey(id) {
        switch (id) {
        case "wifi": return "menu.control.wifi";
        case "bluetooth": return "menu.control.bluetooth";
        case "focus": return "menu.control.focus";
        case "volume": return "menu.control.volume";
        case "battery": return "menu.control.battery";
        case "accessibility": return "menu.control.accessibility";
        default: return "";
        }
    }

    function categoryLabel(id) {
        for (var i = 0; i < root.categories.length; ++i) {
            if (root.categories[i].id === id)
                return root.categories[i].label;
        }
        return id;
    }

    // Every catalogue id, in catalogue order (the Edit Controls "All Controls"
    // list). `screen-mirroring` sits with connectivity after Bluetooth.
    readonly property var catalogueIds: {
        var out = [];
        for (var i = 0; i < root.modules.length; ++i) {
            if (root.modules[i].id === "airdrop")
                out.push(root.modules[i].id, "screen-mirroring");
            else
                out.push(root.modules[i].id);
        }
        return out;
    }

    // The controls a category shows. `all-controls` is the whole catalogue.
    function categoryModules(categoryId) {
        if (categoryId === "all-controls")
            return root.catalogueIds.slice();
        var out = [];
        for (var i = 0; i < root.catalogueIds.length; ++i) {
            var id = root.catalogueIds[i];
            if (root.moduleCategory(id) === categoryId)
                out.push(id);
        }
        return out;
    }

    // ── Spans and the measured grid ───────────────────────────────────────
    readonly property var spanNames: ["1x1", "2x1", "2x2", "4x1"]

    // [columns, rows] for a span name.
    function spanCells(span) {
        switch (span) {
        case "1x1": return [1, 1];
        case "2x1": return [2, 1];
        case "2x2": return [2, 2];
        case "4x1": return [4, 1];
        default: return [2, 1];
        }
    }

    // The measured pixel size of one span.
    function spanSize(span) {
        var cells = root.spanCells(span);
        var cols = cells[0];
        var rows = cells[1];
        return {
            w: cols * root.cellSize + (cols - 1) * root.gap,
            h: rows * root.cellSize + (rows - 1) * root.gap
        };
    }

    // Cycle through the spans that fit the 4-column field.
    function nextSpan(span) {
        var at = root.spanNames.indexOf(span);
        if (at < 0)
            return "2x1";
        return root.spanNames[(at + 1) % root.spanNames.length];
    }

    // Parse the persisted `quickSettings.spans` list ("id=2x1") into a map.
    function parseSpans(list) {
        var out = {};
        if (!list)
            return out;
        for (var i = 0; i < list.length; ++i) {
            var entry = String(list[i]);
            var at = entry.indexOf("=");
            if (at <= 0)
                continue;
            var id = entry.slice(0, at);
            var span = entry.slice(at + 1);
            if (root.spanNames.indexOf(span) >= 0)
                out[id] = span;
        }
        return out;
    }

    function spanFor(id, spans) {
        if (spans && spans[id] !== undefined)
            return spans[id];
        return root.defaultSpan(id);
    }

    function serializeSpans(spans) {
        var out = [];
        for (var id in spans) {
            if (root.spanNames.indexOf(spans[id]) >= 0)
                out.push(id + "=" + spans[id]);
        }
        out.sort();
        return out;
    }

    // Pack the ordered module ids into the measured 4-column grid. The first
    // free cell wins; a 2×2 module reserves two rows and columns. `footer`
    // modules (Edit Controls) are not part of the grid.
    function layout(ids, spans) {
        var out = [];
        var occupancy = [];
        var limit = Math.max(8, ids.length * 2 + 4);

        function ensure(row) {
            while (occupancy.length <= row)
                occupancy.push([false, false, false, false]);
        }
        function free(row, col, rows, cols) {
            ensure(row + rows - 1);
            for (var r = 0; r < rows; ++r) {
                for (var c = 0; c < cols; ++c) {
                    if (occupancy[row + r][col + c])
                        return false;
                }
            }
            return true;
        }
        function mark(row, col, rows, cols) {
            for (var r = 0; r < rows; ++r) {
                for (var c = 0; c < cols; ++c)
                    occupancy[row + r][col + c] = true;
            }
        }

        for (var i = 0; i < ids.length; ++i) {
            var id = ids[i];
            if (root.module(id) === null)
                continue;
            var span = root.spanFor(id, spans);
            if (span === "footer")
                continue;
            var cells = root.spanCells(span);
            var cols = cells[0];
            var rows = cells[1];
            var placed = false;
            for (var row = 0; row < limit && !placed; ++row) {
                for (var col = 0; col + cols <= root.columns; ++col) {
                    if (!free(row, col, rows, cols))
                        continue;
                    mark(row, col, rows, cols);
                    var size = root.spanSize(span);
                    out.push({
                        id: id,
                        span: span,
                        row: row,
                        col: col,
                        x: col * (root.cellSize + root.gap),
                        y: row * (root.cellSize + root.gap),
                        w: size.w,
                        h: size.h
                    });
                    placed = true;
                    break;
                }
            }
        }
        return out;
    }

    function gridHeight(layout) {
        var height = 0;
        for (var i = 0; i < layout.length; ++i)
            height = Math.max(height, layout[i].y + layout[i].h);
        return height;
    }

    // The three persisted pages: page 0 is `modules`; pages 1 and 2 are the
    // optional extra lists. `pageCount` lets an empty extra page exist.
    function pageModules(modules, page2, page3, page) {
        if (page === 1)
            return page2 ? Array.from(page2) : [];
        if (page === 2)
            return page3 ? Array.from(page3) : [];
        return modules ? Array.from(modules) : [];
    }
}
