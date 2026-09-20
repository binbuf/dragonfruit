// SPDX-License-Identifier: MIT
// Shell process controller (T-09): owns the QML menu bar, renders it into
// the compositor's chrome surface, and maps compositor broadcasts onto the
// bar's data properties. System-service status items are placeholders until
// the T-20 adapters land.
#pragma once

#include <QObject>
#include <QString>

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

private:
    void applyStatusItems();
    void applyFocusedApp();
    void render();
    // Coalesce a render onto the next event-loop turn (QML visual state has
    // changed but the compositor has not been told yet).
    void scheduleRender();
    // Render every ~16 ms for `ms`, to capture a popup open/close animation.
    void startAnimationRenders(int ms);
    // Resize the chrome surface to the bar height plus the open dropdown.
    void updateSurfaceHeight();

    ShellProtocol *m_protocol = nullptr;
    QQmlEngine *m_engine = nullptr;
    QQuickWindow *m_window = nullptr;
    QQuickItem *m_item = nullptr;
    QSocketNotifier *m_notifier = nullptr;
    QTimer *m_animationTimer = nullptr;

    int m_width = 0;
    int m_height = 0;
    int m_barHeight = 28;
    // The current chrome-surface height (bar, or bar + open dropdown).
    int m_surfaceHeight = 28;
    // True while a dropdown is open (the surface may then be taller than the
    // bar; the compositor's pre-layout full-output configure is still ignored).
    bool m_menuOpen = false;
    bool m_renderPending = false;
    int m_animationTicks = 0;
    Qt::MouseButtons m_buttons = Qt::NoButton;
    QString m_appId;
    QString m_appTitle;
    bool m_placeholders = false;
};
