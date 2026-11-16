// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings
import Dragonfruit.ControlCenter

// The Quick Settings pane (T227, completed by T277).
//
// The pane is a management surface, not a second Control Center. It reads the
// one `ControlCenterRegistry` the live panel and the shell's Edit Controls
// surface use, and writes only the durable `quickSettings.*` / `menu.control.*`
// keys settingsd owns. Every change is visible in the live preview below and
// in the shell panel immediately; there is no second module list here.
//
// The pane deliberately keeps the module catalogue in categories (the same 22
// categories Edit Controls uses), pages with names, per-control spans, the
// menu-bar-only set, and one Reset that restores the whole surface. A module
// with no provider is listed with the registry's reason and is never addable.
Item {
    id: root

    readonly property var registry: ControlCenterRegistry
    readonly property var categories: ControlCenterRegistry.categories
    readonly property var defaultModules: ControlCenterRegistry.defaultModules
    readonly property var catalogueIds: ControlCenterRegistry.catalogueIds

    // ── Durable state ────────────────────────────────────────────────────
    // An absent key means "use the measured default"; an explicit empty list
    // means the user removed every control. The two states are different and
    // Reset is the only way back to the default list.
    readonly property var modules: {
        var value = Settings.values["quickSettings.modules"];
        if (value === undefined || value === null)
            return root.defaultModules.slice();
        return Array.from(value);
    }
    readonly property var menuBarOnly: {
        var value = Settings.values["quickSettings.menuBarOnly"];
        return value === undefined || value === null ? [] : Array.from(value);
    }
    readonly property var page2: {
        var value = Settings.values["quickSettings.page2"];
        return value === undefined || value === null ? [] : Array.from(value);
    }
    readonly property var page3: {
        var value = Settings.values["quickSettings.page3"];
        return value === undefined || value === null ? [] : Array.from(value);
    }
    readonly property var pageNames: {
        var value = Settings.values["quickSettings.pageNames"];
        return value === undefined || value === null ? [] : Array.from(value);
    }
    readonly property int pageCount: {
        var value = Number(Settings.values["quickSettings.pageCount"]);
        return isFinite(value) ? Math.max(1, Math.min(3, value)) : 1;
    }
    readonly property int activePage: {
        var value = Number(Settings.values["quickSettings.activePage"]);
        return isFinite(value) ? Math.max(0, Math.min(root.pageCount - 1, value)) : 0;
    }
    readonly property var spanMap:
        ControlCenterRegistry.parseSpans(Settings.values["quickSettings.spans"])
    readonly property bool perDisplay:
        Settings.values["quickSettings.perDisplay"] === true

    // The page currently being edited. Page 0 is `modules`; pages 1 and 2 are
    // the optional extra lists.
    function pageList(page) {
        if (page === 1)
            return root.page2.slice();
        if (page === 2)
            return root.page3.slice();
        return root.modules.slice();
    }

    readonly property var activeList: root.pageList(root.activePage)

    function pageName(page) {
        if (root.pageNames[page] !== undefined
                && String(root.pageNames[page]).length > 0)
            return String(root.pageNames[page]);
        return qsTr("Page %1").arg(page + 1);
    }

    function pageKey(page) {
        if (page === 1)
            return "quickSettings.page2";
        if (page === 2)
            return "quickSettings.page3";
        return "quickSettings.modules";
    }

    function writePage(page, list) {
        Settings.setList(root.pageKey(page), list);
    }

    // ── Catalogue helpers ────────────────────────────────────────────────
    function moduleLabel(id) {
        return ControlCenterRegistry.moduleLabel(id);
    }

    function moduleOwner(id) {
        return ControlCenterRegistry.moduleOwner(id);
    }

    function moduleReason(id) {
        return ControlCenterRegistry.moduleReason(id);
    }

    function isAddable(id) {
        return ControlCenterRegistry.isAddable(id);
    }

    // The executable map's row id for the module, so a deep link or a search
    // result lands on the rendered row instead of a duplicate.
    function moduleSettingsId(id) {
        switch (id) {
        case "wifi": return "wifi";
        case "bluetooth": return "bluetooth";
        case "focus": return "focus";
        case "screen-mirroring": return "screenMirroring";
        case "now-playing": return "nowPlaying";
        case "volume": return "sound";
        case "brightness": return "display";
        case "battery": return "battery";
        case "dark": return "darkMode";
        case "stage-manager": return "stage";
        case "capture-screen": return "screenRecording";
        case "keyboard-brightness": return "keyboardBrightness";
        case "hearing-accessibility": return "hearing";
        default: return "module-" + id;
        }
    }

    function modulePage(id) {
        for (var page = 0; page < root.pageCount; ++page) {
            if (root.pageList(page).indexOf(id) >= 0)
                return page;
        }
        return -1;
    }

    function moduleEnabled(id) {
        return root.modulePage(id) >= 0;
    }

    function moduleDescription(id) {
        if (id === "edit-controls")
            return qsTr("Always shown in the panel footer");
        if (root.isMenuBarOnly(id))
            return qsTr("Menu bar only");
        var page = root.modulePage(id);
        if (!root.isAddable(id))
            return root.moduleReason(id);
        if (page >= 0)
            return qsTr("%1 \u00b7 %2").arg(root.moduleOwner(id))
                                      .arg(root.pageName(page));
        return qsTr("%1 \u00b7 Not in Quick Settings").arg(root.moduleOwner(id));
    }

    function isMenuBarOnly(id) {
        return root.menuBarOnly.indexOf(id) >= 0;
    }

    function menuBarKey(id) {
        return ControlCenterRegistry.menuBarKey(id);
    }

    function spanOf(id) {
        return ControlCenterRegistry.spanFor(id, root.spanMap);
    }

    function spanLabel(id) {
        var span = root.spanOf(id);
        return span === "footer" ? qsTr("\u2014") : span;
    }

    function cycleSpan(id) {
        var next = ControlCenterRegistry.nextSpan(root.spanOf(id));
        var map = {};
        for (var key in root.spanMap)
            map[key] = root.spanMap[key];
        map[id] = next;
        Settings.setList("quickSettings.spans",
                         ControlCenterRegistry.serializeSpans(map));
    }

    // ── Module editing ───────────────────────────────────────────────────
    function removeFromEveryPage(id) {
        for (var page = 0; page < root.pageCount; ++page) {
            var list = root.pageList(page);
            var at = list.indexOf(id);
            if (at >= 0) {
                list.splice(at, 1);
                root.writePage(page, list);
            }
        }
    }

    function setModuleEnabled(id, enabled) {
        if (id === "edit-controls")
            return;
        if (enabled) {
            if (!root.isAddable(id))
                return;
            if (root.isMenuBarOnly(id)) {
                var only = root.menuBarOnly.slice();
                var onlyAt = only.indexOf(id);
                if (onlyAt >= 0) {
                    only.splice(onlyAt, 1);
                    Settings.setList("quickSettings.menuBarOnly", only);
                }
            }
            root.removeFromEveryPage(id);
            var list = root.pageList(root.activePage);
            list.push(id);
            root.writePage(root.activePage, list);
        } else {
            root.removeFromEveryPage(id);
        }
    }

    function moveModuleTo(id, index) {
        var list = root.pageList(root.activePage);
        var at = list.indexOf(id);
        if (at < 0)
            return;
        list.splice(at, 1);
        var target = Math.max(0, Math.min(list.length, index));
        list.splice(target, 0, id);
        root.writePage(root.activePage, list);
    }

    function moveModule(id, delta) {
        var list = root.pageList(root.activePage);
        root.moveModuleTo(id, list.indexOf(id) + delta);
    }

    function setMenuBarOnly(id, enabled) {
        var only = root.menuBarOnly.slice();
        var at = only.indexOf(id);
        if (enabled && at < 0)
            only.push(id);
        else if (!enabled && at >= 0)
            only.splice(at, 1);
        else
            return;
        Settings.setList("quickSettings.menuBarOnly", only);
        if (enabled) {
            root.removeFromEveryPage(id);
        } else if (!root.moduleEnabled(id)) {
            var page0 = root.pageList(0);
            page0.push(id);
            root.writePage(0, page0);
        }
        var key = root.menuBarKey(id);
        if (key.length > 0 && Settings.values[key] !== true)
            Settings.set(key, true);
    }

    // ── Pages ────────────────────────────────────────────────────────────
    function selectPage(page) {
        if (page < 0 || page >= root.pageCount)
            return;
        Settings.set("quickSettings.activePage", page);
    }

    function addPage() {
        if (root.pageCount >= 3)
            return;
        var next = root.pageCount + 1;
        var names = root.pageNames.slice();
        while (names.length < next)
            names.push("");
        if (String(names[next - 1]).length === 0)
            names[next - 1] = qsTr("Page %1").arg(next);
        Settings.setList("quickSettings.pageNames", names);
        Settings.set("quickSettings.pageCount", next);
        Settings.set("quickSettings.activePage", next - 1);
    }

    function renamePage(page, name) {
        if (page < 0 || page >= root.pageCount)
            return;
        var names = root.pageNames.slice();
        while (names.length <= page)
            names.push("");
        names[page] = String(name).trim().length > 0
            ? String(name).trim() : qsTr("Page %1").arg(page + 1);
        Settings.setList("quickSettings.pageNames", names);
    }

    function removePage(page) {
        if (root.pageCount <= 1 || page <= 0)
            return;
        var last = root.pageCount - 1;
        // Keep pages contiguous: removing page 1 shifts page 2 into page 1.
        if (page === 1 && last === 2)
            Settings.setList("quickSettings.page2", root.page3.slice());
        Settings.setList(root.pageKey(last), []);
        var names = root.pageNames.slice();
        while (names.length <= last)
            names.push("");
        names.splice(last, 1);
        while (names.length > 0 && String(names[names.length - 1]).length === 0)
            names.pop();
        Settings.setList("quickSettings.pageNames", names);
        Settings.set("quickSettings.pageCount", last);
        if (root.activePage >= last)
            Settings.set("quickSettings.activePage", Math.max(0, last - 1));
    }

    // ── Reset ────────────────────────────────────────────────────────────
    function resetAll() {
        Settings.setList("quickSettings.modules", root.defaultModules.slice());
        Settings.setList("quickSettings.menuBarOnly", []);
        Settings.setList("quickSettings.page2", []);
        Settings.setList("quickSettings.page3", []);
        Settings.setList("quickSettings.pageNames", []);
        Settings.set("quickSettings.pageCount", 1);
        Settings.set("quickSettings.activePage", 0);
        Settings.setList("quickSettings.spans", []);
        Settings.set("quickSettings.perDisplay", false);
        // Reset owns the menu-bar marks these modules use.
        for (var i = 0; i < ControlCenterRegistry.menuBarIds.length; ++i) {
            var key = ControlCenterRegistry.menuBarKey(
                ControlCenterRegistry.menuBarIds[i]);
            if (key.length > 0)
                Settings.set(key, true);
        }
    }

    // ── Catalogue groups for the management surface ──────────────────────
    property string selectedCategory: "all-controls"

    readonly property var categoryOptions: {
        var out = [{ label: qsTr("All Controls"), value: "all-controls" }];
        for (var i = 0; i < root.categories.length; ++i) {
            if (root.categories[i].id === "all-controls")
                continue;
            out.push({ label: root.categories[i].label,
                       value: root.categories[i].id });
        }
        return out;
    }

    function groupsFor(idsOnlyAddable) {
        var out = [];
        if (root.selectedCategory !== "all-controls") {
            var ids = ControlCenterRegistry.categoryModules(root.selectedCategory);
            if (idsOnlyAddable) {
                var filtered = [];
                for (var f = 0; f < ids.length; ++f) {
                    if (root.isAddable(ids[f]) && ids[f] !== "edit-controls")
                        filtered.push(ids[f]);
                }
                ids = filtered;
            }
            if (ids.length > 0) {
                out.push({ id: root.selectedCategory,
                           title: ControlCenterRegistry.categoryLabel(
                                      root.selectedCategory),
                           ids: ids });
            }
            return out;
        }
        for (var i = 0; i < root.categories.length; ++i) {
            var category = root.categories[i];
            if (category.id === "all-controls")
                continue;
            var list = ControlCenterRegistry.categoryModules(category.id);
            if (idsOnlyAddable) {
                var only = [];
                for (var j = 0; j < list.length; ++j) {
                    if (root.isAddable(list[j]) && list[j] !== "edit-controls")
                        only.push(list[j]);
                }
                list = only;
            }
            if (list.length > 0)
                out.push({ id: category.id, title: category.label, ids: list });
        }
        // `edit-controls` lives in the all-controls category; the grouped view
        // still shows it as the fixed footer action.
        if (!idsOnlyAddable && root.catalogueIds.indexOf("edit-controls") >= 0) {
            var seenEditControls = false;
            for (var g = 0; g < out.length; ++g) {
                if (out[g].ids.indexOf("edit-controls") >= 0)
                    seenEditControls = true;
            }
            if (!seenEditControls)
                out.push({ id: "edit-controls", title: qsTr("Edit Controls"),
                           ids: ["edit-controls"] });
        }
        return out;
    }

    readonly property var moduleGroups: root.groupsFor(false)
    readonly property var addableGroups: root.groupsFor(true)

    readonly property var unavailableIds: {
        var out = [];
        for (var i = 0; i < root.catalogueIds.length; ++i) {
            var id = root.catalogueIds[i];
            if (id !== "edit-controls" && !root.isAddable(id))
                out.push(id);
        }
        return out;
    }

    readonly property var addableIds: {
        var out = [];
        for (var i = 0; i < root.catalogueIds.length; ++i) {
            var id = root.catalogueIds[i];
            if (id !== "edit-controls" && root.isAddable(id))
                out.push(id);
        }
        return out;
    }

    // ── Live preview ─────────────────────────────────────────────────────
    readonly property var previewLayout: {
        var ids = [];
        for (var i = 0; i < root.activeList.length; ++i) {
            var id = root.activeList[i];
            if (root.isMenuBarOnly(id))
                continue;
            if (ControlCenterRegistry.module(id) === null)
                continue;
            ids.push(id);
        }
        return ControlCenterRegistry.layout(ids, root.spanMap);
    }
    readonly property bool previewHasFooter:
        root.activeList.indexOf("edit-controls") >= 0
        && !root.isMenuBarOnly("edit-controls")
    readonly property int previewGridHeight:
        ControlCenterRegistry.gridHeight(root.previewLayout)
    readonly property int previewHeight:
        Math.max(64, root.previewGridHeight + (root.previewHasFooter ? 44 : 0))
    readonly property int previewWidth: ControlCenterRegistry.moduleFieldWidth

    // Test surface.
    property alias moduleRepeater: moduleRepeater
    property alias menuBarRepeater: menuBarRepeater
    property alias resetButton: resetButton
    property alias perDisplayRow: perDisplayRow
    property alias categorySelect: categorySelect
    property alias pageTabs: pageTabs
    property alias addPageButton: addPageButton
    property alias renamePageButton: renamePageButton
    property alias removePageButton: removePageButton
    property alias previewRect: previewRect
    property alias addControlsButton: addControlsButton
    property alias addControlsDialog: addControlsDialog
    property alias renameDialog: renameDialog
    property alias renameInput: renameInput
    property alias pagesGroup: pagesGroup
    property alias previewGroup: previewGroup
    property alias modulesGroup: modulesGroup
    property alias menuBarGroup: menuBarGroup
    property alias resetGroup: resetGroup

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Pages ───────────────────────────────────────────────────────
        SettingsGroup {
            id: pagesGroup
            width: parent.width
            title: qsTr("Pages")

            Text {
                width: parent.width
                text: qsTr("Pages hold different sets of controls. The first "
                           + "page is the one the panel opens on by default; "
                           + "each page has its own name.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Row {
                id: pageTabs
                property string settingsId: "pages"
                width: parent.width
                height: 34
                spacing: Theme.primitive.spacing.xs

                Repeater {
                    model: root.pageCount

                    delegate: Button {
                        required property int index
                        objectName: "quickSettingsPage-" + index
                        text: root.pageName(index)
                        variant: root.activePage === index ? "primary" : "secondary"
                        accessibleName: qsTr("Show %1").arg(root.pageName(index))
                        onClicked: root.selectPage(index)
                    }
                }

                Button {
                    id: addPageButton
                    objectName: "quickSettingsAddPage"
                    text: qsTr("+")
                    accessibleName: qsTr("Add a page")
                    enabled: root.pageCount < 3
                    onClicked: root.addPage()
                }
                Button {
                    id: renamePageButton
                    property string settingsId: "pageNames"
                    objectName: "quickSettingsRenamePage"
                    text: qsTr("Rename\u2026")
                    accessibleName: qsTr("Rename the active page")
                    onClicked: {
                        root.renameDialog.renameIndex = root.activePage;
                        root.renameInput.text = root.pageName(root.activePage);
                        root.renameDialog.show();
                    }
                }
                Button {
                    id: removePageButton
                    objectName: "quickSettingsRemovePage"
                    text: qsTr("\u2212")
                    accessibleName: qsTr("Remove the active page")
                    enabled: root.activePage > 0
                    onClicked: root.removePage(root.activePage)
                }
            }
        }

        // ── Live preview ────────────────────────────────────────────────
        SettingsGroup {
            id: previewGroup
            width: parent.width
            title: qsTr("Preview")

            Text {
                property string settingsId: "preview"
                width: parent.width
                text: qsTr("The measured 4-column grid for the active page. "
                           + "It updates as modules, spans, and menu-bar-only "
                           + "choices change.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Item {
                width: parent.width
                height: root.previewHeight + 2 * Theme.primitive.spacing.sm

                Rectangle {
                    id: previewRect
                    objectName: "quickSettingsPreview"
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.verticalCenter: parent.verticalCenter
                    width: root.previewWidth + 2 * Theme.primitive.spacing.sm
                    height: root.previewHeight + 2 * Theme.primitive.spacing.sm
                    radius: Theme.primitive.radius.lg
                    color: Theme.color.surfaceSunken
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border

                    Item {
                        anchors.centerIn: parent
                        width: root.previewWidth
                        height: root.previewHeight

                        Repeater {
                            model: root.previewLayout

                            delegate: Rectangle {
                                required property var modelData
                                x: modelData.x
                                y: modelData.y
                                width: modelData.w
                                height: modelData.h
                                radius: Theme.primitive.radius.md
                                color: Theme.color.surfaceElevated
                                border.width: Theme.controls.window.borderWidth
                                border.color: Theme.color.border

                                Text {
                                    anchors.fill: parent
                                    anchors.margins: 4
                                    text: root.moduleLabel(modelData.id)
                                    color: Theme.color.textPrimary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    wrapMode: Text.WordWrap
                                    elide: Text.ElideRight
                                    verticalAlignment: Text.AlignVCenter
                                }
                            }
                        }

                        Rectangle {
                            visible: root.previewHasFooter
                            x: 0
                            y: root.previewGridHeight
                            width: root.previewWidth
                            height: 40
                            radius: Theme.primitive.radius.md
                            color: Theme.color.surfaceElevated
                            border.width: Theme.controls.window.borderWidth
                            border.color: Theme.color.border
                            Text {
                                anchors.centerIn: parent
                                text: qsTr("Edit Controls")
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                        }

                        Text {
                            visible: root.previewLayout.length === 0
                                    && !root.previewHasFooter
                            anchors.centerIn: parent
                            text: qsTr("This page is empty")
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                        }
                    }
                }
            }
        }

        // ── Modules ─────────────────────────────────────────────────────
        SettingsGroup {
            id: modulesGroup
            property string settingsId: "otherModules"
            width: parent.width
            title: qsTr("Modules")

            SettingsRow {
                width: parent.width
                label: qsTr("Category")
                description: qsTr("The same categories Edit Controls uses. "
                                  + "A control with no provider is listed with "
                                  + "its reason and cannot be added.")
                settingsId: "modules"
                owner: "settingsd + shell/control-center"
                settingsKey: "quickSettings.modules"
                controlData: Select {
                    id: categorySelect
                    objectName: "quickSettingsCategory"
                    model: root.categoryOptions
                    onActivated: (index) => root.selectedCategory =
                        root.categoryOptions[index].value
                }
            }

            Repeater {
                id: moduleRepeater
                property string settingsId: "spans"
                model: root.moduleGroups

                delegate: Column {
                    id: moduleGroupColumn
                    required property var modelData
                    width: parent ? parent.width : 0
                    spacing: 0

                    Text {
                        width: parent.width
                        text: moduleGroupColumn.modelData.title
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        font.weight: Theme.primitive.font.weightMedium
                        leftPadding: Theme.controls.settingsRow.paddingH
                        rightPadding: Theme.controls.settingsRow.paddingH
                        topPadding: Theme.primitive.spacing.sm
                        bottomPadding: Theme.primitive.spacing.xs
                    }

                    Repeater {
                        model: moduleGroupColumn.modelData.ids

                        delegate: SettingsRow {
                            required property string modelData
                            required property int index
                            width: parent.width
                            label: root.moduleLabel(modelData)
                            description: root.moduleDescription(modelData)
                            settingsId: root.moduleSettingsId(modelData)
                            owner: root.moduleOwner(modelData).length > 0
                                ? root.moduleOwner(modelData) : "unavailable"
                            settingsKey: "quickSettings.modules"
                            enabled: root.moduleEnabled(modelData)
                                     || root.isAddable(modelData)
                            showSeparator: index < moduleGroupColumn.modelData.ids.length - 1

                            DropArea {
                                anchors.fill: parent
                                keys: ["text/plain"]
                                onDropped: (drop) => {
                                    if (drop.hasText)
                                        root.moveModuleTo(String(drop.text), index);
                                }
                            }

                            controlData: Row {
                                spacing: 2

                                Rectangle {
                                    id: dragHandle
                                    width: 14
                                    height: 22
                                    radius: 3
                                    color: Theme.color.surfaceSunken
                                    visible: modelData !== "edit-controls"
                                             && root.moduleEnabled(modelData)
                                    Drag.active: handleDrag.active
                                    Drag.mimeData: ({ "text/plain": modelData })
                                    Drag.hotSpot: Qt.point(width / 2, height / 2)
                                    Text {
                                        anchors.centerIn: parent
                                        text: "\u22ee"
                                        color: Theme.color.textTertiary
                                        font.pixelSize: Theme.primitive.font.sizeSm
                                    }
                                    DragHandler {
                                        id: handleDrag
                                        target: null
                                    }
                                }
                                Button {
                                    visible: modelData !== "edit-controls"
                                    text: "\u2191"
                                    accessibleName: qsTr("Move %1 up").arg(root.moduleLabel(modelData))
                                    enabled: root.moduleEnabled(modelData)
                                    onClicked: root.moveModule(modelData, -1)
                                }
                                Button {
                                    visible: modelData !== "edit-controls"
                                    text: "\u2193"
                                    accessibleName: qsTr("Move %1 down").arg(root.moduleLabel(modelData))
                                    enabled: root.moduleEnabled(modelData)
                                    onClicked: root.moveModule(modelData, 1)
                                }
                                Button {
                                    visible: modelData !== "edit-controls"
                                    objectName: "quickSettingsSpan-" + modelData
                                    text: root.spanLabel(modelData)
                                    accessibleName: qsTr("Resize %1, currently %2")
                                        .arg(root.moduleLabel(modelData))
                                        .arg(root.spanLabel(modelData))
                                    enabled: root.moduleEnabled(modelData)
                                    onClicked: root.cycleSpan(modelData)
                                }
                                Toggle {
                                    visible: modelData !== "edit-controls"
                                    objectName: "quickSettingsToggle-" + modelData
                                    text: ""
                                    accessibleName: qsTr("Show %1 in Quick Settings")
                                        .arg(root.moduleLabel(modelData))
                                    checked: root.moduleEnabled(modelData)
                                    enabled: root.isAddable(modelData)
                                             || root.moduleEnabled(modelData)
                                    onToggled: (checked) =>
                                        root.setModuleEnabled(modelData, checked)
                                }
                                Text {
                                    visible: modelData === "edit-controls"
                                    text: qsTr("Always in the panel footer")
                                    color: Theme.color.textTertiary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Menu Bar Only ───────────────────────────────────────────────
        SettingsGroup {
            id: menuBarGroup
            width: parent.width
            title: qsTr("Menu Bar Only")

            Text {
                property string settingsId: "menuBarItems"
                width: parent.width
                text: qsTr("Keep these items in the menu bar and remove them "
                           + "from the Quick Settings panel.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: menuBarRepeater
                model: ControlCenterRegistry.menuBarIds

                delegate: SettingsRow {
                    required property string modelData
                    required property int index
                    width: parent.width
                    label: root.moduleLabel(modelData)
                    description: root.isMenuBarOnly(modelData)
                        ? qsTr("In the menu bar only") : qsTr("In the menu bar and panel")
                    settingsId: "menu-bar-" + modelData
                    owner: "settingsd -> shell/MenuBar"
                    settingsKey: root.menuBarKey(modelData)
                    showSeparator: index < menuBarRepeater.count - 1

                    controlData: Toggle {
                        objectName: "quickSettingsMenuBar-" + modelData
                        text: ""
                        accessibleName: qsTr("Keep %1 in the menu bar only")
                            .arg(root.moduleLabel(modelData))
                        checked: root.isMenuBarOnly(modelData)
                        onToggled: (checked) => root.setMenuBarOnly(modelData, checked)
                    }
                }
            }
        }

        // ── Displays ────────────────────────────────────────────────────
        SettingsGroup {
            id: displayGroup
            width: parent.width
            title: qsTr("Displays")

            SettingsRow {
                id: perDisplayRow
                width: parent.width
                label: qsTr("Show on all displays")
                description: qsTr("Disabled: the shell ships one Quick Settings "
                                  + "surface. Per-display panels need a "
                                  + "compositor overlay per output.")
                settingsId: "perDisplay"
                owner: "settingsd -> shell/control-center"
                settingsKey: "quickSettings.perDisplay"
                enabled: false
                showSeparator: false

                controlData: Toggle {
                    text: ""
                    accessibleName: qsTr("Show Quick Settings on all displays")
                    checked: root.perDisplay
                    enabled: false
                }
            }
        }

        // ── Add Controls ────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            title: qsTr("Add Controls")

            SettingsRow {
                width: parent.width
                label: qsTr("Add controls\u2026")
                description: qsTr("Add a control to the active page. "
                                  + "Unavailable modules are listed with their "
                                  + "reason and are not addable.")
                settingsId: "addControls"
                owner: "settingsd + shell/control-center"
                settingsKey: "quickSettings.modules"
                showSeparator: false

                controlData: Button {
                    id: addControlsButton
                    objectName: "quickSettingsAddControls"
                    text: qsTr("Add Controls\u2026")
                    onClicked: root.addControlsDialog.show()
                }
            }
        }

        // ── Reset ───────────────────────────────────────────────────────
        SettingsGroup {
            id: resetGroup
            width: parent.width
            title: qsTr("Reset")

            SettingsRow {
                width: parent.width
                label: qsTr("Reset Quick Settings")
                description: qsTr("Restores the modules, pages, names, spans, "
                                  + "menu-bar-only set, per-display policy, and "
                                  + "the menu-bar marks this surface owns.")
                settingsId: "reset"
                owner: "settingsd"
                settingsKey: "quickSettings.modules"
                showSeparator: false

                controlData: Button {
                    id: resetButton
                    objectName: "quickSettingsReset"
                    text: qsTr("Reset")
                    accessibleName: qsTr("Reset Quick Settings")
                    onClicked: root.resetAll()
                }
            }
        }
    }

    // The Add Controls sheet. It is the catalogue, not a second registry: the
    // category list and every row come from ControlCenterRegistry.
    Dialog {
        id: addControlsDialog
        objectName: "quickSettingsAddControlsDialog"
        title: qsTr("Add Controls")
        message: qsTr("Choose a control for %1. A control with no provider "
                      + "is listed with its reason and cannot be added.")
            .arg(root.pageName(root.activePage))
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Select {
                id: addCategorySelect
                objectName: "quickSettingsAddCategory"
                width: parent.width
                model: root.categoryOptions
                currentIndex: {
                    for (var i = 0; i < root.categoryOptions.length; ++i) {
                        if (root.categoryOptions[i].value === root.selectedCategory)
                            return i;
                    }
                    return 0;
                }
                onActivated: (index) => root.selectedCategory =
                    root.categoryOptions[index].value
            }

            Repeater {
                model: root.addableGroups

                delegate: Column {
                    id: addGroupColumn
                    required property var modelData
                    width: parent ? parent.width : 0
                    spacing: 0

                    Text {
                        width: parent.width
                        text: addGroupColumn.modelData.title
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        font.weight: Theme.primitive.font.weightMedium
                        topPadding: Theme.primitive.spacing.sm
                        bottomPadding: Theme.primitive.spacing.xs
                    }

                    Repeater {
                        model: addGroupColumn.modelData.ids

                        delegate: SettingsRow {
                            required property string modelData
                            width: parent.width
                            label: root.moduleLabel(modelData)
                            description: root.moduleOwner(modelData)
                            settingsId: "add-" + modelData
                            owner: root.moduleOwner(modelData)
                            settingsKey: "quickSettings.modules"
                            showSeparator: false

                            controlData: Button {
                                objectName: "quickSettingsAdd-" + modelData
                                text: root.moduleEnabled(modelData)
                                    ? qsTr("Added") : qsTr("Add")
                                enabled: !root.moduleEnabled(modelData)
                                onClicked: root.setModuleEnabled(modelData, true)
                            }
                        }
                    }
                }
            }

            Text {
                width: parent.width
                visible: root.unavailableIds.length > 0
                text: qsTr("Unavailable on this system")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
                topPadding: Theme.primitive.spacing.sm
            }

            Repeater {
                model: root.unavailableIds

                delegate: SettingsRow {
                    required property string modelData
                    width: parent.width
                    label: root.moduleLabel(modelData)
                    description: root.moduleReason(modelData)
                    settingsId: "unavailable-" + modelData
                    owner: "unavailable"
                    settingsKey: ""
                    enabled: false
                    showSeparator: false

                    controlData: Toggle {
                        text: ""
                        checked: false
                        enabled: false
                        accessibleName: qsTr("%1 is unavailable")
                            .arg(root.moduleLabel(modelData))
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

    // Rename a page. Declared inside the pane so the T266 Dialog adopts the
    // window overlay layer instead of clipping inside the detail scroll.
    Dialog {
        id: renameDialog
        objectName: "quickSettingsRenameDialog"
        title: qsTr("Rename Page")
        message: qsTr("Choose a name for this Quick Settings page.")
        dismissible: true

        property int renameIndex: 0

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            TextInput {
                id: renameInput
                objectName: "quickSettingsRenameInput"
                width: parent.width
                height: Theme.controls.button.height
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: renameDialog.reject()
            }
            Button {
                text: qsTr("Rename")
                variant: "primary"
                onClicked: {
                    root.renamePage(renameDialog.renameIndex, renameInput.text);
                    renameDialog.accept();
                }
            }
        }
    }
}
