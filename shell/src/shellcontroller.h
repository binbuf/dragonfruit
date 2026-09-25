// SPDX-License-Identifier: MIT
// Shell process controller (T-09): owns the QML menu bar, renders it into
// the compositor's chrome surface, and maps compositor broadcasts onto the
// bar's data properties. System-service status items come from the T-07
// session-bus bridge host (services/system-status).
#pragma once

#include <QObject>
#include <QHash>
#include <QString>
#include <QStringList>
#include <QVariant>
#include <QVariantList>

#include "desktopentry.h"
#include "dockmodel.h"
#include "dockpins.h"
#include "docksettings.h"
#include "downloadsmonitor.h"
#include "framecommitgate.h"
#include "shellprotocol.h"
#include "systemstatusmodel.h"
#include "trashmonitor.h"

class SystemStatusClient;
class QQmlEngine;
class QQuickWindow;
class QQuickItem;
class QSocketNotifier;
class QFileSystemWatcher;
class QTimer;

class ShellController : public QObject
{
    Q_OBJECT

public:
    explicit ShellController(QObject *parent = nullptr);
    ~ShellController() override;

    // Connect, authenticate, create the menu-bar surface, and load the QML.
    // Status items come from the live bridge host; when the host (or a
    // daemon) is absent every item degrades to hidden, the "daemon absent"
    // state. The `DF_STATUS_FIXTURE` environment variable selects the
    // fixture client for headless/capture use only.
    bool start(const QString &socketName, const QString &tokenHex, int barHeight);

    QString lastError() const;

private slots:
    void onConfigured(int width, int height, quint32 serial);
    void onFocusedAppChanged(const QString &appId, const QString &title);
    void onControlCenterRequested();
    void onMissionControlRequested();
    void onStatusItemActivated(const QString &itemId);
    // Wi-Fi and volume popovers (T-07.5a): the bar's gestures become bridge
    // host calls, and the host's views become the model's state.
    void onStatusMenuOpened();
    void onStatusMenuClosed();
    void onStatusMenuRefreshRequested(const QString &itemId);
    void onWifiJoinRequested(const QString &ssid, const QString &secret);
    void onVolumeSetRequested(double volume);
    void onMuteToggleRequested();
    void onWifiState(const QByteArray &json);
    void onAudioState(const QByteArray &json);
    void onBatteryState(const QByteArray &json);
    void onStatusReport(const QByteArray &json);
    void onClockTick();
    void onAppMenuOpened(int index);
    void onAppMenuClosed();
    void onAppMenuTriggered(int menuIndex, int itemIndex, const QVariant &item);
    void onPointerMoved(qreal x, qreal y);
    void onPointerButton(qreal x, qreal y, quint32 button, bool pressed);
    void onPointerLeft();
    void onKeyboardFocused(bool focused);
    void onKeyEvent(quint32 key, bool pressed);
    void onDockConfigured(int width, int height, quint32 serial);
    void onDockStateChanged(const QVariantList &entries);
    void onDockEntryActivated(const QVariant &entry);
    void onDockEntryContextMenu(const QVariant &entry, qreal x, qreal y);
    void onDockDividerContextMenu(qreal x, qreal y);
    void onDockEntryMenuAction(const QString &action, const QVariant &payload);
    void onDockWindowActivated(const QString &windowId);
    void onDockPinnedOrderChanged(const QVariant &desktopIds);
    void onDockPopoverChanged();
    void onDockRevealStateChanged();
    void onDockKeyboardFocused(bool focused);
    void onDockKeyboardFocusReleaseRequested();
    // External drag-and-drop onto the Dock (T-10 section 12).
    void onDockExternalDragEntered(bool payloadIsApp, qreal x, qreal y);
    void onDockExternalDragMoved(qreal x, qreal y);
    void onDockExternalDragLeft();
    void onDockExternalDropped(bool payloadIsApp, const QString &desktopId,
                               const QStringList &paths, qreal x, qreal y);
    void onDockExternalDropRequested(const QString &targetId, const QString &targetKind,
                                     const QString &desktopId, bool payloadIsApp);
    // The Downloads stack (T-10 section 17).
    void onDockDownloadActivated(const QString &path);
    void onDockDownloadsFolderRequested();
    void onDockDownloadsViewed();
    void onDownloadsChanged();
    // The divider resize handle writes `dock.size` (T-10 section 5).
    void onDockSizePreview(qreal fraction);
    void onDockSizeChanged(qreal fraction);
    void onInputAction(const QString &action, const QString &source);
    void onDockPointerMoved(qreal x, qreal y);
    void onDockPointerButton(qreal x, qreal y, quint32 button, bool pressed);
    void onDockPointerLeft();
    void onDockPopupPointerMoved(qreal x, qreal y);
    void onDockPopupPointerButton(qreal x, qreal y, quint32 button, bool pressed);
    void onDockPopupPointerLeft();
    // Mission Control overview chrome (T-11 Slice B).
    void onOverviewConfigured(int width, int height, quint32 serial);
    void onOverviewChanged(bool active);
    void onOverviewProgress(qreal progress, const QString &action);
    void onOverviewDataChanged();
    void onOverviewWorkspaceActivated(int index);
    void onOverviewWindowActivated(const QString &windowId);
    void onOverviewWindowMovedToWorkspace(const QString &windowId, int index);
    void onOverviewDismissRequested();
    void onOverviewPointerMoved(qreal x, qreal y);
    void onOverviewPointerButton(qreal x, qreal y, quint32 button, bool pressed);
    void onOverviewPointerLeft();
    void onOverviewKeyboardFocused(bool focused);
    // App-switcher overlay (T-06.2a).
    void onSwitcherConfigured(int width, int height, quint32 serial);
    void onAppSwitcherChanged(bool active, const QVariantList &entries,
                              const QString &selectedAppId, int direction);
    void onDockLaunchTick();
    void onDockAttention(const QString &appId);
    void onDockAnimationTick();
    void onSettingsFileChanged();
    void onTrashChanged();

private:
    void applyStatusItems();
    // Push the model's decoded Wi-Fi/volume views onto the bar's popovers and
    // rebuild the status slots from them.
    void applyStatusMenuData();
    void applyFocusedApp();
    void render();
    void renderDock();
    // Mission Control overview chrome (T-11 Slice B): re-read the workspace /
    // minimized-window projection from the protocol into the QML, and commit
    // the transparent chrome surface while the overview is open.
    void refreshOverviewData();
    void renderOverview();
    void scheduleOverviewRender();
    // App-switcher overlay (T-06.2a): a fourth offscreen scene, unmapped until
    // the compositor opens the switcher; it draws the centered app cards, the
    // selection highlight, and the scrim while the compositor renders the live
    // preview surfaces underneath.
    void renderSwitcher();
    void scheduleSwitcherRender();
    // Rebuild the Dock's ordered entries (pinned + running) and hand them to
    // the QML scene.
    void rebuildDockEntries();
    // Add the per-entry `bounce` phase (0..1) from the launch/attention
    // clocks (T-10 section 8.1).
    QVariantList withBounce(QVariantList entries) const;
    // Start the 16 ms Dock animation clock if it is not already running.
    void ensureDockAnimation();
    // Clear any attention bounce for `appId` (FR-4: stops on click or focus).
    void clearAttention(const QString &appId);
    // Launch a pinned app through the interim `.desktop` resolver (T-23
    // replaces this). Bounded by a launch timeout; failure raises a notice.
    void launchDockApp(const QString &desktopId);
    // Launch an app with file arguments (a file dropped on an app icon, T-10
    // section 12); the files are substituted for the Exec file field codes.
    void launchDockAppWithFiles(const QString &desktopId, const QStringList &files);
    void failDockLaunch(const QString &desktopId, const QString &reason);
    // Clear a transient "failed" launch state after the notice has shown.
    void scheduleLaunchStateClear(const QString &desktopId);
    // Coalesce a Dock render onto the next event-loop turn.
    void scheduleDockRender();
    // Open the Trash in Files (`org.dragonfruit.Files1` / the Files .desktop;
    // T-18 owns the app). The entry point is wired now.
    void openTrashInFiles();
    // Reveal an app's executable in Files (the Dock "Show in Files" /
    // Command-click action, T-10 section 13). T-18 owns Files; the entry
    // point is wired now.
    void showDockAppInFiles(const QString &desktopId, const QString &appId);
    // Push the `dock.*` settings onto the Dock QML and, when the geometry
    // changed, reconfigure the chrome surface (T-10 section 19).
    void applyDockSettings(bool reconfigure);
    // Persist `dock.*` (interim; settingsd owns this at T-15).
    void saveDockSettings();
    // Apply an already-set `dock.size` to the Dock QML and reconfigure the
    // surface without rebuilding entries (the divider drag must not reset the
    // QML delegate that holds the pointer; T-10 section 5).
    void applyDockSizeOnly();
    // Map `dock.size` (0..1) onto the icon-size token range.
    int iconSizeForSize(double size) const;
    // Recompute the T-10 section 5.1 overflow clamp from `m_dockAllEntries`
    // and the current output length: the effective icon size, the entries
    // with overflow temporary/recent hidden, and the overflow flags.
    DockOverflowResult computeDockOverflow() const;
    // Adopt an overflow result: set the effective icon size and the baseline
    // bar/magnified-band thickness. `allowEntries` is false for the divider
    // resize, where the Repeater model must stay stable while the QML delegate
    // holds the pointer; when `allowEntries` is true the entries are only
    // re-published if the hidden set changed unless `forceEntries` is set.
    void applyDockOverflowResult(const DockOverflowResult &overflow, bool allowEntries,
                                 bool forceEntries = false);
    // Log the one-per-session overflow warning (section 5.1).
    void warnDockOverflow(const DockOverflowResult &overflow);
    // Map the `dock.position` string onto the protocol edge enum.
    ShellProtocol::DockPosition dockPosition() const;
    // Coalesce a render onto the next event-loop turn (QML visual state has
    // changed but the compositor has not been told yet).
    void scheduleRender();
    // Scene-graph frame hooks (FR-14): a frame the Dock/menu-bar window just
    // rendered is committed on the next event-loop turn, so QML-driven
    // animation (magnification, popover fade, drag gaps) reaches the
    // compositor without a sampling timer.
    void onDockAfterRendering();
    void onMenuAfterRendering();
    // Force the launch state to neutral (no focus ring, no hover) and commit
    // it, so the first visible frame is not a transient highlight.
    void settleInitialState();
    // Read the open dropdown's rectangle from the bar and place (or hide) the
    // overlay popup surface accordingly.
    void updatePopupGeometry();

    ShellProtocol *m_protocol = nullptr;
    // The T-07.5a/T-07.5b bridge: the decoded status model and its host client
    // (the live D-Bus client, or the fixture client under `DF_STATUS_FIXTURE`).
    SystemStatusModel *m_statusModel = nullptr;
    SystemStatusClient *m_statusClient = nullptr;
    QQmlEngine *m_engine = nullptr;
    QQuickWindow *m_window = nullptr;
    QQuickItem *m_item = nullptr;
    QQuickWindow *m_dockWindow = nullptr;
    QQuickItem *m_dockItem = nullptr;
    // Mission Control overview chrome (T-11 Slice B): a third offscreen scene
    // rendered into the full-output `overlay` surface while the overview is
    // open. The compositor draws the live window surfaces underneath.
    QQuickWindow *m_overviewWindow = nullptr;
    QQuickItem *m_overviewItem = nullptr;
    int m_overviewWidth = 0;
    int m_overviewHeight = 0;
    bool m_overviewActive = false;
    bool m_overviewRenderPending = false;
    Qt::MouseButtons m_overviewButtons = Qt::NoButton;
    FrameCommitGate m_overviewFrameGate;
    bool m_overviewSceneGraphCommitLogged = false;
    // App-switcher overlay (T-06.2a): a fourth offscreen scene rendered into
    // the full-output `app-switcher` overlay while the compositor's machine is
    // open.
    QQuickWindow *m_switcherWindow = nullptr;
    QQuickItem *m_switcherItem = nullptr;
    int m_switcherWidth = 0;
    int m_switcherHeight = 0;
    bool m_switcherActive = false;
    bool m_switcherRenderPending = false;
    FrameCommitGate m_switcherFrameGate;
    bool m_switcherSceneGraphCommitLogged = false;
    QSocketNotifier *m_notifier = nullptr;
    QTimer *m_launchTimer = nullptr;
    QTimer *m_dockAnimTimer = nullptr;
    QFileSystemWatcher *m_settingsWatcher = nullptr;
    // Interim home-trash state for the Dock's Trash entry (section 16). The
    // GIO/GVfs backend replaces it when the dev headers are available.
    TrashMonitor *m_trash = nullptr;
    // Downloads-stack state for the Dock (section 17). Files-core (T-17)
    // replaces this watch with its folder monitor.
    DownloadsMonitor *m_downloads = nullptr;
    // Recency-ordered app ids for the suggested entries (`dock.showRecentApps`,
    // T-10 section 17); fed by focus changes, capped and de-duplicated.
    QStringList m_recentAppIds;

    // Interim app-index stand-in (T-23) and Dock pin persistence (T-15).
    DesktopEntryIndex m_index;
    DockPins m_pins;
    DockSettings m_settings;
    // The shell's running-window projection, kept so the pinned set can be
    // merged on every change.
    QVariantList m_runningEntries;
    // The full built Dock entries (before the section 5.1 overflow clamp), so
    // a size or geometry change can re-clamp without rebuilding.
    QVariantList m_dockAllEntries;
    // How many temporary/recent entries the last applied layout hid, so a
    // geometry change only resets the Repeater model when it must.
    int m_dockHiddenTemporary = 0;
    int m_dockHiddenRecent = 0;
    // One warning per session for the Dock-overflow error state (section 5.1).
    bool m_dockOverflowWarned = false;
    // Pinned desktop id -> "launching" | "failed" (transient).
    QHash<QString, QString> m_launchStates;
    // Pinned desktop id -> deadline (ms since epoch) for the launch timeout.
    QHash<QString, qint64> m_launchDeadlines;
    // Pinned desktop id -> launch-bounce start (ms since epoch); the bounce
    // finishes on its own even after the first window resolves the launch.
    QHash<QString, qint64> m_launchStart;
    // Compositor app id -> attention-bounce start and deadline (T-10 FR-4).
    QHash<QString, qint64> m_attentionStart;
    QHash<QString, qint64> m_attentionUntil;
    // The in-flight external-drop payload (T-10 section 12). The shell holds
    // the payload and resolves the action when the Dock reports the target.
    // `payloadIsApp` is the drop-time classification (a single `.desktop` URI
    // is an app alias even though the drag source advertised files).
    bool m_externalPayloadIsApp = false;
    QString m_externalDesktopId;
    QStringList m_externalPaths;
    qreal m_externalDropX = 0;
    qreal m_externalDropY = 0;

    int m_width = 0;
    int m_height = 0;
    int m_barHeight = 28;
    // Dock surface geometry (T-10): the surface extent perpendicular to its
    // edge (`m_dockThickness`, the baseline bar plus the magnify band) and
    // the configured surface size. For a bottom Dock the width comes from the
    // configure and the height is `m_dockThickness`; a vertical Dock is the
    // mirror image.
    int m_dockWidth = 0;
    int m_dockHeight = 0;
    int m_dockThickness = 0;
    int m_dockBarThickness = 0;
    ShellProtocol::DockPosition m_dockPosition = ShellProtocol::DockPosition::Bottom;
    bool m_dockRenderPending = false;
    Qt::MouseButtons m_dockButtons = Qt::NoButton;
    // The Dock popover (context menu / window chooser) in Dock-scene
    // coordinates, plus the offset the scene is placed at inside the offscreen
    // window so a popover can render above (bottom Dock) or beside (vertical
    // Dock) the bar without leaving the buffer.
    int m_dockItemOffsetX = 0;
    int m_dockItemOffsetY = 0;
    int m_dockPopoverX = 0;
    int m_dockPopoverY = 0;
    int m_dockPopoverWidth = 0;
    int m_dockPopoverHeight = 0;
    bool m_dockPopoverMapped = false;
    // Scene-graph frame gates (FR-14): the Dock and menu-bar windows commit
    // from `afterRendering`, and the gate suppresses the re-entrant frame the
    // `grabWindow()` readback itself produces.
    FrameCommitGate m_dockFrameGate;
    FrameCommitGate m_menuFrameGate;
    // One-shot diagnostics: confirm the scene-graph commit path is live.
    bool m_dockSceneGraphCommitLogged = false;
    bool m_menuSceneGraphCommitLogged = false;
    // True while the Dock chrome surface holds the keyboard (T-10 section
    // 20), so key events are routed to the Dock scene for navigation.
    bool m_dockKeyboardFocused = false;
    // The open dropdown's window-space rectangle (valid while a menu is open).
    int m_popupX = 0;
    int m_popupY = 0;
    int m_popupWidth = 0;
    int m_popupHeight = 0;
    // True while a dropdown is open; the overlay popup surface is mapped then.
    bool m_menuOpen = false;
    bool m_renderPending = false;
    Qt::MouseButtons m_buttons = Qt::NoButton;
    QString m_appId;
    QString m_appTitle;
};
