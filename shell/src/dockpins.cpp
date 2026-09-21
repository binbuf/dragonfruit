// SPDX-License-Identifier: MIT
#include "dockpins.h"

#include "desktopentry.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QSaveFile>
#include <QStandardPaths>

namespace {

constexpr int kSchema = 1;
const char kKeysField[] = "keys";
const char kSchemaField[] = "schema";
const char kPinnedKey[] = "dock.pinned";

} // namespace

DockPins::DockPins(const QString &filePath)
    : m_filePath(filePath)
{
}

QString DockPins::defaultFilePath()
{
    const QString configHome =
        QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
    const QString base = configHome.isEmpty() ? QStringLiteral(".") : configHome;
    return base + QStringLiteral("/dragonfruit/settings.json");
}

bool DockPins::load()
{
    m_ids.clear();
    m_error.clear();

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
    const QJsonObject keys =
        document.object().value(QLatin1String(kKeysField)).toObject();
    const QJsonArray pinned = keys.value(QLatin1String(kPinnedKey)).toArray();
    for (const QJsonValue &value : pinned) {
        const QString id = value.toString();
        if (!id.isEmpty() && !m_ids.contains(id))
            m_ids.append(id);
    }
    return true;
}

bool DockPins::fileExists() const
{
    return QFile::exists(m_filePath);
}

bool DockPins::save()
{
    QDir().mkpath(QFileInfo(m_filePath).absolutePath());

    // Re-read the file so keys written by another owner (DockSettings, a
    // future settingsd, or an unknown key) survive this write.
    QJsonObject root;
    QFile existing(m_filePath);
    if (existing.exists() && existing.open(QIODevice::ReadOnly | QIODevice::Text)) {
        const QJsonDocument document = QJsonDocument::fromJson(existing.readAll());
        if (document.isObject())
            root = document.object();
    }
    root.insert(QLatin1String(kSchemaField), kSchema);
    QJsonObject keys = root.value(QLatin1String(kKeysField)).toObject();
    QJsonArray pinned;
    for (const QString &id : m_ids)
        pinned.append(id);
    keys.insert(QLatin1String(kPinnedKey), pinned);
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

void DockPins::setIds(const QStringList &ids)
{
    m_ids = ids;
}

bool DockPins::add(const QString &id)
{
    if (id.isEmpty() || m_ids.contains(id))
        return false;
    m_ids.append(id);
    return true;
}

bool DockPins::remove(const QString &id)
{
    return m_ids.removeOne(id);
}

bool DockPins::move(int from, int to)
{
    if (from < 0 || from >= m_ids.size() || to < 0 || to >= m_ids.size() || from == to)
        return false;
    m_ids.move(from, to);
    return true;
}

QStringList DockPins::resolveDefaultPins(const DesktopEntryIndex &index)
{
    QStringList resolved;
    auto add = [&resolved](const QString &id) {
        if (!id.isEmpty() && !resolved.contains(id))
            resolved.append(id);
    };

    // Our own first-party apps, by id (the only legal desktop name).
    const DesktopEntry files = index.byId(QStringLiteral("org.dragonfruit.Files.desktop"));
    if (files.valid)
        add(files.id);
    const DesktopEntry settings = index.byId(QStringLiteral("org.dragonfruit.Settings.desktop"));
    if (settings.valid)
        add(settings.id);

    // Terminal and browser are chosen by the freedesktop registered category
    // so no distribution-specific desktop id is hardcoded (T-01 FR-3).
    auto firstWithCategory = [&index](const QString &category) -> QString {
        for (const DesktopEntry &entry : index.entries()) {
            if (entry.noDisplay)
                continue;
            if (entry.categories.contains(category, Qt::CaseInsensitive))
                return entry.id;
        }
        return QString();
    };
    add(firstWithCategory(QStringLiteral("TerminalEmulator")));
    add(firstWithCategory(QStringLiteral("WebBrowser")));
    return resolved;
}
