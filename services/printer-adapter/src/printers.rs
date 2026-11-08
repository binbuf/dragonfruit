// SPDX-License-Identifier: MIT
//! The live transport: CUPS and SANE through the client tools they ship.
//!
//! The **printers** half is CUPS. CUPS ships no stable machine-readable local
//! API, and its own client tools are the intended front door, so the adapter
//! reads one `lpstat -l -t` run (scheduler state, default destination, device
//! URIs, accepting state, queues, long queue info, and queued jobs) and writes
//! through `lpadmin -d`, `cupsaccept`/`cupsreject`, and `cancel`. This is the
//! same reuse the audio adapter makes of `pw-dump`/`wpctl` and the input
//! adapter of `libinput list-devices`: the tools' human-oriented output churn
//! is pinned in [`printers_from_lpstat`] and [`scanners_from_scanimage`], both
//! tested against captured fixtures (see
//! [docs/design/adr/0140-printers-and-scanners-adapter.md]).
//!
//! The **scanners** half is SANE, enumerated through `scanimage -L`.
//!
//! The source constructs no process until it is called, so building an adapter
//! is free and a session without CUPS or SANE still boots. A tool that cannot
//! be spawned, or a `lpstat` that reports the scheduler is not running, is
//! treated as absence; a running stack that reports no queue or no device is a
//! present, empty half. A write whose tool exits non-zero is classified as a
//! policy denial or a failure.
//!
//! [docs/design/adr/0140-printers-and-scanners-adapter.md]: ../../../docs/design/adr/0140-printers-and-scanners-adapter.md

use std::collections::HashMap;
use std::process::{Command, Output};

use dragonfruit_system_adapters::AdapterError;

use crate::source::{
    PrintData, PrintJobData, PrintOutcome, PrintSource, PrinterData, PrinterDeviceData,
    PrinterOptionData, PrinterState, ScanDeviceData, ScannerKind,
};

/// The CUPS status tool (`cups-client`).
pub const LPSTAT_BIN: &str = "lpstat";
/// The CUPS administration tool.
pub const LPADMIN_BIN: &str = "lpadmin";
/// The CUPS accept tool.
pub const CUPSACCEPT_BIN: &str = "cupsaccept";
/// The CUPS reject tool.
pub const CUPSREJECT_BIN: &str = "cupsreject";
/// The CUPS job-cancel tool (`cups-client`).
pub const CANCEL_BIN: &str = "cancel";
/// The SANE command-line frontend (`sane-utils`).
pub const SCANIMAGE_BIN: &str = "scanimage";
/// The CUPS discovery tool (`cups-client`).
pub const LPINFO_BIN: &str = "lpinfo";
/// The CUPS per-queue option tool (`cups-client`).
pub const LPOPTIONS_BIN: &str = "lpoptions";

/// The live CUPS + SANE source.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HostPrint;

impl HostPrint {
    /// A source reading CUPS and SANE with no process started until `read`.
    pub fn new() -> Self {
        HostPrint
    }
}

impl PrintSource for HostPrint {
    fn read(&mut self) -> Result<Option<PrintData>, AdapterError> {
        let cups = cups_read();
        let sane = sane_read();
        if cups.is_none() && sane.is_none() {
            return Ok(None);
        }
        let (printers, default_printer) = match cups {
            Some((printers, default_printer)) => (Some(printers), default_printer),
            None => (None, String::new()),
        };
        Ok(Some(PrintData {
            printers,
            default_printer,
            scanners: sane,
        }))
    }

    fn set_default_printer(&mut self, name: &str) -> PrintOutcome {
        run_write(LPADMIN_BIN, &["-d", name])
    }

    fn set_printer_accepting_jobs(&mut self, name: &str, accepting: bool) -> PrintOutcome {
        let bin = if accepting {
            CUPSACCEPT_BIN
        } else {
            CUPSREJECT_BIN
        };
        run_write(bin, &[name])
    }

    fn cancel_job(&mut self, job_id: u32) -> PrintOutcome {
        run_write(CANCEL_BIN, &[&job_id.to_string()])
    }

    fn discover(&mut self) -> Result<Vec<PrinterDeviceData>, AdapterError> {
        let output =
            run(LPINFO_BIN, &["-v"]).ok_or_else(|| AdapterError::new("lpinfo: not available"))?;
        Ok(printer_devices_from_lpinfo(&combined(&output)))
    }

    fn add_printer(&mut self, name: &str, uri: &str, driver: &str) -> PrintOutcome {
        let mut args = vec!["-p", name, "-E", "-v", uri];
        if !driver.is_empty() {
            args.push("-m");
            args.push(driver);
        }
        run_write(LPADMIN_BIN, &args)
    }

    fn printer_options(&mut self, name: &str) -> Result<Vec<PrinterOptionData>, AdapterError> {
        let output = run(LPOPTIONS_BIN, &["-p", name, "-l"])
            .ok_or_else(|| AdapterError::new("lpoptions: not available"))?;
        Ok(printer_options_from_lpoptions(&combined(&output)))
    }

    fn set_printer_option(&mut self, name: &str, key: &str, value: &str) -> PrintOutcome {
        run_write(
            LPOPTIONS_BIN,
            &["-p", name, "-o", &format!("{key}={value}")],
        )
    }
}

/// Decode one `lpinfo -v` run into the discovery list.
///
/// Each non-empty line is `<class> <uri>`, optionally followed by a human
/// description. Unknown shapes are skipped.
pub fn printer_devices_from_lpinfo(output: &str) -> Vec<PrinterDeviceData> {
    let mut devices = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(' ') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else { continue };
        let Some(uri) = parts.next() else { continue };
        if !uri.contains("://") && !uri.starts_with("ipp:") && !uri.starts_with("usb:") {
            continue;
        }
        let description = parts.collect::<Vec<_>>().join(" ");
        devices.push(PrinterDeviceData {
            kind: kind.to_owned(),
            uri: uri.to_owned(),
            description,
        });
    }
    devices
}

/// Decode one `lpoptions -p <queue> -l` run into the option list.
///
/// A line is `<key>/<label>: <choice> <choice> …`, with the current choice
/// prefixed by `*`. The label falls back to the key.
pub fn printer_options_from_lpoptions(output: &str) -> Vec<PrinterOptionData> {
    let mut options = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        let Some((head, choices)) = line.split_once(':') else {
            continue;
        };
        let (key, label) = match head.split_once('/') {
            Some((key, label)) => (key.trim(), label.trim()),
            None => (head.trim(), head.trim()),
        };
        if key.is_empty() {
            continue;
        }
        let mut value = String::new();
        let mut values = Vec::new();
        for token in choices.split_whitespace() {
            if let Some(current) = token.strip_prefix('*') {
                value = current.to_owned();
                values.push(current.to_owned());
            } else {
                values.push(token.to_owned());
            }
        }
        options.push(PrinterOptionData {
            key: key.to_owned(),
            label: label.to_owned(),
            value,
            choices: values,
        });
    }
    options
}

/// One `lpstat -l -t` read. `None` means the scheduler is absent.
fn cups_read() -> Option<(Vec<PrinterData>, String)> {
    let output = run(LPSTAT_BIN, &["-l", "-t"])?;
    let text = combined(&output);
    if !text.contains("scheduler is running") {
        return None;
    }
    Some(printers_from_lpstat(&text))
}

/// One `scanimage -L` read. `None` means SANE is absent.
fn sane_read() -> Option<Vec<ScanDeviceData>> {
    let output = run(SCANIMAGE_BIN, &["-L"])?;
    Some(scanners_from_scanimage(&combined(&output)))
}

/// Decode one `lpstat -l -t` run into the raw queue read and the default
/// destination name.
///
/// The output is the `-t` summary — scheduler state, the default destination,
/// the device URIs, the accepting state, the queue list with its long info,
/// and the queued jobs. Unknown lines are ignored so a tool that grows a field
/// keeps working; missing attributes stay at their default. This is the one
/// place the tool's spelling is known: it is pinned by the fixture and the
/// churn stays behind the source seam.
pub fn printers_from_lpstat(output: &str) -> (Vec<PrinterData>, String) {
    let mut default = String::new();
    let mut devices: HashMap<String, String> = HashMap::new();
    let mut accepting: HashMap<String, bool> = HashMap::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("system default destination:") {
            default = rest.trim().to_owned();
        } else if let Some(rest) = trimmed.strip_prefix("device for ") {
            if let Some((name, uri)) = rest.split_once(':') {
                devices.insert(name.trim().to_owned(), uri.trim().to_owned());
            }
        } else if !line.starts_with('\t') && !trimmed.starts_with("printer ") {
            // An accepting line: `<name> accepting requests since …` or
            // `<name> not accepting requests since …`.
            let mut parts = trimmed.split_whitespace();
            if let (Some(name), Some(verb)) = (parts.next(), parts.next()) {
                match verb {
                    "accepting" => {
                        accepting.insert(name.to_owned(), true);
                    }
                    "not" if parts.next() == Some("accepting") => {
                        accepting.insert(name.to_owned(), false);
                    }
                    _ => {}
                }
            }
        }
    }

    let mut printers: Vec<PrinterData> = Vec::new();
    let mut current: Option<PrinterData> = None;
    let mut jobs: Vec<(String, PrintJobData)> = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if line.starts_with('\t') {
            let Some(printer) = current.as_mut() else {
                continue;
            };
            if let Some(value) = trimmed.strip_prefix("Description:") {
                printer.display_name = value.trim().to_owned();
            } else if let Some(value) = trimmed.strip_prefix("Location:") {
                printer.location = value.trim().to_owned();
            } else if let Some(value) = trimmed.strip_prefix("Status:") {
                printer.state_message = value.trim().to_owned();
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("printer ") {
            if let Some(printer) = current.take() {
                printers.push(printer);
            }
            let name = rest
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_owned();
            let after = rest[name.len()..].trim();
            let lower = after.to_ascii_lowercase();
            current = Some(PrinterData {
                name,
                state: PrinterState::from_name(&lower),
                enabled: !lower.contains("disabled"),
                ..PrinterData::default()
            });
            continue;
        }
        if let Some((name, job)) = parse_job_line(trimmed) {
            jobs.push((name, job));
        }
    }
    if let Some(printer) = current.take() {
        printers.push(printer);
    }

    for printer in &mut printers {
        if let Some(uri) = devices.get(&printer.name) {
            printer.uri = uri.clone();
        }
        printer.accepting_jobs = accepting
            .get(&printer.name)
            .copied()
            .unwrap_or(printer.enabled);
        printer.is_default = !default.is_empty() && printer.name == default;
        printer.jobs = jobs
            .iter()
            .filter(|(name, _)| name == &printer.name)
            .map(|(_, job)| job.clone())
            .collect();
    }

    (printers, default)
}

/// One queued-job line from `lpstat`. The format is
/// `<destination>-<id> <user> <size> <date…>`; a line that does not parse is
/// not a job.
fn parse_job_line(line: &str) -> Option<(String, PrintJobData)> {
    let mut parts = line.split_whitespace();
    let token = parts.next()?;
    let (name, id) = token.rsplit_once('-')?;
    let id: u32 = id.parse().ok()?;
    let user = parts.next()?.to_owned();
    let size = parts
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    Some((name.to_owned(), PrintJobData { id, user, size }))
}

/// Decode one `scanimage -L` run into the raw scanner list.
///
/// Each device is one line of the form
/// ``device `backend:name' is a <free-form description>``. The description is
/// kept verbatim; only the physical form is derived from it.
pub fn scanners_from_scanimage(output: &str) -> Vec<ScanDeviceData> {
    let mut scanners = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("device `") else {
            continue;
        };
        let Some((device, tail)) = rest.split_once('\'') else {
            continue;
        };
        let description = tail
            .trim()
            .strip_prefix("is a ")
            .unwrap_or(tail.trim())
            .trim()
            .to_owned();
        scanners.push(ScanDeviceData {
            device: device.trim().to_owned(),
            kind: ScannerKind::from_description(&description),
            description,
        });
    }
    scanners
}

/// Run a tool with the C locale and capture its output. `None` means the tool
/// could not be spawned (not installed): absence, not an error.
fn run(bin: &str, args: &[&str]) -> Option<Output> {
    Command::new(bin)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .ok()
}

/// A write tool's outcome: absence when it cannot be spawned, `Applied` on a
/// zero exit, else a classified denial or failure.
fn run_write(bin: &str, args: &[&str]) -> PrintOutcome {
    match run(bin, args) {
        None => PrintOutcome::Absent,
        Some(output) if output.status.success() => PrintOutcome::Applied,
        Some(output) => classify(&combined(&output)),
    }
}

/// Map a non-zero tool exit to a denial or a failure.
fn classify(message: &str) -> PrintOutcome {
    let message = message.trim();
    let lower = message.to_ascii_lowercase();
    if lower.contains("not authorized")
        || lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("forbidden")
        || lower.contains("not permitted")
        || lower.contains("policy")
        || lower.contains("not allowed")
    {
        PrintOutcome::Denied(format!("CUPS: {message}"))
    } else {
        PrintOutcome::Failed(AdapterError::new(format!("CUPS: {message}")))
    }
}

/// stdout plus stderr, so a failure message on either stream is classified.
fn combined(output: &Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const LPSTAT_LONG: &str = "\
scheduler is running\n\
system default destination: Canon_MF230\n\
device for Canon_MF230: ipp://192.168.0.5/ipp/print\n\
device for HP_LaserJet: ipp://192.168.0.9/ipp/print\n\
Canon_MF230 accepting requests since Mon 28 Sep 2026 10:00:00 AM UTC\n\
HP_LaserJet not accepting requests since Mon 28 Sep 2026 10:00:00 AM UTC\n\
printer Canon_MF230 is idle.  enabled since Mon 28 Sep 2026 10:00:00 AM UTC\n\
\tDescription: Canon MF230\n\
\tLocation: Office\n\
\tStatus: Idle, Last Used\n\
printer HP_LaserJet disabled since Mon 28 Sep 2026 10:00:00 AM UTC -\n\
\tStatus: toner low\n\
Canon_MF230-12 dan 2048 Mon 28 Sep 2026 10:05:00 AM UTC\n\
";

    #[test]
    fn the_source_is_free_to_construct() {
        let _ = HostPrint::new();
    }

    #[test]
    fn the_tool_names_are_the_host_ones() {
        assert_eq!(LPSTAT_BIN, "lpstat");
        assert_eq!(LPADMIN_BIN, "lpadmin");
        assert_eq!(CANCEL_BIN, "cancel");
        assert_eq!(SCANIMAGE_BIN, "scanimage");
    }

    #[test]
    fn lpstat_decodes_queues_the_default_and_the_jobs() {
        let (printers, default) = printers_from_lpstat(LPSTAT_LONG);
        assert_eq!(default, "Canon_MF230");
        assert_eq!(printers.len(), 2);

        let canon = &printers[0];
        assert_eq!(canon.name, "Canon_MF230");
        assert_eq!(canon.display_name, "Canon MF230");
        assert_eq!(canon.location, "Office");
        assert_eq!(canon.uri, "ipp://192.168.0.5/ipp/print");
        assert_eq!(canon.state, PrinterState::Idle);
        assert_eq!(canon.state_message, "Idle, Last Used");
        assert!(canon.accepting_jobs);
        assert!(canon.enabled);
        assert!(canon.is_default);
        assert_eq!(canon.jobs.len(), 1);
        assert_eq!(canon.jobs[0].id, 12);
        assert_eq!(canon.jobs[0].user, "dan");
        assert_eq!(canon.jobs[0].size, 2048);

        let hp = &printers[1];
        assert_eq!(hp.name, "HP_LaserJet");
        assert_eq!(hp.state, PrinterState::Stopped);
        assert!(!hp.accepting_jobs);
        assert!(!hp.enabled);
        assert!(!hp.is_default);
    }

    #[test]
    fn a_running_scheduler_with_no_queue_is_an_empty_list() {
        let text =
            "scheduler is running\nno system default destination\nlpstat: No destinations added.\n";
        let (printers, default) = printers_from_lpstat(text);
        assert!(printers.is_empty());
        assert!(default.is_empty());
    }

    #[test]
    fn scanimage_decodes_devices_and_kinds() {
        let text = "\
device `epson2:net:192.168.0.7' is a Epson GT-1500 flatbed scanner\n\
device `pixma:04A9173E' is a Canon PIXMA MG3600 series\n";
        let scanners = scanners_from_scanimage(text);
        assert_eq!(scanners.len(), 2);
        assert_eq!(scanners[0].device, "epson2:net:192.168.0.7");
        assert_eq!(scanners[0].description, "Epson GT-1500 flatbed scanner");
        assert_eq!(scanners[0].kind, ScannerKind::Flatbed);
        assert_eq!(scanners[1].device, "pixma:04A9173E");
        assert_eq!(scanners[1].kind, ScannerKind::Unknown);
    }

    #[test]
    fn a_no_scanners_message_is_an_empty_list() {
        let text = "No scanners were identified. If you were expecting something\n\
different, check that the scanner is plugged in.\n";
        assert!(scanners_from_scanimage(text).is_empty());
    }

    #[test]
    fn a_policy_message_classifies_as_denied() {
        assert_eq!(
            classify("lpadmin: Not authorized"),
            PrintOutcome::Denied("CUPS: lpadmin: Not authorized".to_owned())
        );
    }

    #[test]
    fn any_other_message_is_a_failure() {
        let outcome = classify("lpadmin: Unable to connect to server");
        assert!(matches!(outcome, PrintOutcome::Failed(_)));
        assert_eq!(
            outcome.error().map(|error| error.message()),
            Some("CUPS: lpadmin: Unable to connect to server")
        );
    }

    #[test]
    fn lpinfo_decodes_devices() {
        let text = "network ipp://192.168.0.12/ipp/print Office LaserJet\ndirect usb://Canon/MF230 Canon MF230 USB\nnetwork beh\n";
        let devices = printer_devices_from_lpinfo(text);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].kind, "network");
        assert_eq!(devices[0].uri, "ipp://192.168.0.12/ipp/print");
        assert_eq!(devices[0].description, "Office LaserJet");
        assert_eq!(devices[1].kind, "direct");
        assert_eq!(devices[1].uri, "usb://Canon/MF230");
    }

    #[test]
    fn lpoptions_decodes_labels_current_values_and_choices() {
        let text = "PageSize/Media Size: *A4 Letter\nDuplex/Double-Sided: None *DuplexNoTumble\n";
        let options = printer_options_from_lpoptions(text);
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].key, "PageSize");
        assert_eq!(options[0].label, "Media Size");
        assert_eq!(options[0].value, "A4");
        assert_eq!(options[0].choices, vec!["A4", "Letter"]);
        assert_eq!(options[1].key, "Duplex");
        assert_eq!(options[1].value, "DuplexNoTumble");
    }

    #[test]
    fn a_missing_tool_is_absence() {
        assert_eq!(
            run_write("dragonfruit-no-such-cups-tool", &[]),
            PrintOutcome::Absent
        );
    }
}
