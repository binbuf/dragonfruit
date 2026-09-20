// SPDX-License-Identifier: MIT
#include "docksettings.h"

#include "dockpins.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonValue>
#include <QSaveFile>

namespace {

constexpr int kSchema = 1;
const char kKeysField[] = "keys";
const char kSchemaField[] = "schema";

// The eventual `org.dragonfruit.Settings1` named-key shape (T-15).
const char kSizeKey[] = "dock.size";
const char kMagnificationKey[] = "dock.magnification";
const char kPositionKey[] = "dock.position";
const char kAutohideKey[] = "dock.autohide";
const char kAnimateOpeningKey[] = "dock.animateOpening";
const char kShowIndicatorsKey[] = "dock.showIndicators";
const char kMinimizeIntoTileIconKey[] = "dock.minimizeIntoTileIcon";
const char kMinimizedAnimationKey[] = "dock.minimizedAnimation";
const char kTitlebarDoubleClickKey[] = "dock.titlebarDoubleClick";
const char kShowRecentAppsKey[] = "dock.showRecentApps";
const char kReduceMotionKey[] = "accessibility.reduceMotion";

QString readString(const QJsonObject &keys, const char *key, const QString &fallback)
{
    const QJsonValue value = keys.value(QLatin1String(key));
    return value.isString() ? value.toString() : fallback;
}

bool readBool(const QJsonObject &keys, const char *key, bool fallback)
{
    const QJsonValue value = keys.value(QLatin1String(key));
    return value.isBool() ? value.toBool() : fallback;
}

double readReal(const QJsonObject &keys, const char *key, double fallback)
{
    const QJsonValue value = keys.value(QLatin1String(key));
    return value.isDouble() ? value.toDouble() : fallback;
}

} // namespace

DockSettings::DockSettings(const QString &filePath)
    : m_filePath(filePath)
{
}

QString DockSettings::defaultFilePath()
{
    return DockPins::defaultFilePath();
}

bool DockSettings::load()
{
    m_error.clear();
    m_size = 0.5;
    m_magnification = 0.5;
    m_position = QStringLiteral("bottom");
    m_autohide = false;
    m_animateOpening = true;
    m_showIndicators = true;
    m_minimizeIntoTileIcon = false;
    m_minimizedAnimation = QStringLiteral("scale");
    m_titlebarDoubleClick = QStringLiteral("zoom");
    m_showRecentApps = false;
    m_reduceMotion = false;

    QFile file(m_filePath);
    if (!file.exists())
        return true;
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        m_error = QStringLiteral("cannot read %1").arg(m_filePath);
        return false;
    }
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        m_error = QStringLiteral("malformed settings file %1: %2")
                      .arg(m_filePath, parseError.errorString());
        return false;
    }
    const QJsonObject keys = document.object().value(QLatin1String(kKeysField)).toObject();

    setSize(readReal(keys, kSizeKey, 0.5));
    setMagnification(readReal(keys, kMagnificationKey, 0.5));
    setPosition(readString(keys, kPositionKey, QStringLiteral("bottom")));
    m_autohide = readBool(keys, kAutohideKey, false);
    m_animateOpening = readBool(keys, kAnimateOpeningKey, true);
    m_showIndicators = readBool(keys, kShowIndicatorsKey, true);
    m_minimizeIntoTileIcon = readBool(keys, kMinimizeIntoTileIconKey, false);
    setMinimizedAnimation(readString(keys, kMinimizedAnimationKey, QStringLiteral("scale")));
    setTitlebarDoubleClick(readString(keys, kTitlebarDoubleClickKey, QStringLiteral("zoom")));
    m_showRecentApps = readBool(keys, kShowRecentAppsKey, false);
    m_reduceMotion = readBool(keys, kReduceMotionKey, false);
    return true;
}

bool DockSettings::fileExists() const
{
    return QFile::exists(m_filePath);
}

bool DockSettings::save()
{
    QDir().mkpath(QFileInfo(m_filePath).absolutePath());

    // Re-read the file so keys written by another owner (DockPins, a future
    // settingsd, or an unknown key) survive this write.
    QJsonObject root;
    QFile existing(m_filePath);
    if (existing.exists() && existing.open(QIODevice::ReadOnly | QIODevice::Text)) {
        const QJsonDocument document = QJsonDocument::fromJson(existing.readAll());
        if (document.isObject())
            root = document.object();
    }
    root.insert(QLatin1String(kSchemaField), kSchema);
    QJsonObject keys = root.value(QLatin1String(kKeysField)).toObject();

    keys.insert(QLatin1String(kSizeKey), m_size);
    keys.insert(QLatin1String(kMagnificationKey), m_magnification);
    keys.insert(QLatin1String(kPositionKey), m_position);
    keys.insert(QLatin1String(kAutohideKey), m_autohide);
    keys.insert(QLatin1String(kAnimateOpeningKey), m_animateOpening);
    keys.insert(QLatin1String(kShowIndicatorsKey), m_showIndicators);
    keys.insert(QLatin1String(kMinimizeIntoTileIconKey), m_minimizeIntoTileIcon);
    keys.insert(QLatin1String(kMinimizedAnimationKey), m_minimizedAnimation);
    keys.insert(QLatin1String(kTitlebarDoubleClickKey), m_titlebarDoubleClick);
    keys.insert(QLatin1String(kShowRecentAppsKey), m_showRecentApps);
    keys.insert(QLatin1String(kReduceMotionKey), m_reduceMotion);
    root.insert(QLatin1String(kKeysField), keys);

    QSaveFile file(m_filePath);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        m_error = QStringLiteral("cannot write %1").arg(m_filePath);
        return false;
    }
    file.write(QJsonDocument(root).toJson(QJsonDocument::Indented));
    if (!file.commit()) {
        m_error = QStringLiteral("cannot commit %1").arg(m_filePath);
        return false;
    }
    return true;
}

void DockSettings::setSize(double value)
{
    m_size = qBound(0.0, value, 1.0);
}

void DockSettings::setMagnification(double value)
{
    m_magnification = qBound(0.0, value, 1.0);
}

void DockSettings::setPosition(const QString &value)
{
    m_position = (value == QLatin1String("left") || value == QLatin1String("right"))
                     ? value
                     : QStringLiteral("bottom");
}

void DockSettings::setMinimizedAnimation(const QString &value)
{
    m_minimizedAnimation = (value == QLatin1String("genie") || value == QLatin1String("none"))
                               ? value
                               : QStringLiteral("scale");
}

void DockSettings::setTitlebarDoubleClick(const QString &value)
{
    m_titlebarDoubleClick = (value == QLatin1String("minimize") || value == QLatin1String("none"))
                                ? value
                                : QStringLiteral("zoom");
}

bool DockSettings::equals(const DockSettings &other) const
{
    return m_size == other.m_size && m_magnification == other.m_magnification
           && m_position == other.m_position && m_autohide == other.m_autohide
           && m_animateOpening == other.m_animateOpening
           && m_showIndicators == other.m_showIndicators
           && m_minimizeIntoTileIcon == other.m_minimizeIntoTileIcon
           && m_minimizedAnimation == other.m_minimizedAnimation
           && m_titlebarDoubleClick == other.m_titlebarDoubleClick
           && m_showRecentApps == other.m_showRecentApps
           && m_reduceMotion == other.m_reduceMotion;
}
