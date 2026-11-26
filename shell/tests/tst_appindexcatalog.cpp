// SPDX-License-Identifier: GPL-3.0-only
#include "appindexclient.h"

#include <QDBusConnection>
#include <QDBusContext>
#include <QDBusMessage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSignalSpy>
#include <QTest>

namespace {
const QString kService = QStringLiteral("org.dragonfruit.AppIndex1");
const QString kPath = QStringLiteral("/org/dragonfruit/AppIndex1");

QString catalogWith(const QString &name)
{
    const QJsonArray rows{QJsonObject{
        {QStringLiteral("desktopId"), QStringLiteral("org.example.Editor.desktop")},
        {QStringLiteral("name"), name},
        {QStringLiteral("valid"), true},
    }};
    return QString::fromUtf8(QJsonDocument(rows).toJson(QJsonDocument::Compact));
}

class FakeAppIndex : public QObject, protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.AppIndex1")
public:
    void reply(const QString &json)
    {
        const auto request = requests.takeFirst();
        request.first.send(request.second.createReply(QVariant(json)));
    }

    QList<QPair<QDBusConnection, QDBusMessage>> requests;
    int subscriptions = 0;

public slots:
    QString Subscribe(const QString &interests)
    {
        ++subscriptions;
        return interests;
    }
    QString Enumerate()
    {
        setDelayedReply(true);
        requests.append({connection(), message()});
        return {};
    }

signals:
    void Changed(const QString &interests);
};
} // namespace

class TestAppIndexCatalog : public QObject
{
    Q_OBJECT
private slots:
    void coalescesRefreshAndKeepsTheLastCompleteSnapshot()
    {
        auto serviceBus = QDBusConnection::connectToBus(
            QDBusConnection::SessionBus, QStringLiteral("catalog-test-service"));
        QVERIFY(serviceBus.isConnected());
        FakeAppIndex service;
        QVERIFY(serviceBus.registerObject(kPath, &service,
                                          QDBusConnection::ExportAllSlots
                                              | QDBusConnection::ExportAllSignals));
        QVERIFY(serviceBus.registerService(kService));

        AppIndexCatalog catalog;
        QSignalSpy changed(&catalog, &AppIndexCatalog::catalogChanged);
        QSignalSpy ownerChanged(&catalog, &AppIndexCatalog::ownerChanged);
        catalog.start();
        QTRY_COMPARE(service.requests.size(), 1);
        QCOMPARE(service.subscriptions, 1);
        QCOMPARE(changed.count(), 0);

        service.reply(catalogWith(QStringLiteral("Initial")));
        QTRY_COMPARE(changed.count(), 1);

        emit service.Changed(QStringLiteral("identity"));
        QTRY_COMPARE(service.requests.size(), 1);
        for (int index = 0; index < 6; ++index)
            catalog.refresh();
        service.reply(QStringLiteral("[{}]"));
        QTRY_COMPARE(service.requests.size(), 1);
        QCOMPARE(catalog.index().entries().first().name, QStringLiteral("Initial"));
        QCOMPARE(changed.count(), 1);
        service.reply(catalogWith(QStringLiteral("Initial")));
        QTest::qWait(50);
        QVERIFY(service.requests.isEmpty());
        QCOMPARE(changed.count(), 1);

        // Keep an old reply outstanding as the well-known name changes.
        catalog.refresh();
        QTRY_COMPARE(service.requests.size(), 1);
        QVERIFY(serviceBus.unregisterService(kService));
        QTRY_VERIFY(!catalog.available());
        QCOMPARE(catalog.index().entries().first().name, QStringLiteral("Initial"));

        auto replacementBus = QDBusConnection::connectToBus(
            QDBusConnection::SessionBus, QStringLiteral("catalog-test-replacement"));
        FakeAppIndex replacement;
        QVERIFY(replacementBus.registerObject(kPath, &replacement,
                                              QDBusConnection::ExportAllSlots));
        QVERIFY(replacementBus.registerService(kService));
        QTRY_COMPARE(ownerChanged.count(), 3);
        service.reply(catalogWith(QStringLiteral("Obsolete")));
        QTRY_COMPARE(replacement.requests.size(), 1);
        QCOMPARE(replacement.subscriptions, 1);
        QCOMPARE(catalog.index().entries().first().name, QStringLiteral("Initial"));
        QCOMPARE(changed.count(), 1);

        replacement.reply(QStringLiteral("[]"));
        QTRY_COMPARE(changed.count(), 2);
        QVERIFY(catalog.index().isEmpty());
        QVERIFY(replacementBus.unregisterService(kService));
    }
};

QTEST_GUILESS_MAIN(TestAppIndexCatalog)
#include "tst_appindexcatalog.moc"
