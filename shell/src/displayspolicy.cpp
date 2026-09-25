// SPDX-License-Identifier: MIT
#include "displayspolicy.h"

namespace {

QVariant readValue(const QVariantMap &values, const QString &key, const QVariant &fallback)
{
    const QVariant value = values.value(key);
    return value.isValid() ? value : fallback;
}

} // namespace

DisplaySettings displaySettingsFromValues(const QVariantMap &values)
{
    DisplaySettings settings;
    settings.scale = readValue(values, QStringLiteral("display.scale"), settings.scale).toDouble();
    settings.rotation =
        readValue(values, QStringLiteral("display.rotation"), settings.rotation).toString();
    settings.brightness =
        readValue(values, QStringLiteral("display.brightness"), settings.brightness).toDouble();
    return settings;
}

uint32_t outputTransformFromName(const QString &name)
{
    // df_output.transform: normal=0, 90=1, 180=2, 270=3.
    if (name == QLatin1String("90"))
        return 1;
    if (name == QLatin1String("180"))
        return 2;
    if (name == QLatin1String("270"))
        return 3;
    return 0;
}