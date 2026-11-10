// SPDX-License-Identifier: MIT
// Notification Center state tests (T204).
//
// The service still owns delivery, expiry, Focus/DND and the banner; this
// suite drives the shell-side normalization/selection controller directly and
// through the mock service client, so it covers the empty/populated states,
// stable identity, active-only actions, DND-suppressed history, reconnect and
// the close reasons. No Wayland, QML or session bus.
#include <QSignalSpy>
#include <QtTest>

#include "notificationcenter.h"
#include "notificationclient.h"
#include "notificationmodel.h"

namespace {

QVariantMap historyEntry(quint32 id, const QString &summary, const QString &reason,
                         quint64 closedAt, bool suppressed, const QVariantList &actions)
{
    QVariantMap entry;
    entry.insert(QStringLiteral("id"), id);
    entry.insert(QStringLiteral("appName"), QStringLiteral("Mail"));
    entry.insert(QStringLiteral("summary"), summary);
    entry.insert(QStringLiteral("body"), QStringLiteral("Body"));
    entry.insert(QStringLiteral("urgency"), QStringLiteral("normal"));
    entry.insert(QStringLiteral("reason"), reason.isEmpty() ? QVariant() : reason);
    entry.insert(QStringLiteral("closedAt"), closedAt == 0 ? QVariant() : closedAt);
    entry.insert(QStringLiteral("suppressed"), suppressed);
    entry.insert(QStringLiteral("actions"), actions);
    return entry;
}

QVariantList action(const QString &key, const QString &label)
{
    return { QVariantMap{ { QStringLiteral("key"), key },
                          { QStringLiteral("label"), label } } };
}

} // namespace

class TestNotificationCenter : public QObject
{
    Q_OBJECT

private slots:
    void history_normalizes_to_stable_keys_and_active_rows();
    void duplicate_ids_keep_the_newest_row();
    void history_groups_contiguous_rows_by_app();
    void selection_follows_the_key_across_updates();
    void empty_state_is_safe();
    void keyboard_traversal_clamps_at_the_ends();
    void active_rows_route_actions_and_history_rows_never_do();
    void suppressed_history_has_no_actions();
    void service_absence_clears_and_reconnect_repopulates();
    void close_reports_one_reason_and_teardown_clears();
    void every_close_reason_is_reported_verbatim();
    void mock_service_round_trip_updates_the_center();
};

void TestNotificationCenter::history_normalizes_to_stable_keys_and_active_rows()
{
    const QVariantList raw{
        historyEntry(2, QStringLiteral("active"), QString(), 0, false,
                     action(QStringLiteral("reply"), QStringLiteral("Reply"))),
        historyEntry(3, QStringLiteral("quiet"), QString(), 0, true, {}),
        historyEntry(1, QStringLiteral("gone"), QStringLiteral("expired"), 99, false,
                     action(QStringLiteral("open"), QStringLiteral("Open"))),
    };
    const QVariantList rows = NotificationCenter::normalizeHistory(raw);
    QCOMPARE(rows.size(), 3);
    QCOMPARE(rows.at(0).toMap().value(QStringLiteral("key")).toString(),
             QStringLiteral("2"));
    QCOMPARE(rows.at(0).toMap().value(QStringLiteral("active")).toBool(), true);
    QCOMPARE(rows.at(0).toMap().value(QStringLiteral("canAct")).toBool(), true);
    QCOMPARE(rows.at(0).toMap().value(QStringLiteral("status")).toString(),
             QStringLiteral("active"));
    QCOMPARE(rows.at(0).toMap().value(QStringLiteral("actions")).toList().size(), 1);

    QCOMPARE(rows.at(1).toMap().value(QStringLiteral("active")).toBool(), false);
    QCOMPARE(rows.at(1).toMap().value(QStringLiteral("status")).toString(),
             QStringLiteral("suppressed"));
    QVERIFY(rows.at(1).toMap().value(QStringLiteral("actions")).toList().isEmpty());

    QCOMPARE(rows.at(2).toMap().value(QStringLiteral("active")).toBool(), false);
    QCOMPARE(rows.at(2).toMap().value(QStringLiteral("status")).toString(),
             QStringLiteral("expired"));
    QVERIFY(rows.at(2).toMap().value(QStringLiteral("actions")).toList().isEmpty());
}

void TestNotificationCenter::duplicate_ids_keep_the_newest_row()
{
    const QVariantList raw{
        historyEntry(7, QStringLiteral("newest"), QString(), 0, false, {}),
        historyEntry(7, QStringLiteral("replayed"), QStringLiteral("closed"), 5, false, {}),
    };
    const QVariantList rows = NotificationCenter::normalizeHistory(raw);
    QCOMPARE(rows.size(), 1);
    QCOMPARE(rows.first().toMap().value(QStringLiteral("summary")).toString(),
             QStringLiteral("newest"));
}

void TestNotificationCenter::history_groups_contiguous_rows_by_app()
{
    QVariantMap first = historyEntry(3, QStringLiteral("one"), QString(), 0, false, {});
    first.insert(QStringLiteral("appName"), QStringLiteral("Mail"));
    QVariantMap second = historyEntry(2, QStringLiteral("two"), QString(), 0, false, {});
    second.insert(QStringLiteral("appName"), QStringLiteral("mail"));
    QVariantMap third = historyEntry(1, QStringLiteral("three"), QString(), 0, false, {});
    third.insert(QStringLiteral("appName"), QStringLiteral("Chat"));

    const QVariantList rows = NotificationCenter::normalizeHistory({ first, second, third });
    QCOMPARE(rows.size(), 3);

    // The first two rows are one contiguous Mail group (case-insensitive);
    // the header lands on the run's first row and carries the run length.
    QCOMPARE(rows.at(0).toMap().value(QStringLiteral("group")).toString(),
             QStringLiteral("mail"));
    QCOMPARE(rows.at(0).toMap().value(QStringLiteral("groupStart")).toBool(), true);
    QCOMPARE(rows.at(0).toMap().value(QStringLiteral("groupCount")).toInt(), 2);
    QCOMPARE(rows.at(1).toMap().value(QStringLiteral("groupStart")).toBool(), false);
    QCOMPARE(rows.at(1).toMap().value(QStringLiteral("groupCount")).toInt(), 2);

    QCOMPARE(rows.at(2).toMap().value(QStringLiteral("group")).toString(),
             QStringLiteral("chat"));
    QCOMPARE(rows.at(2).toMap().value(QStringLiteral("groupStart")).toBool(), true);
    QCOMPARE(rows.at(2).toMap().value(QStringLiteral("groupCount")).toInt(), 1);
}

void TestNotificationCenter::selection_follows_the_key_across_updates()
{
    NotificationCenter center;
    center.setHistory({
        historyEntry(2, QStringLiteral("two"), QString(), 0, false, {}),
        historyEntry(3, QStringLiteral("three"), QString(), 0, false, {}),
    });
    center.open();
    QCOMPARE(center.selectedIndex(), 0);
    center.selectIndex(1);
    QCOMPARE(center.selectedKey(), QStringLiteral("3"));

    // A new notification is prepended; the old selection must follow id 3,
    // not stay at index 1.
    center.setHistory({
        historyEntry(4, QStringLiteral("four"), QString(), 0, false, {}),
        historyEntry(2, QStringLiteral("two"), QString(), 0, false, {}),
        historyEntry(3, QStringLiteral("three"), QString(), 0, false, {}),
    });
    QCOMPARE(center.selectedKey(), QStringLiteral("3"));
    QCOMPARE(center.selectedIndex(), 2);
}

void TestNotificationCenter::empty_state_is_safe()
{
    NotificationCenter center;
    QVERIFY(center.isEmpty());
    center.open();
    QCOMPARE(center.selectedIndex(), -1);
    QCOMPARE(center.activeCount(), 0);
    QVERIFY(!center.activateSelected());
    QVERIFY(!center.dismissSelected());
    center.moveSelection(1);
    QCOMPARE(center.selectedIndex(), -1);
    center.selectFirst();
    center.selectLast();
    QCOMPARE(center.selectedIndex(), -1);
}

void TestNotificationCenter::keyboard_traversal_clamps_at_the_ends()
{
    NotificationCenter center;
    center.setHistory({
        historyEntry(1, QStringLiteral("one"), QString(), 0, false, {}),
        historyEntry(2, QStringLiteral("two"), QString(), 0, false, {}),
        historyEntry(3, QStringLiteral("three"), QString(), 0, false, {}),
    });
    center.open();
    center.selectFirst();
    QCOMPARE(center.selectedIndex(), 0);
    center.moveSelection(-1);
    QCOMPARE(center.selectedIndex(), 0);
    center.moveSelection(1);
    QCOMPARE(center.selectedIndex(), 1);
    center.selectLast();
    QCOMPARE(center.selectedIndex(), 2);
    center.moveSelection(1);
    QCOMPARE(center.selectedIndex(), 2);
    center.selectIndex(99);
    QCOMPARE(center.selectedIndex(), 2);
}

void TestNotificationCenter::active_rows_route_actions_and_history_rows_never_do()
{
    NotificationCenter center;
    center.setHistory({
        historyEntry(2, QStringLiteral("with default"), QString(), 0, false,
                     action(QStringLiteral("default"), QStringLiteral("Open"))),
        historyEntry(3, QStringLiteral("plain active"), QString(), 0, false, {}),
        historyEntry(1, QStringLiteral("expired"), QStringLiteral("expired"), 9, false,
                     action(QStringLiteral("open"), QStringLiteral("Open"))),
        historyEntry(4, QStringLiteral("quiet"), QString(), 0, true,
                     action(QStringLiteral("open"), QStringLiteral("Open"))),
    });
    QSignalSpy invokeSpy(&center, &NotificationCenter::invokeRequested);
    QSignalSpy dismissSpy(&center, &NotificationCenter::dismissRequested);
    center.open();

    // The first active row is selected on open.
    QCOMPARE(center.selectedKey(), QStringLiteral("2"));
    QVERIFY(center.activateSelected());
    QCOMPARE(invokeSpy.count(), 1);
    QCOMPARE(invokeSpy.first().at(0).toUInt(), 2u);
    QCOMPARE(invokeSpy.first().at(1).toString(), QStringLiteral("default"));

    // An active notification without actions has nothing to invoke; Return
    // dismisses it, matching the banner's default-or-dismiss rule.
    center.selectIndex(1);
    QCOMPARE(center.selectedKey(), QStringLiteral("3"));
    QVERIFY(center.activateSelected());
    QCOMPARE(dismissSpy.count(), 1);
    QCOMPARE(dismissSpy.first().at(0).toUInt(), 3u);

    // History-only rows never emit an action or a dismissal.
    invokeSpy.clear();
    dismissSpy.clear();
    for (const QString &key : { QStringLiteral("1"), QStringLiteral("4") }) {
        for (int i = 0; i < center.history().size(); ++i) {
            if (center.history().at(i).toMap().value(QStringLiteral("key")).toString()
                == key) {
                center.selectIndex(i);
                break;
            }
        }
        QVERIFY(!center.activateSelected());
        QVERIFY(!center.dismissSelected());
        QVERIFY(!center.invoke(center.history().at(center.selectedIndex())
                                   .toMap().value(QStringLiteral("id")).toUInt(),
                               QStringLiteral("open")));
    }
    QCOMPARE(invokeSpy.count(), 0);
    QCOMPARE(dismissSpy.count(), 0);
}

void TestNotificationCenter::suppressed_history_has_no_actions()
{
    NotificationCenter center;
    center.setHistory({
        historyEntry(5, QStringLiteral("quiet"), QString(), 0, true,
                     action(QStringLiteral("open"), QStringLiteral("Open"))),
    });
    const QVariantMap row = center.history().first().toMap();
    QCOMPARE(row.value(QStringLiteral("active")).toBool(), false);
    QCOMPARE(row.value(QStringLiteral("canAct")).toBool(), false);
    QVERIFY(row.value(QStringLiteral("actions")).toList().isEmpty());
    center.open();
    QVERIFY(!center.activateSelected());
    QVERIFY(!center.dismissSelected());
}

void TestNotificationCenter::service_absence_clears_and_reconnect_repopulates()
{
    NotificationCenter center;
    center.setAvailable(true);
    center.setHistory({
        historyEntry(1, QStringLiteral("live"), QString(), 0, false, {}),
    });
    QCOMPARE(center.history().size(), 1);

    center.setAvailable(false);
    QCOMPARE(center.available(), false);
    QVERIFY(center.isEmpty());
    QCOMPARE(center.selectedIndex(), -1);
    // A late reply from the dead service must not resurrect stale rows.
    center.setHistory({
        historyEntry(1, QStringLiteral("late"), QString(), 0, false, {}),
    });
    QVERIFY(center.isEmpty());

    center.setAvailable(true);
    center.setHistory({
        historyEntry(2, QStringLiteral("reconnected"), QString(), 0, false, {}),
    });
    QCOMPARE(center.history().size(), 1);
    QCOMPARE(center.history().first().toMap().value(QStringLiteral("summary")).toString(),
             QStringLiteral("reconnected"));
}

void TestNotificationCenter::close_reports_one_reason_and_teardown_clears()
{
    NotificationCenter center;
    center.setHistory({
        historyEntry(1, QStringLiteral("live"), QString(), 0, false, {}),
    });
    QSignalSpy openSpy(&center, &NotificationCenter::openChanged);
    QSignalSpy closedSpy(&center, &NotificationCenter::closed);

    center.open();
    QCOMPARE(openSpy.count(), 1);
    QCOMPARE(openSpy.first().at(0).toBool(), true);
    center.close(QStringLiteral("lock"));
    QCOMPARE(closedSpy.count(), 1);
    QCOMPARE(closedSpy.first().at(0).toString(), QStringLiteral("lock"));
    QCOMPARE(openSpy.count(), 2);
    QCOMPARE(openSpy.last().at(0).toBool(), false);
    // A second close is a no-op.
    center.close(QStringLiteral("output"));
    QCOMPARE(closedSpy.count(), 1);

    center.clear();
    QVERIFY(center.isEmpty());
    QCOMPARE(center.selectedIndex(), -1);
}

void TestNotificationCenter::every_close_reason_is_reported_verbatim()
{
    NotificationCenter center;
    QSignalSpy closedSpy(&center, &NotificationCenter::closed);
    for (const QString &reason : { QStringLiteral("escape"), QStringLiteral("outside"),
                                   QStringLiteral("lock"), QStringLiteral("output"),
                                   QStringLiteral("competing") }) {
        center.open();
        QVERIFY(center.isOpen());
        center.close(reason);
        QVERIFY(!center.isOpen());
        QCOMPARE(closedSpy.count(), 1);
        QCOMPARE(closedSpy.takeFirst().at(0).toString(), reason);
    }
}

void TestNotificationCenter::mock_service_round_trip_updates_the_center()
{
    MockNotificationClient client(nullptr, false);
    NotificationModel model;
    NotificationCenter center;
    QObject::connect(&client, &NotificationClient::historyChanged, &model,
                     &NotificationModel::applyHistoryJson);
    QObject::connect(&model, &NotificationModel::changed, &center, [&center, &model]() {
        center.setHistory(model.history());
    });
    QObject::connect(&client, &NotificationClient::availableChanged, &center,
                     &NotificationCenter::setAvailable);
    QObject::connect(&center, &NotificationCenter::invokeRequested, &client,
                     &NotificationClient::invoke);
    QObject::connect(&center, &NotificationCenter::dismissRequested, &client,
                     &NotificationClient::dismiss);
    center.setAvailable(client.isAvailable());
    center.open();

    client.notify(QStringLiteral("Mail"), QStringLiteral("New message"),
                  QStringLiteral("From Ada"), QStringLiteral("normal"),
                  { QStringLiteral("reply") }, { QStringLiteral("Reply") });
    QCOMPARE(center.history().size(), 1);
    QCOMPARE(center.history().first().toMap().value(QStringLiteral("active")).toBool(),
             true);
    QCOMPARE(center.history().first().toMap().value(QStringLiteral("canAct")).toBool(),
             true);

    QSignalSpy invokeSpy(&center, &NotificationCenter::invokeRequested);
    QVERIFY(center.activateSelected());
    QCOMPARE(invokeSpy.count(), 0); // no default action; Return dismisses
    QCOMPARE(center.history().first().toMap().value(QStringLiteral("active")).toBool(),
             false);
    QVERIFY(center.history().first().toMap().value(QStringLiteral("actions"))
                .toList().isEmpty());

    // DND suppresses the banner but still records history; the Center shows
    // the row with no active action.
    client.setFocusMode(QStringLiteral("dnd"));
    client.notify(QStringLiteral("Chat"), QStringLiteral("Quiet"), QString(),
                  QStringLiteral("normal"), {}, {});
    QCOMPARE(center.history().size(), 2);
    QCOMPARE(center.history().first().toMap().value(QStringLiteral("active")).toBool(),
             false);
    QCOMPARE(center.history().first().toMap().value(QStringLiteral("status")).toString(),
             QStringLiteral("suppressed"));
}

QTEST_MAIN(TestNotificationCenter)
#include "tst_notificationcenter.moc"
