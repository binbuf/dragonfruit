# 0270 — General system sub-panes: host writes, searchable pickers, and Linux reasons

- **Status:** Accepted (T270)
- **Date:** 2026-11-15

## Context

T269 made the General list independent and completed About, Software Update,
Storage, and Autofill & Passwords. The remaining sub-panes were still the T226
first pass: Date & Time and Language & Region exposed raw `Select` lists of
IANA zones and locale ids, the preferred-language list did not exist, login
items could be toggled but not added or removed, Sharing had no hostname and
no per-service options, Backup had no destination picker or validation, Startup
Disk printed decimal `GB` for binary sizes and duplicated the device row, and
Transfer or Reset had no Transfer or Erase structure. General host writes were
also fire-and-forget: a polkit denial or missing backend left the control in
the state the user attempted.

The design target is §4.6.4–4.6.8 of
[24-system-settings.md](../24-system-settings.md). Each sub-pane has its own
owner and its own absence path; a settingsd-backed row must stay live when the
host adapter is absent.

## Decision

1. **One searchable picker for long option lists.** `SearchablePicker` joins
   the design-system components beside `Select`: the same current-label and
   chevron anatomy, a bounded scrolling list, substring filtering, and the
   Down/Up/Return/Escape keyboard path. Date & Time, Language & Region, and the
   login-item picker use it instead of a raw `Select` or a bespoke dialog.
2. **The preferred-language order is a durable settingsd key.**
   `locale.languages` (`as`, schema revision 29) holds the order; its first
   entry is the locale the host is asked to apply with `localectl`. Opening the
   pane seeds the visible list from the host locale without writing the key
   (the T268 no-write-on-open rule). Add, remove, and reorder write the list;
   only a changed first entry also writes the host locale.
3. **The preview formats through the selected locale.** Medium is the locale's
   long date without its weekday (Qt exposes short and long only), so short,
   medium, and long are distinct. The number preview distinguishes the locale,
   a decimal point, and a decimal comma through `toLocaleString`.
4. **Host writes report their outcome.** The General client emits
   `writeFinished(operation, outcome, message)`; the bridge exposes
   `generalWriteOperation`, `generalWriteState`, and `generalWriteError`. A pane
   shows `busy` on its control and the reason on the row for `denied`, `failed`,
   or `absent`. A failed write never leaves the control claiming success.
5. **Add/remove login items are explicit host writes.** The host offers the
   installed `.desktop` applications that are not already login items; Add
   writes a user autostart entry and Remove deletes a user entry or writes a
   `Hidden=true` user override for a system entry. An entry whose command is
   missing is a visible broken-item warning, not a silent no-op.
6. **Sharing owns a hostname and per-service details.** Local hostname reads
   `hostnamectl` (with honest fallbacks) and writes `hostnamectl set-hostname`
   after inline DNS-label validation. Each service's Options sheet lists the
   unit, address, and the real port/share/queue count the host reports; the row
   description no longer repeats on/off or "requires authentication".
7. **Backup validates before it writes.** The destination is an absolute path
   or a repository URL; a portal folder picker fills it when available. Schedule
   and Back up automatically disable when no backend is installed. The absence
   note names the missing system status host, not a missing backend.
8. **Startup Disk is a list plus one confirmed restart.** `lsblk -J` supplies
   every disk with the boot disk first; sizes are GiB. Restart is one
   `systemctl reboot` request behind a confirmation dialog. Changing the
   startup disk remains read-only and points at the firmware menu.
9. **Transfer or Reset states the Linux limits.** Transfer rows are disabled
   delegates with their reason; Reset is destructive-styled and requires typing
   `RESET`; Erase All Content is a read-only Linux reason, never a fake button.
10. **“Set time zone automatically” is a real inert row.** No location service
    is installed, so the toggle stays visible and disabled with that reason
    instead of pretending to write.

## Consequences

- Schema revision 29 adds `locale.languages`; the C++ schema mirror, the key
  doc, and the schema-doc test move with it.
- The General host gains `AddAutostart`, `RemoveAutostart`, `SetHostname`, and
  `RestartSystem`; the Settings bridge gains the matching invokables and one
  write-state property set.
- `SearchablePicker` is the shared control for any long option list; a pane
  that needs search must not re-roll a field-and-list popup.
- The live Wayland visual check (open each sub-pane, change a value, stop the
  adapter) remains T284's; this task records the offscreen light/dark evidence
  and the host-absent stills under `docs/captures/t270-general-*`.
- The Linux gaps are explicit: no geoclue/location service, no migration
  assistant, no sealed-system erase, and no bootloader rewrite.
