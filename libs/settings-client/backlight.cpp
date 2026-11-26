// SPDX-License-Identifier: GPL-3.0-only
#include "backlight.h"

#include <QDir>
#include <QFile>
#include <QtGlobal>

namespace {

QString readTrimmed(const QString &path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text))
        return QString();
    return QString::fromUtf8(file.readAll()).trimmed();
}

// Raw hardware wins over a platform shim, which wins over a firmware-emulated
// device. This keeps a laptop with both `intel_backlight` and `acpi_video0`
// on the real hardware path.
int deviceScore(const BacklightDevice &device)
{
    if (device.type == QLatin1String("raw"))
        return 0;
    if (device.type == QLatin1String("platform"))
        return 1;
    if (device.type == QLatin1String("firmware"))
        return 2;
    return 3;
}

} // namespace

BacklightDevice resolveBacklightDevice(const QString &root)
{
    BacklightDevice best;
    int bestScore = 4;

    const QDir directory(root);
    const QStringList names = directory.entryList(
        QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name);
    for (const QString &name : names) {
        const QString path = directory.filePath(name);
        bool ok = false;
        const int maxBrightness =
            readTrimmed(path + QStringLiteral("/max_brightness"))
                .toInt(&ok);
        if (!ok || maxBrightness <= 0)
            continue;

        BacklightDevice candidate;
        candidate.name = name;
        candidate.path = path;
        candidate.maxBrightness = maxBrightness;
        candidate.type = readTrimmed(path + QStringLiteral("/type"));

        const int score = deviceScore(candidate);
        if (score < bestScore) {
            best = candidate;
            bestScore = score;
        }
    }
    return best;
}

int mapBrightnessLevel(double level, int maxBrightness)
{
    if (maxBrightness <= 0)
        return 0;
    const double clamped = qBound(0.0, level, 1.0);
    return qBound(0, qRound(clamped * maxBrightness), maxBrightness);
}
