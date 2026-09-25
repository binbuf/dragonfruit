// SPDX-License-Identifier: MIT
#include "notificationmodel.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonParseError>

NotificationModel::NotificationModel(QObject *parent)
    : QObject(parent)
{
}

QVariantMap NotificationModel::banner() const
{
    if (m_banners.isEmpty())
        return {};
    return m_banners.constLast().toMap();
}

QVariantList NotificationModel::parseList(const QByteArray &json, QString *error)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isArray()) {
        if (error)
            *error = QStringLiteral("malformed notification payload");
        return {};
    }
    return document.toVariant().toList();
}

void NotificationModel::applyBannersJson(const QByteArray &json)
{
    m_banners = parseList(json);
    emit changed();
}

void NotificationModel::applyHistoryJson(const QByteArray &json)
{
    m_history = parseList(json);
    emit changed();
}

void NotificationModel::applyFocusPolicyJson(const QByteArray &json)
{
    const QJsonDocument document = QJsonDocument::fromJson(json);
    // The policy view is a JSON object (`mode`/`allowList`/`batchedCount`);
    // anything else is the safe "unavailable" default.
    if (document.isObject())
        m_focusPolicy = document.toVariant().toMap();
    else
        m_focusPolicy.clear();
    emit changed();
}
