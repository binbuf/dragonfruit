# System Integration

## Summary

Wi-Fi, Bluetooth, sound, and power UI are much easier than they initially
appear because we are replacing the **presentation**, not the
hardware-management stack. Every subsystem below already exposes a D-Bus (or
equivalent) API intended exactly for custom desktop frontends.

## The adapters

| Subsystem | Service | What we consume |
|---|---|---|
| Networking | NetworkManager | Device/access-point enumeration; activate, deactivate, create, edit, delete connections. `libnm` provides a higher-level client on top of the D-Bus service. |
| Bluetooth | BlueZ | `org.bluez.Adapter1` and device objects; `bluetoothd` keeps handling controller/protocol details. |
| Audio | PipeWire + WirePlumber | PipeWire supplies the audio/video graph; WirePlumber is the policy/session manager and provides an API intended for management/status applications. |
| Power | UPower | Batteries and power devices over the system bus. |
| Power profiles | power-profiles-daemon | performance / balanced / power-saver selection where the service exists (Battery pane, Control Center). |
| Storage | UDisks2 + GIO/GVfs | UDisks provides block devices, monitoring, and mount operations; GIO has abstractions for user-interesting volumes and mounts. |
| Printing & scanning | CUPS + SANE | IPP printing via the standard `org.cups.*` / freedesktop print APIs; drivers and queues stay with CUPS. |
| Secrets | Secret Service API | Reuse the host keyring (e.g. gnome-keyring); we never build a credential store, and first-party apps never cache secrets themselves. |
| Date & time | systemd-timedated | Timezone and NTP settings for the Settings panes. |
| User accounts | accountsservice | User list, avatars, account type — for Settings' Users & Groups pane. |
| Seat/session | systemd / logind | Session lifetime, `graphical-session.target`, VT management. |
| Privileged operations | host policy infrastructure | polkit / authentication agent integration; we never roll our own privilege escalation. |

## Principles

1. **Replace the presentation, not the stack.** The Control Center (see
   [04-shell.md](04-shell.md)) and Settings (see [08-settings.md](08-settings.md))
   are completely custom UIs over these daemons.
2. **Adapters are thin and testable.** Each adapter is a small, mockable
   module exposing a stable internal API; nothing above the adapter knows
   which daemon implements it.
3. **The compositor owns what the compositor owns.** Displays, brightness,
   and keyboard settings go through our compositor API, not through
   system daemons — the compositor owns physical outputs and input.
4. **Degrade gracefully when a service is absent.** Minimal VMs, servers,
   and CI containers lack Bluetooth, batteries, and printers. Every adapter
   exposes an explicit "unavailable" state; panes hide or disable accordingly,
   and nothing blocks session startup on one daemon.

## D-Bus conventions

Our services own names under `org.dragonfruit.*` on the **user session bus**
(settings, menus, app index, notifications, portal). Nothing of ours needs a
system-bus service of its own; privileged operations are delegated to
existing system services behind polkit, whose authentication prompt is
rendered by our shell. Absence of a daemon is a normal state, not an error
(see principle 4 above).

## Portals

Once the desktop is a real Wayland session, `xdg-desktop-portal` support
arrives **surprisingly early** in the sequence (see
[ROADMAP.md](../ROADMAP.md)). The portal project is specifically designed
around a common frontend working with desktop-environment-specific backends.

We ship:

```text
xdg-desktop-portal-dragonfruit
```

for our native file chooser, screenshots, and screen sharing. The
FileChooser interface is delegated to **Files in chooser mode**, backed by the
same `files-core` library — one browsing implementation, one set of semantics
(see [09-files.md](09-files.md)). This becomes especially important for
browsers, Electron programs, Flatpak applications, conferencing programs,
and screen capture.
