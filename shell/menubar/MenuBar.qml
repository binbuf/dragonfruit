// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The top menu bar (T-09): anchored chrome surface with reserved zones. From
// left to right it hosts the always-present system menu (the dragonfruit mark,
// T-09), the always-present application menu (the focused app's name and its
// About/Settings/Hide/Quit items; Files on the empty desktop), the focused
// application's own menus (menu-broker, T-22), system status items (Wi-Fi,
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
    // The fixed system menu (brand mark): About/Settings/App Store/Sleep/…
    // [{ label, shortcut, type, enabled, action }]
    property var systemMenuItems: []
    // The fixed application menu: About <App>/Settings/Hide/Quit, synthesized
    // from the focused app (or Files on the desktop) until T-22 owns it.
    property var applicationMenuItems: []
    // The focused application's exported top-level menus.
    // [{ title: "File", items: [ {label, shortcut, type, enabled, action} ] }]
    property var appMenuModel: []
    // T-14.2b: the global application-menu toggle (`menu.global`). When off,
    // the focused app's exported menus are not shown globally so first-party
    // apps keep their own local menu presentation; the fixed system and
    // application menus stay.
    property bool globalMenuEnabled: true
    // [{ id, icon, iconSource, label, accessibleName, available, enabled,
    //    selected, tint }]; third-party StatusNotifier items join this same
    //    row with `id: "tray:<name>"` and an `iconSource`, so they get
    //    identical sizing/hover treatment (T-14.3).
    property var statusItems: []
    // The open tray item's DBusMenu rows (T-14.3), a design-system menu model.
    property var trayMenu: []
    property string trayMenuService: ""
    property bool trayMenuOpen: false
    // The bridge host's decoded status views (T-07.5a/T-07.5b); empty until
    // the shell applies them, which hides the corresponding item.
    property var wifiMenu: ({})
    property var volumeMenu: ({})
    property var batteryMenu: ({})
    // Which status popover is open: "" | "wifi" | "volume" | "battery".
    property string openStatusItem: ""
    // Keyboard navigation (T-07.5b): the status slot the arrow keys select.
    // -1 means no slot is selected.
    property int keyboardStatusIndex: -1
    readonly property bool statusMenuOpen: openStatusItem !== ""
    property bool showDate: false
    property bool showSeconds: false
    // T-15.9b: `menu.showBackground`. When off the bar keeps its items and its
    // reserved zone but draws no chrome material, so the desktop shows through.
    property bool showBackground: true
    // The compositor reports when the shell/menu surface holds focus; when
    // it is lost the open menu dismisses (FR-3).
    property bool shellFocused: true

    // The bar's top-level menus, left to right: the fixed system menu, the
    // fixed application menu, then whatever the focused app exported. Keeping
    // them in one model is what lets drag-through and the open-menu-tracks-
    // focus rules span all three groups. `kind` selects the rendering
    // treatment ("system" -> brand mark, "app" -> bold title).
    readonly property var topLevelMenus: {
        var out = [{ kind: "system", title: qsTr("System"), items: systemMenuItems },
                   { kind: "app", title: appName, items: applicationMenuItems }];
        // The global-menu toggle (T-14.2b) suppresses the app's exported menus
        // only; the fixed system and application menus are always present.
        if (!globalMenuEnabled)
            return out;
        for (var i = 0; i < appMenuModel.length; ++i) {
            var m = appMenuModel[i] || {};
            out.push({ kind: "menu", title: m.title || "", items: m.items || [] });
        }
        return out;
    }

    // --- Interaction state ------------------------------------------------
    property int openMenuIndex: -1
    property int hoverMenuIndex: -1

    // Geometry of the open dropdown in window coordinates. The shell process
    // renders the popup into a separate `overlay` chrome surface placed at
    // this rectangle, so the transient menu composites above fullscreen
    // windows while the bar keeps its own `top` surface and reserved zone.
    readonly property var _dropdownRect: {
        if (openStatusItem !== "") {
            var statusPopup = openStatusItem === "wifi" ? wifiMenuPopup
                            : (openStatusItem === "volume" ? volumeMenuPopup
                            : (openStatusItem === "battery" ? batteryMenuPopup : null));
            if (statusPopup && statusPopup.popup.open) {
                var statusOrigin = statusPopup.mapToItem(menuBar, statusPopup.popup.x,
                                                         statusPopup.popup.y);
                return { x: statusOrigin.x, y: statusOrigin.y,
                         w: statusPopup.popup.width, h: statusPopup.popup.height };
            }
        }
        if (trayMenuOpen && trayContextMenu.open) {
            var trayRect = trayContextMenu.contentRect;
            var trayOrigin = trayContextMenu.mapToItem(menuBar, trayRect.x, trayRect.y);
            return { x: trayOrigin.x, y: trayOrigin.y, w: trayRect.w, h: trayRect.h };
        }
        if (openMenuIndex < 0)
            return { x: 0, y: 0, w: 0, h: 0 };
        var item = menuRepeater.itemAt(openMenuIndex);
        if (!item || !item.popup)
            return { x: 0, y: 0, w: 0, h: 0 };
        // `contentRect` is the dropdown unioned with any open submenu, so the
        // overlay surface grows to contain the nested panel instead of
        // clipping it.
        var rect = item.contentRect;
        if (rect && rect.w > 0 && rect.h > 0) {
            var origin = item.mapToItem(menuBar, rect.x, rect.y);
            return { x: origin.x, y: origin.y, w: rect.w, h: rect.h };
        }
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
    // A row of an open StatusNotifier tray menu was chosen; the shell forwards
    // it to the owning item's DBusMenu (T-14.3).
    signal trayMenuTriggered(string service, int id)
    signal trayMenuClosed()
    // Wi-Fi and volume popovers (T-07.5a). The bar raises the user's action;
    // the shell controller forwards it to the bridge host.
    signal wifiJoinRequested(string ssid, string secret)
    signal volumeSetRequested(double volume)
    signal muteToggleRequested()
    signal statusMenuRefreshRequested(string itemId)
    signal statusMenuOpened()
    signal statusMenuClosed()
    signal controlCenterRequested()
    signal missionControlRequested()
    signal clockActivated()

    implicitHeight: Theme.controls.menuBar.height
    implicitWidth: appMenuRow.implicitWidth + statusRow.implicitWidth
                   + 2 * Theme.controls.menuBar.paddingH

    color: menuBar.showBackground ? Theme.color.chrome : "transparent"

    function openMenu(index) {
        if (index < 0 || index >= menuRepeater.count || index === openMenuIndex)
            return;
        var previous = openMenuIndex;
        openMenuIndex = index;
        if (previous >= 0) {
            var previousItem = menuRepeater.itemAt(previous);
            if (previousItem)
                previousItem.closeMenu();
        }
        var item = menuRepeater.itemAt(index);
        if (item)
            item.openMenu();
    }

    function closeMenus() {
        closeTrayMenu();
        var index = openMenuIndex;
        openMenuIndex = -1;
        hoverMenuIndex = -1;
        switchTimer.stop();
        // Only announce a close when a menu was actually open. A menu item's
        // own handler may already have closed it, and the bar's click-away
        // handler runs afterwards; emitting twice made the dismissal janky.
        if (index >= 0) {
            var item = menuRepeater.itemAt(index);
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
        return menuRepeater.itemAt(index);
    }

    function statusItemAt(index) {
        return statusRepeater.itemAt(index);
    }

    // The live status-item delegate for `id` ("wifi"/"volume"), or null before
    // the Repeater has produced it. Used to anchor the popovers.
    function statusItemFor(id) {
        for (var i = 0; i < statusRepeater.count; ++i) {
            var item = statusRepeater.itemAt(i);
            if (item && item.itemId === id)
                return item;
        }
        return null;
    }

    // The live tray StatusItem for a registered name ("tray:<name>"), or null.
    function trayItemFor(service) {
        return statusItemFor("tray:" + service);
    }

    // Open a tray item's DBusMenu beneath its slot, right-aligned so it stays
    // inside the bar when the slot is near the right edge. The shell sets
    // `trayMenu` before calling this.
    function openTrayMenu(service) {
        if (!service || service.length === 0)
            return;
        trayMenuService = service;
        closeMenus();
        closeStatusMenu();
        trayMenuOpen = true;
        var slot = trayItemFor(service);
        var menuWidth = Math.max(trayContextMenu.width, Theme.controls.contextMenu.minWidth);
        var anchorX = slot ? slot.mapToItem(menuBar, slot.width, 0).x : menuBar.width;
        var left = Math.max(0, Math.min(menuBar.width - menuWidth, anchorX - menuWidth));
        trayContextMenu.showAt(left, menuBar.height);
    }

    function closeTrayMenu() {
        if (!trayMenuOpen)
            return;
        trayMenuOpen = false;
        trayContextMenu.hide();
        trayMenuClosed();
    }

    // Open the popover for a status item. A hidden item (daemon absent, or a
    // machine with no battery) has nothing to open, matching the graceful-
    // degradation rule.
    function openStatusMenu(id) {
        if (id !== "wifi" && id !== "volume" && id !== "battery")
            return;
        var data = id === "wifi" ? wifiMenu
                 : (id === "volume" ? volumeMenu : batteryMenu);
        if (!data || data.state === "unavailable")
            return;
        if (id === "battery" && data.present === false)
            return;
        closeMenus();
        closeStatusMenu();
        openStatusItem = id;
        if (id === "wifi")
            wifiMenuPopup.open = true;
        else if (id === "volume")
            volumeMenuPopup.open = true;
        else
            batteryMenuPopup.open = true;
        statusMenuRefreshRequested(id);
        statusMenuOpened();
    }

    function closeStatusMenu() {
        // `openStatusItem` is the single owner of "a status popover is open".
        // Guarding on it (not on the popups) makes the re-entrant call from a
        // popup's own `closed` signal a no-op instead of double-emitting.
        if (openStatusItem === "")
            return;
        openStatusItem = "";
        wifiMenuPopup.open = false;
        volumeMenuPopup.open = false;
        batteryMenuPopup.open = false;
        statusMenuClosed();
        keyboardStatusIndex = -1;
        // Return keyboard focus to the bar so the arrow keys keep driving the
        // status row after a popover closes.
        forceActiveFocus();
    }

    // --- Keyboard navigation (T-07.5b) ------------------------------------
    // The bar keeps focus; Left/Right move a ring across the focusable status
    // slots and Return/Space opens the selected slot's popover (or activates
    // it). Escape closes. This is the keyboard path to every status menu.
    function moveStatusFocus(delta) {
        var n = statusRepeater.count;
        if (n <= 0) {
            keyboardStatusIndex = -1;
            return;
        }
        var i = keyboardStatusIndex < 0 ? (delta > 0 ? -1 : 0) : keyboardStatusIndex;
        for (var step = 0; step < n; ++step) {
            i = ((i + delta) % n + n) % n;
            var item = statusRepeater.itemAt(i);
            if (item && item.available) {
                keyboardStatusIndex = i;
                return;
            }
        }
        keyboardStatusIndex = -1;
    }

    function activateKeyboardStatus() {
        if (keyboardStatusIndex < 0)
            return;
        var item = statusRepeater.itemAt(keyboardStatusIndex);
        if (!item)
            return;
        if (item.itemId === "wifi" || item.itemId === "volume" || item.itemId === "battery") {
            openStatusMenu(item.itemId);
        } else {
            closeMenus();
            closeStatusMenu();
            statusItemActivated(item.itemId);
        }
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
            if (wasOpen < topLevelMenus.length)
                openMenu(wasOpen);
            else
                closeMenus();
        }
    }

    onShellFocusedChanged: {
        if (!shellFocused) {
            closeMenus();
            closeStatusMenu();
            closeTrayMenu();
        }
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
            // Ignore taps on the app-menu row and the status row: those items
            // own their tap (open/toggle), and without this guard the click-
            // away handler would close the surface it just opened.
            if (p.x >= appMenuRow.x && p.x <= appMenuRow.x + appMenuRow.width
                    && p.y >= appMenuRow.y && p.y <= appMenuRow.y + appMenuRow.height)
                return;
            if (p.x >= statusRow.x && p.x <= statusRow.x + statusRow.width
                    && p.y >= statusRow.y && p.y <= statusRow.y + statusRow.height)
                return;
            menuBar.closeMenus();
            menuBar.closeStatusMenu();
            menuBar.closeTrayMenu();
        }
    }

    Keys.onEscapePressed: (event) => {
        menuBar.closeMenus();
        menuBar.closeStatusMenu();
        menuBar.closeTrayMenu();
        event.accepted = true;
    }

    // Keyboard navigation across the status row (T-07.5b). Left/Right move
    // the selection ring; Return/Space opens the selected item's menu (or
    // raises its action). A popup that holds focus handles its own keys.
    Keys.onLeftPressed: (event) => {
        if (!menuBar.statusMenuOpen) {
            menuBar.moveStatusFocus(-1);
            event.accepted = true;
        }
    }

    Keys.onRightPressed: (event) => {
        if (!menuBar.statusMenuOpen) {
            menuBar.moveStatusFocus(1);
            event.accepted = true;
        }
    }

    Keys.onReturnPressed: (event) => {
        if (!menuBar.statusMenuOpen) {
            menuBar.activateKeyboardStatus();
            event.accepted = true;
        }
    }

    Keys.onSpacePressed: (event) => {
        if (!menuBar.statusMenuOpen) {
            menuBar.activateKeyboardStatus();
            event.accepted = true;
        }
    }

    Row {
        id: appMenuRow
        objectName: "appMenuRow"
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.menuBar.paddingH
        anchors.verticalCenter: parent.verticalCenter
        height: Theme.controls.menuBarMenu.barHeight

        Repeater {
            id: menuRepeater
            objectName: "menuRepeater"
            model: menuBar.topLevelMenus

            delegate: MenuBarMenu {
                required property var modelData
                required property int index

                // The shell does not implement Tab navigation across the bar
                // yet; keep the offscreen window from auto-focusing the first
                // title and drawing its FocusRing when it is shown.
                activeFocusOnTab: false
                showFocusRing: false

                showLogo: modelData.kind === "system"
                emphasized: modelData.kind === "app"
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
                required property int index

                keyboardFocus: menuBar.keyboardStatusIndex === index
                itemId: modelData.id !== undefined ? modelData.id : ""
                icon: modelData.icon !== undefined ? modelData.icon : ""
                iconSource: modelData.iconSource !== undefined ? modelData.iconSource : ""
                label: modelData.label !== undefined ? modelData.label : ""
                accessibleName: modelData.accessibleName !== undefined ? modelData.accessibleName : ""
                available: modelData.available !== false
                enabled: modelData.enabled !== false
                selected: modelData.selected === true
                level: modelData.level !== undefined ? modelData.level : 0.8
                tint: modelData.tint !== undefined ? modelData.tint : Theme.color.textPrimary
                backgroundColor: menuBar.color

                onActivated: (id) => {
                    if (id === "wifi" || id === "volume" || id === "battery") {
                        menuBar.openStatusMenu(id);
                        return;
                    }
                    menuBar.closeMenus();
                    menuBar.closeStatusMenu();
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
                menuBar.closeStatusMenu();
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
                menuBar.closeStatusMenu();
                menuBar.missionControlRequested();
            }
        }
    }

    // The Wi-Fi and volume popovers (T-07.5a). They live in the bar scene,
    // anchored to their status item; the shell commits the union rectangle to
    // the `overlay` popup surface (see `_dropdownRect`).
    WifiMenu {
        id: wifiMenuPopup
        objectName: "wifiMenu"
        model: menuBar.wifiMenu
        anchorItem: menuBar.statusItemFor("wifi")
        onJoinRequested: (ssid, secret) => menuBar.wifiJoinRequested(ssid, secret)
        onRefreshRequested: menuBar.statusMenuRefreshRequested("wifi")
        onClosed: menuBar.closeStatusMenu()
    }

    VolumeMenu {
        id: volumeMenuPopup
        objectName: "volumeMenu"
        model: menuBar.volumeMenu
        anchorItem: menuBar.statusItemFor("volume")
        onVolumeSetRequested: (volume) => menuBar.volumeSetRequested(volume)
        onMuteToggleRequested: menuBar.muteToggleRequested()
        onRefreshRequested: menuBar.statusMenuRefreshRequested("volume")
        onClosed: menuBar.closeStatusMenu()
    }

    BatteryMenu {
        id: batteryMenuPopup
        objectName: "batteryMenu"
        model: menuBar.batteryMenu
        anchorItem: menuBar.statusItemFor("battery")
        onRefreshRequested: menuBar.statusMenuRefreshRequested("battery")
        onClosed: menuBar.closeStatusMenu()
    }

    // A StatusNotifier tray item's DBusMenu (T-14.3). It reuses the
    // design-system ContextMenu, so tray menus and the Dock's context menus
    // cannot diverge; the shell commits its `contentRect` as the overlay
    // popup rectangle (see `_dropdownRect`).
    ContextMenu {
        id: trayContextMenu
        objectName: "trayContextMenu"
        model: menuBar.trayMenu
        accessibleName: qsTr("Tray menu")
        onTriggered: (index, item) => {
            var id = (item && item.id !== undefined) ? item.id : -1;
            menuBar.trayMenuTriggered(menuBar.trayMenuService, id);
        }
        onClosed: menuBar.closeTrayMenu()
    }
}
