// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.ControlCenter

// The Menu Bar pane (T-15.9b, T277, restyled by T293a).
//
// The menu bar is shell-native: the shell's `MenuBar` owns the chrome, the
// status row, and the clock, the menu-broker resolves the focused app's
// menus, and settingsd owns the durable preferences. This pane is the
// configuration half and writes only settingsd keys, so every control applies
// live and persists.
//
// T293a follows the mockup order:
//
//   1. Menu Bar Behaviour — `menu.autoHide`, `menu.showBackground`,
//      `menu.recentItems`. The mockup's "Show suggestions in the menu bar
//      gallery" row is a map omission (no owner).
//   2. OS Menu Glyph — a real radio group over `menu.osGlyph`
//      (`dragonfruit` default, `tux` alternate). Each row carries an
//      actual-size menu-bar strip: the real bar height and the real
//      `Theme.controls.menuBar.iconSize` mark.
//   3. Menu Bar Controls — Clock + Clock Options, Search, Wi-Fi, Bluetooth,
//      Battery, Focus, Sound, Accessibility, Quick Settings, and Workspaces,
//      each with its own options. Display stays a disabled row with its exact
//      reason; Now Playing, Text Input, Backups, and Timer are map omissions.
//   4. Allow in the Menu Bar — the live third-party StatusNotifier list and
//      its empty state (`menu.tray.hidden`). The mockup's File Manager, Music,
//      and Sync Agent names are examples, never fixed rows.
//
// The footer owns Add Controls…, Customize Controls…, Advanced…, and Reset
// Menu Bar…. Customize Controls… opens the in-place **Menu Bar Controls**
// sub-pane (T293b): the former Quick Settings surface, mounted in place of the
// pane list through `subPaneLoader` and writing the same `quickSettings.*` keys
// and `ControlCenterRegistry`. The `control-center`/`quick-settings` deep links
// land on the same sub-pane. The Linux-only rows (`menu.overflow`,
// `menu.global`, `quickSettings.perDisplay`) live in the Advanced Menu Bar
// sheet with their real owners and reasons.
//
// A control with no menu-bar provider is shown disabled with the exact reason
// and is never advertised as live.
Item {
    id: root

    // Pane-to-pane navigation (the existing signal shape every pane uses).
    signal sectionRequested(string paneId, string section)

    // T293b: the in-place sub-pane ("" is the Menu Bar list).
    property string subPaneId: ""
    readonly property string subPaneTitle: {
        switch (root.subPaneId) {
        case "controls": return qsTr("Menu Bar Controls");
        default: return "";
        }
    }
    // Emitted when the sub-pane body exists, so a deep link that was waiting
    // for it can finish landing.
    signal subPaneLoaded(string subPaneId)

    function openSubPane(id) {
        // Re-assigning the same id is a no-op: the loader keeps the mounted
        // body, so a reveal that re-opens the open sub-pane never reloads it.
        var next = String(id);
        if (next !== root.subPaneId)
            root.subPaneId = next;
    }

    function closeSubPane() {
        root.subPaneId = "";
    }

    function subPaneComponent(id) {
        switch (id) {
        case "controls": return controlsComponent;
        default: return null;
        }
    }

    // ── Behavior keys ───────────────────────────────────────────────────
    readonly property string autoHide:
        Settings.values["menu.autoHide"] !== undefined
            ? String(Settings.values["menu.autoHide"]) : "full-screen"
    readonly property bool showBackground:
        Settings.values["menu.showBackground"] !== false
    readonly property int recentItems:
        Settings.values["menu.recentItems"] !== undefined
            ? Number(Settings.values["menu.recentItems"]) : 10

    // ── Clock option keys ───────────────────────────────────────────────
    readonly property bool clockShowDate:
        Settings.values["menu.clock.showDate"] !== false
    readonly property bool clockShowSeconds:
        Settings.values["menu.clock.showSeconds"] === true

    // ── OS menu glyph ───────────────────────────────────────────────────
    readonly property string osGlyph:
        Settings.values["menu.osGlyph"] !== undefined
            ? String(Settings.values["menu.osGlyph"]) : "dragonfruit"

    readonly property var glyphOptions: [
        { value: "dragonfruit", label: qsTr("Dragonfruit"),
          description: qsTr("The default mark. Each preview is the menu bar's "
                            + "real height and glyph size.") },
        { value: "tux", label: qsTr("Tux"),
          description: qsTr("The traditional Linux penguin, for a familiar "
                            + "desktop look.") }
    ]

    readonly property int glyphIndex: {
        for (var i = 0; i < root.glyphOptions.length; ++i) {
            if (root.glyphOptions[i].value === root.osGlyph)
                return i;
        }
        return 0;
    }

    function setGlyph(index) {
        var option = root.glyphOptions[index];
        if (option !== undefined)
            Settings.set("menu.osGlyph", option.value);
    }

    readonly property var autoHideOptions: [
        { value: "never", label: qsTr("Never") },
        { value: "always", label: qsTr("Always") },
        { value: "full-screen", label: qsTr("In Full Screen Only") }
    ]

    readonly property var recentOptions: [
        { value: 0, label: qsTr("None") },
        { value: 5, label: qsTr("5") },
        { value: 10, label: qsTr("10") },
        { value: 15, label: qsTr("15") },
        { value: 20, label: qsTr("20") },
        { value: 30, label: qsTr("30") },
        { value: 50, label: qsTr("50") }
    ]

    readonly property var overflowOptions: [
        { value: "automatic", label: qsTr("Automatic") },
        { value: "always", label: qsTr("Always show the overflow menu") }
    ]

    // The Menu Bar Controls list, in the mockup order. `options` opens the
    // per-control options; `disabled` is an honest absence (no menu-bar item
    // exists), never a fake toggle. Clock is the first row and owns its own
    // options. Workspaces is the Linux-only active-workspace indicator.
    readonly property var controlRows: [
        { key: "menu.control.search", label: qsTr("Search") },
        { key: "menu.control.wifi", label: qsTr("Wi-Fi") },
        { key: "menu.control.bluetooth", label: qsTr("Bluetooth") },
        { key: "menu.control.battery", label: qsTr("Battery"),
          options: "battery" },
        { key: "menu.control.focus", label: qsTr("Focus"),
          options: "focus" },
        { key: "menu.control.volume", label: qsTr("Sound"),
          options: "sound" },
        { key: "menu.control.accessibility", label: qsTr("Accessibility") },
        { key: "menu.control.controlCenter", label: qsTr("Quick Settings") },
        { key: "menu.control.display", label: qsTr("Display"),
          disabled: true,
          reason: qsTr("Display has no menu-bar item; the Display module "
                      + "lives in Quick Settings.") },
        { key: "menu.control.workspaces", label: qsTr("Workspaces") }
    ]

    // T293a: the mockup controls with no status item. Each is a map omission
    // with its reason; the pane never renders a disabled placeholder toggle
    // for one.
    readonly property var omittedControls: [
        { id: "nowPlaying", label: qsTr("Now Playing"),
          reason: qsTr("No media status item exists yet; the shell has no "
                      + "player provider to control.") },
        { id: "textInput", label: qsTr("Text Input"),
          reason: qsTr("No input-method status item exists yet; the input "
                      + "source is chosen in Keyboard.") },
        { id: "backups", label: qsTr("Backups"),
          reason: qsTr("No backup status item exists yet; backups are set in "
                      + "General > Backups.") },
        { id: "timer", label: qsTr("Timer"),
          reason: qsTr("No timer service or clock timer exists yet.") }
    ]

    // The mockup Behaviour row with no Linux owner.
    readonly property string omittedBehaviourReason:
        qsTr("The menu-bar gallery has no owner in this desktop.")

    readonly property var thirdPartyItems: Settings.thirdPartyMenuBarItems
    readonly property bool thirdPartyAvailable: Settings.thirdPartyMenuBarAvailable

    function controlVisible(key) {
        return Settings.values[key] !== false;
    }

    function controlRow(index) {
        return root.controlRows[index];
    }

    function controlDisabled(row) {
        return row !== undefined && row.disabled === true;
    }

    function controlReason(row) {
        return row !== undefined && row.reason !== undefined
            ? String(row.reason) : "";
    }

    // The executable map's row id for a control. Sound is the one renamed row.
    function controlSettingsId(key) {
        if (key === "menu.control.volume")
            return "sound";
        return String(key).replace("menu.control.", "");
    }

    // ── Per-control options ─────────────────────────────────────────────
    function optionKey(kind) {
        switch (kind) {
        case "battery": return "power.showPercentage";
        case "focus": return "menu.control.focusAlways";
        case "sound": return "menu.control.volumeWhenActive";
        default: return "";
        }
    }

    function optionTitle(kind) {
        switch (kind) {
        case "battery": return qsTr("Battery Options");
        case "focus": return qsTr("Focus Options");
        case "sound": return qsTr("Sound Options");
        default: return qsTr("Options");
        }
    }

    function optionLabel(kind) {
        switch (kind) {
        case "battery": return qsTr("Show percentage");
        case "focus": return qsTr("Show when Focus is off");
        case "sound": return qsTr("Hide while muted");
        default: return qsTr("Option");
        }
    }

    function optionDescription(kind) {
        switch (kind) {
        case "battery":
            return qsTr("Draw the battery percentage beside the glyph.");
        case "focus":
            return qsTr("Show the Focus mark even when no Focus mode is on.");
        case "sound":
            return qsTr("Hide the Sound mark while the output is muted.");
        default:
            return "";
        }
    }

    function optionChecked(kind) {
        var key = root.optionKey(kind);
        return key.length > 0 && Settings.values[key] === true;
    }

    function setOption(kind, checked) {
        var key = root.optionKey(kind);
        if (key.length > 0)
            Settings.set(key, checked);
    }

    function openControlOptions(kind) {
        controlOptionsDialog.optionKind = String(kind);
        controlOptionsDialog.show();
    }

    // The options button behind a control row ("" for a row without options).
    function controlOptionsButton(index) {
        var row = root.controlsRepeater.itemAt(index);
        if (row === undefined || row === null)
            return null;
        for (var i = 0; i < row.children.length; ++i) {
            var child = row.children[i];
            if (child.objectName !== undefined
                    && String(child.objectName).indexOf("menuBarControlOptions-") === 0)
                return child;
        }
        return null;
    }

    // ── Third-party StatusNotifier items ────────────────────────────────
    readonly property var hiddenTray: {
        var value = Settings.values["menu.tray.hidden"];
        return value === undefined || value === null ? [] : Array.from(value);
    }

    function trayAllowed(name) {
        return root.hiddenTray.indexOf(name) < 0;
    }

    function setTrayAllowed(name, allowed) {
        var hidden = root.hiddenTray.slice();
        var at = hidden.indexOf(name);
        if (allowed && at >= 0)
            hidden.splice(at, 1);
        else if (!allowed && at < 0)
            hidden.push(name);
        else
            return;
        Settings.setList("menu.tray.hidden", hidden);
    }

    // ── Linux-only rows (the Advanced Menu Bar sheet) ───────────────────
    readonly property string overflowPolicy:
        Settings.values["menu.overflow"] !== undefined
            ? String(Settings.values["menu.overflow"]) : "automatic"
    readonly property bool workspacesVisible:
        Settings.values["menu.control.workspaces"] !== false
    readonly property bool globalMenuEnabled:
        Settings.values["menu.global"] !== false
    readonly property bool perDisplayPanel:
        Settings.values["quickSettings.perDisplay"] === true

    // The rows that live in the closed Advanced Menu Bar sheet, in rendered
    // order. `SettingsMap.rowSurface()` answers "dialog" for exactly these
    // ids, so a deep link opens the sheet and lands on the row.
    readonly property var advancedRowIds: ["overflow", "globalMenu", "perDisplayPanel"]

    function overflowIndex() {
        for (var i = 0; i < root.overflowOptions.length; ++i) {
            if (root.overflowOptions[i].value === root.overflowPolicy)
                return i;
        }
        return 0;
    }

    function isAdvancedRow(rowId) {
        return root.advancedRowIds.indexOf(String(rowId)) >= 0;
    }

    // A Dialog adopts the window overlay layer, so the shell cannot find its
    // rows by walking the pane body; the pane names them here.
    function advancedRowFor(rowId) {
        switch (String(rowId)) {
        case "overflow": return overflowRow;
        case "globalMenu": return globalMenuRow;
        case "perDisplayPanel": return perDisplayPanelRow;
        }
        return null;
    }

    function rowTargetFor(rowId) {
        if (root.isAdvancedRow(rowId))
            return root.advancedRowFor(rowId);
        // T293b: a deep link into the Menu Bar Controls sub-pane finds its row
        // by walking the mounted body, so the sheet hooks stay separate.
        return null;
    }

    function openAdvancedMenuBar() {
        advancedDialog.show();
    }
    function closeAdvancedMenuBar() {
        advancedDialog.hide();
    }

    // The shell's closed-surface hook: open the owning surface before it looks
    // for the row. T293b opens the in-place Menu Bar Controls sub-pane for its
    // rows; the Advanced Menu Bar sheet for the three Linux-only rows.
    function openRowSurface(rowId) {
        var subPane = SettingsMap.subPaneForRow("menu-bar", String(rowId));
        if (subPane.length > 0) {
            root.openSubPane(subPane);
            return true;
        }
        if (root.isAdvancedRow(rowId)) {
            root.openAdvancedMenuBar();
            return true;
        }
        return false;
    }

    // The footer's Customize Controls… opens the in-place Menu Bar Controls
    // sub-pane (T293b). The signal keeps the footer's shape; the shell answers
    // it with `openSubPane("controls")` on this pane.
    function customizeControls() {
        root.sectionRequested("menu-bar", "controls");
    }

    // ── Reset ───────────────────────────────────────────────────────────
    // Reset Menubar covers every key this pane writes: the behaviour rows,
    // the OS glyph, the clock options, control visibility, the per-control
    // options, the third-party allowances, and the Linux-only rows.
    readonly property var resetKeys: [
        { key: "menu.autoHide", value: "full-screen" },
        { key: "menu.showBackground", value: true },
        { key: "menu.recentItems", value: 10 },
        { key: "menu.osGlyph", value: "dragonfruit" },
        { key: "menu.clock.showDate", value: true },
        { key: "menu.clock.showSeconds", value: false },
        { key: "menu.control.clock", value: true },
        { key: "menu.control.focusAlways", value: false },
        { key: "menu.control.volumeWhenActive", value: false },
        { key: "power.showPercentage", value: false },
        { key: "menu.overflow", value: "automatic" },
        { key: "menu.global", value: true },
        { key: "quickSettings.perDisplay", value: false }
    ]

    function resetMenuBar() {
        for (var i = 0; i < root.resetKeys.length; ++i)
            Settings.set(root.resetKeys[i].key, root.resetKeys[i].value);
        Settings.setList("menu.tray.hidden", []);
        for (var r = 0; r < root.controlRows.length; ++r) {
            var row = root.controlRows[r];
            if (row.disabled === true)
                continue;
            Settings.set(row.key, true);
        }
        root.resetRequested();
    }

    signal resetRequested()

    // ── Test surface ────────────────────────────────────────────────────
    property alias autoHideSelect: autoHideSelect
    property alias backgroundToggle: backgroundToggle
    property alias recentSelect: recentSelect
    property alias clockOptionsButton: clockOptionsButton
    property alias clockDialog: clockDialog
    property alias clockDateToggle: clockDateToggle
    property alias clockSecondsToggle: clockSecondsToggle
    property alias clockVisibilityToggle: clockVisibilityToggle
    property alias glyphGroup: glyphGroup
    property alias glyphRepeater: glyphRepeater
    property alias glyphKeyAdapter: glyphKeyAdapter
    property alias controlsRepeater: controlsRepeater
    property alias absenceNote: absenceNote
    property alias thirdPartyRepeater: thirdPartyRepeater
    property alias thirdPartyGroup: thirdPartyGroup
    property alias thirdPartyEmptyNote: thirdPartyEmptyNote
    property alias addControlsButton: addControlsButton
    property alias addControlsDialog: addControlsDialog
    property alias customizeControlsButton: customizeControlsButton
    property alias controlOptionsDialog: controlOptionsDialog
    property alias controlOptionsToggle: controlOptionsToggle
    property alias overflowSelect: overflowSelect
    property alias globalMenuToggle: globalMenuToggle
    property alias perDisplayPanelRow: perDisplayPanelRow
    property alias advancedDialog: advancedDialog
    property alias footer: footer
    property alias resetButton: resetButton
    property alias behaviorGroup: behaviorGroup
    property alias controlsGroup: controlsGroup
    property alias clockRow: clockRow
    property alias advancedDockRowHandle: overflowRow
    // T293b: the in-place Menu Bar Controls sub-pane (its body lives in
    // `MenuBarControlsPane.qml` and `subPaneLoader` mounts it).
    property alias listLayout: listColumn
    property alias subPaneLayout: subPaneColumn
    property alias subPaneLoader: subPaneLoader
    property alias subPaneBackButton: backButton

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function indexFor(options, value) {
        for (var i = 0; i < options.length; ++i) {
            if (Number(options[i].value) === Number(value))
                return i;
        }
        return options.length - 1;
    }

    function autoHideIndex(value) {
        for (var i = 0; i < root.autoHideOptions.length; ++i) {
            if (root.autoHideOptions[i].value === value)
                return i;
        }
        return root.autoHideOptions.length - 1;
    }

    // T293a: the footer has four actions; the shared row shrinks them together
    // so nothing overflows the content column at the measured window width.
    readonly property real footerAvailableWidth:
        width - 2 * Theme.controls.settingsRow.paddingH
    readonly property real footerNaturalWidth:
        4 * (2 * Theme.controls.button.paddingH + footerMeasure0.contentWidth)
        + 3 * Theme.primitive.spacing.sm
    function footerItemWidth() {
        var count = 4;
        var gaps = (count - 1) * Theme.primitive.spacing.sm;
        var even = (root.footerAvailableWidth - gaps) / count;
        var widest = Math.max(footerMeasure0.contentWidth,
                              footerMeasure1.contentWidth,
                              footerMeasure2.contentWidth);
        var natural = widest + 2 * Theme.controls.button.paddingH;
        var item = natural > 0
                ? even * root.footerNaturalWidth / (count * natural + gaps)
                : even;
        return Math.max(Theme.controls.button.height, Math.min(even, item));
    }

    Component.onCompleted: Settings.refreshThirdPartyMenuBarItems()

    // Hidden natural-width measurements for the footer derivation. They sit
    // off-screen so the Row never sizes from the buttons' own widths (that
    // would loop).
    Text {
        id: footerMeasure0
        visible: false
        x: -10000
        text: qsTr("Customize Controls…")
        font.pixelSize: Theme.controls.button.fontSize
    }
    Text {
        id: footerMeasure1
        visible: false
        x: -10000
        text: qsTr("Add Controls…")
        font.pixelSize: Theme.controls.button.fontSize
    }
    Text {
        id: footerMeasure2
        visible: false
        x: -10000
        text: qsTr("Reset Menu Bar…")
        font.pixelSize: Theme.controls.button.fontSize
    }

    // T293b: the pane list and the in-place Menu Bar Controls sub-pane, the
    // same two-column shape General uses. Only one column is visible, so the
    // root `implicitHeight: content.implicitHeight` measures the open surface.
    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Column {
            id: listColumn
            width: parent.width
            spacing: Theme.primitive.spacing.lg
            visible: root.subPaneId === ""

            // ── Absence ─────────────────────────────────────────────────
            SettingsGroup {
                width: parent.width
                visible: !Settings.available

                SettingsAbsenceNotice {
                    id: absenceNote
                    objectName: "menuBarAbsenceNote"
                    kind: "live-on-defaults"
                    showWhenLive: true
                    reason: qsTr("The settings daemon is not running, so these "
                                 + "preferences apply from the built-in defaults. "
                                 + "The menu bar itself is drawn by the shell.")
                }
            }

            // ── Menu Bar Behaviour ──────────────────────────────────────────
            SettingsGroup {
                id: behaviorGroup
                width: parent.width
                title: qsTr("Menu Bar Behaviour")
                sectionId: "Menu Bar Behaviour"

                Column {
                    width: parent.width
                    spacing: Theme.controls.settingsGroup.rowGap

                    SettingsRow {
                        width: parent.width
                        label: qsTr("Automatically hide and show the menu bar")
                        settingsId: "autoHide"
                        owner: "settingsd -> shell/MenuBar"
                        settingsKey: "menu.autoHide"
                        controlData: Select {
                            id: autoHideSelect
                            accessibleName: qsTr("Automatically hide and show the menu bar")
                            model: root.autoHideOptions
                            onActivated: (index) => Settings.set(
                                "menu.autoHide", root.autoHideOptions[index].value)
                        }
                    }

                    SettingsRow {
                        width: parent.width
                        label: qsTr("Show menu bar background")
                        settingsId: "background"
                        owner: "settingsd -> shell/MenuBar"
                        settingsKey: "menu.showBackground"
                        controlData: Toggle {
                            id: backgroundToggle
                            accessibleName: qsTr("Show menu bar background")
                            onToggled: (checked) => Settings.set(
                                "menu.showBackground", checked)
                        }
                    }

                    SettingsRow {
                        width: parent.width
                        label: qsTr("Recent documents, applications, and servers")
                        settingsId: "recentItems"
                        owner: "settingsd -> shell/MenuBar"
                        settingsKey: "menu.recentItems"
                        showSeparator: false
                        controlData: Select {
                            id: recentSelect
                            accessibleName: qsTr("Recent documents, applications, and servers")
                            model: root.recentOptions
                            onActivated: (index) => Settings.set(
                                "menu.recentItems", root.recentOptions[index].value)
                        }
                    }
                }
            }

            // ── OS Menu Glyph ───────────────────────────────────────────────
            // One AT-SPI radio group: the two rows are radio buttons, each with
            // an actual-size menu-bar strip. Clicking anywhere selects; the
            // keyboard uses Space/Return (arrows move and select).
            SettingsGroup {
                id: glyphGroup
                objectName: "menuBarGlyphGroup"

                HoverHandler {
                    id: groupHover
                }

                width: parent.width
                title: qsTr("OS Menu Glyph")
                sectionId: "OS Menu Glyph"
                // The adapter below hosts the radio group's keyboard navigation.
                activeFocusOnTab: false

                onActiveFocusChanged: if (activeFocus && glyphRepeater.count > 0)
                    glyphRepeater.itemAt(root.glyphIndex).forceActiveFocus()

                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("OS Menu Glyph")
                Accessible.description: qsTr("Choose the mark at the far left of "
                                             + "the menu bar.")

                Column {
                    width: parent.width
                    spacing: Theme.controls.settingsGroup.rowGap

                    // T293a: the group's radio keyboard. Space/Return select the
                    // focused row; Up/Down/Left/Right move the selection like
                    // `SettingsRadioGroup`, and the focus follows it.
                    Item {
                        id: glyphKeyAdapter
                        width: 0
                        height: 0

                        function moveFocus(delta) {
                            var count = glyphRepeater.count;
                            if (count === 0)
                                return true;
                            var next = Math.max(0, Math.min(count - 1,
                                                           root.glyphIndex + delta));
                            if (next === root.glyphIndex)
                                return true;
                            // The selection and the roving focus move together.
                            glyphRepeater.itemAt(next).forceActiveFocus();
                            root.setGlyph(next);
                            return true;
                        }

                        // Space/Return select the focused row; Up/Down/Left/Right
                        // move the selection (the `SettingsRadioGroup` rule).
                        function handleKey(key) {
                            switch (key) {
                            case Qt.Key_Down:
                            case Qt.Key_Right:
                                return glyphKeyAdapter.moveFocus(1);
                            case Qt.Key_Up:
                            case Qt.Key_Left:
                                return glyphKeyAdapter.moveFocus(-1);
                            case Qt.Key_Space:
                            case Qt.Key_Return:
                            case Qt.Key_Enter:
                                // Space/Return confirm the *focused* row, like
                                // `SettingsRadioGroup`.
                                var at = root.glyphIndex;
                                for (var i = 0; i < glyphRepeater.count; ++i) {
                                    var row = glyphRepeater.itemAt(i);
                                    if (row !== null && row.activeFocus)
                                        at = i;
                                }
                                root.setGlyph(at);
                                return true;
                            }
                            return false;
                        }
                    }

                    Text {
                        objectName: "menuBarGlyphDescription"
                        property string settingsId: "osGlyph"
                        width: parent.width
                        text: qsTr("Choose the mark at the far left of the menu bar. "
                                   + "Dragonfruit is the default; Tux gives a "
                                   + "traditional Linux look. Each preview is shown at "
                                   + "the menu bar's real size and shadow.")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                        leftPadding: Theme.controls.settingsRow.paddingH
                        rightPadding: Theme.controls.settingsRow.paddingH
                        topPadding: Theme.primitive.spacing.xs
                        bottomPadding: Theme.primitive.spacing.xs
                    }

                    Repeater {
                        id: glyphRepeater
                        model: root.glyphOptions

                        delegate: SettingsGlyphOptionRow {
                            required property var modelData
                            required property int index

                            objectName: "menuBarGlyphRow-" + index
                            width: parent.width
                            label: modelData.label
                            description: modelData.description
                            glyph: modelData.value
                            settingsKey: "menu.osGlyph"
                            selected: index === root.glyphIndex
                            keyTarget: glyphKeyAdapter
                            openSample: groupHover.hovered && index === root.glyphIndex
                            showSeparator: index < glyphRepeater.count - 1
                            onChosen: root.setGlyph(index)
                        }
                    }
                }
            }

            // ── Menu Bar Controls ───────────────────────────────────────────
            SettingsGroup {
                id: controlsGroup
                width: parent.width
                title: qsTr("Menu Bar Controls")
                sectionId: "Menu Bar Controls"

                Column {
                    width: parent.width
                    spacing: Theme.controls.settingsGroup.rowGap

                    Text {
                        objectName: "menuBarControlsDescription"
                        width: parent.width
                        text: qsTr("System controls you keep in the menu bar. "
                                   + "Customize Controls… edits the Quick Settings "
                                   + "layout that these controls mirror.")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                        leftPadding: Theme.controls.settingsRow.paddingH
                        rightPadding: Theme.controls.settingsRow.paddingH
                        topPadding: Theme.primitive.spacing.xs
                        bottomPadding: Theme.primitive.spacing.xs
                    }

                    // The first control is the one Clock row, with its options. It is
                    // not duplicated in the list below.
                    SettingsRow {
                        id: clockRow
                        width: parent.width
                        label: qsTr("Clock")
                        description: qsTr("Show the date and time in the menu bar.")
                        settingsId: "clock"
                        owner: "settingsd -> shell/MenuBar"
                        settingsKey: "menu.control.clock"
                        controlData: Row {
                            spacing: Theme.primitive.spacing.xs

                            Toggle {
                                id: clockVisibilityToggle
                                text: ""
                                accessibleName: qsTr("Show the clock in the menu bar")
                                checked: root.controlVisible("menu.control.clock")
                                onToggled: (checked) =>
                                    Settings.set("menu.control.clock", checked)
                            }

                            Button {
                                id: clockOptionsButton
                                objectName: "menuBarClockOptionsButton"
                                text: qsTr("Options\u2026")
                                accessibleName: qsTr("Clock options")
                                onClicked: clockDialog.show()
                            }
                        }
                    }

                    Repeater {
                        id: controlsRepeater
                        model: root.controlRows

                        delegate: SettingsRow {
                            required property var modelData
                            required property int index

                            property alias toggle: controlToggle
                            // T293a: the Workspaces row's own toggle, exposed on
                            // the pane without a second control.
                            property alias workspacesToggleAlias: controlToggle
                            width: parent.width
                            label: modelData.label
                            description: root.controlDisabled(modelData)
                                ? root.controlReason(modelData) : ""
                            settingsId: root.controlSettingsId(modelData.key)
                            owner: "settingsd -> shell/MenuBar"
                            settingsKey: modelData.key
                            enabled: !root.controlDisabled(modelData)
                            showSeparator: index < controlsRepeater.count - 1

                            controlData: Row {
                                spacing: Theme.primitive.spacing.xs

                                Toggle {
                                    id: controlToggle
                                    objectName: "menuBarControlToggle"
                                    text: ""
                                    enabled: !root.controlDisabled(modelData)
                                    accessibleName: modelData.label
                                    onToggled: (checked) =>
                                        Settings.set(modelData.key, checked)
                                }

                                Button {
                                    objectName: "menuBarControlOptions-" + index
                                    visible: modelData.options !== undefined
                                    enabled: !root.controlDisabled(modelData)
                                    text: qsTr("Options\u2026")
                                    accessibleName: qsTr("%1 options").arg(modelData.label)
                                    onClicked: root.openControlOptions(
                                                   String(modelData.options))
                                }
                            }

                            Binding {
                                target: controlToggle
                                property: "checked"
                                value: root.controlVisible(modelData.key)
                            }
                        }
                    }
                }
            }

            // ── Allow in the Menu Bar ───────────────────────────────────────
            SettingsGroup {
                id: thirdPartyGroup
                width: parent.width
                title: qsTr("Allow in the Menu Bar")
                sectionId: "Allow in the Menu Bar"

                Column {
                    width: parent.width
                    spacing: Theme.controls.settingsGroup.rowGap

                    SettingsRow {
                        width: parent.width
                        label: qsTr("Third-party items")
                        description: root.thirdPartyAvailable
                            ? qsTr("Applications can add their own menu-bar items. "
                                   + "Turn one off to keep it out of the menu bar; its "
                                   + "application keeps running.")
                            : qsTr("The menu-bar item host is not running, so no "
                                   + "third-party items are available.")
                        settingsId: "thirdParty"
                        owner: "StatusNotifier registry"
                        settingsKey: ""
                        showSeparator: root.thirdPartyItems.length > 0
                    }

                    Repeater {
                        id: thirdPartyRepeater
                        model: root.thirdPartyItems

                        delegate: SettingsRow {
                            required property var modelData
                            required property int index
                            width: parent.width
                            label: modelData.label !== undefined
                                ? String(modelData.label) : String(modelData.name)
                            description: String(modelData.name)
                            settingsId: "thirdParty-" + index
                            owner: "StatusNotifier registry"
                            settingsKey: "menu.tray.hidden"
                            showSeparator: index < thirdPartyRepeater.count - 1

                            controlData: Toggle {
                                objectName: "menuBarThirdPartyToggle-" + index
                                text: ""
                                accessibleName: qsTr("Allow %1 in the menu bar")
                                    .arg(String(modelData.label))
                                checked: root.trayAllowed(String(modelData.name))
                                onToggled: (checked) =>
                                    root.setTrayAllowed(String(modelData.name), checked)
                            }
                        }
                    }

                    Text {
                        id: thirdPartyEmptyNote
                        objectName: "menuBarThirdPartyEmpty"
                        visible: root.thirdPartyAvailable
                                 && root.thirdPartyItems.length === 0
                        width: parent.width
                        text: qsTr("No third-party menu-bar items are installed.")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        leftPadding: Theme.controls.settingsRow.paddingH
                        rightPadding: Theme.controls.settingsRow.paddingH
                        topPadding: Theme.primitive.spacing.xs
                        bottomPadding: Theme.primitive.spacing.xs
                    }
                }
            }

            // ── Footer ──────────────────────────────────────────────────────
            SettingsFooter {
                id: footer
                objectName: "menuBarFooter"
                width: parent.width

                readonly property real itemWidth: root.footerItemWidth()

                Button {
                    id: addControlsButton
                    objectName: "menuBarAddControls"
                    variant: "ghost"
                    width: footer.itemWidth
                    implicitWidth: footer.itemWidth
                    text: qsTr("Add Controls…")
                    accessibleName: qsTr("Add menu-bar controls")
                    onClicked: addControlsDialog.show()
                }

                Button {
                    id: customizeControlsButton
                    objectName: "menuBarCustomizeControls"
                    variant: "ghost"
                    width: footer.itemWidth
                    implicitWidth: footer.itemWidth
                    text: qsTr("Customize Controls…")
                    accessibleName: qsTr("Edit the Quick Settings layout")
                    onClicked: root.customizeControls()
                }

                Button {
                    id: advancedButton
                    objectName: "menuBarAdvanced"
                    variant: "ghost"
                    width: footer.itemWidth
                    implicitWidth: footer.itemWidth
                    text: qsTr("Advanced…")
                    accessibleName: qsTr("Open advanced menu bar settings")
                    onClicked: root.openAdvancedMenuBar()
                }

                Button {
                    id: resetButton
                    objectName: "menuBarReset"
                    variant: "ghost"
                    width: footer.itemWidth
                    implicitWidth: footer.itemWidth
                    text: qsTr("Reset Menu Bar…")
                    accessibleName: qsTr("Reset the menu bar")
                    onClicked: root.resetMenuBar()
                }
            }

            Item {
                width: parent.width
                height: 0
            }
        }

        // ── The in-place Menu Bar Controls sub-pane (T293b) ────────────
        Column {
            id: subPaneColumn
            width: parent.width
            spacing: Theme.primitive.spacing.lg
            visible: root.subPaneId !== ""

            Button {
                id: backButton
                objectName: "menuBarSubPaneBack"
                text: qsTr("‹ Menu Bar")
                onClicked: root.closeSubPane()
            }

            Loader {
                id: subPaneLoader
                width: parent.width
                sourceComponent: root.subPaneComponent(root.subPaneId)
                onLoaded: root.subPaneLoaded(root.subPaneId)
            }
        }
    }

    // The Menu Bar Controls body is the former Quick Settings pane, not a
    // re-implementation: it reads the one `ControlCenterRegistry` and writes
    // its `quickSettings.*` keys unchanged. It is registered in
    // `Dragonfruit.ControlCenter` (`shell/control-center`) because importing
    // `Dragonfruit.Settings` from here is a dependency cycle.
    Component {
        id: controlsComponent
        MenuBarControlsPane { }
    }

    // The Workspaces row's toggle (the Linux-only active-workspace
    // indicator). It lives in the controls Repeater, so the pane names it.
    function workspacesToggle() {
        for (var i = 0; i < root.controlRows.length; ++i) {
            if (root.controlRows[i].key === "menu.control.workspaces") {
                var row = root.controlsRepeater.itemAt(i);
                return row !== null && row !== undefined
                        ? row.workspacesToggleAlias : null;
            }
        }
        return null;
    }

    // The per-control options sheet. One dialog carries the one option the
    // selected control owns; the key is the durable preference.
    Dialog {
        id: controlOptionsDialog
        objectName: "menuBarControlOptionsDialog"
        property string optionKind: "battery"
        title: root.optionTitle(optionKind)
        message: qsTr("Choose how this control appears in the menu bar.")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.md

            SettingsRow {
                width: parent.width
                label: root.optionLabel(controlOptionsDialog.optionKind)
                description: root.optionDescription(
                                 controlOptionsDialog.optionKind)
                showSeparator: false

                controlData: Toggle {
                    id: controlOptionsToggle
                    objectName: "menuBarControlOptionToggle"
                    text: ""
                    accessibleName: root.optionLabel(
                                        controlOptionsDialog.optionKind)
                    onToggled: (checked) =>
                        root.setOption(controlOptionsDialog.optionKind, checked)
                }

                Binding {
                    target: controlOptionsToggle
                    property: "checked"
                    value: root.optionChecked(controlOptionsDialog.optionKind)
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: controlOptionsDialog.accept()
            }
        }
    }

    // The Clock Options sheet: the two clock options the shell's clock reads.
    Dialog {
        id: clockDialog
        objectName: "menuBarClockDialog"
        title: qsTr("Clock Options")
        message: qsTr("Choose what the menu-bar clock shows.")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.md

            SettingsRow {
                width: parent.width
                label: qsTr("Show date")
                controlData: Toggle {
                    id: clockDateToggle
                    accessibleName: qsTr("Show date")
                    onToggled: (checked) => Settings.set(
                        "menu.clock.showDate", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Display the time with seconds")
                showSeparator: false
                controlData: Toggle {
                    id: clockSecondsToggle
                    accessibleName: qsTr("Display the time with seconds")
                    onToggled: (checked) => Settings.set(
                        "menu.clock.showSeconds", checked)
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: clockDialog.accept()
            }
        }
    }

    // The Add Controls sheet: the same visibility keys as the list, without a
    // second control catalogue. A control with no menu-bar item stays disabled
    // with its reason.
    Dialog {
        id: addControlsDialog
        objectName: "menuBarAddControlsDialog"
        title: qsTr("Add Menu Bar Controls")
        message: qsTr("Choose which controls appear in the menu bar.")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Repeater {
                model: root.controlRows

                delegate: SettingsRow {
                    required property var modelData
                    width: parent.width
                    label: modelData.label
                    description: root.controlDisabled(modelData)
                        ? root.controlReason(modelData) : ""
                    settingsId: "add-" + modelData.key.replace("menu.control.", "")
                    owner: "settingsd -> shell/MenuBar"
                    settingsKey: modelData.key
                    enabled: !root.controlDisabled(modelData)
                    showSeparator: false

                    controlData: Toggle {
                        objectName: "menuBarAddToggle-" + modelData.key
                        text: ""
                        enabled: !root.controlDisabled(modelData)
                        accessibleName: qsTr("Show %1 in the menu bar")
                            .arg(modelData.label)
                        checked: root.controlVisible(modelData.key)
                        onToggled: (checked) =>
                            Settings.set(modelData.key, checked)
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: addControlsDialog.accept()
            }
        }
    }

    // T293a: the Advanced Menu Bar sheet. The three Linux-only rows keep
    // their real owner, key, and reason; the sheet is an organization change.
    Dialog {
        id: advancedDialog
        objectName: "menuBarAdvancedDialog"
        title: qsTr("Advanced Menu Bar")
        message: qsTr("Menu-bar behaviour that is specific to this desktop. A "
                      + "row the shell cannot provide stays visible and "
                      + "disabled with the reason.")

        // The overflow row holds a popup beside its label, so the sheet asks
        // for a wider card than the 480 pt default (T292b's `preferredWidth`).
        preferredWidth: 560

        contentData: Item {
            implicitWidth: Math.min(560, Math.max(480, root.Window.width > 0
                                                  ? root.Window.width - 48 : 560))
            implicitHeight: advancedGroup.implicitHeight
            width: parent.width
            height: implicitHeight

            SettingsGroup {
                id: advancedGroup
                width: parent.width
                title: qsTr("Advanced Menu Bar")
                sectionId: "Advanced Menu Bar"

                SettingsKeyRow {
                    id: overflowRow
                    width: parent.width
                    label: qsTr("Overflow policy")
                    settingsId: "overflow"
                    owner: "settingsd -> shell/MenuBar"
                    settingsKey: "menu.overflow"
                    description: qsTr("Items that do not fit move into the "
                                      + "overflow menu; they are never "
                                      + "clipped.")
                    controlData: Select {
                        id: overflowSelect
                        objectName: "menuBarOverflowPolicy"
                        accessibleName: qsTr("Overflow policy")
                        model: root.overflowOptions
                        onActivated: (index) => Settings.set(
                            "menu.overflow", root.overflowOptions[index].value)
                    }
                }

                SettingsKeyRow {
                    id: globalMenuRow
                    width: parent.width
                    label: qsTr("Show application menus in the menu bar")
                    settingsId: "globalMenu"
                    owner: "settingsd -> shell/MenuBar"
                    settingsKey: "menu.global"
                    description: qsTr("When off, first-party applications keep "
                                      + "their own local menus. Only the active "
                                      + "display owns the application menu.")
                    controlData: Toggle {
                        id: globalMenuToggle
                        objectName: "menuBarGlobalMenu"
                        text: ""
                        accessibleName: qsTr("Show application menus in the menu bar")
                        checked: root.globalMenuEnabled
                        onToggled: (checked) => Settings.set("menu.global", checked)
                    }
                }

                SettingsKeyRow {
                    id: perDisplayPanelRow
                    width: parent.width
                    label: qsTr("Show on every display")
                    settingsId: "perDisplayPanel"
                    owner: "settingsd -> shell"
                    settingsKey: "quickSettings.perDisplay"
                    description: qsTr("Disabled: the shell ships one menu-bar "
                                      + "surface per session. Per-display panels "
                                      + "need a compositor overlay per output.")
                    enabled: false
                    showSeparator: false
                    controlData: Toggle {
                        text: ""
                        checked: root.perDisplayPanel
                        enabled: false
                        accessibleName: qsTr("Show on every display")
                    }
                }
            }
        }
    }

    // Keep the controls in step with external changes.
    Binding {
        target: autoHideSelect
        property: "currentIndex"
        value: root.autoHideIndex(root.autoHide)
    }
    Binding {
        target: backgroundToggle
        property: "checked"
        value: root.showBackground
    }
    Binding {
        target: recentSelect
        property: "currentIndex"
        value: root.indexFor(root.recentOptions, root.recentItems)
    }
    Binding {
        target: clockDateToggle
        property: "checked"
        value: root.clockShowDate
    }
    Binding {
        target: clockSecondsToggle
        property: "checked"
        value: root.clockShowSeconds
    }
    Binding {
        target: overflowSelect
        property: "currentIndex"
        value: root.overflowIndex()
    }
}
