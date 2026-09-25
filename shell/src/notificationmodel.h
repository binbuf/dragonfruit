// SPDX-License-Identifier: MIT
// Shell-side notification model (T-11.1a).
//
// The shell renders banners and the notification-center history from the
// notification service's shell-facing JSON views
// (`org.dragonfruit.Notifications1`, services/notifications). This model is
// the one decode seam: the service's `Banners()` and `History()` payloads
// become the maps the QML draws. Nothing here names D-Bus, so it is
// unit-testable with plain JSON.
#pragma once

#include <QObject>
#include <QVariantList>
#include <QVariantMap>

class NotificationModel : public QObject
{
    Q_OBJECT

public:
    explicit NotificationModel(QObject *parent = nullptr);

    // The active banners, oldest first (the service's order).
    QVariantList banners() const { return m_banners; }
    // The recorded history, most recent first.
    QVariantList history() const { return m_history; }
    // Whether a banner should be on screen.
    bool hasBanner() const { return !m_banners.isEmpty(); }
    // The newest banner, or an empty map. One banner shows at a time in
    // T-11.1a; the older ones stay in `banners()` and the history.
    QVariantMap banner() const;

    // Decode a JSON array payload. Returns an empty list on a parse failure,
    // with `error` set when non-null.
    static QVariantList parseList(const QByteArray &json, QString *error = nullptr);

public slots:
    // Apply the service's banner/history payloads. A malformed payload
    // clears the view (the safe "nothing to show" default).
    void applyBannersJson(const QByteArray &json);
    void applyHistoryJson(const QByteArray &json);

signals:
    void changed();

private:
    QVariantList m_banners;
    QVariantList m_history;
};