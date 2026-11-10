// SPDX-License-Identifier: MIT
#include "widgetmodel.h"

#include <QSet>

namespace {

QVariantMap widgetEntry(const QString &kind, const QString &title,
                        const QString &category, const QString &description)
{
    return {
        { QStringLiteral("kind"), kind },
        { QStringLiteral("title"), title },
        { QStringLiteral("category"), category },
        { QStringLiteral("description"), description },
        { QStringLiteral("sizes"),
          QStringList{ QStringLiteral("small"), QStringLiteral("medium"),
                       QStringLiteral("large"), QStringLiteral("extra-large") } },
    };
}

} // namespace

WidgetModel::WidgetModel(QObject *parent)
    : QObject(parent)
    , m_catalog(defaultCatalog())
    , m_categories(defaultCategories())
{
    reset();
}

QStringList WidgetModel::defaultCategories()
{
    return {
        QStringLiteral("Clock"),       QStringLiteral("Calendar"),
        QStringLiteral("Weather"),     QStringLiteral("Reminders"),
        QStringLiteral("Notes"),       QStringLiteral("Stocks"),
        QStringLiteral("Battery"),     QStringLiteral("Activity"),
        QStringLiteral("News"),        QStringLiteral("Music"),
        QStringLiteral("Podcasts"),    QStringLiteral("Books"),
        QStringLiteral("Software Center"),   QStringLiteral("Devices"),
        QStringLiteral("Home"),        QStringLiteral("Photos"),
        QStringLiteral("Files"),       QStringLiteral("Contacts"),
        QStringLiteral("Tips"),
    };
}

QVariantList WidgetModel::defaultCatalog()
{
    return {
        widgetEntry(QStringLiteral("clock"), QStringLiteral("Clock"),
                    QStringLiteral("Clock"),
                    QStringLiteral("The current time and date.")),
        widgetEntry(QStringLiteral("calendar"), QStringLiteral("Calendar"),
                    QStringLiteral("Calendar"),
                    QStringLiteral("The live month grid with today marked.")),
        widgetEntry(QStringLiteral("weather"), QStringLiteral("Weather"),
                    QStringLiteral("Weather"),
                    QStringLiteral("Current conditions when a provider is present.")),
        widgetEntry(QStringLiteral("reminders"), QStringLiteral("Reminders"),
                    QStringLiteral("Reminders"),
                    QStringLiteral("Upcoming reminders from the task provider.")),
        widgetEntry(QStringLiteral("notes"), QStringLiteral("Notes"),
                    QStringLiteral("Notes"),
                    QStringLiteral("A pinned note from the notes provider.")),
        widgetEntry(QStringLiteral("stocks"), QStringLiteral("Stocks"),
                    QStringLiteral("Stocks"),
                    QStringLiteral("A watchlist when a market provider is present.")),
        widgetEntry(QStringLiteral("battery"), QStringLiteral("Battery"),
                    QStringLiteral("Battery"),
                    QStringLiteral("The battery level and charging state.")),
        widgetEntry(QStringLiteral("screen-time"), QStringLiteral("Activity"),
                    QStringLiteral("Activity"),
                    QStringLiteral("Time in apps when activity is recorded.")),
        widgetEntry(QStringLiteral("news"), QStringLiteral("News"),
                    QStringLiteral("News"),
                    QStringLiteral("Headlines when a feed provider is present.")),
        widgetEntry(QStringLiteral("music"), QStringLiteral("Music"),
                    QStringLiteral("Music"),
                    QStringLiteral("The current track when a player is present.")),
        widgetEntry(QStringLiteral("podcasts"), QStringLiteral("Podcasts"),
                    QStringLiteral("Podcasts"),
                    QStringLiteral("The latest episode when a provider is present.")),
        widgetEntry(QStringLiteral("books"), QStringLiteral("Books"),
                    QStringLiteral("Books"),
                    QStringLiteral("Currently reading when a provider is present.")),
        widgetEntry(QStringLiteral("app-store"), QStringLiteral("Software Center"),
                    QStringLiteral("Software Center"),
                    QStringLiteral("Updates when the package provider is present.")),
        widgetEntry(QStringLiteral("find-my"), QStringLiteral("Devices"),
                    QStringLiteral("Devices"),
                    QStringLiteral("Nearby devices when the provider is present.")),
        widgetEntry(QStringLiteral("home"), QStringLiteral("Home"),
                    QStringLiteral("Home"),
                    QStringLiteral("Accessories when a home provider is present.")),
        widgetEntry(QStringLiteral("photos"), QStringLiteral("Photos"),
                    QStringLiteral("Photos"),
                    QStringLiteral("A memory when a photo provider is present.")),
        widgetEntry(QStringLiteral("files"), QStringLiteral("Files"),
                    QStringLiteral("Files"),
                    QStringLiteral("Recent files when the Files service is present.")),
        widgetEntry(QStringLiteral("contacts"), QStringLiteral("Contacts"),
                    QStringLiteral("Contacts"),
                    QStringLiteral("Favorites when the contacts provider is present.")),
        widgetEntry(QStringLiteral("tips"), QStringLiteral("Tips"),
                    QStringLiteral("Tips"),
                    QStringLiteral("A short desktop tip.")),
    };
}

bool WidgetModel::isKnownSize(const QString &size)
{
    static const QSet<QString> known{ QStringLiteral("small"), QStringLiteral("medium"),
                                      QStringLiteral("large"),
                                      QStringLiteral("extra-large") };
    return known.contains(size);
}

void WidgetModel::reset()
{
    m_widgets.clear();
    m_nextId = 1;
    addWidget(QStringLiteral("calendar"), QStringLiteral("medium"));
}

QVariantMap WidgetModel::entryForKind(const QString &kind) const
{
    for (const QVariant &value : m_catalog) {
        const QVariantMap entry = value.toMap();
        if (entry.value(QStringLiteral("kind")).toString() == kind)
            return entry;
    }
    return {};
}

int WidgetModel::indexOf(const QString &id) const
{
    for (int i = 0; i < m_widgets.size(); ++i) {
        if (m_widgets.at(i).toMap().value(QStringLiteral("id")).toString() == id)
            return i;
    }
    return -1;
}

bool WidgetModel::addWidget(const QString &kind, const QString &size)
{
    const QVariantMap catalogEntry = entryForKind(kind);
    if (catalogEntry.isEmpty() || !isKnownSize(size))
        return false;
    for (const QVariant &value : m_widgets) {
        if (value.toMap().value(QStringLiteral("kind")).toString() == kind)
            return false;
    }
    QVariantMap widget{
        { QStringLiteral("id"), QStringLiteral("widget-%1").arg(m_nextId++) },
        { QStringLiteral("kind"), kind },
        { QStringLiteral("size"), size },
        { QStringLiteral("title"), catalogEntry.value(QStringLiteral("title")) },
        { QStringLiteral("category"), catalogEntry.value(QStringLiteral("category")) },
        // A new widget has no provider until the shell pushes data. Calendar
        // is the one local widget: its month grid is computed in QML.
        { QStringLiteral("available"), kind == QLatin1String("calendar") },
        { QStringLiteral("data"), QVariantMap{} },
    };
    m_widgets.append(widget);
    emit widgetsChanged();
    return true;
}

bool WidgetModel::removeWidget(const QString &id)
{
    const int index = indexOf(id);
    if (index < 0)
        return false;
    m_widgets.removeAt(index);
    emit widgetsChanged();
    return true;
}

bool WidgetModel::resizeWidget(const QString &id, const QString &size)
{
    const int index = indexOf(id);
    if (index < 0 || !isKnownSize(size))
        return false;
    QVariantMap widget = m_widgets.at(index).toMap();
    if (widget.value(QStringLiteral("size")).toString() == size)
        return true;
    widget.insert(QStringLiteral("size"), size);
    m_widgets.replace(index, widget);
    emit widgetsChanged();
    return true;
}

bool WidgetModel::setWidgetData(const QString &id, const QVariantMap &data)
{
    const int index = indexOf(id);
    if (index < 0)
        return false;
    QVariantMap widget = m_widgets.at(index).toMap();
    widget.insert(QStringLiteral("data"), data);
    widget.insert(QStringLiteral("available"), !data.isEmpty());
    m_widgets.replace(index, widget);
    emit widgetsChanged();
    return true;
}

bool WidgetModel::setWidgetAvailable(const QString &id, bool available)
{
    const int index = indexOf(id);
    if (index < 0)
        return false;
    QVariantMap widget = m_widgets.at(index).toMap();
    if (widget.value(QStringLiteral("available")).toBool() == available)
        return true;
    widget.insert(QStringLiteral("available"), available);
    m_widgets.replace(index, widget);
    emit widgetsChanged();
    return true;
}
