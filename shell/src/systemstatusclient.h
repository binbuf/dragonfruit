// SPDX-License-Identifier: MIT
// Shell-side client for the T-07.5a bridge host
// (`org.dragonfruit.SystemStatus1`, services/system-status).
//
// The abstract seam keeps the menu controller free of D-Bus: the live
// `DbusSystemStatusClient` talks to the host, and `MockSystemStatusClient`
// serves a fixture for headless use and the capture script (selected with the
// `DF_STATUS_FIXTURE` environment variable, since `--placeholders` is gone).
// Both publish the host's JSON payloads unchanged, so the model decode path is
// the same in the demo and in a real session.
#pragma once

#include <QByteArray>
#include <QObject>
#include <QString>
#include <QVariantList>

class SystemStatusClient : public QObject
{
    Q_OBJECT

public:
    explicit SystemStatusClient(QObject *parent = nullptr) : QObject(parent) {}
    ~SystemStatusClient() override = default;

    // Whether the host is reachable. When false the menus have nothing to
    // show and the items hide.
    virtual bool isAvailable() const = 0;

    virtual void refreshWifi() = 0;
    virtual void refreshAudio() = 0;
    // The battery item is read-only: there is no write action.
    virtual void refreshBattery() = 0;
    virtual void join(const QString &ssid, const QString &secret) = 0;
    virtual void setVolume(double volume) = 0;
    virtual void setMute(bool muted) = 0;

signals:
    void availableChanged(bool available);
    void wifiState(const QByteArray &json);
    void audioState(const QByteArray &json);
    void batteryState(const QByteArray &json);
    void joinReport(const QByteArray &json);
    void writeReport(const QByteArray &json);
};

// The live client over the user session bus.
class DbusSystemStatusClient : public SystemStatusClient
{
    Q_OBJECT

public:
    explicit DbusSystemStatusClient(QObject *parent = nullptr);

    bool isAvailable() const override;
    void refreshWifi() override;
    void refreshAudio() override;
    void refreshBattery() override;
    void join(const QString &ssid, const QString &secret) override;
    void setVolume(double volume) override;
    void setMute(bool muted) override;

private:
    using ReplySignal = void (SystemStatusClient::*)(const QByteArray &);
    void call(const QString &interface, const QString &method,
              const QVariantList &arguments, ReplySignal replySignal);
    // The service name/path are compiled in so the demo and the live shell
    // address the same object.
    QString m_service;
    QString m_path;
    bool m_available = false;
};

// The fixture client used by `DF_STATUS_FIXTURE` and the headless tests. It
// simulates the host well enough for the demo: volume/mute mutate its view
// and re-emit, so the status glyph and slider move. Its battery view reports
// a present charging battery so the demo can render the item on a host with
// no battery of its own.
class MockSystemStatusClient : public SystemStatusClient
{
    Q_OBJECT

public:
    explicit MockSystemStatusClient(QObject *parent = nullptr);

    bool isAvailable() const override { return true; }
    void refreshWifi() override;
    void refreshAudio() override;
    void refreshBattery() override;
    void join(const QString &ssid, const QString &secret) override;
    void setVolume(double volume) override;
    void setMute(bool muted) override;

private:
    QString m_activeSsid = QStringLiteral("dragonfruit");
    double m_volume = 0.6;
    bool m_muted = false;
};