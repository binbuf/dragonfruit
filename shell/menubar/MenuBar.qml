// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The top menu bar (T-09): anchored chrome surface with reserved zones,
// hosting from left to right the active application's menu (menu-broker,
// T-22; the app name until it lands), system status items (Wi-Fi,
// Bluetooth, volume, battery, clock, Focus/DND, accessibility), the Control
// Center entry point, and a Mission Control button/gesture target.
//
// The shell process owns the Wayland surface and reserved zone; this
// component is the render/interaction core. Data is injected (the shell
// controller maps compositor broadcasts and T-20 adapter events onto these
// properties), so the whole bar is testable headless.
Rectangle {
    id: menuBar

    // --- Data (injected by the shell controller) --------------------------
    property string appName: ""
    property string appId: ""
    // [{ title: "File", items: [ {label, shortcut, type, enabled, ...} ] }]
    property var appMenuModel: []
    // [{ id, icon, label, accessibleName, available, enabled, selected, tint }]
    property var statusItems: []
    property bool showDate: false
    property bool showSeconds: false
    // The compositor reports when the shell/menu surface holds focus; when
    // it is lost the open menu dismisses (FR-3).
    property bool shellFocused: true

    // --- Interaction state ------------------------------------------------
    property int openMenuIndex: -1
    property int hoverMenuIndex: -1

    // Geometry of the open dropdown in window coordinates. The shell process
    // renders the popup into a separate `overlay` chrome surface placed at
    // this rectangle, so the transient menu composites above fullscreen
    // windows while the bar keeps its own `top` surface and reserved zone.
    readonly property var _dropdownRect: {
        if (openMenuIndex < 0)
            return { x: 0, y: 0, w: 0, h: 0 };
        var item = appMenuRepeater.itemAt(openMenuIndex);
        if (!item || !item.popup)
            return { x: 0, y: 0, w: 0, h: 0 };
        var popup = item.popup;
        var topLeft = popup.mapToItem(menuBar, 0, 0);
        return { x: topLeft.x, y: topLeft.y, w: popup.width, h: popup.height };
    }
    readonly property real dropdownX: _dropdownRect.x
    readonly property real dropdownY: _dropdownRect.y
    readonly property real dropdownWidth: _dropdownRect.w
    readonly property real dropdownHeight: _dropdownRect.h

    signal appMenuTriggered(int menuIndex, int itemIndex, var item)
    signal appMenuOpened(int menuIndex)
    signal appMenuClosed()
    signal statusItemActivated(string itemId)
    signal controlCenterRequested()
    signal missionControlRequested()
    signal clockActivated()

    implicitHeight: Theme.controls.menuBar.height
    implicitWidth: appMenuRow.implicitWidth + statusRow.implicitWidth
                   + 2 * Theme.controls.menuBar.paddingH

    color: Theme.color.chrome

    function openMenu(index) {
        if (index < 0 || index >= appMenuRepeater.count || index === openMenuIndex)
            return;
        var previous = openMenuIndex;
        openMenuIndex = index;
        if (previous >= 0) {
            var previousItem = appMenuRepeater.itemAt(previous);
            if (previousItem)
                previousItem.closeMenu();
        }
        var item = appMenuRepeater.itemAt(index);
        if (item)
            item.openMenu();
    }

    function closeMenus() {
        var index = openMenuIndex;
        openMenuIndex = -1;
        hoverMenuIndex = -1;
        switchTimer.stop();
        // Only announce a close when a menu was actually open. A menu item's
        // own handler may already have closed it, and the bar's click-away
        // handler runs afterwards; emitting twice made the dismissal janky.
        if (index >= 0) {
            var item = appMenuRepeater.itemAt(index);
            if (item)
                item.closeMenu();
            appMenuClosed();
        }
    }

    function toggleMenu(index) {
        if (openMenuIndex === index)
            closeMenus();
        else
            openMenu(index);
    }

    // Test/introspection hooks: the shell controller uses these to drive
    // the bar; the QML tests use them to assert without poking at the scene.
    function appMenuAt(index) {
        return appMenuRepeater.itemAt(index);
    }

    function statusItemAt(index) {
        return statusRepeater.itemAt(index);
    }

    // Delayed-hover drag-through: while a menu is open, hovering a sibling
    // top-level title switches to it after a short delay (FR-3).
    function scheduleMenuSwitch(index) {
        if (openMenuIndex >= 0 && openMenuIndex !== index) {
            hoverMenuIndex = index;
            switchTimer.restart();
        } else {
            switchTimer.stop();
        }
    }

    // Live focus-switch tracking: the open menu follows the newly focused
    // application, and closes if that application exports no menu (FR-3).
    function setFocusedApp(name, id, menuModel) {
        var wasOpen = openMenuIndex;
        appName = name;
        appId = id;
        appMenuModel = menuModel || [];
        if (wasOpen >= 0) {
            openMenuIndex = -1;
            if (wasOpen < appMenuModel.length)
                openMenu(wasOpen);
            else
                closeMenus();
        }
    }

    onShellFocusedChanged: {
        if (!shellFocused)
            closeMenus();
    }

    Timer {
        id: switchTimer
        interval: 140
        repeat: false
        onTriggered: {
            if (menuBar.hoverMenuIndex >= 0 && menuBar.openMenuIndex >= 0
                    && menuBar.hoverMenuIndex !== menuBar.openMenuIndex)
                menuBar.openMenu(menuBar.hoverMenuIndex);
        }
    }

    // Tapping empty bar space dismisses an open menu. Taps that land on an
    // app-menu title are ignored here: that title's own handler toggles the
    // menu, and without this guard the click-away would close it immediately.
    TapHandler {
        onTapped: (eventPoint) => {
            const p = eventPoint.position;
            if (p.x >= appMenuRow.x && p.x <= appMenuRow.x + appMenuRow.width
                    && p.y >= appMenuRow.y && p.y <= appMenuRow.y + appMenuRow.height)
                return;
            menuBar.closeMenus();
        }
    }

    Keys.onEscapePressed: (event) => {
        menuBar.closeMenus();
        event.accepted = true;
    }

    Row {
        id: appMenuRow
        objectName: "appMenuRow"
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.menuBar.paddingH
        anchors.verticalCenter: parent.verticalCenter
        height: Theme.controls.menuBarMenu.barHeight

        Text {
            id: appNameText
            objectName: "appName"
            visible: menuBar.appMenuModel.length === 0
            text: menuBar.appName
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.menuBar.fontSize
            font.weight: Theme.primitive.font.weightMedium
            anchors.verticalCenter: parent.verticalCenter
            leftPadding: Theme.controls.menuBarMenu.barPaddingH
            rightPadding: Theme.controls.menuBarMenu.barPaddingH
        }

        Repeater {
            id: appMenuRepeater
            objectName: "appMenuRepeater"
            model: menuBar.appMenuModel

            delegate: MenuBarMenu {
                required property var modelData
                required property int index

                // The shell does not implement Tab navigation across the bar
                // yet; keep the offscreen window from auto-focusing the first
                // title and drawing its FocusRing when it is shown.
                activeFocusOnTab: false
                showFocusRing: false

                title: modelData.title !== undefined ? modelData.title : ""
                model: modelData.items !== undefined ? modelData.items : []

                onTriggered: (itemIndex, item) => menuBar.appMenuTriggered(index, itemIndex, item)
                onOpened: {
                    menuBar.openMenuIndex = index;
                    menuBar.appMenuOpened(index);
                }
                onClosed: {
                    if (menuBar.openMenuIndex === index) {
                        menuBar.openMenuIndex = -1;
                        menuBar.appMenuClosed();
                    }
                }

                HoverHandler {
                    onHoveredChanged: {
                        if (hovered) {
                            menuBar.hoverMenuIndex = index;
                            menuBar.scheduleMenuSwitch(index);
                        }
                    }
                }
            }
        }
    }

    Row {
        id: statusRow
        objectName: "statusRow"
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.menuBar.paddingH
        anchors.verticalCenter: parent.verticalCenter
        height: Theme.controls.menuBar.height
        spacing: Theme.controls.menuBar.statusItemGap

        Repeater {
            id: statusRepeater
            objectName: "statusRepeater"
            model: menuBar.statusItems

            delegate: StatusItem {
                required property var modelData

                itemId: modelData.id !== undefined ? modelData.id : ""
                icon: modelData.icon !== undefined ? modelData.icon : ""
                label: modelData.label !== undefined ? modelData.label : ""
                accessibleName: modelData.accessibleName !== undefined ? modelData.accessibleName : ""
                available: modelData.available !== false
                enabled: modelData.enabled !== false
                selected: modelData.selected === true
                level: modelData.level !== undefined ? modelData.level : 0.8
                tint: modelData.tint !== undefined ? modelData.tint : Theme.color.textPrimary
                backgroundColor: menuBar.color

                onActivated: (id) => {
                    menuBar.closeMenus();
                    menuBar.statusItemActivated(id);
                }
            }
        }

        MenuBarClock {
            objectName: "clock"
            showDate: menuBar.showDate
            showSeconds: menuBar.showSeconds
            onActivated: menuBar.clockActivated()
        }

        StatusItem {
            itemId: "control-center"
            icon: "control-center"
            accessibleName: qsTr("Control Center")
            backgroundColor: menuBar.color
            onActivated: {
                menuBar.closeMenus();
                menuBar.controlCenterRequested();
            }
        }

        StatusItem {
            itemId: "mission-control"
            icon: "mission-control"
            accessibleName: qsTr("Mission Control")
            backgroundColor: menuBar.color
            onActivated: {
                menuBar.closeMenus();
                menuBar.missionControlRequested();
            }
        }
    }
}
