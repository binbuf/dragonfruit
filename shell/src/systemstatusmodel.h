// SPDX-License-Identifier: MIT
// Shell-side system-status model (T-07.5a).
//
// The menu bar's Wi-Fi and volume menus render from the bridge host's JSON
// views (services/system-status, `org.dragonfruit.SystemStatus1`). This model
// is the one decode seam on the shell side: the host's `State()` payload (and
// its action reports) become the maps the QML popovers draw, and the user's
// menu gestures become the request signals the controller forwards to the
// host. Nothing here names a daemon or a D-Bus type, so it is unit-testable
// with plain maps.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class SystemStatusModel : public QObject
{
    Q_OBJECT

public:
    explicit SystemStatusModel(QObject *parent = nullptr);

    // The decoded Wi-Fi / audio views, shaped exactly as the QML consumes
    // them. An unknown kind or a malformed payload clears the view (the safe
    // "absent" default: the item hides).
    QVariantMap wifi() const { return m_wifi; }
    QVariantMap audio() const { return m_audio; }

    // Whether the item is drawn at all (`state != "unavailable"`).
    bool wifiVisible() const;
    bool audioVisible() const;

    // Decode a host `State()` payload (JSON object). Returns the normalized
    // map; an empty map on a parse failure, with `error` set when non-null.
    static QVariantMap parseView(const QByteArray &json, const QString &kind,
                                 QString *error = nullptr);

    // The host's action report (`accepted`/`denied`/`absent`/`failed`), for
    // logging and for tests. Returns empty on a parse failure.
    static QString outcomeOf(const QByteArray &json);

public slots:
    // Apply a decoded view from the host. A payload whose `kind` does not
    // match is ignored, so the two menus cannot cross-pollute.
    void applyWifi(const QVariantMap &view);
    void applyAudio(const QVariantMap &view);
    void applyWifiJson(const QByteArray &json);
    void applyAudioJson(const QByteArray &json);

    // User gestures from the popovers; the controller forwards each to the
    // bridge host. They do not mutate the view (the host re-read is the only
    // source of truth).
    void requestJoin(const QString &ssid, const QString &secret);
    void requestVolume(double volume);
    void requestMute(bool muted);
    void requestRefreshWifi();
    void requestRefreshAudio();

signals:
    void changed();
    void joinRequested(const QString &ssid, const QString &secret);
    void volumeRequested(double volume);
    void muteRequested(bool muted);
    void refreshWifiRequested();
    void refreshAudioRequested();

private:
    static QVariantMap normalize(const QVariantMap &view, const QString &kind);
    QVariantMap m_wifi;
    QVariantMap m_audio;
};