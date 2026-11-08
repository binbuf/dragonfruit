// SPDX-License-Identifier: MIT
#include "focusschedule.h"

#include "notificationclient.h"
#include "settingsclient.h"

#include <QDebug>
#include <QJsonDocument>
#include <QJsonObject>

namespace {

const QStringList kDays{
    QStringLiteral("mon"), QStringLiteral("tue"), QStringLiteral("wed"),
    QStringLiteral("thu"), QStringLiteral("fri"), QStringLiteral("sat"),
    QStringLiteral("sun"),
};

QString dayToken(const QDateTime &now)
{
    // Qt's dayOfWeek is 1 (Monday) … 7 (Sunday); the list is 0-based.
    return kDays.at(now.date().dayOfWeek() - 1);
}

QString previousDayToken(const QDateTime &now)
{
    return kDays.at((now.date().dayOfWeek() + 5) % 7);
}

} // namespace

FocusSchedule::FocusSchedule(SettingsClient *settings,
                             NotificationClient *notifications, QObject *parent)
    : QObject(parent)
    , m_settings(settings)
    , m_notifications(notifications)
{
    m_timer.setInterval(30 * 1000);
    connect(&m_timer, &QTimer::timeout, this, &FocusSchedule::evaluate);
}

void FocusSchedule::start()
{
    if (m_settings) {
        connect(m_settings, &SettingsClient::changed, this,
                [this](const QString &key, const QVariant &) {
                    if (key.startsWith(QStringLiteral("focus.schedule.")))
                        evaluate();
                });
        connect(m_settings, &SettingsClient::refreshed, this,
                &FocusSchedule::evaluate);
    }
    if (m_notifications) {
        connect(m_notifications, &NotificationClient::focusPolicyChanged, this,
                [this](const QByteArray &json) {
                    if (!m_lastActive)
                        return;
                    const QJsonDocument document = QJsonDocument::fromJson(json);
                    const QString mode = document.isObject()
                        ? document.object().value(QStringLiteral("mode")).toString()
                        : QString();
                    if (mode == QLatin1String("focus")) {
                        m_applied = true;
                    } else {
                        // A manual Off/DND choice wins until the next window.
                        m_applied = false;
                        m_userOverride = true;
                    }
                });
        connect(m_notifications, &NotificationClient::availableChanged, this,
                [this](bool) { evaluate(); });
    }
    if (!m_timer.isActive())
        m_timer.start();
    evaluate();
}

bool FocusSchedule::parseTime(const QString &text, int *minutes)
{
    const QString trimmed = text.trimmed();
    const int colon = trimmed.indexOf(QLatin1Char(':'));
    if (colon <= 0 || colon != trimmed.length() - 3)
        return false;
    bool hoursOk = false;
    bool minutesOk = false;
    const int hours = trimmed.left(colon).toInt(&hoursOk);
    const int mins = trimmed.mid(colon + 1).toInt(&minutesOk);
    if (!hoursOk || !minutesOk || hours < 0 || hours > 23 || mins < 0
            || mins > 59)
        return false;
    if (minutes)
        *minutes = hours * 60 + mins;
    return true;
}

bool FocusSchedule::windowActive(const QString &start, const QString &end,
                                 const QStringList &days, const QDateTime &now)
{
    int startMinutes = 0;
    int endMinutes = 0;
    if (!parseTime(start, &startMinutes) || !parseTime(end, &endMinutes))
        return false;
    if (startMinutes == endMinutes)
        return true; // A zero-length window means all day.

    QStringList normalized;
    for (const QString &day : days) {
        const QString token = day.trimmed().toLower();
        if (kDays.contains(token) && !normalized.contains(token))
            normalized.append(token);
    }
    const bool everyDay = normalized.isEmpty();
    const int minutes = now.time().hour() * 60 + now.time().minute();
    const QString today = dayToken(now);

    if (startMinutes < endMinutes) {
        if (!everyDay && !normalized.contains(today))
            return false;
        return minutes >= startMinutes && minutes < endMinutes;
    }

    // Overnight: the window starts on `today` and ends tomorrow, or started
    // yesterday and ends today.
    if (minutes >= startMinutes)
        return everyDay || normalized.contains(today);
    if (minutes < endMinutes)
        return everyDay || normalized.contains(previousDayToken(now));
    return false;
}

bool FocusSchedule::activeNow() const
{
    if (!m_settings)
        return false;
    const bool enabled = m_settings->boolean(
        QStringLiteral("focus.schedule.enabled"), false);
    if (!enabled)
        return false;
    return windowActive(
        m_settings->string(QStringLiteral("focus.schedule.start"),
                           QStringLiteral("22:00")),
        m_settings->string(QStringLiteral("focus.schedule.end"),
                           QStringLiteral("07:00")),
        m_settings->stringList(QStringLiteral("focus.schedule.days")),
        QDateTime::currentDateTime());
}

void FocusSchedule::evaluate()
{
    if (!m_settings || !m_notifications)
        return;

    const bool active = activeNow();
    const bool wasActive = m_lastActive;
    m_lastActive = active;
    if (active && !wasActive)
        m_userOverride = false;

    if (!m_notifications->isAvailable())
        return;

    if (active) {
        if (!m_applied && !m_userOverride) {
            m_notifications->setFocusMode(QStringLiteral("focus"));
            m_applied = true;
        }
    } else {
        if (m_applied)
            m_notifications->setFocusMode(QStringLiteral("off"));
        m_applied = false;
        m_userOverride = false;
    }
}
