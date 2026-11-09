// SPDX-License-Identifier: MIT
import QtQuick

// The local Settings search index (T223).
//
// One in-memory index built from the executable map (`SettingsMap.qml`) and the
// pane catalog (`SettingsPanes.qml`). It contains pane titles, every map
// section, and every shipped row, plus synonyms and the row's owner/key/absence
// help text. Search is a pure, synchronous function over that array: no bus, no
// daemon, no per-keystroke I/O.
//
// A result is only returned when it can deep-link: pane results use
// `settings://pane/<id>`, section results point at their first row, and row
// results use `settings://pane/<id>/row/<rowId>`. Unshipped panes/rows are not
// indexed at all (the no-half-panes rule).
//
// Ranking is field-weighted: exact label > label prefix > label word start >
// label substring, then synonyms, section, pane title, and finally help text.
// Pane results win ties over sections, and sections over rows, so a query like
// "wallpaper" opens the pane rather than a row inside it.
pragma Singleton

QtObject {
    id: root

    readonly property int resultLimit: 40

    // Pane-level synonyms. These are the user's words, not a second catalog:
    // the pane title/id remain the only labels shown in the UI.
    readonly property var paneSynonyms: ({
        "wifi": ["wireless", "wlan", "internet", "network"],
        "bluetooth": ["wireless devices", "pair", "devices"],
        "network": ["vpn", "firewall", "ethernet", "internet", "connections"],
        "battery": ["power", "energy", "charge", "low power mode"],
        "general": ["about", "software update", "storage", "language", "region",
                    "date", "time", "sharing", "backup", "reset", "startup disk",
                    "passwords", "login items"],
        "storage": ["disk", "drive", "volume", "space", "disk usage"],
        "appearance": ["theme", "dark mode", "light mode", "accent", "colors",
                       "style", "transparency"],
        "accessibility": ["a11y", "screen reader", "contrast", "motion", "vision",
                          "hearing", "motor"],
        "control-center": ["quick settings", "tiles", "modules"],
        "menu-bar": ["status items", "clock", "menu extras", "menu bar items"],
        "search": ["index", "indexing", "web search", "shortcut"],
        "privacy": ["security", "permissions", "camera", "microphone", "location",
                    "files", "screen recording"],
        "desktop-dock": ["dock", "desktop", "stage", "hot corners", "windows",
                         "widgets"],
        "mission-control": ["overview", "workspaces", "spaces", "hot corners",
                            "gestures"],
        "displays": ["monitor", "screen", "resolution", "scale", "rotation",
                     "brightness", "night light", "refresh rate", "color"],
        "wallpaper": ["background", "desktop picture", "photo", "image",
                      "featured", "abstract", "nature", "water", "cityscape",
                      "earth", "space", "nasa", "scenery", "underwater"],
        "screen-saver": ["screensaver", "idle", "clock"],
        "notifications": ["alerts", "banners", "previews", "badges", "sounds"],
        "sound": ["audio", "volume", "speaker", "microphone", "output", "input",
                  "alert sound"],
        "focus": ["do not disturb", "dnd", "notifications", "modes"],
        "lock-screen": ["lock", "password", "login", "sleep", "screen saver"],
        "biometrics": ["fingerprint", "password", "authentication"],
        "users-groups": ["users", "accounts", "groups", "login", "administrator"],
        "online-accounts": ["accounts", "sync", "cloud"],
        "printers": ["printing", "scanner", "cups", "paper"],
        "keyboard": ["keys", "shortcuts", "repeat", "backlight", "dictation",
                     "input sources"],
        "mouse": ["pointer", "cursor", "scrolling", "click"],
        "trackpad": ["touchpad", "gestures", "tap to click", "scrolling"],
        "game-controllers": ["controller", "gamepad", "joystick"]
    })

    // Row-level synonyms for the reference labels that users describe with a
    // different word. Keyed by `<paneId>/<rowId>`; map rows without an entry
    // still index their label, section, owner, key, and control.
    readonly property var rowSynonyms: ({
        "appearance/scheme": ["theme", "dark mode", "light mode", "auto",
                              "color scheme"],
        "appearance/accent": ["highlight color", "tint", "color"],
        "appearance/transparency": ["attached menu transparency", "glass",
                                    "opacity"],
        "desktop-dock/size": ["dock size", "icon size", "small", "large"],
        "desktop-dock/magnification": ["zoom", "magnify"],
        "desktop-dock/position": ["bottom", "left", "right", "dock location"],
        "desktop-dock/autohide": ["auto hide", "hide dock"],
        "desktop-dock/minimizeReaction": ["bounce", "minimize animation"],
        "notifications/previews": ["show previews", "notification previews",
                                   "always", "when unlocked"],
        "notifications/sleeping": ["display asleep", "sleep"],
        "notifications/locked": ["screen locked", "lock screen"],
        "notifications/sharing": ["mirroring", "screen sharing"],
        "sound/alertSound": ["alert", "notification sound"],
        "sound/alertVolume": ["volume", "loudness"],
        "displays/scale": ["resolution", "scaling", "larger text", "more space"],
        "displays/rotation": ["rotate", "orientation"],
        "keyboard/repeatRate": ["key repeat", "repeat speed"],
        "keyboard/repeatDelay": ["repeat delay"],
        "keyboard/navigation": ["tab", "focus", "keyboard navigation"],
        "mouse/speed": ["pointer speed", "cursor speed"],
        "trackpad/tap": ["tap to click", "click"],
        "trackpad/scrollZoom": ["natural scrolling", "pinch zoom"],
        "general/updates.check": ["check for updates", "software update"],
        "general/updates.install": ["install updates", "software update"],
        "general/language.region": ["language", "region", "locale"],
        "general/time.zone": ["time zone", "clock"],
        "privacy/location": ["location services", "geolocation"],
        "privacy/camera": ["webcam", "camera access"],
        "privacy/microphone": ["mic", "microphone access"],
        "lock-screen/requirePassword": ["require password", "lock delay"],
        "users-groups/users": ["user accounts", "accounts"],
        "wallpaper/abstract": ["abstract", "pattern", "texture"],
        "wallpaper/nature": ["nature", "landscape", "landscapes", "wildlife"],
        "wallpaper/water": ["water", "ocean", "sea", "wave"],
        "wallpaper/cityscapes": ["city", "cityscape", "skyline", "urban"],
        "wallpaper/earth": ["earth", "planet", "satellite"],
        "wallpaper/space": ["space", "nasa", "nebula", "galaxy", "stars",
                            "astronomy"],
        "wallpaper/scenery": ["scenery", "mountains", "landscape"],
        "wallpaper/underwater": ["underwater", "ocean", "reef", "diving"],
        "wallpaper/photos": ["photo", "add photo", "custom", "your photos"],
        "wallpaper/builtin": ["default", "gradient", "colors", "built-in"],
        "wallpaper/fit": ["fill", "fit", "stretch", "center"],
        "wallpaper/showAll": ["all workspaces", "show on all"]
    })

    // Built from the map/catalog and rebuilt when the optional Online Accounts
    // pane appears or disappears (T228); everything else is static.
    property var entries: []

    Component.onCompleted: root.entries = root.buildIndex()

    property Connections onlineAccountsConnection: Connections {
        target: Settings
        function onOnlineAccountsChanged() {
            root.entries = root.buildIndex();
        }
    }

    function keySynonyms(key) {
        if (!key || key === "omit")
            return [];
        return String(key).split(/[^A-Za-z0-9]+/).filter(function(token) {
            return token.length > 1;
        });
    }

    function controlSynonyms(control) {
        if (!control)
            return [];
        return String(control).split(/[^A-Za-z0-9]+/).filter(function(token) {
            return token.length > 1;
        });
    }

    function rowSynonymsFor(paneId, rowId) {
        var list = root.rowSynonyms[paneId + "/" + rowId];
        return list !== undefined ? list : [];
    }

    function synonymsFor(pane, row) {
        var out = [];
        var paneList = root.paneSynonyms[pane.id] || [];
        for (var p = 0; p < paneList.length; ++p)
            out.push(paneList[p]);
        var rowList = root.rowSynonymsFor(pane.id, row.id);
        for (var r = 0; r < rowList.length; ++r)
            out.push(rowList[r]);
        return out;
    }

    // Technical terms (the owner's key/API and control kind) are searchable but
    // weighted below the user's words, so a row key like `wallpaper.source`
    // does not outrank the Wallpaper pane for the query "wallpaper".
    function termsFor(row) {
        var out = [];
        var keyList = root.keySynonyms(row.key);
        for (var k = 0; k < keyList.length; ++k)
            out.push(keyList[k]);
        var controlList = root.controlSynonyms(row.control);
        for (var c = 0; c < controlList.length; ++c)
            out.push(controlList[c]);
        return out;
    }

    function helpFor(pane, row) {
        var parts = [];
        if (row.owner && row.owner !== "omit")
            parts.push(String(row.owner));
        if (row.key && row.key !== "omit")
            parts.push(String(row.key));
        if (row.absence && row.absence !== "omitted")
            parts.push(String(row.absence).replace(/-/g, " "));
        if (pane.absenceNote)
            parts.push(String(pane.absenceNote));
        return parts.join(" ");
    }

    function buildIndex() {
        var out = [];
        for (var i = 0; i < SettingsMap.panes.length; ++i) {
            var pane = SettingsMap.panes[i];
            if (!pane.shipped)
                continue;
            // An optional pane whose host is absent is not in the sidebar, so
            // it must not be a search result either (T228).
            if (SettingsPanes.indexOf(pane.id) < 0)
                continue;
            var catalog = SettingsPanes.paneById(pane.id);
            var paneHelp = [pane.owner, pane.absenceNote].join(" ");
            var paneSynonyms = root.paneSynonyms[pane.id] || [];
            out.push({
                kind: "pane",
                paneId: pane.id,
                paneTitle: pane.title,
                label: pane.title,
                section: "",
                targetRow: "",
                deepLink: pane.deepLink,
                icon: pane.icon,
                subtitle: catalog && catalog.description.length > 0
                          ? catalog.description : pane.absenceNote,
                synonyms: paneSynonyms,
                help: paneHelp
            });

            var rows = SettingsMap.visibleRowsFor(pane.id);
            var seenSections = {};
            for (var r = 0; r < rows.length; ++r) {
                var row = rows[r];
                if (seenSections[row.section] !== true) {
                    seenSections[row.section] = true;
                    out.push({
                        kind: "section",
                        paneId: pane.id,
                        paneTitle: pane.title,
                        label: row.section,
                        section: row.section,
                        targetRow: row.id,
                        deepLink: SettingsMap.rowDeepLink(pane.id, row.id),
                        icon: pane.icon,
                        subtitle: pane.title,
                        synonyms: paneSynonyms,
                        help: paneHelp + " " + row.section
                    });
                }
                out.push({
                    kind: "row",
                    paneId: pane.id,
                    paneTitle: pane.title,
                    label: row.label,
                    section: row.section,
                    targetRow: row.id,
                    deepLink: SettingsMap.rowDeepLink(pane.id, row.id),
                    icon: pane.icon,
                    subtitle: qsTr("%1 \u203a %2").arg(pane.title).arg(row.section),
                    synonyms: root.synonymsFor(pane, row),
                    terms: root.termsFor(row),
                    help: root.helpFor(pane, row)
                });
            }
        }
        return out;
    }

    // Lowercase, strip combining accents, and collapse punctuation/whitespace
    // to one token separator. Keeps non-Latin letters so a localized query can
    // still match a localized label.
    function normalize(text) {
        if (text === undefined || text === null)
            return "";
        var value = String(text).toLowerCase();
        try {
            value = value.normalize("NFD").replace(/[\u0300-\u036f]/g, "");
        } catch (error) {
            // Older JS engines have no String.normalize; the accent-stripping
            // pass is an enhancement, not a requirement.
        }
        return value.replace(/[\s\-_\/.,:;!?()\[\]{}'"\u2018\u2019\u201c\u201d]+/g, " ")
                    .trim();
    }

    function fieldScore(text, query, tokens) {
        var value = root.normalize(text);
        if (value.length === 0)
            return 0;
        var score = 0;
        if (value === query)
            score = 100;
        else if (value.indexOf(query) === 0)
            score = 80;
        else if (value.indexOf(" " + query) >= 0)
            score = 60;
        else if (value.indexOf(query) >= 0)
            score = 40;
        var hits = 0;
        for (var i = 0; i < tokens.length; ++i) {
            if (value.indexOf(tokens[i]) >= 0)
                hits += 1;
        }
        if (tokens.length > 0 && hits === tokens.length)
            score = Math.max(score, 30 + hits * 5);
        return score;
    }

    function scoreEntry(entry, query, tokens) {
        var label = root.fieldScore(entry.label, query, tokens);
        var synonym = 0;
        var synonyms = entry.synonyms || [];
        for (var i = 0; i < synonyms.length; ++i)
            synonym = Math.max(synonym, root.fieldScore(synonyms[i], query, tokens));
        // A pane/section entry's label is already the pane/section name; only
        // row entries need the contextual section/pane/subtitle fields, or the
        // same word would count three times and outrank an exact pane title.
        var rowEntry = entry.kind === "row";
        var section = rowEntry ? root.fieldScore(entry.section, query, tokens) : 0;
        var pane = rowEntry ? root.fieldScore(entry.paneTitle, query, tokens) : 0;
        var help = root.fieldScore(entry.help, query, tokens);
        var subtitle = rowEntry ? root.fieldScore(entry.subtitle, query, tokens) : 0;
        var terms = 0;
        var termList = entry.terms || [];
        for (var t = 0; t < termList.length; ++t)
            terms = Math.max(terms, root.fieldScore(termList[t], query, tokens));
        var score = label * 10 + synonym * 7 + section * 4 + pane * 3
                    + help * 1.5 + subtitle * 2 + terms * 2;
        if (label === 100)
            score += 500;
        else if (label === 80)
            score += 150;
        if (score <= 0)
            return 0;
        if (entry.kind === "pane")
            score += 30;
        else if (entry.kind === "section")
            score += 20;
        else
            score += 10;
        return score;
    }

    function copyEntry(entry, score) {
        var result = {};
        for (var key in entry)
            result[key] = entry[key];
        result.score = score;
        return result;
    }

    function isPaneNameMatch(entry, query) {
        if (entry.kind !== "pane")
            return false;
        var label = root.normalize(entry.label);
        return label === query || label.indexOf(query) === 0;
    }

    // The one search entry point. `query` is a user string; `limit` is an
    // optional cap (defaults to `resultLimit`). Returns a new array sorted by
    // descending score.
    function search(query, limit) {
        var normalized = root.normalize(query);
        if (normalized.length === 0)
            return [];
        var tokens = normalized.split(" ").filter(function(token) {
            return token.length > 0;
        });
        var scored = [];
        for (var i = 0; i < root.entries.length; ++i) {
            var entry = root.entries[i];
            var score = root.scoreEntry(entry, normalized, tokens);
            if (score > 0)
                scored.push({ entry: entry, score: score });
        }
        scored.sort(function(a, b) {
            // A pane whose own name matches the query is the strongest result:
            // "display" opens Displays before a row that happens to be called
            // Display, and "battery" opens Battery before the menu-bar row.
            var aPane = root.isPaneNameMatch(a.entry, normalized) ? 1 : 0;
            var bPane = root.isPaneNameMatch(b.entry, normalized) ? 1 : 0;
            if (aPane !== bPane)
                return bPane - aPane;
            if (b.score !== a.score)
                return b.score - a.score;
            var order = { pane: 0, section: 1, row: 2 };
            if (order[a.entry.kind] !== order[b.entry.kind])
                return order[a.entry.kind] - order[b.entry.kind];
            return String(a.entry.label).localeCompare(String(b.entry.label));
        });
        var count = limit !== undefined ? Math.max(0, limit) : root.resultLimit;
        var out = [];
        for (var s = 0; s < scored.length && s < count; ++s)
            out.push(root.copyEntry(scored[s].entry, scored[s].score));
        return out;
    }

    // Convenience for the debug view/tests: the first result of each kind.
    function firstOfKind(query, kind) {
        var results = root.search(query);
        for (var i = 0; i < results.length; ++i) {
            if (results[i].kind === kind)
                return results[i];
        }
        return null;
    }
}
