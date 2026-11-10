// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Edit Controls catalogue (T235): a 736×900 pt management surface with a
// 245 pt category sidebar and the measured 22 categories. It is not a second
// settings app: every row writes the same `quickSettings.*` keys the live
// panel reads, and the shell forwards those writes to settingsd.
Item {
    id: root

    // ── Inputs from the shell / Control Center ───────────────────────────
    property var panelModules: []
    property var page2: []
    property var page3: []
    property int pageCount: 1
    property int activePage: 0
    property var menuBarOnly: []
    property var spans: ({})
    property bool perDisplay: false
    // The live module states, keyed by id: `{ available, subtitle }`.
    property var states: ({})

    // ── Outputs ──────────────────────────────────────────────────────────
    signal closed()
    signal quickSettingsWriteRequested(string key, var value)

    readonly property int sidebarWidth: ControlCenterRegistry.editSidebarWidth
    readonly property int windowWidth: ControlCenterRegistry.editWindowWidth
    readonly property int windowHeight: ControlCenterRegistry.editWindowHeight
    readonly property var categoryList: ControlCenterRegistry.categories

    property string selectedCategory: "all-controls"
    property string statusMessage: ""

    // ── Pure helpers (also the test surface) ─────────────────────────────
    function pageList(page) {
        if (page === 1)
            return root.page2 ? Array.from(root.page2) : [];
        if (page === 2)
            return root.page3 ? Array.from(root.page3) : [];
        return root.panelModules ? Array.from(root.panelModules) : [];
    }

    function pageKey(page) {
        if (page === 1)
            return "quickSettings.page2";
        if (page === 2)
            return "quickSettings.page3";
        return "quickSettings.modules";
    }

    function writePage(page, list) {
        // Optimistic local update so the management surface responds before
        // the settingsd echo lands; the echo then converges on the same list.
        if (page === 1)
            root.page2 = list;
        else if (page === 2)
            root.page3 = list;
        else
            root.panelModules = list;
        root.quickSettingsWriteRequested(root.pageKey(page), list);
    }

    function isInPanel(id) {
        for (var page = 0; page < 3; ++page) {
            if (root.pageList(page).indexOf(id) >= 0)
                return true;
        }
        return false;
    }

    function isMenuBarOnly(id) {
        return root.menuBarOnly.indexOf(id) >= 0;
    }

    function moduleState(id) {
        var value = root.states[id];
        return value !== undefined ? value : { available: true, subtitle: "" };
    }

    function spanOf(id) {
        return ControlCenterRegistry.spanFor(id, root.spans);
    }

    function categoryModules(categoryId) {
        return ControlCenterRegistry.categoryModules(categoryId);
    }

    function categoryCount(categoryId) {
        return root.categoryModules(categoryId).length;
    }

    // Add/remove a control on the active page. A control is in at most one
    // page; adding it to the active page removes it from the others.
    function setEnabled(id, enabled) {
        if (id === "edit-controls")
            return;
        var page = root.activePage;
        var list = root.pageList(page);
        var at = list.indexOf(id);
        if (enabled && at < 0) {
            list.push(id);
            root.writePage(page, list);
            for (var other = 0; other < 3; ++other) {
                if (other === page)
                    continue;
                var otherList = root.pageList(other);
                var otherAt = otherList.indexOf(id);
                if (otherAt >= 0) {
                    otherList.splice(otherAt, 1);
                    root.writePage(other, otherList);
                }
            }
            if (root.isMenuBarOnly(id))
                root.setMenuBarOnly(id, false);
        } else if (!enabled && at >= 0) {
            list.splice(at, 1);
            root.writePage(page, list);
        }
    }

    function moveModule(id, delta) {
        var page = root.activePage;
        var list = root.pageList(page);
        var at = list.indexOf(id);
        var to = at + delta;
        if (at < 0 || to < 0 || to >= list.length)
            return;
        list.splice(at, 1);
        list.splice(to, 0, id);
        root.writePage(page, list);
    }

    // Cycle the control through the four measured spans.
    function cycleSpan(id) {
        var next = ControlCenterRegistry.nextSpan(root.spanOf(id));
        var map = {};
        for (var key in root.spans)
            map[key] = root.spans[key];
        map[id] = next;
        root.spans = map;
        root.quickSettingsWriteRequested("quickSettings.spans",
                            ControlCenterRegistry.serializeSpans(map));
        root.statusMessage = qsTr("%1 is now %2")
            .arg(ControlCenterRegistry.moduleLabel(id)).arg(next);
    }

    // The menu-bar pinning path is a separate target from the panel grid
    // (ADR 0222): the toggle removes the control from every panel page and
    // keeps its `menu.control.*` mark live.
    function setMenuBarOnly(id, enabled) {
        var only = root.menuBarOnly ? Array.from(root.menuBarOnly) : [];
        var at = only.indexOf(id);
        if (enabled && at < 0)
            only.push(id);
        else if (!enabled && at >= 0)
            only.splice(at, 1);
        else
            return;
        root.menuBarOnly = only;
        root.quickSettingsWriteRequested("quickSettings.menuBarOnly", only);

        if (enabled) {
            for (var page = 0; page < 3; ++page) {
                var list = root.pageList(page);
                var moduleAt = list.indexOf(id);
                if (moduleAt >= 0) {
                    list.splice(moduleAt, 1);
                    root.writePage(page, list);
                }
            }
        } else if (!root.isInPanel(id)) {
            var page0 = root.pageList(0);
            page0.push(id);
            root.writePage(0, page0);
        }
        var key = ControlCenterRegistry.menuBarKey(id);
        if (key.length > 0)
            root.quickSettingsWriteRequested(key, true);
    }

    function addPage() {
        if (root.pageCount >= 3)
            return;
        var next = root.pageCount + 1;
        root.pageCount = next;
        root.activePage = next - 1;
        root.quickSettingsWriteRequested("quickSettings.pageCount", next);
        root.quickSettingsWriteRequested("quickSettings.activePage", next - 1);
    }

    function removePage(page) {
        if (root.pageCount <= 1 || page <= 0)
            return;
        var last = root.pageCount - 1;
        // Keep pages contiguous: removing page 1 shifts page 2 into page 1.
        if (page === 1 && last === 2) {
            root.writePage(1, root.page3 ? Array.from(root.page3) : []);
        }
        root.writePage(last, []);
        root.pageCount = last;
        root.quickSettingsWriteRequested("quickSettings.pageCount", last);
        if (root.activePage >= last) {
            root.activePage = Math.max(0, last - 1);
            root.quickSettingsWriteRequested("quickSettings.activePage",
                                             Math.max(0, last - 1));
        }
    }

    function selectPage(page) {
        if (page < 0 || page >= root.pageCount)
            return;
        root.activePage = page;
        root.quickSettingsWriteRequested("quickSettings.activePage", page);
    }

    function reset() {
        root.panelModules = ControlCenterRegistry.defaultModules.slice();
        root.page2 = [];
        root.page3 = [];
        root.pageCount = 1;
        root.activePage = 0;
        root.menuBarOnly = [];
        root.spans = ({});
        root.perDisplay = false;
        root.quickSettingsWriteRequested("quickSettings.modules",
                            ControlCenterRegistry.defaultModules.slice());
        root.quickSettingsWriteRequested("quickSettings.page2", []);
        root.quickSettingsWriteRequested("quickSettings.page3", []);
        root.quickSettingsWriteRequested("quickSettings.pageCount", 1);
        root.quickSettingsWriteRequested("quickSettings.activePage", 0);
        root.quickSettingsWriteRequested("quickSettings.menuBarOnly", []);
        root.quickSettingsWriteRequested("quickSettings.spans", []);
        root.quickSettingsWriteRequested("quickSettings.perDisplay", false);
        root.statusMessage = qsTr("Restored the default Quick Settings");
    }

    function selectCategory(id) {
        root.selectedCategory = id;
        controlList.positionViewAtBeginning();
    }

    // ── Scene ────────────────────────────────────────────────────────────
    implicitWidth: root.windowWidth
    implicitHeight: root.windowHeight
    width: root.windowWidth
    height: root.windowHeight

    Accessible.role: Accessible.Dialog
    Accessible.name: qsTr("Edit Controls")

    Rectangle {
        id: surface
        anchors.fill: parent
        radius: Theme.primitive.radius.xxl
        color: Theme.color.surface
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border

        Row {
            anchors.fill: parent
            anchors.margins: Theme.primitive.spacing.md
            spacing: Theme.primitive.spacing.md

            // ── 245 pt category sidebar ──────────────────────────────────
            Rectangle {
                id: sidebar
                objectName: "editControlsSidebar"
                width: root.sidebarWidth
                height: parent.height
                radius: Theme.primitive.radius.lg
                color: Theme.color.surfaceSunken

                Column {
                    anchors.fill: parent
                    anchors.margins: Theme.primitive.spacing.sm
                    spacing: Theme.primitive.spacing.xs

                    Text {
                        text: qsTr("Edit Controls")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeXl
                        font.weight: Theme.primitive.font.weightSemibold
                    }

                    Text {
                        width: parent.width
                        text: qsTr("Choose what appears in Quick Settings.")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }

                    ListView {
                        id: categoryList
                        width: parent.width
                        height: parent.height - y - Theme.primitive.spacing.sm
                        clip: true
                        model: root.categoryList
                        spacing: 2
                        currentIndex: {
                            for (var i = 0; i < root.categoryList.length; ++i) {
                                if (root.categoryList[i].id === root.selectedCategory)
                                    return i;
                            }
                            return 0;
                        }

                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            width: categoryList.width
                            height: 28
                            radius: Theme.primitive.radius.sm
                            color: root.selectedCategory === modelData.id
                                ? Theme.color.accentMuted
                                : categoryHover.hovered ? Theme.color.surfaceElevated
                                                        : "transparent"
                            Accessible.role: Accessible.ListItem
                            Accessible.name: modelData.label
                            Accessible.selected: root.selectedCategory === modelData.id

                            HoverHandler { id: categoryHover }
                            TapHandler {
                                onTapped: root.selectCategory(modelData.id)
                            }

                            Text {
                                anchors.left: parent.left
                                anchors.leftMargin: Theme.primitive.spacing.xs
                                anchors.right: categoryCountText.left
                                anchors.rightMargin: Theme.primitive.spacing.xs
                                anchors.verticalCenter: parent.verticalCenter
                                text: modelData.label
                                color: root.selectedCategory === modelData.id
                                    ? Theme.color.accent : Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                            }

                            Text {
                                id: categoryCountText
                                anchors.right: parent.right
                                anchors.rightMargin: Theme.primitive.spacing.xs
                                anchors.verticalCenter: parent.verticalCenter
                                text: root.categoryCount(modelData.id)
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeXs
                            }
                        }
                    }
                }
            }

            // ── Controls and page management ─────────────────────────────
            Column {
                width: parent.width - sidebar.width - Theme.primitive.spacing.md
                height: parent.height
                spacing: Theme.primitive.spacing.sm

                Row {
                    width: parent.width
                    height: 36

                    Text {
                        text: ControlCenterRegistry.categoryLabel(root.selectedCategory)
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeXl
                        font.weight: Theme.primitive.font.weightSemibold
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Item { width: parent.width - doneButton.width - 260; height: 1 }

                    Button {
                        id: doneButton
                        objectName: "editControlsDone"
                        text: qsTr("Done")
                        accessibleName: qsTr("Close Edit Controls")
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: root.closed()
                    }
                }

                // Page tabs: multiple pages, add/remove, active page.
                Row {
                    id: pageTabs
                    objectName: "editControlsPageTabs"
                    width: parent.width
                    height: 30
                    spacing: Theme.primitive.spacing.xs

                    Repeater {
                        model: root.pageCount
                        delegate: Rectangle {
                            required property int index
                            width: 92
                            height: 28
                            radius: Theme.primitive.radius.sm
                            color: root.activePage === index
                                ? Theme.color.accentMuted : Theme.color.surfaceSunken
                            border.width: root.activePage === index ? 1 : 0
                            border.color: Theme.color.accent
                            Accessible.role: Accessible.PageTab
                            Accessible.name: qsTr("Page %1").arg(index + 1)
                            Accessible.selected: root.activePage === index
                            Text {
                                anchors.centerIn: parent
                                text: qsTr("Page %1").arg(index + 1)
                                color: root.activePage === index
                                    ? Theme.color.accent : Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                            TapHandler {
                                onTapped: root.selectPage(index)
                            }
                        }
                    }

                    Button {
                        objectName: "editControlsAddPage"
                        text: qsTr("+")
                        accessibleName: qsTr("Add a Quick Settings page")
                        enabled: root.pageCount < 3
                        onClicked: root.addPage()
                    }

                    Button {
                        objectName: "editControlsRemovePage"
                        text: qsTr("\u2212")
                        accessibleName: qsTr("Remove this Quick Settings page")
                        enabled: root.activePage > 0
                        onClicked: root.removePage(root.activePage)
                    }

                    Button {
                        objectName: "editControlsReset"
                        text: qsTr("Reset")
                        accessibleName: qsTr("Restore the default Quick Settings")
                        onClicked: root.reset()
                    }
                }

                Text {
                    objectName: "editControlsStatus"
                    width: parent.width
                    height: root.statusMessage.length > 0 ? 18 : 0
                    visible: root.statusMessage.length > 0
                    text: root.statusMessage
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }

                Row {
                    id: perDisplayRow
                    width: parent.width
                    height: 26
                    spacing: Theme.primitive.spacing.sm

                    Toggle {
                        objectName: "editControlsPerDisplay"
                        checked: root.perDisplay
                        accessibleName: qsTr("Show Quick Settings on all displays")
                        anchors.verticalCenter: parent.verticalCenter
                        onToggled: (checked) =>
                            root.quickSettingsWriteRequested("quickSettings.perDisplay",
                                                             checked)
                    }

                    Text {
                        width: parent.width - 60
                        text: qsTr("Show on all displays (per-display surfaces remain OPEN)")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }

                ListView {
                    id: controlList
                    objectName: "editControlsList"
                    width: parent.width
                    height: parent.height - y
                    clip: true
                    model: root.categoryModules(root.selectedCategory)
                    spacing: Theme.primitive.spacing.xxs

                    delegate: Rectangle {
                        id: controlRow
                        required property string modelData
                        width: controlList.width
                        height: 54
                        radius: Theme.primitive.radius.md
                        color: rowHover.hovered ? Theme.color.surfaceElevated
                                                : Theme.color.surfaceSunken

                        readonly property var state: root.moduleState(modelData)
                        readonly property bool inPanel: root.isInPanel(modelData)
                        readonly property bool menuBar: root.isMenuBarOnly(modelData)
                        readonly property string owner:
                            ControlCenterRegistry.module(modelData) !== null
                                ? String(ControlCenterRegistry.module(modelData).owner)
                                : ""

                        HoverHandler { id: rowHover }

                        Row {
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.sm

                            Icon {
                                name: ControlCenterRegistry.moduleGlyph(modelData)
                                size: 20
                                color: Theme.color.textPrimary
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Column {
                                width: parent.width - 300
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 1

                                Text {
                                    text: ControlCenterRegistry.moduleLabel(modelData)
                                    color: Theme.color.textPrimary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    font.weight: Theme.primitive.font.weightMedium
                                }

                                Text {
                                    width: parent.width
                                    text: controlRow.owner.length > 0
                                        ? controlRow.owner : qsTr("Unavailable on this system")
                                    color: controlRow.state.available === false
                                        ? Theme.color.textTertiary
                                        : Theme.color.textSecondary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    elide: Text.ElideRight
                                }
                            }

                            Button {
                                objectName: "editControlsSpan-" + modelData
                                text: root.spanOf(modelData) === "footer"
                                    ? qsTr("\u2014") : root.spanOf(modelData)
                                accessibleName: qsTr("Resize %1, currently %2")
                                    .arg(ControlCenterRegistry.moduleLabel(modelData))
                                    .arg(root.spanOf(modelData))
                                anchors.verticalCenter: parent.verticalCenter
                                onClicked: root.cycleSpan(modelData)
                            }

                            Toggle {
                                objectName: "editControlsPanel-" + modelData
                                checked: controlRow.inPanel
                                enabled: modelData !== "edit-controls"
                                accessibleName: qsTr("Show %1 in Quick Settings")
                                    .arg(ControlCenterRegistry.moduleLabel(modelData))
                                anchors.verticalCenter: parent.verticalCenter
                                onToggled: (checked) => root.setEnabled(modelData, checked)
                            }

                            Toggle {
                                objectName: "editControlsMenuBar-" + modelData
                                visible: ControlCenterRegistry.menuBarKey(modelData).length > 0
                                checked: controlRow.menuBar
                                accessibleName: qsTr("Keep %1 in the menu bar only")
                                    .arg(ControlCenterRegistry.moduleLabel(modelData))
                                anchors.verticalCenter: parent.verticalCenter
                                onToggled: (checked) => root.setMenuBarOnly(modelData, checked)
                            }
                        }
                    }
                }
            }
        }
    }
}
