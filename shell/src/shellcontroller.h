// SPDX-License-Identifier: MIT
// Shell process controller (T-09): owns the QML menu bar, renders it into
// the compositor's chrome surface, and maps compositor broadcasts onto the
// bar's data properties. System-service status items are placeholders until
// the T-20 adapters land.
#pragma once

#include <QObject>
#include <QHash>
#include <QString>
#include <QVariant>
#include <QVariantList>

#include "desktopentry.h"
#include "dockpins.h"

class ShellProtocol;
class QQmlEngine;
class QQuickWindow;
class QQuickItem;
class QSocketNotifier;
class QTimer;

class ShellController : public QObject
{
    Q_OBJECT

public:
    explicit ShellController(QObject *parent = nullptr);
    ~ShellController() override;

    // Connect, authenticate, create the menu-bar surface, and load the QML.
    // `placeholders` shows the demo status items (T-20 not yet landed); when
    // false every status item degrades to hidden, the "daemon absent" state.
    bool start(const QString &socketName, const QString &tokenHex, int barHeight,
               bool placeholders);

    QString lastError() const;

private slots:
    void onConfigured(int width, int height, quint32 serial);
    void onFocusedAppChanged(const QString &appId, const QString &title);
    void onControlCenterRequested();
    void onMissionControlRequested();
    void onStatusItemActivated(const QString &itemId);
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
    void onDockPointerMoved(qreal x, qreal y);
    void onDockPointerButton(qreal x, qreal y, quint32 button, bool pressed);
    void onDockPointerLeft();
    void onDockLaunchTick();
    void onDockAttention(const QString &appId);
    void onDockAnimationTick();

private:
    void applyStatusItems();
    void applyFocusedApp();
    void render();
    void renderDock();
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
    void failDockLaunch(const QString &desktopId, const QString &reason);
    // Clear a transient "failed" launch state after the notice has shown.
    void scheduleLaunchStateClear(const QString &desktopId);
    // Coalesce a Dock render onto the next event-loop turn.
    void scheduleDockRender();
    // Coalesce a render onto the next event-loop turn (QML visual state has
    // changed but the compositor has not been told yet).
    void scheduleRender();
    // Render every ~16 ms for `ms`, to capture a popup open/close animation.
    void startAnimationRenders(int ms);
    // Force the launch state to neutral (no focus ring, no hover) and commit
    // it, so the first visible frame is not a transient highlight.
    void settleInitialState();
    // Read the open dropdown's rectangle from the bar and place (or hide) the
    // overlay popup surface accordingly.
    void updatePopupGeometry();

    ShellProtocol *m_protocol = nullptr;
    QQmlEngine *m_engine = nullptr;
    QQuickWindow *m_window = nullptr;
    QQuickItem *m_item = nullptr;
    QQuickWindow *m_dockWindow = nullptr;
    QQuickItem *m_dockItem = nullptr;
    QSocketNotifier *m_notifier = nullptr;
    QTimer *m_animationTimer = nullptr;
    QTimer *m_launchTimer = nullptr;
    QTimer *m_dockAnimTimer = nullptr;

    // Interim app-index stand-in (T-23) and Dock pin persistence (T-15).
    DesktopEntryIndex m_index;
    DockPins m_pins;
    // The shell's running-window projection, kept so the pinned set can be
    // merged on every change.
    QVariantList m_runningEntries;
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

    int m_width = 0;
    int m_height = 0;
    int m_barHeight = 28;
    // Dock surface geometry (T-10): full output width, `m_dockHeight` tall
    // so magnified artwork can rise above the baseline bar.
    int m_dockWidth = 0;
    int m_dockHeight = 0;
    int m_dockBarThickness = 0;
    bool m_dockRenderPending = false;
    Qt::MouseButtons m_dockButtons = Qt::NoButton;
    // The open dropdown's window-space rectangle (valid while a menu is open).
    int m_popupX = 0;
    int m_popupY = 0;
    int m_popupWidth = 0;
    int m_popupHeight = 0;
    // True while a dropdown is open; the overlay popup surface is mapped then.
    bool m_menuOpen = false;
    bool m_renderPending = false;
    int m_animationTicks = 0;
    Qt::MouseButtons m_buttons = Qt::NoButton;
    QString m_appId;
    QString m_appTitle;
    bool m_placeholders = false;
};
