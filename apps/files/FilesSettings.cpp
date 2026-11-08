// SPDX-License-Identifier: MIT
#include "FilesSettings.h"

#include "settingsclient.h"

#include <QString>

FilesSettings::FilesSettings(QObject *parent)
    : QObject(parent)
{
    // The same seam as Settings: `DF_SETTINGS_FIXTURE` selects the
    // deterministic in-process client for headless QML tests; otherwise the
    // live session-bus client serves the schema defaults when the daemon is
    // absent.
    if (qEnvironmentVariableIsSet("DF_SETTINGS_FIXTURE"))
        m_client = new MockSettingsClient(this);
    else
        m_client = new DbusSettingsClient(this);

    connect(m_client, &SettingsClient::changed, this,
            [this](const QString &, const QVariant &) { emit valuesChanged(); });
    connect(m_client, &SettingsClient::availableChanged, this,
            [this](bool) { emit availableChanged(); });
}

QVariantMap FilesSettings::values() const
{
    return m_client->values();
}

bool FilesSettings::available() const
{
    return m_client->isAvailable();
}

QString FilesSettings::glassRollout() const
{
    // Only an explicit override selects a rail; an empty result lets the QML
    // shell read `Theme.glass.rollout.defaultMode` from the shared tokens.
    const QString requested =
        qEnvironmentVariable("DRAGONFRUIT_GLASS_ROLLOUT").trimmed().toLower();
    return (requested == QLatin1String("legacy")
            || requested == QLatin1String("replacement"))
        ? requested : QString();
}

bool FilesSettings::appChromeSupported() const
{
    return !qEnvironmentVariableIsSet("DF_APP_CHROME_SOLID");
}

void FilesSettings::set(const QString &key, const QVariant &value)
{
    m_client->set(key, value);
}
