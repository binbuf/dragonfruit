// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T267 System Settings accessibility, keyboard, RTL, and long-string tests.
// The cases render the real shell and shared components offscreen; the
// deterministic settings fixture keeps them off the host bus.
Item {
    id: stage
    width: 1000
    height: 900

    WindowOverlay { id: stageOverlay }

    Component { id: shellComponent; SettingsShell { } }

    Component {
        id: rowComponent
        SettingsRow {
            width: 500
            label: "Pointer speed"
            description: "How fast the pointer moves."
            controlData: Toggle { checked: true }
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
            width: 180
            model: ["Automatic", "Always", "When scrolling", "Never"]
        }
    }

    Component {
        id: dialogComponent
        Dialog {
            id: dialogRoot
            title: "A dialog title"
            message: "A dialog message."
            contentData: Column {
                width: parent ? parent.width : 0
                spacing: Theme.primitive.spacing.sm
                Text { width: parent.width; text: "First body line"; wrapMode: Text.WordWrap }
                Text { width: parent.width; text: "Second body line"; wrapMode: Text.WordWrap }
            }
            buttonsData: Row {
                Button { text: "Cancel"; onClicked: dialogRoot.reject() }
                Button { text: "Save"; variant: "primary"; onClicked: dialogRoot.accept() }
            }
        }
    }

    Component {
        id: sheetComponent
        Sheet {
            id: sheetRoot
            title: "Sheet title"
            message: "Sheet message."
            buttonsData: Row {
                Button { text: "Cancel"; onClicked: sheetRoot.reject() }
                Button { text: "Done"; variant: "primary"; onClicked: sheetRoot.accept() }
            }
        }
    }

    Component {
        id: absenceComponent
        SettingsAbsenceNotice {
            width: 420
            kind: "disabled-with-reason"
            reason: "The system status service is not running."
        }
    }

    Component {
        id: longSidebarComponent
        Sidebar {
            width: 232
            sections: [{
                title: "A section title long enough to elide in the sidebar",
                items: [{
                    label: "Barrierefreiheit und Bedienungshilfen für Menschen mit besonderen Anforderungen",
                    icon: "settings"
                }]
            }]
        }
    }

    Component {
        id: longTitleComponent
        TitleBar {
            width: 508
            title: "A pane title long enough that it must elide in the middle of the toolbar"
        }
    }

    Component {
        id: longResultsComponent
        SettingsSearchResults {
            width: 232
            query: "long"
            results: [{
                kind: "row", label: "A search result label that is long enough to elide",
                subtitle: "A section subtitle that is also deliberately long",
                paneTitle: "A pane title that is deliberately long enough to elide",
                section: "A section whose name is also long enough to elide",
                icon: "settings",
                groupStart: true, groupTitle: "A group title long enough to elide",
                deepLink: "settings://pane/appearance"
            }]
        }
    }

    Component {
        id: longDialogComponent
        Dialog {
            id: longDialogRoot
            title: "A dialog title that is long enough to need wrapping at the minimum window width"
            message: "This dialog message is deliberately long so the card has to wrap and fit inside the 720x480 window instead of growing past it."
            buttonsData: Row {
                Button { text: "Close"; onClicked: longDialogRoot.reject() }
            }
        }
    }

    Component {
        id: aboutPanelComponent
        AboutPanel {
            identity: ({
                deviceName: "A device name that is much longer than the panel width allows",
                processor: "A processor description that is also very long",
                memoryLabel: "128 GB of memory with a long label",
                serial: "SERIAL-0123456789-ABCDEF",
                hasSerial: true,
                osLabel: "Dragonfruit Linux 44 with a long distribution description"
            })
            productName: "A product name that is much longer than the panel width"
        }
    }

    Component {
        id: aboutPageComponent
        AboutPage {
            width: 508
            identity: ({
                deviceName: "A device name that is much longer than the detail column allows and must wrap",
                processor: "A processor description that is also very long",
                memoryLabel: "128 GB of memory with a long label",
                serial: "SERIAL-0123456789-ABCDEF",
                hasSerial: true,
                osLabel: "Dragonfruit Linux 44 with a long distribution description that must wrap"
            })
            productName: "A product name that is much longer than the detail column"
        }
    }

    TestCase {
        id: testCase
        name: "SettingsAccessibility"
        when: windowShown

        function init() {
            Settings.set("settings.lastPane", "");
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
        }

        function makeShell(width, height) {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: width || 740,
                                                height: height || 820 });
            waitForRendering(stage);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            return shell;
        }

        function ready(shell) {
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
        }

        // -- Keyboard map -------------------------------------------------------

        function test_keyboard_map_is_complete() {
            var actions = Keymap.bindings.map(function(binding) {
                return binding.action;
            });
            var required = ["focus-search", "back", "forward", "dismiss",
                            "next", "previous", "first", "last", "activate",
                            "next-focus", "previous-focus"];
            for (var i = 0; i < required.length; ++i)
                verify(actions.indexOf(required[i]) >= 0,
                       "the map is missing " + required[i]);

            compare(Keymap.actionFor("shell", Qt.Key_F, Qt.ControlModifier),
                    "focus-search");
            compare(Keymap.actionFor("shell", Qt.Key_Left, Qt.AltModifier),
                    "back");
            compare(Keymap.actionFor("shell", Qt.Key_Right, Qt.AltModifier),
                    "forward");
            compare(Keymap.actionFor("shell", Qt.Key_Escape, Qt.NoModifier),
                    "dismiss");
            compare(Keymap.actionFor("sidebar", Qt.Key_Down, Qt.NoModifier),
                    "next");
            compare(Keymap.actionFor("sidebar", Qt.Key_Up, Qt.NoModifier),
                    "previous");
            compare(Keymap.actionFor("sidebar", Qt.Key_Home, Qt.NoModifier),
                    "first");
            compare(Keymap.actionFor("sidebar", Qt.Key_End, Qt.NoModifier),
                    "last");
            compare(Keymap.actionFor("sidebar", Qt.Key_Return, Qt.NoModifier),
                    "activate");
            compare(Keymap.actionFor("sidebar", Qt.Key_Space, Qt.NoModifier),
                    "activate");
            compare(Keymap.actionFor("list", Qt.Key_End, Qt.NoModifier),
                    "last");
            compare(Keymap.actionFor("any", Qt.Key_Tab, Qt.ShiftModifier),
                    "previous-focus");
        }

        function test_shell_global_keyboard_commands() {
            var shell = makeShell();
            shell.forceActiveFocus();
            keyClick(Qt.Key_F, Qt.ControlModifier);
            tryVerify(function() { return shell.searchField.activeFocus; });

            shell.searchText = "display";
            shell.commitSearch();
            keyClick(Qt.Key_Escape);
            compare(shell.searchText, "");
            tryVerify(function() { return shell.searchField.activeFocus; });
            keyClick(Qt.Key_Escape);
            tryVerify(function() { return shell.sidebar.activeFocus; });

            shell.selectPane("displays");
            ready(shell);
            shell.selectPane("wallpaper");
            ready(shell);
            shell.sidebar.forceActiveFocus();
            keyClick(Qt.Key_Left, Qt.AltModifier);
            compare(shell.currentPaneId, "displays");
            keyClick(Qt.Key_Right, Qt.AltModifier);
            compare(shell.currentPaneId, "wallpaper");

            shell.selectPane("general");
            ready(shell);
            shell.openSubPane("about");
            tryVerify(function() { return shell.currentSubPaneId === "about"; });
            shell.sidebar.forceActiveFocus();
            keyClick(Qt.Key_Escape);
            tryVerify(function() { return shell.currentSubPaneId === ""; });
        }

        // -- Keyboard traversal -------------------------------------------------

        function test_keyboard_traversal_covers_every_pane() {
            var shell = makeShell();
            var missing = [];
            for (var i = 0; i < SettingsPanes.shippedPanes.length; ++i) {
                var id = SettingsPanes.shippedPanes[i].id;
                shell.selectPane(id);
                tryVerify(function() {
                    return shell.loadedPaneId === id
                        && shell.paneBody.item !== null;
                });
                waitForRendering(stage);
                var target = shell.firstFocusable(shell.paneBody.item);
                if (target === null) {
                    // A pane whose adapter is still resolving may mount its
                    // absence state one frame after the body loads.
                    wait(100);
                    waitForRendering(stage);
                    target = shell.firstFocusable(shell.paneBody.item);
                }
                if (target === null)
                    missing.push(id);
            }
            compare(missing.length, 0,
                    "panes without a keyboard stop: " + JSON.stringify(missing));
        }

        function test_search_results_keyboard_path() {
            var shell = makeShell();
            shell.forceActiveFocus();
            keyClick(Qt.Key_F, Qt.ControlModifier);
            shell.searchText = "display";
            shell.commitSearch();
            tryVerify(function() { return shell.searchResultsList.hasResults; });
            keyClick(Qt.Key_Down);
            tryVerify(function() { return shell.searchResultsList.activeFocus; });
            compare(shell.searchResultsList.currentIndex, 0);
            keyClick(Qt.Key_End);
            compare(shell.searchResultsList.currentIndex,
                    shell.searchResultsList.results.length - 1);
            keyClick(Qt.Key_Home);
            compare(shell.searchResultsList.currentIndex, 0);
            keyClick(Qt.Key_Escape);
            compare(shell.searchText, "");
            tryVerify(function() { return shell.searchField.activeFocus; });
        }

        function test_popup_and_dialog_keyboard_paths() {
            var select = createTemporaryObject(selectComponent, stage, {});
            waitForRendering(stage);
            select.forceActiveFocus();
            keyClick(Qt.Key_Down);
            tryVerify(function() { return select.menu.open; });
            keyClick(Qt.Key_Down);
            keyClick(Qt.Key_Return);
            tryVerify(function() { return !select.menu.open; });
            compare(select.currentIndex, 1);

            var host = createTemporaryObject(shellComponent, stage,
                                             { width: 720, height: 480 });
            waitForRendering(stage);
            var row = createTemporaryObject(rowComponent, host, { width: 420 });
            waitForRendering(stage);
            row.forceActiveFocus();
            tryVerify(function() { return row.activeFocus; });
            var dialog = createTemporaryObject(dialogComponent, host, {});
            dialog.show();
            tryVerify(function() { return dialog.open; });
            waitForRendering(stage);
            keyClick(Qt.Key_Tab);
            keyClick(Qt.Key_Tab);
            var focused = host.Window.window.activeFocusItem;
            verify(focused !== null);
            var inside = focused.mapToItem(dialog.card, 0, 0);
            verify(inside.x >= -1 && inside.y >= -1
                   && inside.x <= dialog.card.width + 1
                   && inside.y <= dialog.card.height + 1,
                   "Tab must stay inside the dialog card");
            keyClick(Qt.Key_Escape);
            tryVerify(function() { return !dialog.open; });
            tryVerify(function() { return row.activeFocus; });
        }

        function test_sheet_traps_focus_and_restores_it() {
            var host = createTemporaryObject(shellComponent, stage,
                                             { width: 720, height: 480 });
            waitForRendering(stage);
            var row = createTemporaryObject(rowComponent, host, { width: 420 });
            waitForRendering(stage);
            row.forceActiveFocus();
            var sheet = createTemporaryObject(sheetComponent, host, {});
            sheet.show();
            tryVerify(function() { return sheet.open; });
            waitForRendering(stage);
            keyClick(Qt.Key_Tab);
            keyClick(Qt.Key_Tab);
            var focused = host.Window.window.activeFocusItem;
            verify(focused !== null);
            var inside = focused.mapToItem(sheet.card, 0, 0);
            verify(inside.x >= -1 && inside.y >= -1
                   && inside.x <= sheet.card.width + 1
                   && inside.y <= sheet.card.height + 1,
                   "Tab must stay inside the sheet card");
            keyClick(Qt.Key_Escape);
            tryVerify(function() { return !sheet.open; });
            tryVerify(function() { return row.activeFocus; });
        }

        // -- AT-SPI -------------------------------------------------------------

        function test_atspi_roles_names_and_values() {
            var shell = makeShell();
            compare(shell.Accessible.role, Accessible.Pane);
            compare(shell.Accessible.name, "System Settings");
            verify(shell.Accessible.description.length > 0);
            compare(shell.appWindow.Accessible.role, Accessible.Window);
            compare(shell.titleBar.Accessible.role, Accessible.TitleBar);
            compare(shell.sidebar.Accessible.role, Accessible.List);
            compare(shell.sidebar.Accessible.name, "Sidebar");
            compare(shell.searchField.Accessible.role, Accessible.EditableText);
            compare(shell.searchField.Accessible.searchEdit, true);
            verify(shell.searchField.Accessible.description.length > 0);

            var sidebarRow = shell.sidebar.rowAtIndex(shell.sidebar.currentIndex);
            verify(sidebarRow !== null);
            compare(sidebarRow.Accessible.role, Accessible.ListItem);
            compare(sidebarRow.Accessible.name, sidebarRow.entry.label);
            verify(sidebarRow.Accessible.description.length > 0);

            shell.searchText = "display";
            shell.commitSearch();
            tryVerify(function() { return shell.searchResultsList.hasResults; });
            compare(shell.searchResultsList.Accessible.role, Accessible.List);
            verify(shell.searchResultsList.Accessible.name.indexOf("result") >= 0);
            verify(shell.searchResultsList.Accessible.description.length > 0);
            var delegate = shell.searchResultsList.listView.itemAtIndex(0);
            verify(delegate !== null);
            compare(delegate.Accessible.role, Accessible.ListItem);
            verify(delegate.Accessible.name.length > 0);
            verify(delegate.Accessible.description.length > 0);

            var row = createTemporaryObject(rowComponent, stage, {});
            waitForRendering(stage);
            compare(row.Accessible.role, Accessible.Grouping);
            compare(row.Accessible.name, "Pointer speed");
            verify(row.Accessible.description.length > 0);
            var toggle = row.control.controlItem;
            compare(toggle.Accessible.role, Accessible.Switch);
            compare(toggle.Accessible.name, "Pointer speed");
            compare(toggle.Accessible.checkable, true);
            compare(toggle.Accessible.checked, true);

            var sliderRow = createTemporaryObject(sliderRowComponent, stage, {});
            waitForRendering(stage);
            var slider = sliderRow.control.controlItem;
            compare(slider.Accessible.role, Accessible.Slider);
            compare(slider.Accessible.name, "Pointer speed");
            verify(slider.Accessible.description.indexOf("4") >= 0);
            compare(slider.minimumValue, 0);
            compare(slider.maximumValue, 10);

            var select = createTemporaryObject(selectComponent, stage, {});
            waitForRendering(stage);
            compare(select.Accessible.role, Accessible.ComboBox);
            verify(select.Accessible.description.indexOf("Automatic") >= 0);
            select.openMenu();
            tryVerify(function() { return select.menu.open; });
            compare(select.menu.Accessible.role, Accessible.PopupMenu);
            verify(select.menu.Accessible.description.length > 0);
            select.menu.hide();

            var dialog = createTemporaryObject(dialogComponent, shell, {});
            dialog.show();
            tryVerify(function() { return dialog.open; });
            compare(dialog.Accessible.role, Accessible.Dialog);
            compare(dialog.Accessible.name, dialog.title);
            compare(dialog.Accessible.description, dialog.message);
            dialog.reject();

            shell.showDebugView("general");
            tryVerify(function() { return shell.debugView.open; });
            compare(shell.debugView.Accessible.role, Accessible.Dialog);
            verify(shell.debugView.Accessible.name.length > 0);
            compare(shell.debugView.rowList.Accessible.role, Accessible.List);
            verify(shell.debugView.rowList.Accessible.name.length > 0);
            tryVerify(function() {
                return shell.debugView.rowList.itemAtIndex(0) !== null;
            });
            var debugRow = shell.debugView.rowList.itemAtIndex(0);
            verify(debugRow !== null);
            compare(debugRow.Accessible.role, Accessible.ListItem);
            verify(debugRow.Accessible.description.length > 0);
            shell.debugView.hide();
        }

        function test_absence_and_validation_notes_are_named() {
            var note = createTemporaryObject(absenceComponent, stage, {});
            waitForRendering(stage);
            compare(note.visible, true);
            compare(note.Accessible.role, Accessible.Grouping);
            compare(note.Accessible.name, note.reason);
            verify(note.Accessible.description.length > 0);
            var noteText = findChild(note, "settingsAbsenceNoteText");
            verify(noteText !== null);
            compare(noteText.Accessible.role, Accessible.AlertMessage);
            compare(noteText.Accessible.name, note.reason);

            var row = createTemporaryObject(rowComponent, stage, {});
            waitForRendering(stage);
            row.errorText = "The value is not valid.";
            compare(row.errorLabel.Accessible.role, Accessible.AlertMessage);
            compare(row.errorLabel.Accessible.name, "The value is not valid.");
            verify(row.Accessible.description.indexOf("not valid") >= 0);
        }

        // -- Adaptive presentation ---------------------------------------------

        function test_adaptive_keys_apply_live_without_a_restart() {
            var shell = makeShell();
            compare(Theme.reducedMotion, false);
            compare(Glass.reduceTransparency, false);
            compare(Glass.increaseContrast, false);

            Settings.set("accessibility.reduceMotion", true);
            tryVerify(function() { return Theme.reducedMotion === true; });
            compare(Theme.motion.popupOpen.duration, 0);
            compare(Theme.motion.hover.duration, 0);

            Settings.set("accessibility.reduceTransparency", true);
            tryVerify(function() { return Glass.reduceTransparency === true; });

            Settings.set("accessibility.increaseContrast", true);
            tryVerify(function() { return Glass.increaseContrast === true; });

            Settings.set("accessibility.reduceMotion", false);
            tryVerify(function() { return Theme.reducedMotion === false; });
            verify(Theme.motion.popupOpen.duration > 0);
        }

        // -- RTL ----------------------------------------------------------------

        function test_rtl_mirrors_the_split_and_row_layout() {
            var shell = makeShell();
            compare(shell.rtl, false);
            compare(shell.sidebarColumn.x, 0);
            verify(shell.detailColumn.x > 0);

            shell.rtl = true;
            waitForRendering(stage);
            verify(shell.sidebarColumn.x > 0,
                   "the sidebar must move to the right in RTL (x="
                   + shell.sidebarColumn.x + ")");
            compare(shell.detailColumn.x, 0);
            compare(shell.titleBar.rtl, true);

            var row = createTemporaryObject(rowComponent, shell, {});
            waitForRendering(stage);
            compare(row.LayoutMirroring.enabled, true);
            compare(row.labelText.horizontalAlignment, Text.AlignRight);
            verify(row.control.x < row.labelColumn.x,
                   "the control must move to the left of the label in RTL");

            shell.rtl = false;
            waitForRendering(stage);
            compare(shell.sidebarColumn.x, 0);
            compare(row.labelText.horizontalAlignment, Text.AlignLeft);
            verify(row.control.x > row.labelColumn.x);
        }

        // -- Long strings / pseudo-locale sweep --------------------------------

        function test_long_string_sweep_sidebar_title_results_and_about() {
            var sidebar = createTemporaryObject(longSidebarComponent, stage, {});
            waitForRendering(stage);
            var label = findChild(sidebar, "sidebarRowLabel");
            var header = findChild(sidebar, "sidebarHeaderText");
            verify(label !== null && header !== null);
            compare(label.elide, Text.ElideRight);
            verify(label.truncated, "the long sidebar label must elide");
            verify(label.x + label.width <= sidebar.width + 1);
            verify(header.truncated, "the long section header must elide");

            var title = createTemporaryObject(longTitleComponent, stage, {});
            waitForRendering(stage);
            compare(title.titleText.elide, Text.ElideMiddle);
            verify(title.titleText.truncated, "the long title must elide");
            verify(title.titleText.width > 0);

            var results = createTemporaryObject(longResultsComponent, stage, {});
            waitForRendering(stage);
            var resultLabel = findChild(results, "searchResultLabel");
            var resultSubtitle = findChild(results, "searchResultSubtitle");
            verify(resultLabel !== null && resultSubtitle !== null);
            compare(resultLabel.elide, Text.ElideRight);
            verify(resultLabel.truncated, "the long result label must elide");
            verify(resultSubtitle.truncated, "the long subtitle must elide");

            var host = createTemporaryObject(shellComponent, stage,
                                             { width: 720, height: 480 });
            waitForRendering(stage);
            var dialog = createTemporaryObject(longDialogComponent, host, {});
            dialog.show();
            tryVerify(function() { return dialog.open; });
            waitForRendering(stage);
            verify(dialog.card.width <= host.width - 2 * dialog.windowMargin + 0.5);
            compare(dialog.titleText.wrapMode, Text.WordWrap);
            verify(dialog.titleText.width > 0);
            dialog.reject();

            var panel = createTemporaryObject(aboutPanelComponent, stage, {});
            waitForRendering(stage);
            compare(panel.heading.elide, Text.ElideRight);
            verify(panel.heading.truncated,
                   "the long About panel device name must elide");

            var page = createTemporaryObject(aboutPageComponent, stage, {});
            waitForRendering(stage);
            var pageName = findChild(page, "aboutPageName");
            var pageOs = findChild(page, "aboutPageOs");
            verify(pageName !== null && pageOs !== null);
            verify(pageName.width > 0);
            verify(pageOs.width > 0);
            verify(pageName.contentWidth <= pageName.width
                   || pageName.wrapMode !== Text.NoWrap
                   || pageName.elide !== Text.ElideNone,
                   "the long About device name must wrap or elide");
        }

        // -- Reference sizes ----------------------------------------------------

        function test_reference_sizes_have_no_overlap_or_grip_conflict() {
            var sizes = [[740, 820], [720, 480]];
            for (var i = 0; i < sizes.length; ++i) {
                var shell = createTemporaryObject(shellComponent, stage,
                                                  { width: sizes[i][0],
                                                    height: sizes[i][1] });
                waitForRendering(stage);
                ready(shell);
                compare(shell.sidebarColumn.width + shell.detailColumn.width,
                        shell.width);
                verify(shell.searchField.x >= 0
                       && shell.searchField.x + shell.searchField.width
                          <= shell.sidebarColumn.width + 0.5);
                verify(shell.titleBar.x >= 0
                       && shell.titleBar.x + shell.titleBar.width
                          <= shell.detailColumn.width + 0.5);
                verify(shell.detailScroll.height > 0);
                verify(shell.detailScroll.cornerInset > 0,
                       "the scrollbars must stop short of the resize corner");
                verify(shell.resizeEdges.cornerAvoidance > 0,
                       "the resize grips must avoid the scrollbar corner");
                verify(shell.sidebar.scrollable || shell.sidebar.contentHeight > 0);
                shell.destroy();
            }
        }
    }
}
