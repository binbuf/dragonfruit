// SPDX-License-Identifier: MIT
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
            verify(select.menu.availableRect !== null);
            verify(select.menu.y >= 0);
            verify(select.menu.y + select.menu.height <= host.height + 0.5);
            verify(select.menu.y < select.mapToItem(host, 0, 0).y,
                   "a popup near the bottom flips above its anchor");
            select.menu.activate(1);
            compare(select.currentIndex, 1);
            tryVerify(function() { return !select.menu.open; });
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
            var id = "menu-bar";
            var index = shell.sidebarIndexOf(id);
            var row = shell.sidebar.rowAtIndex(index);
            verify(row !== null);
            shell.sidebar.flickable.contentY =
                Math.max(0, shell.sidebar.flickable.contentHeight
                            - shell.sidebar.flickable.height);
            waitForRendering(stage);
            var point = row.mapToItem(shell, 20, row.height / 2);
            mouseClick(shell, point.x, point.y);
            compare(shell.currentPaneId, id,
                    "the click hits the row now under the stationary pointer");
        }

        Component { id: shellComponent; SettingsShell { } }
    }
}
