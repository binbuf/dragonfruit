// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// The Files tab bar (T333): the measured 702×28 pt row below the unified
// toolbar, one tab per browsing context plus a trailing disclosure button.
//
// The bar owns no state. It paints `tabs` (`{ id, title }`) and reports intent:
// `tabSelected(index)` on a click, `tabCloseRequested(index)` from a tab's
// close affordance, `newTabRequested()` from the disclosure button, and
// `contextRequested(index, x, y)` on a right-click. The shell's `FilesTabs`
// model is the source of truth, so the window title, the breadcrumb and the
// contents always follow the active tab. Every rule here is state-free
// (entries, the width band, the wrapping step), so `tst_files_shell` tests it
// through a real bar instance.
//
// Keyboard: the bar is one focus stop. Left/Right move the selection, and
// Return/Space on a close affordance would need a second focus stop — the
// shell's Cmd+1…9 / Ctrl+Tab / Cmd+W chords are the keyboard path instead.
// Pointer: clicking a background tab selects it; clicking the active tab
// closes it (a tab strip affordance, like a segmented control's re-click); the
// x on each tab closes that tab. A right-click on a tab or on "New Tab..."
// opens the shell's tab menu (index -1 for the strip's end).
Rectangle {
    id: root

    // `{ id, title }` per tab, left to right.
    property var tabs: []
    property int currentIndex: -1
    // Whether the trailing New Tab button is offered.
    property bool newTabEnabled: true
    property string accessibleName: qsTr("Tabs")

    signal tabSelected(int index)
    signal tabCloseRequested(int index)
    signal newTabRequested()
    // A right-click on a tab (the tab menu) or on the trailing New Tab button,
    // in bar-local coordinates.
    signal contextRequested(int index, real x, real y)

    readonly property real tabHeight: Theme.controls.accessoryBar.tab
    // The tab model exposes the close affordance only when there is more than
    // one tab (closing the only tab would close the window).
    readonly property bool canClose: tabs.length > 1
    // One entry per tab, `{ index, id, title, current }`; the delegates and the
    // tests both read it.
    readonly property var entries: {
        var out = [];
        for (var i = 0; i < root.tabs.length; ++i) {
            out.push({ index: i, id: root.tabs[i].id,
                       title: root.tabs[i].title, current: i === root.currentIndex });
        }
        return out;
    }
    // The tabs share the strip's free width (the trailing New Tab button is
    // one tab slot), between 80 pt and 240 pt: 240 keeps a long folder name
    // readable, 80 keeps the close affordance tappable. No tabs is no width.
    readonly property real tabWidth: root.tabs.length === 0 ? 0
                                    : Math.max(80, Math.min(240,
                                               Math.max(0, root.width - root.tabHeight)
                                               / root.tabs.length))

    implicitHeight: root.tabHeight
    implicitWidth: tabRow.implicitWidth + tabHeight

    function select(index) {
        if (index < 0 || index >= root.tabs.length)
            return false;
        root.tabSelected(index);
        return true;
    }

    // Left/Right step to the neighbouring tab (wrapping): Files has no
    // disabled tab.
    function step(delta) {
        var count = root.tabs.length;
        if (count <= 1)
            return false;
        var from = root.currentIndex < 0 ? 0 : root.currentIndex;
        var next = ((from + delta) % count + count) % count;
        root.select(next);
        return true;
    }

    activeFocusOnTab: true
    color: Material.appChromeFill("filesTabBar", Theme.color.surfaceMuted)
    antialiasing: true

    Row {
        id: tabRow
        objectName: "filesTabRow"
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 0

        Repeater {
            id: tabRepeater
            objectName: "filesTabRepeater"
            model: root.entries

            delegate: Item {
                id: tabItem
                required property var modelData
                required property int index

                readonly property bool current: tabItem.modelData.current
                readonly property bool hovered: tabHover.hovered

                width: root.tabWidth
                height: root.tabHeight

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 3
                    radius: Theme.primitive.radius.sm
                    color: tabItem.current ? Theme.color.surfaceElevated
                          : (tabItem.hovered ? Theme.color.controlFill
                                             : "transparent")
                    border.width: tabItem.current ? Theme.controls.window.borderWidth : 0
                    border.color: Theme.color.border
                    antialiasing: true
                }

                Text {
                    id: tabLabel
                    anchors.left: parent.left
                    anchors.right: closeButton.visible ? closeButton.left : parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: Theme.primitive.spacing.md
                    anchors.rightMargin: Theme.primitive.spacing.xs
                    text: tabItem.modelData.title
                    elide: Text.ElideMiddle
                    color: tabItem.current ? Theme.color.textPrimary
                                           : Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    font.weight: tabItem.current ? Theme.primitive.font.weightMedium
                                                 : Theme.primitive.font.weightRegular
                }

                Icon {
                    id: closeButton
                    objectName: "tabClose" + tabItem.index
                    visible: root.canClose
                    name: "x"
                    size: 10
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    anchors.verticalCenter: parent.verticalCenter
                    color: closeHover.hovered ? Theme.color.textPrimary
                                              : Theme.color.textTertiary
                    Accessible.role: Accessible.Button
                    Accessible.name: qsTr("Close tab %1").arg(tabItem.modelData.title)

                    TapHandler {
                        onTapped: root.tabCloseRequested(tabItem.index)
                    }
                    HoverHandler {
                        id: closeHover
                    }
                }

                // A background tab is selected; the active tab closes on a
                // re-click (the tab strip is also a control). The right button
                // opens the tab menu at the click point.
                TapHandler {
                    acceptedButtons: Qt.RightButton
                    onTapped: (eventPoint) => {
                        var p = tabItem.mapToItem(root, eventPoint.position.x,
                                                  eventPoint.position.y);
                        root.contextRequested(tabItem.index, p.x, p.y);
                    }
                }

                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    onTapped: {
                        if (tabItem.current)
                            root.tabCloseRequested(tabItem.index);
                        else
                            root.tabSelected(tabItem.index);
                    }
                }

                HoverHandler {
                    id: tabHover
                }

                Accessible.role: Accessible.PageTab
                Accessible.name: tabItem.modelData.title
                Accessible.checkable: true
                Accessible.checked: tabItem.current
                Accessible.onPressAction: root.tabSelected(tabItem.index)
            }
        }
    }

    Item {
        id: newTabButton
        objectName: "filesNewTabButton"
        width: root.tabHeight
        height: root.tabHeight
        anchors.left: tabRow.right
        anchors.verticalCenter: parent.verticalCenter
        opacity: root.newTabEnabled ? 1.0 : 0.4

        // Tabler has no bare plus in the vendored vocabulary; Phosphor does
        // (the T163 rule: a glyph is a resource, the component styles it).
        PhosphorIcon {
            anchors.centerIn: parent
            name: "plus"
            size: 12
            color: newTabHover.hovered ? Theme.color.textPrimary
                                       : Theme.color.textSecondary
        }

        HoverHandler {
            id: newTabHover
        }

        TapHandler {
            acceptedButtons: Qt.RightButton
            onTapped: (eventPoint) => {
                var p = parent.mapToItem(root, eventPoint.position.x,
                                         eventPoint.position.y);
                root.contextRequested(-1, p.x, p.y);
            }
        }

        TapHandler {
            acceptedButtons: Qt.LeftButton
            onTapped: {
                if (root.newTabEnabled)
                    root.newTabRequested();
            }
        }

        Accessible.role: Accessible.Button
        Accessible.name: qsTr("New Tab")
        Accessible.onPressAction: root.newTabRequested()
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Left) {
            root.step(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Right) {
            root.step(1);
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.PageTabList
    Accessible.name: root.accessibleName
}
