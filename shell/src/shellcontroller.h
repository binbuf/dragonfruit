// SPDX-License-Identifier: GPL-3.0-or-later
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

private:
    void applyStatusItems();
    void applyFocusedApp();
    void render();

    ShellProtocol *m_protocol = nullptr;
    QQmlEngine *m_engine = nullptr;
    QQuickWindow *m_window = nullptr;
    QQuickItem *m_item = nullptr;
    QSocketNotifier *m_notifier = nullptr;

    int m_width = 0;
    int m_height = 0;
    int m_barHeight = 28;
    QString m_appId;
    QString m_appTitle;
    bool m_placeholders = false;
};
