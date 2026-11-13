// SPDX-License-Identifier: MIT
// Pure Dock entry model (T-10, sections 4/7): merges the persisted pinned set
// with the shell's running-window projection into the ordered entry list the
// Dock renders. No Wayland, no QML, no state — unit-testable in isolation.
#pragma once

#include <QHash>
#include <QPoint>
#include <QRect>
#include <QSize>
#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>

class DesktopEntryIndex;
struct DesktopEntry;

// The compositor-visible `app_id` a launched entry announces (T-14.7l): its
// `StartupWMClass` when present, else the desktop id without its `.desktop`
// suffix. This is the key `df_toplevel_manager.set_launch_origin` is stored
// under; an empty result means no key can be resolved and the launch keeps the
// centering fallback. Pure and unit-tested (tst_dockcore).
QString dockLaunchAppId(const DesktopEntry &entry);

// A bounded `desktopId -> tile rectangle` map for the Dock's launch-origin
// hand-off (T-14.7l). The Dock records the acted-on entry's tile from QML; the
// shell reads it when it launches the app so the compositor can play the
// window motion from the real icon. Bounded so a session that never maps (or
// pins change) cannot grow it without bound; re-recording an existing key
// refreshes it without growing. Pure and unit-tested (tst_dockcore).
class DockTileRects
{
public:
    static constexpr int kCapacity = 32;

    // Record (or refresh) `id`'s tile. When the map is full and `id` is new,
    // the oldest recorded key is evicted. `rect` with a non-positive extent is
    // ignored so a garbage rect can never be handed to the compositor.
    void record(const QString &id, const QRect &rect);
    // The last recorded tile for `id`, or an invalid rect on a miss.
    QRect rectFor(const QString &id) const;
    int size() const { return m_rects.size(); }
    void clear();

private:
    QHash<QString, QRect> m_rects;
    QStringList m_order; // recording order; oldest first
};

// Clamp a launch-origin rect to the output it describes (T-14.7l). An invalid
// `outputBounds` leaves the rect untouched; a rect that does not intersect the
// output returns an invalid rect so the caller keeps the centered fallback.
// Pure and unit-tested (tst_dockcore).
QRect clampDockTileRect(const QRect &rect, const QRect &outputBounds);

// The Dock's slice of the settings schema (T-08.2a). The values are read from
// the `org.dragonfruit.Settings1` client; this struct is the typed view the
// controller and the layout use. The defaults mirror the schema and are the
// same ones the client seeds when settingsd is absent.
struct DockConfig {
    double size = 0.5;
    double magnification = 0.5;
    QString position = QStringLiteral("bottom");
    bool autohide = false;
    bool animateOpening = true;
    bool showIndicators = true;
    bool minimizeIntoTileIcon = false;
    QString minimizedAnimation = QStringLiteral("scale");
    QString titlebarDoubleClick = QStringLiteral("zoom");
    bool showRecentApps = false;
    // Opt-in hover-open for the window chooser (T-14.7p): dwell on a grouped
    // app entry to open its chooser, and retarget along the Dock. Default off
    // keeps the macOS click contract.
    bool chooserOnHover = false;
    // Opt-in minimize-to-icon reaction (T-14.7s): a bounded one-hop bounce on
    // the acting app's entry when one of its windows minimizes, so the
    // window's destination is legible at the Dock. Default off (macOS has no
    // such reaction); disabled under reduced motion by the QML renderer.
    bool minimizeReaction = false;
    bool reduceMotion = false;
    QStringList pinned;
    // Ordered absolute folder paths pinned to the Dock as folder stacks
    // (T-14.7k). Independent of `pinned` so app pins keep their exact
    // `desktopId` semantics.
    QStringList pinnedFolders;

    bool operator==(const DockConfig &) const = default;
};

// Read the Dock keys out of a settings value map (missing keys fall back to
// the schema defaults). Pure and unit-testable.
DockConfig dockConfigFromValues(const QVariantMap &values);

// The recency list the Dock should render (T236): `recentIds` when
// `dock.showRecentApps` is on, empty when it is off. One place decides the
// policy so a QML/controller branch cannot drift. Pure and unit-tested
// (tst_dockcore).
QStringList dockRecentIds(const DockConfig &config, const QStringList &recentIds);

// Map `dock.size` (0..1) onto the icon-size token range. Pure: the QML token
// bounds are passed in by the caller.
int dockIconSize(double size, int iconMin, int iconMax);

// Keyboard reordering (T-14.7t): move the pinned id at `index` one slot by
// `delta`, within the pinned region only. A move that would leave the region
// (0..size-1) is a no-op, never a wrap, as is an out-of-range index or a
// `delta` of 0. The returned list is the complete new `dock.pinned` value the
// Dock emits through the existing drag signal. Pure and unit-tested
// (tst_dockcore); the Dock's QML mirror (`movePinnedEntry`) keeps the same
// semantics because the Dock QML module has no C++ singleton.
QStringList movePinnedEntry(const QStringList &pinnedIds, int index, int delta);

// The default pinned set (T-10 section 19): Files, Settings, Terminal,
// Browser, resolved against the installed `.desktop` corpus by trying a small
// candidate list per slot and taking the first that resolves. Unresolved slots
// are skipped so the Dock never opens with broken default tiles.
QStringList resolveDefaultDockPins(const DesktopEntryIndex &index);

// Build the Dock's ordered entries. `running` is the shell projection
// (kind "temporary" with `windows`, `active`, plus per-window "minimized"
// entries); a pinned running app keeps the projection's `active` flag through
// the merge so its running dot can distinguish the focused app.
// `launchStates` maps a pinned desktop id to "launching" | "failed" (absent =
// idle). `recentIds` is the recency-ordered app list for the suggested
// entries (T-10 section 17; empty when `dock.showRecentApps` is off). Pinned
// entries come first in `pinnedIds` order, then unmatched temporary running
// apps, then suggested recents, then minimized windows (the Dock inserts the
// divider between app entries and minimized entries).
QVariantList buildDockEntries(const QStringList &pinnedIds, const DesktopEntryIndex &index,
                              const QVariantList &running,
                              const QHash<QString, QString> &launchStates,
                              const QStringList &recentIds = {});

// What an app menu command can actually do for one entry. The
// capability map travels with the entry so QML can disable an unavailable row
// with a reason, and dispatch revalidates the same fields before acting. The
// controller has no sticky-window/all-Spaces or app-assignment model yet
// (T-04/T-05), login items are T-24, and the compositor exposes only the
// unfiltered overview request, so those capabilities are false until the
// backend lands. Pure and unit-tested (tst_dockcore).
struct DockActionCapabilities {
    bool assignThisDesktop = false;
    bool assignAllDesktops = false;
    bool assignNone = false;
    bool openAtLogin = false;
    bool keepInDock = false;
    bool showAppWindows = false;
    bool showOverview = true;
    // Force Quit needs a stable termination identity acquired at launch (a
    // pidfd-backed process handle or a validated managed unit). The controller
    // sets the entry's `forceQuitAvailable` field from its live launch records;
    // an app the shell did not launch has neither, so the row must not promise
    // a forceful termination it cannot perform.
    bool forceQuit = false;

    QVariantMap toVariantMap() const;
};

DockActionCapabilities dockActionCapabilities(const QVariantMap &entry);

// Revalidate one menu action against the entry it would act on. `target`
// distinguishes the three `assign_to` choices. Actions that are not
// capability-gated return true. Pure and unit-tested (tst_dockcore).
bool dockActionAvailable(const QVariantMap &entry, const QString &action,
                         const QString &target = {});

// Human-readable fallback name for an unresolved identity: the last
// reverse-DNS segment, with any `.desktop` suffix removed.
QString displayNameForIdentity(const QString &identity);

// How the shell should open an app identified by `desktopId`: activate a
// running window when one matches, otherwise launch the `.desktop` entry. The
// caller owns the side effects (`ShellProtocol::activateApp` /
// `launchDockApp`); this is the pure decision the fixed menus and the Dock
// share. `desktopId` may be a full desktop id or a compositor identity that
// `index.resolve` maps to one. Pure and unit-tested (tst_dockcore).
struct AppOpenPlan {
    bool resolved = false; // the identity resolves in the corpus
    bool running = false;  // a projected running window matches it
    QString appId;         // compositor app id to activate (when running)
    QString desktopId;     // resolved desktop id to launch (when not running)
};
AppOpenPlan planAppOpen(const DesktopEntryIndex &index, const QVariantList &running,
                        const QString &desktopId);

// T236: what the shell does when the compositor answers `activate_app`. A
// found window is the activation itself; a vanished app falls back to a launch
// when its installed entry is still launchable, otherwise to a visible notice.
// Pure and unit-tested (tst_dockcore).
enum class DockActivationFallback {
    None,
    Launch,
    Notice,
};
DockActivationFallback dockActivationFallback(bool found, bool launchable);

// Pending quit requests keyed by app id. Each request carries a generation so
// an older fallback timeout cannot clear a newer request for the same app
// (reliability review §6). Pure and unit-tested (tst_dockcore).
class DockQuitRequests
{
public:
    // Start (or supersede) a request and return its generation. An empty app
    // id is not a request and returns 0.
    quint64 begin(const QString &appId);
    // Clear only when `generation` is still the current request.
    bool clearIfCurrent(const QString &appId, quint64 generation);
    // Clear whatever request is current, if any (exit or attention observed).
    bool cancel(const QString &appId);
    bool contains(const QString &appId) const;
    bool isEmpty() const { return m_pending.isEmpty(); }
    // Sorted app ids for deterministic iteration.
    QStringList appIds() const;

private:
    QHash<QString, quint64> m_pending;
    quint64 m_next = 0;
};

// Recent/suggested apps (T-10 section 17): up to `limit` entries for the
// recency-ordered `recentIds` that are not pinned and not already running.
// Gated by `dock.showRecentApps` at the shell; this only builds the entries.
// The result is appended to the app region (before the divider), each with
// `kind: "recent"` and `running: false`, so a click launches it.
QVariantList buildRecentEntries(const QStringList &recentIds, const QStringList &pinnedIds,
                                const QVariantList &running, const DesktopEntryIndex &index,
                                int limit = 3);

// The Dock's region structure (T-14.7v): the reference separates the app
// region into a pinned prefix and a temporary/recent tail, and separates the
// app region from the minimized group and the fixed stacks/Trash tail by
// hairline rules. A divider is a projection of this structure, never an entry:
// the shell supplies the boundaries (derived here from the entry kinds and the
// fixed tail) and the Dock draws them. A divider is emitted between each pair
// of *adjacent non-empty* regions, so an empty region never leaves an orphaned
// or doubled rule.
struct DockRegionPlan {
    int pinned = 0;    // the pinned prefix
    int tail = 0;      // temporary + recent + overflow running groups
    int minimized = 0; // per-window minimized rows (0 when hidden)
    int fixed = 0;     // the stacks + Trash tail
    int dividerCount() const; // adjacent non-empty region pairs
    bool appRegion() const { return pinned + tail > 0; }
};

// Derive the region plan from a full Dock entry list (no dividers; the fixed
// tail is counted separately as `fixedCount`). `minimizedVisible` mirrors the
// `dock.minimizeIntoTileIcon` setting: a hidden minimized region is empty, so
// it contributes no boundary. Pure and unit-tested (tst_dockcore).
DockRegionPlan planDockRegions(const QVariantList &entries, int fixedCount,
                               bool minimizedVisible);

// The along-axis spacing tokens the Dock renders with. Kept separate from the
// entry model so the fit calculation and the QML plate use one rule: icons are
// `iconSize`, dividers `dividerWidth`, an icon pair `gap`, either side of a
// divider `dividerGap`, and both plate ends `paddingAlong`.
struct DockLayoutSpacing {
    int gap = 0;
    int dividerGap = 0;
    int dividerWidth = 1;
    int paddingAlong = 0;
};

// The pure along-axis layout of a Dock entry list: the region plan plus the
// exact number of icons, dividers, icon gaps, divider gaps, and end paddings
// the QML renderer will lay out. `lengthFor` is the authoritative content
// length used by both the overflow fit and the rendered plate (T-14.7v).
struct DockLayoutMetrics {
    DockRegionPlan regions;
    int iconCount = 0;
    int dividerCount = 0;
    int iconGapCount = 0;
    int dividerGapCount = 0;
    int paddingCount = 0;

    int itemCount() const { return iconCount + dividerCount; }
    qint64 lengthFor(int iconSize, const DockLayoutSpacing &spacing) const;
};

// Count the exact rendered layout for `entries` plus the separately supplied
// fixed tail. A divider is internal, so every boundary contributes two
// `dividerGap`s; all other adjacent icon pairs contribute one `gap`. The
// plate adds `paddingAlong` at each end when it has any content. Pure and
// unit-tested (tst_dockcore).
DockLayoutMetrics planDockLayout(const QVariantList &entries, int fixedCount,
                                 bool minimizedVisible);

// The T-10 section 5.1 overflow clamp, revisited by T-14.7q. A Dock wider
// than its output is an error state: `dock.size` is clamped so the content
// fits, and when even the minimum icon cannot fit everything, running groups
// (temporary entries) that would be dropped are consolidated into one terminal
// `overflow` entry carrying them, instead of vanishing silently (ADR 0103).
// The overflow entry is the last app-region entry: `{ id: "__overflow__",
// kind: "overflow", hiddenCount, windowCount, groups: [entry, ...] }`. Recents
// (suggestions, not running groups) are still dropped silently, and pinned,
// minimized, and fixed entries are never dropped. The icon-size clamp remains
// the outer fallback when even one overflow cell cannot fit. `fixedCount` is
// the actual fixed tail (the Downloads stack, every pinned folder, and the
// Trash); the divider is counted from the regions that survive. `minimizedVisible`
// is false when `dock.minimizeIntoTileIcon` hides the minimized entries.
struct DockOverflowResult {
    QVariantList entries; // `entries`, overflow removable entries replaced by
                          // the terminal overflow entry when one is present
    int iconSize = 0;     // effective icon size in px (never above requested)
    int hiddenTemporary = 0; // running groups folded into the overflow entry
    int hiddenRecent = 0;    // recents dropped silently
    // The number of hidden groups represented by the overflow entry (0 when no
    // cell is shown), so the caller can detect a change that the raw hidden
    // counts miss.
    int overflowShown = 0;
    bool clamped = false;    // the size was reduced or entries were hidden
    bool overflowed = false; // content still exceeds the axis at the minimum
};

DockOverflowResult applyDockOverflow(const QVariantList &entries, int availableLength,
                                     int requestedIconSize, int iconMin, int iconMax,
                                     int gap, int dividerWidth, int fixedCount = 2,
                                     bool minimizedVisible = true);

// The production overload: fit against the same divider gaps and end padding
// the QML plate renders. `fixedCount` is the actual fixed tail (Downloads,
// every pinned folder, and Trash), not a constant. When live running groups
// must be folded, an overflow cell is reserved even if that means reporting
// `overflowed` for a later scroll path; hidden groups are never silently
// dropped.
DockOverflowResult applyDockOverflow(const QVariantList &entries, int availableLength,
                                     int requestedIconSize, int iconMin, int iconMax,
                                     const DockLayoutSpacing &spacing,
                                     int fixedCount = 2,
                                     bool minimizedVisible = true);

// The padding the Dock scene needs around the resting strip so the current
// overlay (popover card or hover label) is inside the offscreen buffer. The
// scene is the strip plus these margins; sizing it to the content instead of
// the full placement budget keeps a small tooltip from paying for a tall
// menu, and a session-scoped grow-only budget avoids a resize per hover
// (reliability §12). `overlay` is in Dock-local coordinates; `outputSize`
// bounds the result to the output work area and may be invalid. Pure and
// unit-tested (tst_dockcore).
struct DockSceneMargins {
    int top = 0;
    int right = 0;
    int bottom = 0;
    int left = 0;

    bool operator==(const DockSceneMargins &) const = default;
    bool isZero() const
    {
        return top == 0 && right == 0 && bottom == 0 && left == 0;
    }
    QSize sceneSize(const QSize &dock) const
    {
        return QSize(dock.width() + left + right,
                     dock.height() + top + bottom);
    }
    QPoint itemOrigin() const { return QPoint(left, top); }
};

// The minimal margins that contain `overlay` plus the Dock strip, capped to
// `outputSize` when it is valid. An empty overlay yields zero margins.
DockSceneMargins dockSceneMargins(const QSize &dock, const QRect &overlay,
                                  const QSize &outputSize = QSize());

// Grow-only scene budget for one pointer/keyboard interaction session. While
// a session is active the margins never shrink, so sweeping between tiles or
// opening a submenu does not recreate the render target; when the session
// ends the allocation returns to zero. The Dock strip itself never changes
// size. Pure and unit-tested (tst_dockcore).
class DockSceneBudget
{
public:
    DockSceneMargins update(const QSize &dock, const QRect &overlay,
                            const QSize &outputSize, bool sessionActive);
    DockSceneMargins margins() const { return m_margins; }
    void reset();

private:
    DockSceneMargins m_margins;
    QSize m_dockSize;
};

// T-10 section 8.1 bounce clocks. The measured reference launch lift is one
// 15 pt hop over ~300 ms; the full multi-bounce is unresolved, so the launch
// clock is that single lift rather than an invented multi-hop. An attention
// bounce repeats one hop every `kAttentionBounceHopMs` until the controller's
// deadline. Both return the current phase in [0,1], where the caller maps it
// to an offset with `sin(pi * phase)`; `dockLaunchBouncePhase` returns -1 once
// the launch lift has finished. Pure and unit-tested (tst_dockcore).
constexpr qint64 kLaunchBounceMs = 300;
constexpr qint64 kAttentionBounceHopMs = 400;
constexpr qint64 kAttentionBounceMs = 2000;
double dockLaunchBouncePhase(qint64 elapsedMs);
double dockAttentionBouncePhase(qint64 elapsedMs);

// T-14.7s minimize-to-icon reaction clock. One short hop (the caller maps the
// phase with `sin(pi * phase)`): the phase is the linear progress in [0,1]
// while the reaction is in flight and -1 once it has finished. The duration
// mirrors `component.dock.minimizeReaction.duration` in the token source; the
// QML owns the amplitude/easing so reduced motion can drop the translation.
constexpr qint64 kMinimizeReactionMs = 320;
double dockMinimizeReactionPhase(qint64 elapsedMs);
