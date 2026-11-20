// SPDX-License-Identifier: MIT
import QtQuick

// The local Settings search index (T223, rebuilt for T265).
//
// One in-memory index built once from the executable map (`SettingsMap.qml`)
// and the pane catalog (`SettingsPanes.qml`). It contains pane titles, pane
// ids, every map section, and every shipped row, plus synonyms and the row's
// owner/key/absence help text. Search is a pure, synchronous function over that
// array: no bus, no daemon, no per-keystroke I/O.
//
// A result is only returned when it can deep-link: pane results use
// `settings://pane/<id>`, section results point at their first row, and row
// results use `settings://pane/<id>/row/<rowId>`. Unshipped panes/rows are not
// indexed at all (the no-half-panes rule); a row whose surface the resolver
// cannot open is omitted (`SettingsMap.isRowLandable`).
//
// Ranking is field-weighted: exact label > label prefix > label word start >
// label substring, then synonyms, pane description/section, and finally the
// technical help text (owner, key, absence kind). Technical-only matches are
// always sorted below any user-word match, so an owner query still works but
// never outranks a setting's own name.
//
// Results are grouped by pane. Each pane group shows at most `paneLimit`
// entries; a group with more matches ends with a `showAll` row that expands
// it. `search()` returns that flat, grouped model (pane entries first, group
// headers marked by `groupStart`).
pragma Singleton

QtObject {
    id: root

    // The most result rows shown at once (groups are never cut in half).
    readonly property int resultLimit: 60
    // How many entries one pane group shows before its "Show all" row.
    readonly property int paneLimit: 4

    // Pane-level synonyms. These are the user's words, not a second catalog:
    // the pane title/id remain the only labels shown in the UI. Lowercase
    // aliases are matched, never rendered.
    readonly property var paneSynonyms: ({
        "wifi": ["wifi", "wi fi", "wireless", "wlan", "internet", "network",
                 "hotspot", "hidden network", "ask to join", "scan"],
        "bluetooth": ["wireless devices", "pair", "pairing", "devices",
                      "bluetooth", "headphones", "speaker", "discoverable"],
        "network": ["vpn", "firewall", "ethernet", "internet", "connections",
                    "service order", "other services", "more"],
        "vpn": ["vpn", "virtual private network", "tunnel", "wireguard",
                "openvpn"],
        "battery": ["power", "energy", "charge", "low power mode",
                    "battery health", "battery level", "usage history", "history",
                    "options", "adapter"],
        "general": ["about", "software update", "storage", "language", "region",
                    "date", "time", "date and time", "sharing", "backup", "backups",
                    "restic", "pika", "timeshift", "keyring", "reset",
                    "startup disk", "passwords", "passwords & keys", "keys",
                    "login items", "autofill", "auto fill", "time machine"],
        "storage": ["disk", "drive", "volume", "space", "disk usage"],
        "appearance": ["theme", "dark mode", "light mode", "accent", "colors",
                       "style", "transparency", "scroll bars", "sidebar icon size",
                       "wallpaper tint"],
        "accessibility": ["a11y", "screen reader", "contrast", "motion", "vision",
                          "hearing", "motor", "zoom", "magnifier", "speech",
                          "read and speak", "physical", "physical and motor",
                          "pointer control", "accessibility shortcut", "hotkey",
                          "key binding", "accessibility"],
        "menu-bar": ["menu bar controls", "control center", "quick settings",
                     "controls", "tiles", "modules", "status items", "clock",
                     "menu extras", "menu bar items"],
        "search": ["spotlight", "index", "indexing", "web search", "shortcut"],
        "privacy": ["security", "permissions", "camera", "microphone", "location",
                    "files", "screen recording"],
        "desktop-dock": ["dock", "desktop", "stage", "workspace focus",
                         "hot corners", "windows", "widgets", "browser",
                         "workspace overview", "workspace", "workspaces",
                         "mission control", "overview", "spaces"],
        "displays": ["monitor", "screen", "resolution", "scale", "rotation",
                     "brightness", "night light", "refresh rate", "color",
                     "projector", "tv", "preset", "ambient color", "scaling"],
        "wallpaper": ["background", "desktop picture", "photo", "image",
                      "featured", "abstract", "nature", "water", "cityscape",
                      "earth", "space", "nasa", "scenery", "underwater"],
        "screen-saver": ["screensaver", "screen saver", "idle", "clock"],
        "notifications": ["alerts", "banners", "previews", "badges", "sounds"],
        "sound": ["audio", "volume", "speaker", "microphone", "output", "input",
                  "alert sound"],
        "focus": ["do not disturb", "dnd", "notifications", "modes"],
        "lock-screen": ["lock", "password", "login", "sleep", "screen saver"],
        "users-groups": ["users", "accounts", "groups", "login", "administrator",
                         "touch id", "touchid", "fingerprint", "biometric",
                         "biometrics"],
        "online-accounts": ["internet accounts", "internet account", "accounts",
                            "sync", "cloud", "email"],
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
        "appearance/scheme": ["theme", "dark mode", "light mode", "dark",
                              "light", "auto", "color scheme"],
        "appearance/accent": ["highlight color", "tint", "color"],
        "appearance/highlight": ["text highlight", "selection color",
                                 "selection wash", "highlight color"],
        "appearance/iconStyle": ["icon style", "widget style",
                                 "icon set", "app icons"],
        "appearance/folderColor": ["folder color", "folder tint",
                                   "folder icon color", "folder icon"],
        "appearance/transparency": ["glass tint", "attached menu transparency",
                                    "opacity"],
        "appearance/sidebarIconSize": ["sidebar icon size", "sidebar icons",
                                       "sidebar icon size popup", "icon size"],
        "appearance/wallpaperTint": ["wallpaper tint", "wallpaper tinting",
                                     "tint windows", "window background color",
                                     "tint window background"],
        "appearance/scrollBars": ["scroll bars", "scrollbar", "scroll bar visibility",
                                  "always show scroll bars", "when scrolling",
                                  "automatically based on mouse or trackpad"],
        "appearance/scrollBarClick": ["scroll bar click", "click in the scroll bar",
                                      "jump to the next page",
                                      "jump to the spot that's clicked",
                                      "jump to spot"],
        // T290f: the Advanced Appearance sheet's six Linux-only rows.
        "appearance/iconTheme": ["icon theme", "installed icon theme",
                                 "gtk icon theme", "advanced appearance"],
        "appearance/cursorTheme": ["cursor theme", "pointer theme",
                                   "mouse cursor theme", "advanced appearance"],
        "appearance/cursorSize": ["cursor size", "pointer size",
                                  "mouse cursor size", "advanced appearance"],
        "appearance/windowControls": ["window controls", "traffic lights",
                                      "window buttons", "close minimize zoom",
                                      "advanced appearance"],
        "appearance/material": ["material effects", "glass effects",
                                "transparency effects", "graphics detail",
                                "advanced appearance"],
        "appearance/accentFromWallpaper": ["accent from wallpaper",
                                           "wallpaper accent",
                                           "dynamic accent",
                                           "advanced appearance"],
        // T292a: every rendered Desktop & Dock row, including the two advanced
        // sheets and the temporary Workspace Overview entry.
        "desktop-dock/size": ["dock size", "icon size", "small", "large"],
        "desktop-dock/magnification": ["zoom", "magnify", "magnification size",
                                       "dock magnification"],
        "desktop-dock/position": ["bottom", "left", "right", "dock location",
                                  "dock position"],
        "desktop-dock/minimizeAnimation": ["minimize animation", "genie",
                                           "scale effect", "minimize effect"],
        "desktop-dock/titlebarDoubleClick": ["double click", "maximize",
                                             "title bar", "titlebar"],
        "desktop-dock/minimizeIntoTileIcon": ["minimize into app icon",
                                              "minimize to icon"],
        "desktop-dock/autohide": ["auto hide", "hide dock"],
        "desktop-dock/animateOpening": ["bounce", "animate opening", "launch bounce"],
        "desktop-dock/showIndicators": ["running indicator", "dot", "indicators"],
        "desktop-dock/showRecentApps": ["recent apps", "suggested apps"],
        "desktop-dock/chooserOnHover": ["window chooser", "hover chooser",
                                        "advanced dock"],
        "desktop-dock/minimizeReaction": ["bounce", "minimize reaction",
                                          "advanced dock"],
        "desktop-dock/desktopItems": ["desktop icons", "show items", "show on desktop"],
        "desktop-dock/clickWallpaper": ["click wallpaper", "show desktop",
                                        "reveal desktop"],
        "desktop-dock/stage": ["workspace focus", "stage manager", "stage"],
        "desktop-dock/stageRecent": ["recent apps in workspace focus", "stage recents"],
        "desktop-dock/stageWindowMode": ["windows from an application", "all at once",
                                         "one at a time"],
        "desktop-dock/showWidgets": ["desktop widgets", "widgets"],
        "desktop-dock/dimWidgets": ["dim widgets"],
        "desktop-dock/defaultBrowser": ["web browser", "browser", "default apps",
                                        "html"],
        "notifications/previews": ["show previews", "notification previews",
                                   "always", "when unlocked"],
        "notifications/sleeping": ["display asleep", "sleep"],
        "notifications/locked": ["screen locked", "lock screen"],
        "notifications/sharing": ["mirroring", "screen sharing"],
        "sound/alertSound": ["alert", "notification sound"],
        "sound/alertVolume": ["volume", "loudness"],
        "displays/scale": ["resolution", "scaling", "larger text", "more space"],
        "displays/showAllResolutions": ["show all resolutions", "all sizes",
                                        "resolution list", "scale list"],
        "displays/display": ["display", "built-in display", "monitor", "output",
                             "per output", "displays"],
        "displays/perOutput": ["per output", "displays", "select display"],
        "displays/brightness": ["brightness", "dim", "screen brightness"],
        "displays/autoBrightness": ["automatic brightness", "ambient light sensor"],
        "displays/adaptiveColor": ["ambient color", "true tone", "adaptive color"],
        "displays/colorProfile": ["preset", "color preset", "color profile",
                                  "colord"],
        "displays/refreshRate": ["refresh rate", "hz", "adaptive refresh"],
        "displays/tvMode": ["projector", "projector mode", "tv", "television",
                            "mirror", "extend"],
        "displays/nightLightSheet": ["night light", "night shift", "warm color"],
        "displays/advanced": ["advanced", "hdr", "vrr", "overscan", "arrangement",
                              "rotation", "color profile", "use as"],
        "displays/rotation": ["rotate", "orientation"],
        // T292a: the Desktop & Dock Windows rows, the Workspace Overview entry,
        // and the footer actions.
        "desktop-dock/preferTabs": ["tabs", "tabbed documents"],
        "desktop-dock/askToKeepChanges": ["keep changes", "close documents"],
        "desktop-dock/closeOnQuit": ["close windows on quit", "quit"],
        "desktop-dock/dragToTile": ["tile", "snap windows", "window tiling"],
        "desktop-dock/dragToFill": ["fill screen", "maximize by drag"],
        "desktop-dock/tileModifier": ["hold super", "hold option", "tile modifier"],
        "desktop-dock/tileMargins": ["tile gaps", "tiled margins"],
        "desktop-dock/windowRules": ["window rules", "placement rules",
                                     "advanced windows"],
        "desktop-dock/overview": ["workspace overview", "mission control",
                                  "spaces"],
        "desktop-dock/hotCorners": ["hot corners", "corner actions"],
        "desktop-dock/shortcuts": ["shortcut editor"],
        "keyboard/repeatRate": ["key repeat", "repeat speed"],
        "keyboard/repeatDelay": ["repeat delay"],
        "keyboard/navigation": ["tab", "focus", "keyboard navigation"],
        "mouse/speed": ["pointer speed", "cursor speed"],
        "trackpad/tap": ["tap to click", "click"],
        "trackpad/scrollZoom": ["natural scrolling", "pinch zoom"],
        // T288: the connectivity panes' rendered rows.
        "wifi/power": ["turn on wifi", "wifi on", "radio"],
        "wifi/current": ["current network", "connected network", "ssid"],
        "wifi/saved": ["known networks", "saved networks", "auto join"],
        "wifi/available": ["other networks", "nearby networks", "join network"],
        "wifi/scan": ["scan", "refresh", "find networks"],
        "wifi/hidden": ["hidden network", "join other", "other network", "other"],
        "wifi/ask": ["ask to join", "prompt", "notify", "auto join"],
        "wifi/askHotspots": ["ask to join hotspots", "hotspot prompt"],
        "wifi/hotspot": ["personal hotspot", "share internet", "access point"],
        "wifi/advanced": ["advanced", "tcp ip", "dns", "proxy", "mac address",
                          "802.1x"],
        "bluetooth/power": ["turn on bluetooth", "bluetooth on"],
        "bluetooth/discoverable": ["discoverable", "computer name", "visibility"],
        "bluetooth/paired": ["my devices", "paired devices", "headphones",
                             "keyboard", "mouse"],
        "bluetooth/nearby": ["nearby devices", "searching", "scan", "pair"],
        "bluetooth/advanced": ["advanced", "adapter address", "pairable"],
        "network/wifi": ["wifi", "wireless"],
        "network/firewall": ["firewall", "ufw", "firewalld", "nftables"],
        "network/services": ["other services", "ethernet", "bridge", "vlan",
                             "bond"],
        "network/more": ["more", "add service", "set service order"],
        "network/serviceOrder": ["service order", "priority", "reorder",
                                 "move up", "move down"],
        "network/addService": ["add service", "ethernet", "bridge", "bond",
                               "vlan"],
        "network/vpn": ["vpn", "tunnel", "wireguard"],
        "vpn/vpn": ["vpn connections", "tunnel", "wireguard"],
        "vpn/vpn.wireguard": ["wireguard", "native vpn"],
        "vpn/vpn.add": ["add vpn", "new vpn", "wireguard", "openvpn"],
        "battery/profile": ["low power mode", "power mode", "power profile",
                            "power saver", "balanced", "performance"],
        "battery/health": ["battery health", "capacity", "cycles"],
        "battery/status": ["battery status", "charging", "fully charged",
                           "on battery", "time remaining"],
        "battery/usage": ["history range", "last 24 hours", "last 10 days",
                          "usage range"],
        "battery/level": ["battery level", "charge chart", "history chart"],
        "battery/options": ["options", "battery options", "power options"],
        "battery/options.showPercentage": ["battery percentage", "menu bar",
                                           "percentage"],
        "battery/options.dimOnBattery": ["dim on battery", "brightness"],
        "battery/options.preventSleep": ["prevent sleep", "suspend"],
        "battery/adapter.state": ["power adapter", "power source", "plugged in"],
        "battery/powerSettings": ["power settings", "display off", "lock screen"],
        "general/updates.check": ["check for updates", "software update"],
        "general/updates.install": ["install updates", "software update"],
        "general/updates.alsoAvailable": ["also available", "additional updates",
                                          "other updates"],
        "general/language.region": ["language", "region", "locale"],
        "general/language.languages": ["preferred languages", "language",
                                       "languages"],
        "general/time.zone": ["time zone", "clock"],
        "general/time.city": ["time zone city", "city", "clock"],
        "general/time.auto": ["automatic date and time", "network time", "ntp"],
        "general/passwords.keyring": ["keyring", "system keyring", "secret service",
                                      "passwords", "keys"],
        "general/passwords.source": ["password provider", "keyring", "passwords"],
        "general/backup.backend": ["restic", "pika", "timeshift", "backup backend"],
        "general/backup.add": ["add backup disk", "backup destination", "restic",
                               "timeshift"],
        "general/backup.auto": ["time machine", "back up automatically"],
        "general/backup.now": ["time machine", "back up now"],
        "general/backup.restore": ["time machine", "restore backup"],
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
        "wallpaper/showAll": ["all workspaces", "show on all"],
        // T291a: the Vision sub-pane rows. The omitted rows are not indexed,
        // so only shipped rows carry synonyms.
        "accessibility/vision.screenReader": ["voiceover", "screen reader",
                                              "orca", "assistive technology",
                                              "blind", "low vision"],
        "accessibility/vision.bridge": ["at-spi", "accessibility bridge",
                                        "toolkit bridge", "a11y"],
        "accessibility/vision.magnifier": ["zoom", "magnifier",
                                           "screen magnification", "enlarge"],
        "accessibility/vision.magnifier.zoom": ["zoom level", "magnification",
                                                "zoom factor", "magnify"],
        "accessibility/vision.magnifier.mode": ["follow focus", "follow caret",
                                                "zoom mode", "zoom follows"],
        "accessibility/vision.display.increaseContrast": ["contrast",
                                                          "high contrast"],
        "accessibility/vision.display.reduceTransparency": ["transparency",
                                                            "reduce transparency",
                                                            "opaque", "glass"],
        "accessibility/vision.display.differentiateWithoutColor": [
            "color blind", "colour blind", "differentiate without color",
            "non-color cue", "without color"
        ],
        "accessibility/vision.display.textSize": ["text size", "font size",
                                                  "type size", "larger text"],
        "accessibility/vision.display.pointerSize": ["pointer size",
                                                     "cursor size",
                                                     "mouse cursor size"],
        "accessibility/vision.motion.reduceMotion": ["reduce motion",
                                                     "animation", "motion",
                                                     "reduce animations"],
        "accessibility/vision.motion.dimFlashingLights": ["flashing",
                                                          "flashing lights",
                                                          "dim flashing",
                                                          "photosensitive"],
        "accessibility/vision.readSpeak": ["read and speak", "speech",
                                           "spd-say", "espeak",
                                           "text to speech", "tts"],
        // T291b: the Physical and Motor sub-pane rows. Voice Control, Sticky
        // Keys, Slow Keys, Mouse Keys, Ignore built-in trackpad, and Switch
        // Control are omitted rows, so they carry no synonyms.
        "accessibility/motor.keyboard.fullKeyboardAccess": [
            "full keyboard access", "keyboard navigation", "tab navigation",
            "tab key", "focus between controls"
        ],
        "accessibility/motor.keyboard.onScreenKeyboard": [
            "on-screen keyboard", "onscreen keyboard", "virtual keyboard",
            "onboard", "wvkbd"
        ],
        "accessibility/motor.pointer.doubleClickSpeed": [
            "double click", "double-click", "double click speed",
            "click speed", "pointer control"
        ],
        // T291c: the Hearing sub-pane. Audio is the only rendered group, so
        // only Balance carries synonyms; the omitted hearing rows stay out of
        // the row index.
        "accessibility/hearing.balance": [
            "balance", "audio balance", "stereo balance", "left right",
            "hearing", "audio"
        ],
        // T291d: the Speech sub-pane. Live Speech is the one rendered group;
        // Custom Voice and Vocal Shortcuts are omitted rows and carry no
        // synonyms.
        "accessibility/speech.liveSpeech": [
            "speech", "live speech", "speak sample", "text to speech", "tts",
            "speech dispatcher", "spd-say", "espeak"
        ],
        // T291e: the General sub-pane. The one shipped row is the recorded
        // accessibility shortcut; it carries the collision words (shortcut,
        // hotkey, key binding) so those queries reach it.
        "accessibility/shortcut.recorder": [
            "accessibility shortcut", "shortcut", "shortcuts", "hotkey",
            "hot key", "key binding", "keybinding", "key chord", "record",
            "recorder", "general"
        ]
    })

    // Built once from the map/catalog; rebuilt only when the optional Online
    // Accounts pane appears or disappears (T228). Every normalized/compact
    // field is precomputed here, so a keystroke never re-normalizes the index.
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

    // The pane id is a searchable alias too: `menu-bar` finds Menu Bar and
    // `desktop-dock` finds Desktop & Dock. T285 moved Quick Settings and
    // Overview into those panes; the old names are pane synonyms now.
    function idTerms(paneId) {
        var words = String(paneId).split(/[-_]+/).filter(function(word) {
            return word.length > 0;
        });
        return [words.join(" "), String(paneId)];
    }

    function rowSynonymsFor(paneId, rowId) {
        var list = root.rowSynonyms[paneId + "/" + rowId];
        return list !== undefined ? list : [];
    }

    // Rows carry only their own synonyms. A pane synonym is the pane entry's
    // alias: it must not make every row in the pane a match, or "time machine"
    // would flood the General group with 48 rows and hide the Backup rows.
    function synonymsFor(pane, row) {
        var out = [];
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

    // The hyphenless form, so `wi-fi`, `wifi`, and `Wi Fi` are the same word.
    function compact(text) {
        return root.normalize(text).replace(/\s+/g, "");
    }

    function compactTokens(tokens) {
        var out = [];
        for (var i = 0; i < tokens.length; ++i) {
            var token = root.compact(tokens[i]);
            if (token.length > 0)
                out.push(token);
        }
        return out;
    }

    // Precompute every field search reads. `entry` is a plain object; the
    // normalized copies live next to the display text and are never shown.
    function prepare(entry) {
        entry.nLabel = root.normalize(entry.label);
        entry.cLabel = root.compact(entry.label);
        entry.nSubtitle = root.normalize(entry.subtitle);
        entry.cSubtitle = root.compact(entry.subtitle);
        entry.nSection = root.normalize(entry.section);
        entry.cSection = root.compact(entry.section);
        entry.nPane = root.normalize(entry.paneTitle);
        entry.cPane = root.compact(entry.paneTitle);
        entry.nHelp = root.normalize(entry.help);
        entry.cHelp = root.compact(entry.help);
        entry.nSynonyms = [];
        entry.cSynonyms = [];
        for (var i = 0; i < (entry.synonyms || []).length; ++i) {
            entry.nSynonyms.push(root.normalize(entry.synonyms[i]));
            entry.cSynonyms.push(root.compact(entry.synonyms[i]));
        }
        entry.nTerms = [];
        entry.cTerms = [];
        for (var t = 0; t < (entry.terms || []).length; ++t) {
            entry.nTerms.push(root.normalize(entry.terms[t]));
            entry.cTerms.push(root.compact(entry.terms[t]));
        }
        return entry;
    }

    function buildIndex() {
        var out = [];
        for (var i = 0; i < SettingsMap.panes.length; ++i) {
            var pane = SettingsMap.panes[i];
            if (!pane.shipped)
                continue;
            // A pane whose host is absent is not in the sidebar, so it must
            // not be a search result either (T228). T263 resolves aliases, so
            // General -> Storage rows stay searchable.
            if (!SettingsPanes.isVisible(pane.id))
                continue;
            var catalog = SettingsPanes.paneById(pane.id);
            var description = catalog && catalog.description.length > 0
                    ? String(catalog.description) : "";
            // T265: a truthful subtitle is the pane description (or its
            // sidebar group); the absence note is help text only, never the
            // subtitle of an available pane.
            var paneSubtitle = description.length > 0
                    ? description : SettingsPanes.groupAccessibleTitle(pane.group);
            var paneHelp = [pane.owner,
                            String(pane.absence || "").replace(/-/g, " "),
                            pane.absenceNote].join(" ");
            var paneSynonyms = root.paneSynonyms[pane.id] || [];
            paneSynonyms = paneSynonyms.concat(root.idTerms(pane.id));
            out.push(root.prepare({
                kind: "pane",
                paneId: pane.id,
                paneTitle: pane.title,
                label: pane.title,
                section: "",
                targetRow: "",
                deepLink: pane.deepLink,
                icon: pane.icon,
                subtitle: paneSubtitle,
                synonyms: paneSynonyms,
                terms: [],
                help: paneHelp
            }));

            var rows = SettingsMap.visibleRowsFor(pane.id).filter(function(entry) {
                // A row the resolver cannot land (a closed surface it cannot
                // open) is not a search result either.
                return SettingsMap.isRowLandable(pane.id, entry.id);
            });
            var seenSections = {};
            for (var r = 0; r < rows.length; ++r) {
                var row = rows[r];
                if (seenSections[row.section] !== true) {
                    seenSections[row.section] = true;
                    out.push(root.prepare({
                        kind: "section",
                        paneId: pane.id,
                        paneTitle: pane.title,
                        label: row.section,
                        section: row.section,
                        targetRow: row.id,
                        deepLink: SettingsMap.rowDeepLink(pane.id, row.id),
                        icon: pane.icon,
                        subtitle: paneSubtitle,
                        synonyms: [],
                        terms: [],
                        help: paneHelp + " " + row.section
                    }));
                }
                out.push(root.prepare({
                    kind: "row",
                    paneId: pane.id,
                    paneTitle: pane.title,
                    label: row.label,
                    section: row.section,
                    targetRow: row.id,
                    deepLink: SettingsMap.rowDeepLink(pane.id, row.id),
                    icon: pane.icon,
                    // The pane is the group header, so the row's own section is
                    // the truthful second line. Rows carry no separate
                    // description in the map.
                    subtitle: row.section,
                    synonyms: root.synonymsFor(pane, row),
                    terms: root.termsFor(row),
                    help: root.helpFor(pane, row)
                }));
            }
        }
        return out;
    }

    // Score one precomputed field against the normalized query. `compact`
    // catches the hyphen/space variants (`wifi` == `Wi-Fi`); tokens keep a
    // multi-word query matching in any order.
    function scoreField(value, compactValue, query, queryCompact, tokens,
                        queryCompactTokens) {
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
        // Short compact queries ("e", "a") would match almost every field; only
        // apply the hyphenless comparison once it is a meaningful word.
        if (queryCompact.length >= 3 && compactValue.length > 0) {
            if (compactValue === queryCompact)
                score = Math.max(score, 95);
            else if (compactValue.indexOf(queryCompact) === 0)
                score = Math.max(score, 78);
            else if (compactValue.indexOf(queryCompact) >= 0)
                score = Math.max(score, 38);
        }
        var hits = 0;
        for (var i = 0; i < tokens.length; ++i) {
            if (value.indexOf(tokens[i]) >= 0)
                hits += 1;
        }
        if (tokens.length > 0 && hits === tokens.length)
            score = Math.max(score, 30 + hits * 5);
        var compactHits = 0;
        for (var c = 0; c < queryCompactTokens.length; ++c) {
            if (compactValue.indexOf(queryCompactTokens[c]) >= 0)
                compactHits += 1;
        }
        if (queryCompactTokens.length > 0 && compactHits === queryCompactTokens.length)
            score = Math.max(score, 28 + compactHits * 5);
        return score;
    }

    function bestField(values, compactValues, query, queryCompact, tokens,
                       queryCompactTokens) {
        var best = 0;
        for (var i = 0; i < values.length; ++i) {
            best = Math.max(best, root.scoreField(values[i], compactValues[i],
                                                  query, queryCompact, tokens,
                                                  queryCompactTokens));
        }
        return best;
    }

    function scoreEntry(entry, query, queryCompact, tokens, queryCompactTokens) {
        var label = root.scoreField(entry.nLabel, entry.cLabel, query,
                                    queryCompact, tokens, queryCompactTokens);
        var synonym = root.bestField(entry.nSynonyms, entry.cSynonyms, query,
                                     queryCompact, tokens, queryCompactTokens);
        // A pane/section entry's label is already the pane/section name; only
        // row entries need the contextual section/pane fields, or the same
        // word would count three times and outrank an exact pane title.
        var rowEntry = entry.kind === "row" || entry.kind === "section";
        var section = rowEntry
                ? root.scoreField(entry.nSection, entry.cSection, query,
                                  queryCompact, tokens, queryCompactTokens) : 0;
        var pane = rowEntry
                ? root.scoreField(entry.nPane, entry.cPane, query, queryCompact,
                                  tokens, queryCompactTokens) : 0;
        // T265: the pane description (and the row/section subtitle) is a real
        // search field, not decoration.
        var subtitle = root.scoreField(entry.nSubtitle, entry.cSubtitle, query,
                                       queryCompact, tokens, queryCompactTokens);
        var help = root.scoreField(entry.nHelp, entry.cHelp, query, queryCompact,
                                   tokens, queryCompactTokens);
        var terms = root.bestField(entry.nTerms, entry.cTerms, query,
                                   queryCompact, tokens, queryCompactTokens);

        var primary = label * 10 + synonym * 7 + section * 4 + pane * 3
                + subtitle * 2;
        // Technical-only terms (owner/key/absence/control) stay searchable but
        // are a separate, lower band. They never outrank a user-word match.
        var technical = help * 1 + terms * 0.5;
        var score = primary + technical;
        if (score <= 0)
            return { score: 0, primary: false };
        if (label === 100)
            score += 500;
        else if (label === 80)
            score += 150;
        if (entry.kind === "pane")
            score += 30;
        else if (entry.kind === "section")
            score += 20;
        else
            score += 10;
        return { score: score, primary: primary > 0 };
    }

    function isPaneNameMatch(entry, query, queryCompact) {
        if (entry.kind !== "pane")
            return false;
        if (entry.nLabel === query || entry.nLabel.indexOf(query) === 0)
            return true;
        if (queryCompact.length >= 3 && entry.cLabel.length > 0) {
            return entry.cLabel === queryCompact
                    || entry.cLabel.indexOf(queryCompact) === 0;
        }
        return false;
    }

    function copyEntry(entry, score) {
        var result = {};
        for (var key in entry)
            result[key] = entry[key];
        result.score = score;
        return result;
    }

    // Result count excluding the "Show all" affordances.
    function resultCount(results) {
        var count = 0;
        for (var i = 0; i < (results || []).length; ++i) {
            if (results[i].kind !== "showAll")
                count += 1;
        }
        return count;
    }

    function groupCount(results) {
        var count = 0;
        for (var i = 0; i < (results || []).length; ++i) {
            if (results[i].groupStart === true)
                count += 1;
        }
        return count;
    }

    function compareWithinGroup(a, b, query, queryCompact) {
        // A pane whose own name matches sorts first inside its group, then any
        // user-word match, then technical-only matches, then score.
        var aPane = root.isPaneNameMatch(a.entry, query, queryCompact) ? 1 : 0;
        var bPane = root.isPaneNameMatch(b.entry, query, queryCompact) ? 1 : 0;
        if (aPane !== bPane)
            return bPane - aPane;
        if (a.primary !== b.primary)
            return a.primary ? -1 : 1;
        if (b.score !== a.score)
            return b.score - a.score;
        var order = { pane: 0, section: 1, row: 2 };
        if (order[a.entry.kind] !== order[b.entry.kind])
            return order[a.entry.kind] - order[b.entry.kind];
        return String(a.entry.label).localeCompare(String(b.entry.label));
    }

    // The one search entry point. `query` is a user string; `options` may carry
    // `expanded` (pane ids whose "Show all" was activated), `perPane`, and
    // `limit`. Returns a flat, pane-grouped array sorted by descending score;
    // the first entry of each group carries `groupStart`, and a capped group
    // ends with a `showAll` entry.
    function search(query, options) {
        var normalized = root.normalize(query);
        if (normalized.length === 0)
            return [];
        var queryCompact = root.compact(normalized);
        var tokens = normalized.split(" ").filter(function(token) {
            return token.length > 0;
        });
        var compactTokenList = root.compactTokens(tokens);
        var opts = options || {};
        var expanded = opts.expanded || [];
        var perPane = opts.perPane !== undefined ? Math.max(1, opts.perPane)
                                                 : root.paneLimit;
        var limit = opts.limit !== undefined ? Math.max(1, opts.limit)
                                             : root.resultLimit;

        var buckets = {};
        var order = [];
        for (var i = 0; i < root.entries.length; ++i) {
            var entry = root.entries[i];
            var scored = root.scoreEntry(entry, normalized, queryCompact, tokens,
                                         compactTokenList);
            if (scored.score <= 0)
                continue;
            var bucket = buckets[entry.paneId];
            if (bucket === undefined) {
                bucket = {
                    paneId: entry.paneId,
                    paneTitle: entry.paneTitle,
                    icon: entry.icon,
                    entries: [],
                    best: 0,
                    bestPrimary: false,
                    paneNameMatch: false
                };
                buckets[entry.paneId] = bucket;
                order.push(bucket);
            }
            bucket.entries.push({
                entry: entry,
                score: scored.score,
                primary: scored.primary
            });
            bucket.best = Math.max(bucket.best, scored.score);
            bucket.bestPrimary = bucket.bestPrimary || scored.primary;
            if (root.isPaneNameMatch(entry, normalized, queryCompact))
                bucket.paneNameMatch = true;
        }

        order.sort(function(a, b) {
            if (a.paneNameMatch !== b.paneNameMatch)
                return a.paneNameMatch ? -1 : 1;
            if (a.bestPrimary !== b.bestPrimary)
                return a.bestPrimary ? -1 : 1;
            if (b.best !== a.best)
                return b.best - a.best;
            return String(a.paneTitle).localeCompare(String(b.paneTitle));
        });

        var out = [];
        for (var g = 0; g < order.length; ++g) {
            var group = order[g];
            group.entries.sort(function(a, b) {
                return root.compareWithinGroup(a, b, normalized, queryCompact);
            });
            var total = group.entries.length;
            var isExpanded = expanded.indexOf(group.paneId) >= 0;
            var take = isExpanded ? total : Math.min(total, perPane);
            var items = [];
            for (var e = 0; e < take; ++e) {
                var copy = root.copyEntry(group.entries[e].entry,
                                          group.entries[e].score);
                copy.primaryMatch = group.entries[e].primary;
                copy.technicalOnly = !group.entries[e].primary;
                copy.groupStart = e === 0;
                copy.groupTitle = group.paneTitle;
                copy.groupTotal = total;
                items.push(copy);
            }
            if (!isExpanded && total > perPane) {
                var hidden = total - perPane;
                items.push({
                    kind: "showAll",
                    paneId: group.paneId,
                    paneTitle: group.paneTitle,
                    label: qsTr("Show all %1 results in %2")
                            .arg(hidden).arg(group.paneTitle),
                    section: "",
                    targetRow: "",
                    deepLink: "",
                    icon: group.icon,
                    subtitle: qsTr("%1 results").arg(total),
                    synonyms: [],
                    terms: [],
                    help: "",
                    hiddenCount: hidden,
                    groupStart: false,
                    groupTitle: group.paneTitle,
                    groupTotal: total,
                    score: 0
                });
            }
            // Never cut a pane group in half: stop before a capped group that
            // would exceed the list limit. An expanded group is always kept
            // (it is the group the user just asked to see).
            if (!isExpanded && out.length > 0
                    && out.length + items.length > limit)
                break;
            for (var p = 0; p < items.length; ++p)
                out.push(items[p]);
        }
        return out;
    }

    // Convenience for the debug view/tests: the first result of each kind.
    function firstOfKind(query, kind, options) {
        var results = root.search(query, options);
        for (var i = 0; i < results.length; ++i) {
            if (results[i].kind === kind)
                return results[i];
        }
        return null;
    }
}
