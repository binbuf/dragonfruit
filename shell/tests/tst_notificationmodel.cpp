// SPDX-License-Identifier: MIT
// Notification-model tests (T-11.1a): the service's `Banners()`/`History()`
// JSON decodes to the shell's banner and history views. No D-Bus, QML, or
// Wayland.
#include "notificationmodel.h"

#include <QTest>

class TestNotificationModel : public QObject
{
    Q_OBJECT

private slots:
    void a_banner_payload_decodes()
    {
        NotificationModel model;
        model.applyBannersJson(QByteArrayLiteral(
            "[{\"id\":7,\"appName\":\"Mail\",\"summary\":\"New message\","
            "\"body\":\"From Ada\",\"urgency\":\"normal\",\"deadline\":1000,\"actions\":[]}]"));
        QVERIFY(model.hasBanner());
        const QVariantMap banner = model.banner();
        QCOMPARE(banner.value(QStringLiteral("id")).toUInt(), 7u);
        QCOMPARE(banner.value(QStringLiteral("appName")).toString(), QStringLiteral("Mail"));
        QCOMPARE(banner.value(QStringLiteral("summary")).toString(), QStringLiteral("New message"));
        QCOMPARE(model.banners().size(), 1);
    }

    void the_newest_banner_is_the_one_shown()
    {
        NotificationModel model;
        model.applyBannersJson(QByteArrayLiteral(
            "[{\"id\":1,\"summary\":\"older\"},{\"id\":2,\"summary\":\"newer\"}]"));
        QCOMPARE(model.banner().value(QStringLiteral("id")).toUInt(), 2u);
        QCOMPARE(model.banner().value(QStringLiteral("summary")).toString(),
                 QStringLiteral("newer"));
        QCOMPARE(model.banners().size(), 2);
    }

    void an_empty_payload_clears_the_banner()
    {
        NotificationModel model;
        model.applyBannersJson(QByteArrayLiteral("[{\"id\":1}]"));
        QVERIFY(model.hasBanner());
        model.applyBannersJson(QByteArrayLiteral("[]"));
        QVERIFY(!model.hasBanner());
        QVERIFY(model.banner().isEmpty());
    }

    void a_malformed_payload_is_the_safe_default()
    {
        NotificationModel model;
        QString error;
        QVERIFY(NotificationModel::parseList(QByteArrayLiteral("not json"), &error).isEmpty());
        QVERIFY(!error.isEmpty());
        QVERIFY(NotificationModel::parseList(QByteArrayLiteral("{\"object\":true}")).isEmpty());
        // A malformed apply clears rather than keeps stale state.
        model.applyBannersJson(QByteArrayLiteral("[{\"id\":1}]"));
        model.applyBannersJson(QByteArrayLiteral("}{"));
        QVERIFY(!model.hasBanner());
    }

    void the_history_decodes_with_reasons()
    {
        NotificationModel model;
        model.applyHistoryJson(QByteArrayLiteral(
            "[{\"id\":3,\"appName\":\"Chat\",\"summary\":\"Ping\",\"reason\":\"dismissed\","
            "\"closedAt\":1234}]"));
        QCOMPARE(model.history().size(), 1);
        const QVariantMap entry = model.history().first().toMap();
        QCOMPARE(entry.value(QStringLiteral("reason")).toString(), QStringLiteral("dismissed"));
        QCOMPARE(entry.value(QStringLiteral("closedAt")).toULongLong(), 1234ull);
    }
};

QTEST_MAIN(TestNotificationModel)
#include "tst_notificationmodel.moc"