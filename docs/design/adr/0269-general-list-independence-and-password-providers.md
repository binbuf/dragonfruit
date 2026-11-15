# 0269 — General list independence and password-provider delegation

- **Status:** Accepted (T269)
- **Date:** 2026-11-15

## Context

The General pane was still the T226 shape: one `Settings.updatesAvailable`
gate wrapped the whole list, About and Software Update were dialogs, Storage
was a separate sidebar pane, Autofill & Passwords was missing, and the About
page hardcoded “Display: Not available”, duplicated details behind a text
toggle, and had no real System Report. The 2026-11-14 pane review listed
P0-4, P1-2, P1-7, and P1-18 against this surface.

The host identity and the distribution update provider are already two
halves of one Updates view (ADR 0136): the identity is read even when no
provider is attached. The General host is a separate optional adapter.

## Decision

1. **The General list is unconditional.** Every reference row stays visible
   whether or not either host answers. Each row computes its own summary from
   its own half and says `Unavailable` only for that half. There is no
   whole-list absence note.
2. **About and Software Update read the identity independently of the
   provider.** A host with no provider still shows About and the identity
   rows; Software Update says `No update provider` and never invents
   “Up to Date”.
3. **Every row opens an in-place sub-pane.** Software Update joins About,
   Storage, Autofill & Passwords, Date & Time, Language & Region, Login
   Items & Extensions, Sharing, Startup Disk, Backup, and Transfer or Reset.
   The `storage` pane id and `settings://pane/storage` remain aliases of
   General → Storage. Deep links resolve to the sub-pane, not a dialog.
4. **About is one canonical page with one report sheet.** It shows the real
   host model, processor, memory, graphics, serial, OS/kernel/architecture,
   the live screen name/resolution, and volume/free/total. “System Report…”
   opens a dialog sheet; there is no inline details toggle. Closing About
   restores the user's pre-About window geometry.
5. **Autofill & Passwords is a read-only inventory plus a delegate.** The
   General host probes installed manager executables and the Secret Service
   name owner, reads the collections' `Locked` property when the service
   answers, and exposes an `OpenPasswordManager` request. The pane writes only
   `defaults.passwordManager` (the provider id, empty for the active
   provider). Settings never opens a secret item, reads a credential, or
   stores one. With no provider the pane shows the explanatory note.
6. **Storage is General → Storage.** The existing UDisks2 pane keeps its
   volume mount/eject rows and adds a stacked category chart with
   total/used/free, per-volume capacity from the `df` read, recommendations
   derived from the live categories, and a Manage storage… button that opens
   the Files app. The chart and recommendations stay visible when UDisks2 is
   absent but `df` answers.

## Consequences

- The schema gains revision 28 with `defaults.passwordManager` (text, empty
  default, no secret material).
- `services/general-adapter` gains the password inventory and the delegate
  request; `services/system-status` projects `passwords` and exposes
  `OpenPasswordManager`.
- The Settings map gains `updates.* → software-update`,
  `passwords.* → passwords`, `reset.transfer`, and marks the password rows
  shipped. Software Update is no longer a dialog surface.
- About no longer depends on the update provider. The update tile still hides
  only when the whole host is absent.
- T270 owns the Date & Time, Language & Region, Login Items, Sharing, Backup,
  Startup Disk, and Transfer or Reset content. T273 may deepen the Storage
  chart without changing the General IA.
