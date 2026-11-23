# System Settings logic

| | |
|---|---|
| **Status** | target design; **Energy** is the first completed category |
| **Date** | 2026-11-20 |
| **Scope** | Per-category behaviour: which rows exist on which hardware, and where our logic deliberately differs from the macOS reference |
| **Companions** | [24-system-settings.md](24-system-settings.md), [25-system-settings-visual-parity.md](25-system-settings-visual-parity.md), [settings-business-logic-map.md](settings-business-logic-map.md) |
| **ADR** | [0300](adr/0300-energy-category.md) |

## 1. Purpose

Three documents describe Settings, and they answer different questions:

- [settings-business-logic-map.md](settings-business-logic-map.md) — **who owns
  this row, and what happens when the owner is absent?**
- [25-system-settings-visual-parity.md](25-system-settings-visual-parity.md) —
  **what does the 2026-11-17 mockup look like?**
- This document — **for this machine, which rows should exist, and why do they
  differ from the reference?**

The distinction matters because the macOS reference was captured on one
machine (a MacBook). Dragonfruit ships on laptops and desktops, and macOS
itself varies the same pane by hardware: it is **Battery** on a laptop and
**Energy** on a desktop. A later task that compares a desktop build to the
laptop mockup must be able to tell an intentional difference from a regression.

Rules:

1. One section per category. Fill a category in when its task lands; do not
   pre-write empty sections.
2. Record every deliberate divergence with its reason and owner. A divergence
   without a reason is a bug, not a design decision.
3. This document does not replace the owner/absence contract. Every row still
   needs a `SettingsMap` owner or an omission reason.
4. When this document and the visual target disagree about **content**, this
   document wins for *which rows exist*; `25` wins for *how a row looks*.
5. The mockup is evidence, not a mandate. Never delete a host-backed row only
   to make a screenshot match a laptop capture.

## 2. Energy

**Sidebar title:** Energy. **Pane id:** `battery` (stable).
**Deep links:** `settings://pane/battery`; `settings://pane/energy` is an
alias. **Owners:** UPower (battery state, health, history sampler) and
power-profiles-daemon (profiles).

### 2.1 One label for both hardware classes

macOS uses one pane with two labels: **Battery** on a Mac with a battery and
**Energy** on a Mac desktop. The 2026-11-17 mockup is a MacBook capture, so it
says Battery. Dragonfruit supports both classes and does not want the category
name to depend on the hardware, so the sidebar says **Energy everywhere**.

The id stays `battery` because it is persisted in pane history, deep links, the
executable map, and the capture scripts. Renaming the id would be churn with no
user-visible benefit; the alias gives the label's spelling a working link.

### 2.2 Content is capability-driven, not label-driven

Two daemons are independent, and either may be absent:

| Daemon | Provides | When absent |
|---|---|---|
| UPower | battery presence, charge state, health, history sampler | the battery group and Usage History show their own note; the profile and policy rows stay live |
| power-profiles-daemon | the active and available profiles | Low Power Mode shows "No power profiles are available"; the battery half stays live |

A missing daemon is an **in-pane absence**, never a hidden category. A desktop
with no battery still selects a profile and still owns the policy rows.

### 2.3 Laptop (battery present)

| Group | Row | Owner / key | Notes |
|---|---|---|---|
| Low Power Mode | Low Power Mode | power-profiles-daemon, `power.profile` | Select; the daemon's profile vocabulary |
| Battery | Battery Health | UPower, `Battery.Health` | `Show Detail` reveals capacity/cycles |
| Usage History | range | settingsd, `power.historyRange` | Last 24 Hours / Last 10 Days |
| Usage History | status pill | UPower, `Battery.State` | live charge state plus estimate |
| Usage History | Battery Level chart | system-status sampler | explicit "no samples yet" state |
| Usage History | Last charged to 100% | UPower, `Battery.EnergyFull` | no timestamp; the row says so |
| Usage History | Screen On Usage | session sampler, `power.screenOn` | no sampler; the row says so |
| Options… | Show battery percentage | settingsd, `power.showPercentage` | consumer: shell/MenuBar |
| Options… | Dim on battery | settingsd, `power.dimOnBattery` | consumer: shell/display |
| Options… | Prevent sleep | settingsd, `power.preventSleep` | consumer: shell idle controller |
| Options… | Wake for network access | host firmware, `power.wakeForNetwork` | disabled; owner named |
| Options… | Optimize video streaming | media pipeline, `power.optimizeVideo` | disabled; owner named |
| Options… | Power Adapter | UPower, `Battery.State` | live source |
| Options… | Charge limit | UPower, `Battery.ChargeThreshold` | read-only; no threshold write |
| Options… | Power Settings | settingsd | Display off, Require password, Lock Screen link |

### 2.4 Desktop (no battery)

Same category, same id, same profile row. These groups change:

| Group | Desktop behaviour | Reason |
|---|---|---|
| Battery | the "This system has no battery" note replaces Battery Health | no `Battery` device in UPower |
| Usage History | the "Usage history needs a present battery" note replaces the range, pill, chart, and Screen On Usage; no empty chart is drawn | no battery history to sample |
| Power Adapter | hidden | no battery or adapter state to report |
| Options… → Show battery percentage | stays visible today; T303 hides it when no battery is present | there is no battery glyph to annotate |
| Options… → Dim on battery | stays visible today; T303 hides it when no battery is present | the machine never runs on battery |
| Options… → Prevent sleep / Wake for network / Optimize video / Power Settings | stay visible | they are session, firmware, or media policy |

**Open.** The desktop-only macOS Energy rows are not implemented yet: *Start up
automatically after a power failure*, *UPS options*, *Put hard disks to sleep
when possible* (Intel), and *Power Nap*. They belong here when a real owner is
chosen. Until then they are omissions, not mockup gaps; see §2.6.

### 2.5 Deliberate divergences from the macOS mockup

The mockup is a MacBook capture. These differences are intentional and must not
be "corrected" back to the capture by a later visual-parity task.

| Mockup | Dragonfruit | Reason |
|---|---|---|
| Sidebar label **Battery** | **Energy** on every machine | one label for laptops and desktops; macOS itself uses Energy on desktops |
| Low Power Mode: Never / Always / Only on Battery / Only on Power Adapter | power-saver / balanced / performance | power-profiles-daemon's vocabulary; shared with the Quick Settings tile |
| Battery Health: reference hardware health and optimized charging | UPower capacity and cycle count | the Apple data has no Linux provider |
| Last charged to 100% with a date | "Not available" | UPower exposes no last-charged timestamp |
| Battery Level chart from captured data | sampler-backed chart with an explicit empty state | never fabricate a chart |
| Screen On Usage chart | one honest "Not available" row | no session sampler in this build |
| Options sheet with live Wake for network / Optimize video | both disabled with the owner named | host firmware and the media pipeline are outside the session bus |
| Account / sidebar row | omitted project-wide | not a Linux surface |
| Desktop Energy rows (power-failure restart, UPS, disk sleep, Power Nap) | not implemented; omissions | no owner chosen yet; see §2.6 |

The Linux-only **Power Adapter** and **Charge limit** rows have no mockup
counterpart. They are adaptations, recorded in
[ADR 0293](adr/0293-battery-displays-pane-adaptations.md), not divergences to
remove.

### 2.6 Open items

| Item | Owner to choose | Suggested task |
|---|---|---|
| Desktop-only rows: restart after power failure, UPS, disk sleep, Power Nap | logind / host firmware / UPower | T303 |
| Hide Show battery percentage and Dim on battery when no battery is present | pane visibility, no new owner | T303 |
| A desktop capture in the evidence matrix | T302a | T302a |

## 3. Categories still to document

Fill these in as their tasks land. A category with no section has no
machine-dependent logic recorded yet; it still has an owner/absence contract in
[settings-business-logic-map.md](settings-business-logic-map.md).

| Category | Status |
|---|---|
| Wi-Fi, Bluetooth, Network, VPN | not yet documented |
| General | not yet documented |
| Accessibility | not yet documented |
| Appearance | not yet documented |
| Desktop & Dock, Workspace Overview | not yet documented |
| Displays | not yet documented |
| Menu Bar, Menu Bar Controls | not yet documented |
| Search | not yet documented |
| Wallpaper, Screen Saver | not yet documented |
| Notifications, Focus | not yet documented |
| Sound | not yet documented |
| Lock Screen | not yet documented |
| Privacy & Security | not yet documented |
| Users & Groups, Online Accounts | not yet documented |
| Keyboard, Mouse, Trackpad, Game Controllers | not yet documented |
| Printers & Scanners | not yet documented |
