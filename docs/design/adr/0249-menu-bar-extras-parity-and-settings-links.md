# 0249 — Menu-bar extras parity and Settings deep links

## Status

Accepted (T232).

## Context

T231 moved every menu-bar extra onto one status-item registry and shipped the
Date & Time panel. The remaining extras still drew a subset of their live
state: Wi-Fi had the radio and visible networks but no saved networks, current
network details, or Settings link; Bluetooth had paired devices but no
discovery or nearby devices; Battery had the charge state but no power mode or
Settings link; Sound had volume/mute but no output routing or Settings link;
Search and Quick Settings had no explicit parity case. The Settings links in
the Control Center were log-only stubs, so a click could not reach a pane.

## Decision

1. **One model per extra, no second polling.** Wi-Fi, Bluetooth, Battery, and
   Sound keep reading the bridge host's decoded views
   (`org.dragonfruit.SystemStatus1.*`) exactly as their Settings panes do.
   The extras add only presentation and the explicit writes the host already
   exposes: `SetDefaultSink` for output routing and `SetActiveProfile` for the
   power mode. The shell status client forwards both; the host re-reads and
   pushes the view, so no extra invents state.
2. **Reference order inside each popover.** Wi-Fi shows the radio, the current
   network with an inline `Details…` block, `Known Networks`, `Other
   Networks`, and `Wi-Fi Settings…`. Bluetooth shows the power action, `My
   Devices`, discovery/`Nearby Devices`, and `Bluetooth Settings…`. Battery
   shows charge/time/health, `Power Mode`, and `Battery Settings…`. Sound
   shows the output device list, volume, mute, and `Sound Settings…`.
   A missing daemon keeps the existing hidden/error split; a present but
   unreadable view shows the error text.
3. **Settings links are real deep links.** The shell opens
   `settings://pane/<id>` through one helper. If the Settings app is running it
   owns `org.dragonfruit.SettingsApp1` and receives `Open(url)` on the session
   bus; otherwise the app is launched with `--deep-link <url>`. The app's
   `SettingsShell.deepLink` is the single consumer, so the links land on the
   same pane/row a Settings search result would.
4. **Search and Quick Settings stay registry entries.** Search activation
   opens the Applications drawer (existing T231 path) and its visibility is the
   existing `menu.control.search` key. The Quick Settings item opens the one
   Control Center panel; the panel footer adds `Quick Settings Settings…` so
   the extra also reaches its customization pane.

## Consequences

- The menu-bar extras and their Settings panes share one read path and one
  write path; a routing or power-mode change converges in both surfaces.
- Keyboard traversal (Up/Down/Return) and accessible names/descriptions cover
  every new row, including the disabled/active reason.
- The running-instance deep-link endpoint is a small Settings-app addition;
  the shell never parses a pane id itself.
- The capture matrix (`make t232-extras-capture`) records each extra in light
  and dark, plus the Control Center panel. Live host daemons remain manual
  evidence, as with the other bridge-host surfaces.
