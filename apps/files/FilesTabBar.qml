// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files tab bar (T333): the measured 702×28 pt row below the unified
// toolbar, one tab per browsing context plus a trailing disclosure button.
//
// The bar owns no state. It paints `tabs` (`{ id, title }`) and reports intent:
// `tabSelected(index)` on a click, `tabCloseRequested(index)` from a tab's
// close affordance, and `newTabRequested()` from the disclosure button. The
// shell's `FilesTabs` model is the source of truth, so the window title, the
// breadcrumb and the contents always follow the active tab.
//
// Keyboard: the bar is one focus stop. Left/Right move the selection, and
// Return/Space on a close affordance would need a second focus stop — the
// shell's Cmd+1…9 / Ctrl+Tab / Cmd+W chords are the keyboard path instead.
// Pointer: clicking a background tab selects it; clicking the active tab
// closes it (a tab strip affordance, like a segmented control's re-click); the
// x on each tab closes that tab. A right-click on a tab or on "New Tab..." opens
// the shell's tab menu (index -1 for the strip's end).
Item {
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
    // A right-click on a tab (the tab menu) or on "New Tab..." at the end of
    // the strip, in bar-local coordinates.
    signal contextRequested(int index, real x, real y)

    readonly property real tabHeight: Theme.controls.accessoryBar.tab
    readonly property real tabMaxWidth: 240
    readonly property real tabMinWidth: 80
    // The tab model exposes the close affordance only when there is more than
    // one tab (closing the only tab would close the window).
    readonly property bool canClose: tabs.length > 1
    // One entry per tab, `{ index, id, title, current }`; the delegates and the
    // tests both read it.
    readonly property var entries: {
        var out = [];
        for (var i = 0; i < root.tabs.length; ++i) {
            var tab = root.tabs[i];
            out.push({ index: i, id: tab.id, title: tab.title,
                       current: i === root.currentIndex });
        }
        return out;
    }

    implicitHeight: root.tabHeight
    implicitWidth: tabRow.implicitWidth

    function select(index) {
        if (index < 0 || index >= root.tabs.length)
            return false;
        root.tabSelected(index);
        return true;
    }

    // Left/Right step to the neighbouring tab (wrapping), skipping nothing:
    // Files has no disabled tab.
    function step(delta) {
        if (root.tabs.length === 0)
            return;
        var count = root.tabs.length;
        var next = ((root.currentIndex + delta) % count + count) % count;
        root.select(next);
    }

    activeFocusOnTab: true

    Rectangle {
        anchors.fill: parent
        color: Material.appChromeFill("filesTabBar", Theme.color.surfaceMuted)
    }

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

                width: root.tabs.length === 0 ? 0
                     : Math.max(root.tabMinWidth,
                                Math.min(root.tabMaxWidth,
                                         (Math.max(0, root.width - newTabButton.width))
                                         / Math.max(1, root.tabs.length)))
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
                    name: "close"
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
