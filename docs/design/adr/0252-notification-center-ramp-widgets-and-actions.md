# 0252 — Notification Center ramp, widget stack and action ownership

- Status: accepted
- Date: 2026-11-10
- Task: T234

## Context

T204/T205 mapped the Center and gave it two disjoint material regions, but the
view was still a 360 pt history list with no header actions, no measured
banner-stack row, no widgets and no gallery. The structural review measures a
344 pt right-edge ramp (header, 344×75 pt banner stack, 164 pt Calendar widget,
Edit Widgets/Clear actions) and a separate 1027×552 pt gallery with a 240 pt
sidebar and 19 categories. Clearing the visible list also needs a real service
operation: the shell must not guess which rows disappeared.

## Decision

- **The measured ramp is the view.** `NotificationCenter.qml` is 344 pt wide.
  Its header owns Clear All (close every active banner and drop history) and the
  overflow Clear Notifications (drop history only). Active rows are 75 pt; the
  history rows are compact and carry contiguous per-app grouping metadata
  (`group`, `groupStart`, `groupCount`) from `NotificationCenter::normalizeHistory`.
- **Widgets stay inside the lower material band.** The widget stack is optional
  and lives in the lower panel, so the three measured bands remain banner,
  banner stack and lower panel. A widget is never a third glass sheet. The live
  Calendar card is 164 pt; the gallery thumbnail is 102×102 pt.
- **The gallery is a separate window.** `WidgetGallery.qml` is rendered into a
  centered `widget-gallery` overlay layer surface at 1027×552 pt. It is not a
  sheet inside the Center; opening it retires the Center and any competing
  ambient panel. `WidgetModel` owns the catalogue, add/remove/resize and the
  provider-data seam; a provider-less widget is explicitly unavailable.
- **The service owns clear and expiry.** `ClearAll` closes active banners with
  reason 2 and drops history; `ClearHistory` drops history only. The shell
  calls the service and re-reads `Banners()`/`History()` on `Changed`. The
  banner's shell-side deadline timer only calls the existing `Expire`; it never
  removes the banner locally.
- **Banner motion is shell-local and non-focus-stealing.** The banner surface
  runs a short slide/fade with a reduced-motion collapse and draws up to two
  peeking stack edges from the active count. It never takes keyboard focus.
- **Focus stays one policy.** ADR 0058's `off`/`focus`/`dnd` admission rule and
  the allow list remain the single source of truth. Named profiles
  (Personal/Work/Sleep/custom) are a future presentation layer, not a second
  policy. Per-app settings remain the service's `SetAppPreference` view.
- **Per-display placement is unchanged.** The Center has no explicit output;
  the compositor anchors it on the output that invoked it.

## Consequences

- The three measured materials stay independently keyed; the lower panel's
  region now also covers the widget stack and the action footer.
- A service restart clears the stale view through the existing absence path;
  the clear operations are observable over the same bus interface.
- Named Focus profiles, the live Weather provider, inline reply and Center
  open/close motion remain explicit `OPEN` rows with rerun commands.
