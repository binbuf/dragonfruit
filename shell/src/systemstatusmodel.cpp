// SPDX-License-Identifier: MIT
#include "systemstatusmodel.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QVariantList>

namespace {

const QString kKindWifi = QStringLiteral("wifi");
const QString kKindAudio = QStringLiteral("audio");

} // namespace

SystemStatusModel::SystemStatusModel(QObject *parent)
    : QObject(parent)
{
}

bool SystemStatusModel::wifiVisible() const
{
    return m_wifi.value(QStringLiteral("visible")).toBool();
}

bool SystemStatusModel::audioVisible() const
{
    return m_audio.value(QStringLiteral("visible")).toBool();
}

QVariantMap SystemStatusModel::parseView(const QByteArray &json, const QString &kind, QString *error)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        if (error)
            *error = QStringLiteral("malformed status payload");
        return {};
    }
    QVariantMap view = document.toVariant().toMap();
    if (view.value(QStringLiteral("kind")).toString() != kind) {
        if (error)
            *error = QStringLiteral("unexpected status kind");
        return {};
    }
    return normalize(view, kind);
}

QString SystemStatusModel::outcomeOf(const QByteArray &json)
{
    const QJsonDocument document = QJsonDocument::fromJson(json);
    if (!document.isObject())
        return {};
    return document.object().value(QStringLiteral("outcome")).toString();
}

QVariantMap SystemStatusModel::normalize(const QVariantMap &view, const QString &kind)
{
    QVariantMap normalized = view;
    const QString state = view.value(QStringLiteral("state")).toString();
    const bool available = state == QLatin1String("available");
    const bool hidden = state.isEmpty() || state == QLatin1String("unavailable");
    normalized.insert(QStringLiteral("kind"), kind);
    normalized.insert(QStringLiteral("state"),
                      state.isEmpty() ? QStringLiteral("unavailable") : state);
    normalized.insert(QStringLiteral("visible"), !hidden);
    normalized.insert(QStringLiteral("enabled"), available);
    return normalized;
}

void SystemStatusModel::applyWifi(const QVariantMap &view)
{
    m_wifi = normalize(view, kKindWifi);
    emit changed();
}

void SystemStatusModel::applyAudio(const QVariantMap &view)
{
    m_audio = normalize(view, kKindAudio);
    emit changed();
}

void SystemStatusModel::applyWifiJson(const QByteArray &json)
{
    QString error;
    const QVariantMap view = parseView(json, kKindWifi, &error);
    if (!error.isEmpty())
        return;
    applyWifi(view);
}

void SystemStatusModel::applyAudioJson(const QByteArray &json)
{
    QString error;
    const QVariantMap view = parseView(json, kKindAudio, &error);
    if (!error.isEmpty())
        return;
    applyAudio(view);
}

void SystemStatusModel::requestJoin(const QString &ssid, const QString &secret)
{
    if (ssid.isEmpty())
        return;
    emit joinRequested(ssid, secret);
}

void SystemStatusModel::requestVolume(double volume)
{
    emit volumeRequested(qBound(0.0, volume, 1.0));
}

void SystemStatusModel::requestMute(bool muted)
{
    emit muteRequested(muted);
}

void SystemStatusModel::requestRefreshWifi()
{
    emit refreshWifiRequested();
}

void SystemStatusModel::requestRefreshAudio()
{
    emit refreshAudioRequested();
}