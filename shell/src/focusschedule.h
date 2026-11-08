// SPDX-License-Identifier: MIT
// The Focus schedule (T229a).
//
// The Settings Focus pane stores `focus.schedule.*` keys in settingsd; this
// controller is the one consumer. It applies the notification service's Focus
// mode when a schedule window opens and returns the session to Off when the
// window closes. It never invents a policy: a manual mode change wins, and the
// controller only turns off a Focus it turned on.
//
// The pure half (`windowActive`) is static so the overnight/day logic is
// unit-tested without a bus or a timer.
#pragma once

#include <QDateTime>
#include <QObject>
#include <QStringList>
#include <QTimer>

class NotificationClient;
class SettingsClient;

class FocusSchedule : public QObject
{
    Q_OBJECT

public:
    explicit FocusSchedule(SettingsClient *settings,
                           NotificationClient *notifications,
                           QObject *parent = nullptr);

    // Begin observing settings and the clock. Idempotent.
    void start();

    // Parse a 24-hour `HH:MM` string into minutes since midnight.
    static bool parseTime(const QString &text, int *minutes);
    // Whether `now` falls inside the schedule window. `days` are `mon`…`sun`
    // tokens naming the day the window starts; empty means every day. An end
    // earlier than the start crosses midnight.
    static bool windowActive(const QString &start, const QString &end,
                             const QStringList &days, const QDateTime &now);

    bool activeNow() const;

public slots:
    // Re-read the keys and apply the mode at a window boundary. Called on a
    // 30-second timer, on every settings change, and on a policy change.
    void evaluate();

private:
    SettingsClient *m_settings = nullptr;
    NotificationClient *m_notifications = nullptr;
    QTimer m_timer;
    bool m_lastActive = false;
    bool m_applied = false;
    bool m_userOverride = false;
};
