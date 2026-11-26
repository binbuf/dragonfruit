// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T266 shared-component tests: row bounds and long strings, the group's final
// separator and gap, focus rings and disclosure activation, accessible
// names/values, Select popup placement/clamping/scrolling, Dialog fit/focus
// trap/Close action, ScrollView modes and track clicks, and the stationary-
// click/scroll-under-pointer harness. Headless offscreen + software scene
// graph; the deterministic settings fixture keeps the shell off the host bus.
Item {
    id: stage
    width: 1000
    height: 1400

    // One window overlay for the whole test window. Dialogs and Selects adopt
    // it without a pane-local popupParent.
    WindowOverlay { id: stageOverlay }

    Component {
        id: rowComponent
        SettingsRow {
            width: 500
            label: "Barrierefreiheit und Bedienungshilfen für Menschen mit besonderen Anforderungen an die Darstellung"
            description: "Diese Beschreibung ist absichtlich sehr lang, damit die Zeile über mehrere Zeilen umbricht und das Steuerelement trotzdem vollständig sichtbar bleibt."
            settingsId: "component.long"
            owner: "settingsd"
            settingsKey: "component.long"
            disclosure: true
            trailingAction: "Details..."
            help: "Component help"
            controlData: Toggle { checked: true }
        }
    }

    Component {
        id: groupComponent
        SettingsGroup {
            width: 400
            title: "A group title that is long enough to need wrapping at the card width"

            SettingsRow { width: parent ? parent.width : 0; label: "First" }
            SettingsRow { width: parent ? parent.width : 0; label: "Second" }
            SettingsRow { width: parent ? parent.width : 0; label: "Third" }
        }
    }

    Component {
        id: sliderRowComponent
        SettingsRow {
            width: 500
            label: "Pointer speed"
            controlData: Slider { from: 0; to: 10; value: 4; stepSize: 1 }
        }
    }

    Component {
        id: selectComponent
        Select {
            width: 160
            model: ["Automatic", "Always", "When scrolling", "Never"]
        }
    }

    Component {
        id: longSelectComponent
        Select {
            width: 160
            model: [
                "Automatic", "Always", "When scrolling", "Never",
                "Only when the content overflows the viewport",
                "Jump to the spot that is clicked",
                "Jump to the next page",
                "A very long localized option label that still fits",
                "Option 9", "Option 10", "Option 11", "Option 12",
                "Option 13", "Option 14", "Option 15", "Option 16",
                "Option 17", "Option 18", "Option 19", "Option 20",
                "Option 21", "Option 22", "Option 23", "Option 24"
            ]
        }
    }

    Component {
        id: dialogComponent
        Dialog {
            id: dialogComponentRoot
            title: "A dialog title that is long enough to need a stable width"
            message: "This message is deliberately long so the dialog must fit the window and scroll its body instead of growing past the frame."
            contentData: Column {
                width: parent ? parent.width : 0
                spacing: Theme.primitive.spacing.sm
                Repeater {
                    model: 30
                    delegate: Text {
                        width: parent ? parent.width : 0
                        text: "Row " + (index + 1) + " of a long dialog body that must scroll."
                        wrapMode: Text.Wrap
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }
            buttonsData: Row {
                Button { text: "Cancel"; onClicked: dialogComponentRoot.reject() }
                Button { text: "Save"; variant: "primary"; onClicked: dialogComponentRoot.accept() }
            }
        }
    }

    Component {
        id: hostComponent
        Item {
            width: 720
            height: 480
            property alias overlayer: overlayItem
            WindowOverlay { id: overlayItem }
        }
    }

    // A control under a closed Dialog and a closed Sheet. Both modal roots
    // keep their cards laid out for the close fade, so a closed modal must be
    // input-disabled or its invisible card/scroll body shadows the control.
    Component {
        id: closedModalHostComponent
        Item {
            width: 720
            height: 480
            property alias overlayer: modalOverlay
            property alias select: modalSelect
            property alias dialog: modalDialog
            property alias sheet: modalSheet

            WindowOverlay { id: modalOverlay }

            Select {
                id: modalSelect
                width: 160
                anchors.centerIn: parent
                model: ["Default", "Violet", "Blue"]
            }

            Dialog {
                id: modalDialog
                title: "Closed dialog"
                message: "The card is laid out while the dialog is closed."
                contentData: Item { width: 300; height: 220 }
                buttonsData: Button { text: "OK" }
            }

            Sheet {
                id: modalSheet
                title: "Closed sheet"
                message: "The card is laid out while the sheet is closed."
                contentData: Item { width: 300; height: 220 }
                buttonsData: Button { text: "OK" }
            }
        }
    }

    Component {
        id: scrollComponent
        ScrollView {
            width: 200
            height: 100
            Column {
                width: 200
                height: 400
                Rectangle { width: 200; height: 400; color: "transparent" }
            }
        }
    }

    Component {
        id: emptyScrollComponent
        ScrollView {
            width: 200
            height: 100
            Rectangle { width: 80; height: 40; color: "transparent" }
        }
    }

    Component {
        id: wideScrollComponent
        ScrollView {
            width: 200
            height: 100
            horizontalScroll: true
            Rectangle { width: 500; height: 400; color: "transparent" }
        }
    }


    // T286 shared component set. Each component is created with the minimum
    // props a pane supplies so the anatomy, tokens, and AT-SPI roles are
    // exercised without a pane body.
    Component {
        id: introComponent
        SettingsIntro {
            width: 468
            text: "Manage your overall setup and preferences."
            learnMoreText: "Learn More..."
        }
    }

    Component {
        id: bigListComponent
        SettingsBigList {
            width: 468
            SettingsBigRow {
                width: parent.width
                label: "About"
                secondaryText: "Version and host details"
                category: ({ source: "info", gradientStart: "#8e8e93", gradientEnd: "#58585c" })
            }
            SettingsBigRow {
                width: parent.width
                label: "Software Update"
                category: ({ source: "download-simple", gradientStart: "#3f8cff", gradientEnd: "#1f5fd6" })
            }
        }
    }

    Component {
        id: iconRowComponent
        SettingsIconRow {
            width: 468
            label: "Bluetooth"
            description: "Discoverable while this pane is open"
            category: ({ source: "bluetooth", gradientStart: "#4a9bff", gradientEnd: "#2158d0" })
        }
    }

    Component {
        id: footerComponent
        SettingsFooter {
            width: 468
            alignment: "right"
            Button { text: "Revert" }
            Button { text: "Apply"; variant: "primary" }
        }
    }

    Component {
        id: noteComponent
        SettingsNote {
            width: 468
            text: "This is a muted footnote under the card."
        }
    }

    Component {
        id: bannerComponent
        SettingsBanner {
            width: 468
            title: "Online Accounts"
            text: "The account service is not running."
            actionText: "Open Accounts"
        }
    }

    Component {
        id: serviceRowComponent
        SettingsServiceRow {
            width: 468
            label: "Wi-Fi"
            statusText: "Connected"
            serviceGlyph: "wifi"
        }
    }

    Component {
        id: switchServiceRowComponent
        SettingsServiceRow {
            width: 468
            label: "Firewall"
            statusText: "On"
            serviceGlyph: "shield-check"
            trailingKind: "switch"
        }
    }

    Component {
        id: scanRowComponent
        SettingsScanRow {
            width: 468
        }
    }

    Component {
        id: radioGroupComponent
        SettingsRadioGroup {
            width: 468
            currentIndex: 0
            options: ["Automatic", "Always", "Never"]
        }
    }

    Component {
        id: inlineRadioGroupComponent
        SettingsRadioGroup {
            width: 468
            inline: true
            currentIndex: 1
            options: ["Light", "Dark", "Auto"]
        }
    }

    Component {
        id: scaleSliderComponent
        SettingsScaleSlider {
            from: 0
            to: 10
            value: 5
            stepSize: 1
            accessibleName: "Tracking speed"
        }
    }

    Component {
        id: modeTileComponent
        SettingsModeTile {
            label: "Light"
            selected: true
        }
    }

    Component {
        id: navPillComponent
        SettingsNavPill {
            canGoBack: false
            canGoForward: false
        }
    }

    Component {
        id: statusRowComponent
        SettingsSidebarStatusRow {
            width: 212
            label: "1 Update Available"
            statusLabel: "Software Update"
            icon: "software-update"
            badge: "1"
            category: ({ source: "download-simple", gradientStart: "#3f8cff", gradientEnd: "#1f5fd6" })
        }
    }

    Component {
        id: collectionHeadComponent
        SettingsCollectionHead {
            width: 468
            title: "Dynamic"
            actionText: "Show All"
        }
    }

    Component {
        id: wallTileComponent
        SettingsWallTile {
            label: "Sample Theme"
            selected: true
        }
    }

    Component {
        id: chartComponent
        SettingsChart {
            width: 468
            values: [0.2, 0.5, 0.8, 0.4]
        }
    }

    Component {
        id: heroComponent
        SettingsHero {
            width: 468
            title: "Dragonfruit Linux 44"
            subtitle: "Example host"
        }
    }

    Component {
        id: storageBarComponent
        SettingsStorageBar {
            width: 468
            summaryText: "180 GB of 512 GB used"
            segments: [
                { label: "System", value: 60, color: Theme.color.accent },
                { label: "Documents", value: 40, color: Theme.color.success }
            ]
        }
    }

    Component {
        id: avatarComponent
        SettingsAvatar {
            initials: "DF"
        }
    }

    Component {
        id: deviceRowComponent
        SettingsDeviceRow {
            width: 468
            label: "Built-in Speakers"
            statusText: "Output"
            deviceGlyph: "volume"
        }
    }

    Component {
        id: diskRowComponent
        SettingsDiskRow {
            width: 468
            diskName: "Startup Disk"
            capacityText: "180 GB of 512 GB used"
            usedFraction: 0.35
        }
    }

    Component {
        id: tileComponent
        Item {
            id: tile
            width: 120
            height: 80
            property int activated: 0
            Rectangle { anchors.fill: parent; radius: 8; color: Theme.color.controlFill }
            TapHandler { onTapped: tile.activated += 1 }
        }
    }

    Component {
        id: buttonComponent
        Button {
            width: 48
            text: "A button label that cannot fit"
        }
    }

    Component {
        id: plainButtonComponent
        Button {
            text: "Cancel"
        }
    }

    TestCase {
        id: testCase
        name: "SettingsComponents"
        when: windowShown

        SignalSpy { id: disclosureSpy; signalName: "disclosureClicked" }
        SignalSpy { id: trailingSpy; signalName: "trailingActionClicked" }
        SignalSpy { id: helpSpy; signalName: "helpRequested" }
        SignalSpy { id: navBackSpy; signalName: "backRequested" }

        function test_row_bounds_at_500px() {
            var row = createTemporaryObject(rowComponent, stage, {});
            waitForRendering(stage);
            verify(row.control.width > 0);
            verify(row.control.width <= 500 * 0.6 + 0.5,
                   "the control slot never takes more than 60% of the row");
            verify(row.labelColumn.width > 0,
                   "the label column is never squeezed to zero");
            verify(row.labelText.width > 0);
            verify(row.control.x + row.control.width <= row.width,
                   "the control stays inside the row");
            verify(row.height >= Theme.controls.settingsRow.height);
            var width = row.control.width;
            wait(50);
            fuzzyCompare(row.control.width, width, 0.01);
        }

        function test_long_label_and_description_grow_the_row() {
            var row = createTemporaryObject(rowComponent, stage, {});
            waitForRendering(stage);
            compare(row.labelText.elide, Text.ElideRight);
            verify(row.labelText.truncated,
                   "the long localized label must elide, not push the control out");
            compare(row.descriptionText.wrapMode, Text.WordWrap);
            verify(row.height > Theme.controls.settingsRow.height,
                   "the wrapped description gows the row");
            verify(row.control.x + row.control.width <= row.width);
        }

        function test_group_final_separator_and_one_gap() {
            var group = createTemporaryObject(groupComponent, stage, {});
            waitForRendering(stage);
            compare(group.reserveBottomMargin, false);
            var rows = group.rows.children;
            compare(rows.length, 3);
            compare(rows[0].separatorRect.visible, true);
            compare(rows[1].separatorRect.visible, true);
            compare(rows[2].separatorRect.visible, false,
                    "the final row suppresses its separator");
            verify(group.titleText.wrapMode === Text.Wrap);
        }

        function test_row_focus_ring_and_disclosure_from_the_row() {
            var row = createTemporaryObject(rowComponent, stage, {});
            waitForRendering(stage);
            row.forceActiveFocus();
            tryVerify(function() { return row.activeFocus; });
            verify(row.focusRing.visible);
            compare(row.Accessible.name, row.label);

            disclosureSpy.target = row;
            disclosureSpy.clear();
            // A click in the label column (no prior motion) activates the
            // disclosure.
            mouseClick(row, row.width * 0.25, row.height / 2);
            compare(disclosureSpy.count, 1);

            // The chevron's own handler does not double-count through the row.
            mouseClick(row.disclosureArea, row.disclosureArea.width / 2,
                       row.disclosureArea.height / 2);
            compare(disclosureSpy.count, 2);

            // The chevron and the help control are Tab stops and Return
            // activates them.
            row.disclosureArea.forceActiveFocus();
            tryVerify(function() { return row.disclosureArea.activeFocus; });
            keyClick(Qt.Key_Return);
            compare(disclosureSpy.count, 3);

            helpSpy.target = row;
            helpSpy.clear();
            row.helpArea.forceActiveFocus();
            keyClick(Qt.Key_Space);
            compare(helpSpy.count, 1);
        }

        function test_control_accessible_names_and_ranges() {
            var row = createTemporaryObject(rowComponent, stage, {});
            waitForRendering(stage);
            var toggle = row.control.controlItem;
            compare(toggle.Accessible.name, row.label);
            compare(toggle.Accessible.checkable, true);

            var sliderRow = createTemporaryObject(sliderRowComponent, stage, {});
            waitForRendering(stage);
            var slider = sliderRow.control.controlItem;
            compare(slider.Accessible.name, "Pointer speed");
            compare(slider.minimumValue, 0);
            compare(slider.maximumValue, 10);
            verify(slider.Accessible.description.indexOf("4") >= 0);

            var select = createTemporaryObject(selectComponent, stage, {});
            waitForRendering(stage);
            compare(select.Accessible.name, "");
            verify(select.Accessible.description.indexOf("Automatic") >= 0);

            var longSelect = createTemporaryObject(longSelectComponent, stage, {});
            waitForRendering(stage);
            verify(longSelect.implicitWidth > select.implicitWidth,
                   "the select is sized to the longest option");

            var button = createTemporaryObject(buttonComponent, stage, {});
            waitForRendering(stage);
            verify(button.contentText.truncated,
                   "button text elides instead of overflowing");
            compare(button.Accessible.name, button.text);

            var plain = createTemporaryObject(plainButtonComponent, stage, {});
            waitForRendering(stage);
            verify(plain.width > Theme.controls.button.height,
                   "plain button width=" + plain.width
                   + " implicit=" + plain.implicitWidth);
            verify(plain.contentText.width > 0,
                   "plain button text width=" + plain.contentText.width
                   + " textImplicit=" + plain.contentText.implicitWidth
                   + " visible=" + plain.contentText.visible);
        }

        function test_select_popup_adopts_overlay_and_clamps_near_bottom() {
            var host = createTemporaryObject(hostComponent, stage, {});
            var select = createTemporaryObject(selectComponent, host,
                                               { y: 380 });
            waitForRendering(stage);
            mouseClick(select, select.width / 2, select.height / 2);
            tryVerify(function() { return select.menu.open; });
            compare(select.menu.parent, host.overlayer,
                    "the popup leaves the pane and adopts the window overlay");
            verify(select.menu.effectiveRect !== null,
                   "an anchored popup derives the overlay rectangle");
            verify(select.menu.y >= 0);
            verify(select.menu.y + select.menu.height <= host.height + 0.5);
            verify(select.menu.y < select.mapToItem(host, 0, 0).y,
                   "a popup near the bottom flips above its anchor");
            select.menu.activate(1);
            compare(select.currentIndex, 1);
            tryVerify(function() { return !select.menu.open; });
        }

        function test_select_menu_hover_selects_the_row_under_the_pointer() {
            var host = createTemporaryObject(hostComponent, stage, {});
            var select = createTemporaryObject(selectComponent, host,
                                               { x: 20, y: 20 });
            waitForRendering(stage);
            select.openMenu();
            tryVerify(function() { return select.menu.open; });
            waitForRendering(stage);
            wait(60);
            var menu = select.menu;
            var row = findChild(menu, "menuRow2");
            verify(row !== null, "the third option row exists");
            var p = row.mapToItem(stage, row.width / 2, row.height / 2);
            mouseMove(stage, p.x, p.y);
            wait(60);
            compare(menu.highlightedIndex, 2,
                    "the row under the pointer is the selected row");
        }

        function test_scrolled_select_hover_selects_the_row_under_the_pointer() {
            var host = createTemporaryObject(hostComponent, stage, {});
            // Open on a value near the bottom so the menu scrolls to reveal
            // the checked row before the pointer arrives.
            var select = createTemporaryObject(longSelectComponent, host,
                                               { x: 20, y: 20, currentIndex: 20 });
            waitForRendering(stage);
            select.openMenu();
            tryVerify(function() { return select.menu.open; });
            waitForRendering(stage);
            wait(60);
            var menu = select.menu;
            verify(menu.scrolls, "the long option list scrolls");
            var scroll = findChild(menu, "contextMenuScroll");
            verify(scroll !== null);
            verify(scroll.flickable.contentY > 0,
                   "the checked row is revealed on open");
            var row = findChild(menu, "menuRow18");
            verify(row !== null, "a scrolled option row exists");
            var p = row.mapToItem(stage, row.width / 2, row.height / 2);
            mouseMove(stage, p.x, p.y);
            wait(60);
            compare(menu.highlightedIndex, 18,
                    "the scrolled row under the pointer is selected");
        }

        function test_long_select_scrolls_inside_the_overlay() {
            var host = createTemporaryObject(hostComponent, stage, {});
            var select = createTemporaryObject(longSelectComponent, host,
                                               { y: 400 });
            waitForRendering(stage);
            select.openMenu();
            tryVerify(function() { return select.menu.open; });
            waitForRendering(stage);
            tryVerify(function() { return select.menu.scrolls; });
            verify(select.menu.height <= host.height);
            select.menu.hide();
        }

        function test_dialog_fits_minimum_traps_focus_and_closes() {
            var host = createTemporaryObject(hostComponent, stage, {});
            var dialog = createTemporaryObject(dialogComponent, host, {});
            dialog.show();
            tryVerify(function() { return dialog.open; });
            waitForRendering(stage);
            compare(dialog.parent, host.overlayer,
                    "a pane dialog adopts the window overlay");
            verify(dialog.card.width <= host.width - 2 * dialog.windowMargin + 0.5);
            verify(dialog.card.height <= host.height - 2 * dialog.windowMargin + 0.5,
                   "the dialog card fits the 720x480 window");
            verify(dialog.closeAction.visible,
                   "every dialog has a visible Close action");
            compare(dialog.scrim.width, host.width);
            compare(dialog.scrim.height, host.height);

            // Tab stays inside the card.
            dialog.forceActiveFocus();
            keyClick(Qt.Key_Tab);
            var first = dialog.Window.window.activeFocusItem;
            verify(first !== null);
            keyClick(Qt.Key_Tab);
            var second = dialog.Window.window.activeFocusItem;
            verify(second !== null);
            verify(second !== first, "Tab moves between dialog controls");
            var inside = second.mapToItem(dialog.card, 0, 0);
            verify(inside.x >= -1 && inside.y >= -1
                   && inside.x <= dialog.card.width + 1
                   && inside.y <= dialog.card.height + 1,
                   "focus never leaves the dialog card");

            keyClick(Qt.Key_Escape);
            tryVerify(function() { return !dialog.open; });
        }

        function test_scroll_view_modes_values_and_track_click() {
            var scroll = createTemporaryObject(scrollComponent, stage, {});
            waitForRendering(stage);
            scroll.forceActiveFocus();
            tryVerify(function() { return scroll.activeFocus; });
            compare(scroll.showBars, true, "automatic shows an overflowing bar when focused");
            compare(scroll.scrollBarOrientation, "vertical");
            verify(scroll.focusRing.visible, "the scroll view has a focus ring");
            scroll.flickable.contentY = 100;
            verify(scroll.scrollBarValue > 0);

            scroll.scrollBars = "when-scrolling";
            compare(scroll.showBars, false, "when-scrolling hides a resting bar");
            scroll.scrollBars = "always";
            compare(scroll.showBars, true);

            var empty = createTemporaryObject(emptyScrollComponent, stage, {});
            waitForRendering(stage);
            compare(empty.showBars, false, "automatic hides a non-overflowing bar");
            empty.scrollBars = "always";
            compare(empty.showBars, false, "always never invents an empty bar");

            var wide = createTemporaryObject(wideScrollComponent, stage, {});
            waitForRendering(stage);
            compare(wide.scrollBarOrientation, "both");
            wide.flickable.contentX = 40;
            verify(wide.horizontalScrollBarValue > 0);

            var scrolled = createTemporaryObject(scrollComponent, stage, {});
            waitForRendering(stage);
            scrolled.scrollBarClickBehavior = "jump-to-spot";
            scrolled.clickVerticalTrack(50);
            verify(scrolled.flickable.contentY > 0,
                   "a track click jumps the thumb to the clicked spot");
        }

        function test_closed_modal_does_not_own_the_pointer() {
            var host = createTemporaryObject(closedModalHostComponent, stage,
                                             { y: 700 });
            waitForRendering(stage);
            var select = host.select;
            verify(select !== null, "the host owns the control under the modals");

            // A closed Dialog and a closed Sheet both cover the Select's card
            // rectangle at opacity 0. The click must reach the Select.
            mouseClick(select, select.width / 2, select.height / 2);
            tryVerify(function() { return select.menu.open; }, 5000,
                      "a closed modal must not shadow the control beneath it");
            select.menu.hide();
            waitForRendering(stage);

            // Opening the modal must restore the scrim/card as the pointer
            // owner: the same click now lands on the dialog, not the Select.
            host.dialog.show();
            waitForRendering(stage);
            mouseClick(select, select.width / 2, select.height / 2);
            wait(200);
            compare(select.menu.open, false,
                    "an open modal still owns the pointer");
            host.dialog.hide();
            waitForRendering(stage);
        }

        function test_stationary_click_harness() {
            var tile = createTemporaryObject(tileComponent, stage, {});
            mouseClick(tile, tile.width / 2, tile.height / 2);
            compare(tile.activated, 1, "a stationary tile click activates once");

            var row = createTemporaryObject(rowComponent, stage, {});
            disclosureSpy.target = row;
            disclosureSpy.clear();
            mouseClick(row, row.width * 0.3, row.height / 2);
            compare(disclosureSpy.count, 1, "a stationary row click activates");

            var host = createTemporaryObject(hostComponent, stage, {});
            var select = createTemporaryObject(selectComponent, host, {});
            mouseClick(select, select.width / 2, select.height / 2);
            tryVerify(function() { return select.menu.open; });
            compare(select.menu.parent, host.overlayer);
            select.menu.hide();
            waitForRendering(stage);

            var dialog = createTemporaryObject(dialogComponent, host, {});
            dialog.show();
            tryVerify(function() { return dialog.open; });
            waitForRendering(stage);
            mouseClick(dialog.closeAction, dialog.closeAction.width / 2,
                       dialog.closeAction.height / 2);
            tryVerify(function() { return !dialog.open; });
        }

        function test_scroll_under_pointer_activates_the_row_now_under_it() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 720, height: 480 });
            waitForRendering(stage);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            verify(shell.resizeEdges.cornerAvoidance > 0,
                   "the resize grips stay off the scrollbar corner");
            verify(shell.detailScroll.cornerInset > 0,
                   "the scrollbars stop short of the resize corner");
            // The last row in the reduced 27-entry IA.
            var id = "printers";
            var index = shell.sidebarIndexOf(id);
            var row = shell.sidebar.rowAtIndex(index);
            shell.sidebar.flickable.contentY =
                Math.max(0, shell.sidebar.flickable.contentHeight
                            - shell.sidebar.flickable.height);
            waitForRendering(stage);
            // Re-fetch the delegate: the live status row can rebuild the
            // Repeater model and retire the instance captured above.
            row = shell.sidebar.rowAtIndex(index);
            var point = row.mapToItem(shell, 20, 16);
            mouseClick(shell, point.x, point.y);
            compare(shell.currentPaneId, id,
                    "the click hits the row now under the stationary pointer");
        }

        Component { id: shellComponent; SettingsShell { } }
        // -- T286 shared component set -----------------------------------------

        function test_t286_components_exist_and_lay_out() {
            var cases = [
                [introComponent, "SettingsIntro"],
                [bigListComponent, "SettingsBigList"],
                [iconRowComponent, "SettingsIconRow"],
                [footerComponent, "SettingsFooter"],
                [noteComponent, "SettingsNote"],
                [bannerComponent, "SettingsBanner"],
                [serviceRowComponent, "SettingsServiceRow"],
                [switchServiceRowComponent, "SettingsServiceRow"],
                [scanRowComponent, "SettingsScanRow"],
                [radioGroupComponent, "SettingsRadioGroup"],
                [inlineRadioGroupComponent, "SettingsRadioGroup"],
                [scaleSliderComponent, "SettingsScaleSlider"],
                [modeTileComponent, "SettingsModeTile"],
                [navPillComponent, "SettingsNavPill"],
                [statusRowComponent, "SettingsSidebarStatusRow"],
                [collectionHeadComponent, "SettingsCollectionHead"],
                [wallTileComponent, "SettingsWallTile"],
                [chartComponent, "SettingsChart"],
                [heroComponent, "SettingsHero"],
                [storageBarComponent, "SettingsStorageBar"],
                [avatarComponent, "SettingsAvatar"],
                [deviceRowComponent, "SettingsDeviceRow"],
                [diskRowComponent, "SettingsDiskRow"]
            ];
            for (var i = 0; i < cases.length; ++i) {
                var item = createTemporaryObject(cases[i][0], stage, {});
                verify(item !== null, cases[i][1] + " must instantiate");
                waitForRendering(stage);
                verify(item.width > 0 && item.height > 0,
                       cases[i][1] + " must lay out (w=" + item.width
                       + " h=" + item.height + ")");
                item.destroy();
            }
        }

        function test_t286_row_variants_use_the_measured_heights() {
            var big = createTemporaryObject(bigListComponent, stage, {});
            waitForRendering(stage);
            var rows = big.rows.children;
            compare(rows.length, 2);
            verify(rows[0].height >= Theme.controls.settingsRow.bigHeight,
                   "a big row is at least 52 pt (got "
                   + rows[0].height + ")");
            verify(rows[0].disclosure);
            verify(rows[0].leadingTile !== null);
            compare(rows[0].separatorRect.visible, true);
            compare(rows[1].separatorRect.visible, false);

            var icon = createTemporaryObject(iconRowComponent, stage, {});
            waitForRendering(stage);
            compare(icon.height, Theme.controls.settingsRow.iconHeight);

            var plain = createTemporaryObject(rowComponent, stage, {});
            waitForRendering(stage);
            verify(plain.height >= Theme.controls.settingsRow.descriptionHeight,
                   "a description row is at least 52 pt (got "
                   + plain.height + ")");
            var service = createTemporaryObject(serviceRowComponent, stage, {});
            waitForRendering(stage);
            compare(service.height, Theme.controls.settingsRow.iconHeight);
        }

        function test_t286_footer_note_and_head_geometry() {
            var footer = createTemporaryObject(footerComponent, stage, {});
            waitForRendering(stage);
            verify(footer.row.width > 0);
            verify(footer.row.x > 0, "a right-aligned footer hugs the right");
            compare(footer.height,
                    footer.row.height + Theme.controls.settingsGroup.marginBottom);

            var note = createTemporaryObject(noteComponent, stage, {});
            waitForRendering(stage);
            compare(note.font.pixelSize, Theme.primitive.font.sizeSm);
            compare(note.Accessible.role, Accessible.StaticText);

            var head = createTemporaryObject(collectionHeadComponent, stage, {});
            waitForRendering(stage);
            verify(head.titleText.x >= 0);
            verify(head.actionLink.x > head.titleText.x);
            compare(head.actionLink.Accessible.role, Accessible.Link);
        }

        function test_t286_radio_group_selection_and_keyboard() {
            var group = createTemporaryObject(radioGroupComponent, stage, {});
            waitForRendering(stage);
            compare(group.currentIndex, 0);
            var first = group.repeater.itemAt(0);
            var second = group.repeater.itemAt(1);
            verify(first !== null && second !== null);
            compare(first.Accessible.role, Accessible.RadioButton);
            compare(first.Accessible.checked, true);
            compare(second.Accessible.checked, false);

            // A stationary click on the second row selects it and takes focus.
            mouseClick(second, second.width / 2, second.height / 2);
            compare(group.currentIndex, 1);
            verify(second.activeFocus, "a pointer pick follows the pointer");

            // Down moves the selection and the roving focus together, so a
            // following Space confirms the newly selected option.
            second.forceActiveFocus();
            keyClick(Qt.Key_Down);
            compare(group.currentIndex, 2);
            verify(group.repeater.itemAt(2).activeFocus,
                   "keyboard selection moves the focus ring with the dot");
            keyClick(Qt.Key_Space);
            compare(group.currentIndex, 2,
                    "activating the focused option keeps the selection");
        }

        function test_t286_scale_slider_ticks_and_range() {
            var scale = createTemporaryObject(scaleSliderComponent, stage, {});
            waitForRendering(stage);
            compare(scale.width, Theme.controls.settingsScale.width);
            compare(scale.slider.minimumValue, 0);
            compare(scale.slider.maximumValue, 10);
            verify(scale.tickRow.height > 0);
            verify(scale.tickRow.children.length >= 5);
            compare(scale.slider.Accessible.name, "Tracking speed");
        }

        function test_t286_nav_pill_states_and_stationary_click() {
            var pill = createTemporaryObject(navPillComponent, stage, {});
            waitForRendering(stage);
            compare(pill.width, Theme.controls.settingsWindow.navPillWidth);
            compare(pill.height, Theme.controls.settingsWindow.navPillHeight);
            compare(pill.backButton.width,
                    Theme.controls.settingsWindow.navButtonWidth);
            compare(pill.divider.height,
                    Theme.controls.settingsWindow.navDividerHeight);
            verify(!pill.backButton.enabled);
            verify(!pill.forwardButton.enabled);

            navBackSpy.target = pill;
            navBackSpy.clear();
            pill.canGoBack = true;
            mouseClick(pill.backButton, pill.backButton.width / 2,
                       pill.backButton.height / 2);
            compare(navBackSpy.count, 1, "a stationary Back click reports once");
            pill.forwardButton.forceActiveFocus();
            keyClick(Qt.Key_Return);
            compare(navBackSpy.count, 1,
                    "a disabled Forward is not a dead action");
            navBackSpy.target = null;
        }

        function test_t286_status_row_badge_and_roles() {
            var row = createTemporaryObject(statusRowComponent, stage, {});
            waitForRendering(stage);
            compare(row.height, Theme.controls.settingsSidebar.rowHeight);
            verify(row.badgeRect.visible);
            verify(row.badgeRect.width > 0);
            compare(row.Accessible.role, Accessible.ListItem);
            verify(row.Accessible.name.indexOf("Software Update") >= 0);
            row.badge = "";
            waitForRendering(stage);
            verify(!row.badgeRect.visible, "an empty badge is not painted");
        }

        function test_t286_chart_and_storage_bar_draw_from_values() {
            var chart = createTemporaryObject(chartComponent, stage, {});
            waitForRendering(stage);
            verify(chart.bars.children.length >= 4);
            verify(chart.baselineRect.height > 0);

            var storage = createTemporaryObject(storageBarComponent, stage, {});
            waitForRendering(stage);
            verify(storage.legend.children.length >= 2);
            verify(storage.bar.width > 0);
            verify(storage.summaryText !== undefined);
        }

    }
}
