// SPDX-License-Identifier: MIT
// Shell-side client for the T-11.1a notification service
// (`org.freedesktop.Notifications`, services/notifications).
//
// The abstract seam keeps the shell controller free of D-Bus: the live
// `DbusNotificationClient` talks to the service's shell-facing interface
// (`org.dragonfruit.Notifications1`), and `MockNotificationClient` serves a
// fixture for headless use and the capture script (selected with the
// `DF_NOTIFY_FIXTURE` environment variable). Both publish the same JSON
// payloads, so the model decode path is identical in the demo and a real
// session.
//
// The shell never polls: `Changed` from the service re-reads `Banners()` and
// `History()`, and the service's own expiry thread closes banners at their
// deadline.
#pragma once

#include <QByteArray>
#include <QObject>
#include <QTimer>

class NotificationClient : public QObject
{
    Q_OBJECT

public:
    explicit NotificationClient(QObject *parent = nullptr) : QObject(parent) {}
    ~NotificationClient() override = default;

    // Whether the service is reachable. When false there is nothing to show.
    virtual bool isAvailable() const = 0;

public slots:
    // Re-read `Banners()` and `History()` from the service.
    virtual void refresh() = 0;
    // The user dismissed a banner (reason 2).
    virtual void dismiss(quint32 id) = 0;
    // A banner's timeout elapsed (reason 1).
    virtual void expire(quint32 id) = 0;
    // Set Do Not Disturb (T-11.2a owns the policy that consumes it).
    virtual void setDoNotDisturb(bool enabled) = 0;

signals:
    void availableChanged(bool available);
    void bannersChanged(const QByteArray &json);
    void historyChanged(const QByteArray &json);
};

// The live client over the user session bus.
class DbusNotificationClient : public NotificationClient
{
    Q_OBJECT

public:
    explicit DbusNotificationClient(QObject *parent = nullptr);

    bool isAvailable() const override;

public slots:
    void refresh() override;
    void dismiss(quint32 id) override;
    void expire(quint32 id) override;
    void setDoNotDisturb(bool enabled) override;

private:
    using ReplySignal = void (NotificationClient::*)(const QByteArray &);
    void call(const QString &method, const QVariantList &arguments, ReplySignal replySignal);
    QString m_service;
    QString m_path;
    QString m_interface;
    bool m_available = false;
};

// The fixture client used by `DF_NOTIFY_FIXTURE` and the headless tests. It
// seeds one representative banner so the demo can render it on a host with no
// notification service running.
class MockNotificationClient : public NotificationClient
{
    Q_OBJECT

public:
    explicit MockNotificationClient(QObject *parent = nullptr);

    bool isAvailable() const override { return true; }

public slots:
    void refresh() override;
    void dismiss(quint32 id) override;
    void expire(quint32 id) override;
    void setDoNotDisturb(bool enabled) override;

private:
    void remove(quint32 id, const QString &reason);
    void publish();

    quint32 m_nextId = 1;
    quint32 m_bannerId = 0;
    QByteArray m_bannerJson;
    QByteArray m_historyJson;
    QTimer m_expiry;
};