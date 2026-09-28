// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Printers & Scanners pane tests (T-15.12b). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_PRINTERS_FIXTURE` (see CMakeLists.txt), so the
// `Settings` singleton and the host-stack client are deterministic and
// in-process with no bus (and no CUPS/SANE). The cases cover the two popups,
// the two printer rows and their per-printer detail (accept jobs, set default,
// cancel a job), and the scanner list.
Item {
    id: stage
    width: 1000
    height: 1600

    TestCase {
        id: testCase
        name: "SettingsPrinters"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture is process-global; reset it before every case so each
        // starts on the known two-queue / one-scanner workstation.
        function init() {
            Settings.resetPrintersFixture();
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("printers");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Printers & Scanners pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_queues_and_scanner() {
            var shell = make();
            compare(shell.currentPaneId, "printers");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.printersAvailable, true);
            compare(pane.scannersAvailable, true);
            compare(pane.printers.length, 2);
            compare(pane.scanners.length, 1);
            compare(pane.defaultPrinter, "Canon_MF230");

            // The default queue sorts first.
            compare(findChild(pane.printerRepeater.itemAt(0), "printerNameText").text,
                    "Canon MF230");
            compare(findChild(pane.printerRepeater.itemAt(0), "printerStateText").text,
                    "Idle, Last Used \u00b7 Default");
            compare(findChild(pane.printerRepeater.itemAt(1), "printerNameText").text,
                    "HP LaserJet");
            compare(findChild(pane.printerRepeater.itemAt(1), "printerStateText").text,
                    "Printing");
            compare(findChild(pane.scannerRepeater.itemAt(0), "scannerNameText").text,
                    "Epson GT-1500 flatbed scanner");
            compare(findChild(pane.scannerRepeater.itemAt(0), "scannerKindText").text,
                    "Flatbed");
        }

        function test_default_printer_popup_round_trips() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.defaultSelect.currentIndex, 0);
            compare(pane.defaultSelect.currentValue, "Canon_MF230");

            // The second entry is HP; selecting it repoints the system default.
            pane.defaultSelect.activateIndex(1);
            tryVerify(function() { return pane.defaultPrinter === "HP_LaserJet"; });
            verify(pane.findPrinter("HP_LaserJet").isDefault,
                   "HP is now the default queue");
            compare(pane.findPrinter("Canon_MF230").isDefault, false);
        }

        function test_default_paper_size_popup_writes_settingsd() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.paperSizeSelect.currentValue, "us-letter");

            pane.paperSizeSelect.activateIndex(3); // A4
            tryVerify(function() {
                return Settings.values["printers.defaultPaperSize"] === "a4";
            });
        }

        function test_printer_detail_accept_jobs_round_trips() {
            var shell = make();
            var pane = paneOf(shell);
            var row = findChild(pane.printerRepeater.itemAt(1), "printerRowArea");
            mouseClick(row, row.width / 2, row.height / 2);
            compare(pane.printerDialog.open, true);
            compare(pane.selectedPrinter.name, "HP_LaserJet");
            compare(pane.acceptJobsToggle.checked, true);

            pane.acceptJobsToggle.toggle();
            tryVerify(function() {
                return pane.findPrinter("HP_LaserJet").acceptingJobs === false;
            });
        }

        function test_printer_detail_sets_default_and_cancels_a_job() {
            var shell = make();
            var pane = paneOf(shell);
            var row = findChild(pane.printerRepeater.itemAt(1), "printerRowArea");
            mouseClick(row, row.width / 2, row.height / 2);
            compare(pane.selectedPrinter.name, "HP_LaserJet");
            compare(pane.selectedPrinter.jobCount, 1);

            // HP has one job; cancel it.
            findChild(pane.jobsRepeater.itemAt(0), "cancelJobButton").clicked();
            tryVerify(function() { return pane.selectedPrinter.jobCount === 0; });

            // Make HP the default.
            compare(pane.setDefaultButton.enabled, true);
            pane.setDefaultButton.clicked();
            tryVerify(function() { return pane.defaultPrinter === "HP_LaserJet"; });
        }

        function test_absence_note_hidden_with_a_ready_view() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.absenceNote.visible, false);
            compare(pane.noPrintersNote.visible, false);
            compare(pane.scannerAbsentNote.visible, false);
        }
    }
}