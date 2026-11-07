// SPDX-License-Identifier: MIT
// Notification-model tests (T-11.1a): the service's `Banners()`/`History()`
// JSON decodes to the shell's banner and history views. T204 adds the
// Notification Center state: stable keys, active/history separation, keyboard
// selection and the service invoke/dismiss contract. No D-Bus, QML, or
// Wayland.
#include "notificationcenter.h"
#include "notificationmodel.h"

#include <QSignalSpy>
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

    void the_focus_policy_decodes()
    {
        NotificationModel model;
        model.applyFocusPolicyJson(QByteArrayLiteral(
            "{\"mode\":\"focus\",\"allowList\":[\"Chat\"],\"batchedCount\":2}"));
        const QVariantMap policy = model.focusPolicy();
        QCOMPARE(policy.value(QStringLiteral("mode")).toString(), QStringLiteral("focus"));
        QCOMPARE(policy.value(QStringLiteral("batchedCount")).toInt(), 2);
        QCOMPARE(policy.value(QStringLiteral("allowList")).toStringList(),
                 QStringList{ QStringLiteral("Chat") });
    }

    void a_missing_or_malformed_focus_policy_clears()
    {
        NotificationModel model;
        model.applyFocusPolicyJson(QByteArrayLiteral("{\"mode\":\"dnd\",\"batchedCount\":5}"));
        QCOMPARE(model.focusPolicy().value(QStringLiteral("mode")).toString(),
                 QStringLiteral("dnd"));
        // The service going away sends `{}`; a malformed payload is the same
        // safe "no Focus item" default.
        model.applyFocusPolicyJson(QByteArrayLiteral("{}"));
        QVERIFY(model.focusPolicy().isEmpty());
        model.applyFocusPolicyJson(QByteArrayLiteral("not json"));
        QVERIFY(model.focusPolicy().isEmpty());
        model.applyFocusPolicyJson(QByteArrayLiteral("[1,2]"));
        QVERIFY(model.focusPolicy().isEmpty());
    }

    // --- T204 Notification Center -----------------------------------------

    static QVariantMap entry(quint32 id, const QString &summary, bool suppressed = false,
                             const QString &reason = QString(),
                             qulonglong closedAt = 0)
    {
        QVariantMap map{
            { QStringLiteral("id"), id },
            { QStringLiteral("appName"), QStringLiteral("Mail") },
            { QStringLiteral("summary"), summary },
            { QStringLiteral("body"), QStringLiteral("Body") },
            { QStringLiteral("urgency"), QStringLiteral("normal") },
            { QStringLiteral("suppressed"), suppressed },
            { QStringLiteral("actions"),
              QVariantList{ QVariantMap{ { QStringLiteral("key"), QStringLiteral("reply") },
                                         { QStringLiteral("label"), QStringLiteral("Reply") } },
                            QVariantMap{ { QStringLiteral("key"), QStringLiteral("default") },
                                         { QStringLiteral("label"), QStringLiteral("Open") } } } },
        };
        if (!reason.isEmpty())
            map.insert(QStringLiteral("reason"), reason);
        if (closedAt != 0)
            map.insert(QStringLiteral("closedAt"), closedAt);
        return map;
    }

    void active_history_keeps_actions_but_closed_and_suppressed_rows_do_not()
    {
        const QVariantList normalized = NotificationCenter::normalizeHistory(
            { entry(1, QStringLiteral("active")),
              entry(2, QStringLiteral("expired"), false, QStringLiteral("expired"), 1200),
              entry(3, QStringLiteral("dismissed"), false, QStringLiteral("dismissed"), 1300),
              entry(4, QStringLiteral("quiet"), true) });

        QCOMPARE(normalized.size(), 4);
        const QVariantMap active = normalized.at(0).toMap();
        QCOMPARE(active.value(QStringLiteral("key")).toString(), QStringLiteral("1"));
        QCOMPARE(active.value(QStringLiteral("status")).toString(), QStringLiteral("active"));
        QVERIFY(active.value(QStringLiteral("active")).toBool());
        QVERIFY(active.value(QStringLiteral("canAct")).toBool());
        QCOMPARE(active.value(QStringLiteral("actions")).toList().size(), 2);

        const QVariantMap expired = normalized.at(1).toMap();
        QCOMPARE(expired.value(QStringLiteral("status")).toString(), QStringLiteral("expired"));
        QVERIFY(!expired.value(QStringLiteral("active")).toBool());
        QVERIFY(!expired.value(QStringLiteral("canAct")).toBool());
        QVERIFY(expired.value(QStringLiteral("actions")).toList().isEmpty());

        const QVariantMap dismissed = normalized.at(2).toMap();
        QCOMPARE(dismissed.value(QStringLiteral("status")).toString(),
                 QStringLiteral("dismissed"));
        QVERIFY(dismissed.value(QStringLiteral("actions")).toList().isEmpty());

        const QVariantMap suppressed = normalized.at(3).toMap();
        QCOMPARE(suppressed.value(QStringLiteral("status")).toString(),
                 QStringLiteral("suppressed"));
        QVERIFY(!suppressed.value(QStringLiteral("active")).toBool());
        QVERIFY(suppressed.value(QStringLiteral("actions")).toList().isEmpty());
    }

    void a_duplicate_id_keeps_the_newest_row()
    {
        const QVariantList normalized = NotificationCenter::normalizeHistory(
            { entry(7, QStringLiteral("newest")), entry(7, QStringLiteral("stale")) });
        QCOMPARE(normalized.size(), 1);
        QCOMPARE(normalized.at(0).toMap().value(QStringLiteral("summary")).toString(),
                 QStringLiteral("newest"));
    }

    void selection_follows_the_stable_key_across_reorders()
    {
        NotificationCenter center;
        center.setHistory({ entry(1, QStringLiteral("one")),
                            entry(2, QStringLiteral("two")),
                            entry(3, QStringLiteral("three")) });
        center.open();
        QCOMPARE(center.selectedKey(), QStringLiteral("1"));
        center.moveSelection(1);
        QCOMPARE(center.selectedKey(), QStringLiteral("2"));
        // A service refresh reorders history; the selected notification keeps
        // its identity instead of jumping to a different row index.
        center.setHistory({ entry(3, QStringLiteral("three")),
                            entry(2, QStringLiteral("two")),
                            entry(1, QStringLiteral("one")) });
        QCOMPARE(center.selectedKey(), QStringLiteral("2"));
        QCOMPARE(center.selectedIndex(), 1);
    }

    void invocation_and_dismissal_only_route_active_rows()
    {
        NotificationCenter center;
        center.setHistory({ entry(1, QStringLiteral("active")),
                            entry(2, QStringLiteral("expired"), false,
                                  QStringLiteral("expired"), 1200) });
        QSignalSpy invokeSpy(&center, &NotificationCenter::invokeRequested);
        QSignalSpy dismissSpy(&center, &NotificationCenter::dismissRequested);

        QVERIFY(center.invoke(1, QStringLiteral("reply")));
        QCOMPARE(invokeSpy.count(), 1);
        QCOMPARE(invokeSpy.at(0).at(0).toUInt(), 1u);
        QCOMPARE(invokeSpy.at(0).at(1).toString(), QStringLiteral("reply"));
        // An action the service did not advertise, or a history-only row, is
        // never forwarded.
        QVERIFY(!center.invoke(1, QStringLiteral("missing")));
        QVERIFY(!center.invoke(2, QStringLiteral("reply")));
        QCOMPARE(invokeSpy.count(), 1);

        QVERIFY(center.dismiss(1));
        QCOMPARE(dismissSpy.count(), 1);
        QCOMPARE(dismissSpy.at(0).at(0).toUInt(), 1u);
        QVERIFY(!center.dismiss(2));
        QCOMPARE(dismissSpy.count(), 1);
    }

    void return_uses_default_or_dismisses_and_history_rows_are_inert()
    {
        NotificationCenter center;
        center.setHistory({ entry(1, QStringLiteral("active")) });
        center.open();
        QSignalSpy invokeSpy(&center, &NotificationCenter::invokeRequested);
        QSignalSpy dismissSpy(&center, &NotificationCenter::dismissRequested);
        QVERIFY(center.activateSelected());
        QCOMPARE(invokeSpy.count(), 1);
        QCOMPARE(invokeSpy.at(0).at(1).toString(), QStringLiteral("default"));
        QCOMPARE(dismissSpy.count(), 0);

        // With no default action, Return dismisses the active row.
        center.setHistory({ QVariantMap{
            { QStringLiteral("id"), 2 },
            { QStringLiteral("summary"), QStringLiteral("no default") },
            { QStringLiteral("actions"),
              QVariantList{ QVariantMap{ { QStringLiteral("key"), QStringLiteral("reply") },
                                         { QStringLiteral("label"), QStringLiteral("Reply") } } } },
        } });
        QVERIFY(center.activateSelected());
        QCOMPARE(dismissSpy.count(), 1);

        // A history-only row is never activated or dismissed.
        center.setHistory({ entry(3, QStringLiteral("expired"), false,
                                  QStringLiteral("expired"), 1200) });
        QVERIFY(!center.activateSelected());
        QVERIFY(!center.dismissSelected());
        QCOMPARE(invokeSpy.count(), 1);
        QCOMPARE(dismissSpy.count(), 1);
    }

    void availability_drops_stale_rows_and_reconnect_repopulates()
    {
        NotificationCenter center;
        center.setHistory({ entry(1, QStringLiteral("one")) });
        center.open();
        center.setAvailable(false);
        QVERIFY(center.history().isEmpty());
        QCOMPARE(center.selectedIndex(), -1);
        QVERIFY(!center.available());
        // The service came back: the next History() repopulates the same view.
        center.setAvailable(true);
        center.setHistory({ entry(1, QStringLiteral("one")) });
        QCOMPARE(center.history().size(), 1);
        QVERIFY(center.available());
        // Lock/output teardown clears without touching availability.
        center.clear();
        QVERIFY(center.history().isEmpty());
        QVERIFY(center.available());
    }

    void open_close_toggle_and_empty_state_are_safe()
    {
        NotificationCenter center;
        QSignalSpy closedSpy(&center, &NotificationCenter::closed);
        center.open();
        QVERIFY(center.isOpen());
        QCOMPARE(center.selectedIndex(), -1);
        QVERIFY(!center.activateSelected());
        QVERIFY(!center.dismissSelected());
        center.moveSelection(1); // no-op on an empty list
        QCOMPARE(center.selectedIndex(), -1);
        center.close(QStringLiteral("escape"));
        QVERIFY(!center.isOpen());
        QCOMPARE(closedSpy.count(), 1);
        QCOMPARE(closedSpy.at(0).at(0).toString(), QStringLiteral("escape"));
        // Closing an already-closed Center does not emit a second close.
        center.close(QStringLiteral("outside"));
        QCOMPARE(closedSpy.count(), 1);

        center.toggle();
        QVERIFY(center.isOpen());
        center.toggle();
        QVERIFY(!center.isOpen());
    }
};

QTEST_MAIN(TestNotificationModel)
#include "tst_notificationmodel.moc"