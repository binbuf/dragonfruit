// SPDX-License-Identifier: MIT
// The Notification Center widget stack (T234): the catalogue is the measured
// 19 categories, the default is the live Calendar card, and add/remove/resize
// plus the provider-data seam are pure model behaviour.
#include "widgetmodel.h"

#include <QSignalSpy>
#include <QtTest>

class TestWidgetModel : public QObject
{
    Q_OBJECT

private slots:
    void theCatalogueHasTheMeasuredCategories();
    void theDefaultStackIsTheLiveCalendar();
    void addingRemovingAndResizingAreStable();
    void duplicateKindsAreRejected();
    void providerDataDrivesTheAvailableState();
    void unknownKindsAndSizesAreRejected();
};

void TestWidgetModel::theCatalogueHasTheMeasuredCategories()
{
    const QStringList categories = WidgetModel::defaultCategories();
    QCOMPARE(categories.size(), 19);
    QCOMPARE(categories.first(), QStringLiteral("Clock"));
    QCOMPARE(categories.last(), QStringLiteral("Tips"));

    const QVariantList catalog = WidgetModel::defaultCatalog();
    QCOMPARE(catalog.size(), 19);
    for (const QVariant &value : catalog) {
        const QVariantMap entry = value.toMap();
        QVERIFY(!entry.value(QStringLiteral("kind")).toString().isEmpty());
        QVERIFY(!entry.value(QStringLiteral("title")).toString().isEmpty());
        QVERIFY(categories.contains(entry.value(QStringLiteral("category")).toString()));
    }
}

void TestWidgetModel::theDefaultStackIsTheLiveCalendar()
{
    WidgetModel model;
    const QVariantList widgets = model.widgets();
    QCOMPARE(widgets.size(), 1);
    const QVariantMap calendar = widgets.first().toMap();
    QCOMPARE(calendar.value(QStringLiteral("kind")).toString(),
             QStringLiteral("calendar"));
    QCOMPARE(calendar.value(QStringLiteral("size")).toString(),
             QStringLiteral("medium"));
    QVERIFY(calendar.value(QStringLiteral("available")).toBool());
}

void TestWidgetModel::addingRemovingAndResizingAreStable()
{
    WidgetModel model;
    QSignalSpy spy(&model, &WidgetModel::widgetsChanged);

    QVERIFY(model.addWidget(QStringLiteral("weather"), QStringLiteral("large")));
    QCOMPARE(model.widgets().size(), 2);
    QCOMPARE(spy.count(), 1);
    const QVariantMap weather = model.widgets().at(1).toMap();
    const QString id = weather.value(QStringLiteral("id")).toString();
    QVERIFY(!id.isEmpty());
    QCOMPARE(weather.value(QStringLiteral("size")).toString(), QStringLiteral("large"));
    // A provider-less widget is explicitly unavailable, not invented.
    QVERIFY(!weather.value(QStringLiteral("available")).toBool());

    QVERIFY(model.resizeWidget(id, QStringLiteral("small")));
    QCOMPARE(model.widgets().at(1).toMap().value(QStringLiteral("size")).toString(),
             QStringLiteral("small"));

    QVERIFY(model.removeWidget(id));
    QCOMPARE(model.widgets().size(), 1);
    QVERIFY(!model.removeWidget(id));
}

void TestWidgetModel::duplicateKindsAreRejected()
{
    WidgetModel model;
    QVERIFY(!model.addWidget(QStringLiteral("calendar")));
    QCOMPARE(model.widgets().size(), 1);
}

void TestWidgetModel::providerDataDrivesTheAvailableState()
{
    WidgetModel model;
    QVERIFY(model.addWidget(QStringLiteral("weather")));
    const QString id = model.widgets().at(1).toMap()
                           .value(QStringLiteral("id")).toString();
    QVERIFY(!model.widgets().at(1).toMap().value(QStringLiteral("available")).toBool());

    QVERIFY(model.setWidgetData(id, { { QStringLiteral("temperature"), 18 },
                                      { QStringLiteral("condition"),
                                        QStringLiteral("Clear") } }));
    const QVariantMap weather = model.widgets().at(1).toMap();
    QVERIFY(weather.value(QStringLiteral("available")).toBool());
    QCOMPARE(weather.value(QStringLiteral("data")).toMap()
                 .value(QStringLiteral("temperature")).toInt(), 18);

    // An empty payload is the explicit absence again.
    QVERIFY(model.setWidgetData(id, {}));
    QVERIFY(!model.widgets().at(1).toMap().value(QStringLiteral("available")).toBool());
}

void TestWidgetModel::unknownKindsAndSizesAreRejected()
{
    WidgetModel model;
    QVERIFY(!model.addWidget(QStringLiteral("hologram")));
    QVERIFY(!model.addWidget(QStringLiteral("weather"), QStringLiteral("huge")));
    QVERIFY(!model.resizeWidget(QStringLiteral("widget-1"), QStringLiteral("huge")));
}

QTEST_MAIN(TestWidgetModel)
#include "tst_widgetmodel.moc"
