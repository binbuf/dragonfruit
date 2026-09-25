// SPDX-License-Identifier: MIT
#include "SettingsBridge.h"

#include "settingsclient.h"

SettingsBridge::SettingsBridge(QObject *parent)
    : QObject(parent)
{
    // The Settings app is a consumer, not an owner: it links the exact client
    // the shell uses (ADR 0036). `DF_SETTINGS_FIXTURE` swaps in the
    // deterministic in-process mock for headless QML tests and captures, the
    // same way `DF_STATUS_FIXTURE` selects the status fixture.
    if (qEnvironmentVariableIsSet("DF_SETTINGS_FIXTURE"))
        m_client = new MockSettingsClient(this);
    else
        m_client = new DbusSettingsClient(this);

    connect(m_client, &SettingsClient::changed, this,
            [this](const QString &key, const QVariant &value) {
                emit valuesChanged();
                emit changed(key, value);
            });
    connect(m_client, &SettingsClient::availableChanged, this,
            [this](bool available) { emit availableChanged(available); });
}

SettingsBridge::~SettingsBridge() = default;

QVariantMap SettingsBridge::values() const
{
    return m_client->values();
}

bool SettingsBridge::available() const
{
    return m_client->isAvailable();
}

QVariant SettingsBridge::value(const QString &key, const QVariant &fallback) const
{
    return m_client->value(key, fallback);
}

void SettingsBridge::set(const QString &key, const QVariant &value)
{
    m_client->set(key, value);
}

void SettingsBridge::refresh()
{
    m_client->refresh();
}

QStringList SettingsBridge::keys() const
{
    return m_client->values().keys();
}