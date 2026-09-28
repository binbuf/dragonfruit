// SPDX-License-Identifier: MIT
//! Bridge tests for the Printers and Scanners half of the host (T-15.12b): the
//! view the Settings pane and Control Center tile decode, plus the three
//! explicit queue writes applied once through the adapter.

use dragonfruit_printer_adapter::{
    MockPrint, PrintData, PrintJobData, PrinterData, PrinterState, ScanDeviceData, ScannerKind,
};
use dragonfruit_system_status::PrintersHost;

fn data() -> PrintData {
    PrintData {
        printers: Some(vec![
            PrinterData {
                name: "HP_LaserJet".to_owned(),
                display_name: "HP LaserJet".to_owned(),
                state: PrinterState::Processing,
                state_message: "Printing".to_owned(),
                accepting_jobs: true,
                enabled: true,
                jobs: vec![PrintJobData {
                    id: 9,
                    user: "sam".to_owned(),
                    size: 64,
                }],
                ..PrinterData::default()
            },
            PrinterData {
                name: "Canon_MF230".to_owned(),
                display_name: "Canon MF230".to_owned(),
                make_and_model: "Canon MF230 Series".to_owned(),
                location: "Office".to_owned(),
                uri: "ipp://192.168.0.5/ipp/print".to_owned(),
                state: PrinterState::Idle,
                state_message: "Idle, Last Used".to_owned(),
                accepting_jobs: true,
                enabled: true,
                is_default: true,
                jobs: vec![],
            },
        ]),
        default_printer: "Canon_MF230".to_owned(),
        scanners: Some(vec![ScanDeviceData {
            device: "epson2:net:192.168.0.7".to_owned(),
            description: "Epson GT-1500 flatbed scanner".to_owned(),
            kind: ScannerKind::Flatbed,
        }]),
    }
}

#[test]
fn the_printers_view_exposes_the_queues_scanners_and_default() {
    let mut host = PrintersHost::new(MockPrint::present(data()));
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "printers");
    assert_eq!(view["state"], "available");
    assert_eq!(view["glyph"], "printer");
    assert_eq!(view["label"], "2 Printers, 1 Scanner");
    assert_eq!(view["printersAvailable"], true);
    assert_eq!(view["scannersAvailable"], true);
    assert_eq!(view["defaultPrinter"], "Canon_MF230");
    assert_eq!(view["printers"][0]["name"], "Canon_MF230");
    assert_eq!(view["printers"][0]["stateMessage"], "Idle, Last Used");
    assert_eq!(view["printers"][1]["jobCount"], 1);
    assert_eq!(
        view["scanners"][0]["displayName"],
        "Epson GT-1500 flatbed scanner"
    );
}

#[test]
fn a_host_without_sane_keeps_the_printer_half_live() {
    let mut host = PrintersHost::new(MockPrint::present(PrintData {
        scanners: None,
        ..data()
    }));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["printersAvailable"], true);
    assert_eq!(view["scannersAvailable"], false);
    assert_eq!(view["scannerCount"], 0);
}

#[test]
fn the_queue_writes_apply_once_and_report_polkit_denials() {
    let mut host = PrintersHost::new(MockPrint::present(data()));
    host.refresh();
    assert_eq!(
        host.set_default_printer("HP_LaserJet")["outcome"],
        "applied"
    );
    assert_eq!(
        host.set_printer_accepting_jobs("HP_LaserJet", false)["outcome"],
        "applied"
    );
    assert_eq!(host.cancel_job(9)["outcome"], "applied");
    assert_eq!(host.adapter().source().writes(), 3);

    let mut denied =
        PrintersHost::new(MockPrint::present(data()).deny_writes("cups: not authorized"));
    denied.refresh();
    assert_eq!(
        denied.set_default_printer("HP_LaserJet")["outcome"],
        "denied"
    );
    assert_eq!(denied.view()["state"], "available");
}

#[test]
fn absence_hides_the_item_and_writes_answer_absence() {
    let mut host = PrintersHost::new(MockPrint::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
    assert_eq!(host.set_default_printer("Canon_MF230")["outcome"], "absent");
    assert_eq!(host.cancel_job(1)["outcome"], "absent");
    assert_eq!(host.view()["state"], "unavailable");
}

#[test]
fn a_read_failure_is_a_visible_inert_error() {
    let mut host = PrintersHost::new(MockPrint::failing("CUPS: timeout"));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "error");
    assert_eq!(view["error"], "CUPS: timeout");
}
