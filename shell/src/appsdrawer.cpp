// SPDX-License-Identifier: MIT
#include "appsdrawer.h"

#include <QSet>

#include <algorithm>

namespace {

// The one freedesktop -> pill mapping (T-19.2). The left column is a
// freedesktop main category (the `Categories` list also carries an `X-` and
// desktop-environment suffix set, e.g. `GTK`/`Qt`); the right column is one of
// the canonical pill keys above. Keep this table the single greppable source:
// adding a pill is one row plus the label in `AppsDrawer.qml`.
struct CategoryMapping {
    const char *freedesktop;
    const char *key;
};

const CategoryMapping kCategoryMap[] = {
    {"Development", "developer-tools"},

    {"Office", "productivity"},
    {"Finance", "productivity"},

    {"Utility", "utilities"},
    {"System", "utilities"},
    {"Settings", "utilities"},

    {"AudioVideo", "entertainment"},
    {"Audio", "entertainment"},
    {"Video", "entertainment"},
    {"Player", "entertainment"},

    {"Game", "games"},

    {"Network", "social"},
    {"Chat", "social"},
    {"InstantMessaging", "social"},
    {"Email", "social"},
    {"Telephony", "social"},
    {"VideoConference", "social"},
    {"ContactManagement", "social"},

    {"Graphics", "creativity"},
    {"AudioVideoEditing", "creativity"},
    {"Photography", "creativity"},
    {"Publishing", "creativity"},

    {"Education", "information"},
    {"Science", "information"},
    {"Documentation", "information"},
    {"Dictionary", "information"},
    {"News", "information"},
};

QString canonicalCategory(const QString &raw)
{
    const QString trimmed = raw.trimmed();
    if (trimmed.isEmpty())
        return QString();
    for (const CategoryMapping &mapping : kCategoryMap) {
        if (QLatin1String(mapping.freedesktop) == trimmed)
            return QString::fromLatin1(mapping.key);
    }
    return QString();
}

// Order `keys` by the canonical pill order, dropping duplicates. Pure.
QStringList canonicalOrder(const QSet<QString> &keys)
{
    QStringList ordered;
    for (const QString &key : appsDrawerCategoryKeys()) {
        if (key == QLatin1String("all"))
            continue;
        if (keys.contains(key))
            ordered.append(key);
    }
    return ordered;
}

} // namespace

QStringList appsDrawerCategoryKeys()
{
    return {QStringLiteral("all"),
            QStringLiteral("developer-tools"),
            QStringLiteral("productivity"),
            QStringLiteral("utilities"),
            QStringLiteral("entertainment"),
            QStringLiteral("games"),
            QStringLiteral("social"),
            QStringLiteral("creativity"),
            QStringLiteral("information")};
}

QString appsDrawerCategoryFor(const QString &freedesktopCategory)
{
    return canonicalCategory(freedesktopCategory);
}

QStringList appsDrawerCategoriesFor(const QString &freedesktopCategoryList)
{
    QSet<QString> keys;
    const QStringList parts = freedesktopCategoryList.split(QLatin1Char(';'), Qt::SkipEmptyParts);
    for (const QString &part : parts) {
        const QString key = canonicalCategory(part);
        if (!key.isEmpty())
            keys.insert(key);
    }
    return canonicalOrder(keys);
}

QStringList appsDrawerPresentCategories(const QList<DesktopEntry> &entries)
{
    QSet<QString> keys;
    QSet<QString> seen;
    for (const DesktopEntry &entry : entries) {
        if (!DesktopEntryIndex::isLaunchable(entry))
            continue;
        if (entry.id.isEmpty() || seen.contains(entry.id))
            continue;
        seen.insert(entry.id);
        for (const QString &category : entry.categories) {
            const QString key = canonicalCategory(category);
            if (!key.isEmpty())
                keys.insert(key);
        }
    }
    return canonicalOrder(keys);
}

QList<AppsDrawerRow> buildAppsDrawerList(const QList<DesktopEntry> &entries,
                                         const QString &category, const QString &query)
{
    const QString needle = query.trimmed().toLower();
    const QString selected = category.trimmed();

    QList<AppsDrawerRow> rows;
    rows.reserve(entries.size());
    QSet<QString> seen;
    for (const DesktopEntry &entry : entries) {
        if (!DesktopEntryIndex::isLaunchable(entry))
            continue;
        if (entry.id.isEmpty() || seen.contains(entry.id))
            continue;
        seen.insert(entry.id);

        AppsDrawerRow row;
        row.desktopId = entry.id;
        row.name = entry.name.isEmpty() ? entry.id : entry.name;
        row.iconPath = entry.iconPath;

        QSet<QString> keys;
        for (const QString &raw : entry.categories) {
            const QString key = canonicalCategory(raw);
            if (!key.isEmpty())
                keys.insert(key);
        }
        row.categories = canonicalOrder(keys);

        if (!selected.isEmpty() && selected != QLatin1String("all")
            && !row.categories.contains(selected)) {
            continue;
        }
        if (!needle.isEmpty() && !row.name.toLower().contains(needle)
            && !row.desktopId.toLower().contains(needle)) {
            continue;
        }
        rows.append(row);
    }

    std::sort(rows.begin(), rows.end(), [](const AppsDrawerRow &a, const AppsDrawerRow &b) {
        const int byName = QString::localeAwareCompare(a.name, b.name);
        if (byName != 0)
            return byName < 0;
        return a.desktopId < b.desktopId;
    });
    return rows;
}