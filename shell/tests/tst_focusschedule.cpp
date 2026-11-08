// SPDX-License-Identifier: MIT
// The Focus schedule's pure window logic (T229a): 24-hour parsing, same-day
// and overnight windows, weekday gating, and the empty-days/all-day cases.
#include "focusschedule.h"

#include <QDateTime>
#include <QTest>

class TestFocusSchedule : public QObject
{
    Q_OBJECT

private slots:
    void parseTimeAcceptsOnlyValid24HourText();
    void aSameDayWindowRespectsItsWeekdays();
    void anOvernightWindowCrossesMidnight();
    void emptyDaysMeansEveryDay();
    void equalStartAndEndIsAllDay();
    void malformedTimesAreInactive();

private:
    static QDateTime at(int dayOfWeek, int hour, int minute)
    {
        // 2026-11-02 is a Monday.
        const QDate monday(2026, 11, 2);
        const QDate date = monday.addDays(dayOfWeek - 1);
        return QDateTime(date, QTime(hour, minute));
    }
};

void TestFocusSchedule::parseTimeAcceptsOnlyValid24HourText()
{
    int minutes = -1;
    QVERIFY(FocusSchedule::parseTime(QStringLiteral("00:00"), &minutes));
    QCOMPARE(minutes, 0);
    QVERIFY(FocusSchedule::parseTime(QStringLiteral(" 07:30 "), &minutes));
    QCOMPARE(minutes, 450);
    QVERIFY(FocusSchedule::parseTime(QStringLiteral("23:59"), &minutes));
    QCOMPARE(minutes, 1439);
    QVERIFY(!FocusSchedule::parseTime(QStringLiteral("24:00"), &minutes));
    QVERIFY(!FocusSchedule::parseTime(QStringLiteral("9:5"), &minutes));
    QVERIFY(!FocusSchedule::parseTime(QStringLiteral("noon"), &minutes));
    QVERIFY(!FocusSchedule::parseTime(QString(), &minutes));
}

void TestFocusSchedule::aSameDayWindowRespectsItsWeekdays()
{
    const QStringList days{QStringLiteral("mon"), QStringLiteral("tue")};
    // Monday 09:00–17:00.
    QVERIFY(FocusSchedule::windowActive(QStringLiteral("09:00"),
                                        QStringLiteral("17:00"), days,
                                        at(1, 9, 0)));
    QVERIFY(FocusSchedule::windowActive(QStringLiteral("09:00"),
                                        QStringLiteral("17:00"), days,
                                        at(1, 16, 59)));
    QVERIFY(!FocusSchedule::windowActive(QStringLiteral("09:00"),
                                         QStringLiteral("17:00"), days,
                                         at(1, 17, 0)));
    QVERIFY(!FocusSchedule::windowActive(QStringLiteral("09:00"),
                                         QStringLiteral("17:00"), days,
                                         at(1, 8, 59)));
    // Wednesday is not in the list.
    QVERIFY(!FocusSchedule::windowActive(QStringLiteral("09:00"),
                                         QStringLiteral("17:00"), days,
                                         at(3, 12, 0)));
}

void TestFocusSchedule::anOvernightWindowCrossesMidnight()
{
    const QStringList days{QStringLiteral("fri")};
    // Friday 22:00 -> Saturday 07:00.
    QVERIFY(FocusSchedule::windowActive(QStringLiteral("22:00"),
                                        QStringLiteral("07:00"), days,
                                        at(5, 22, 0)));
    QVERIFY(FocusSchedule::windowActive(QStringLiteral("22:00"),
                                        QStringLiteral("07:00"), days,
                                        at(5, 23, 59)));
    QVERIFY(FocusSchedule::windowActive(QStringLiteral("22:00"),
                                        QStringLiteral("07:00"), days,
                                        at(6, 0, 0)));
    QVERIFY(FocusSchedule::windowActive(QStringLiteral("22:00"),
                                        QStringLiteral("07:00"), days,
                                        at(6, 6, 59)));
    QVERIFY(!FocusSchedule::windowActive(QStringLiteral("22:00"),
                                         QStringLiteral("07:00"), days,
                                         at(6, 7, 0)));
    QVERIFY(!FocusSchedule::windowActive(QStringLiteral("22:00"),
                                         QStringLiteral("07:00"), days,
                                         at(5, 12, 0)));
    // Saturday evening is outside: the window starts on Friday only.
    QVERIFY(!FocusSchedule::windowActive(QStringLiteral("22:00"),
                                         QStringLiteral("07:00"), days,
                                         at(6, 22, 0)));
}

void TestFocusSchedule::emptyDaysMeansEveryDay()
{
    QVERIFY(FocusSchedule::windowActive(QStringLiteral("09:00"),
                                        QStringLiteral("17:00"), {},
                                        at(3, 12, 0)));
    QVERIFY(FocusSchedule::windowActive(QStringLiteral("22:00"),
                                        QStringLiteral("07:00"), {},
                                        at(7, 2, 0)));
}

void TestFocusSchedule::equalStartAndEndIsAllDay()
{
    QVERIFY(FocusSchedule::windowActive(QStringLiteral("00:00"),
                                        QStringLiteral("00:00"), {},
                                        at(2, 3, 0)));
}

void TestFocusSchedule::malformedTimesAreInactive()
{
    QVERIFY(!FocusSchedule::windowActive(QStringLiteral("25:00"),
                                         QStringLiteral("07:00"), {},
                                         at(2, 3, 0)));
    QVERIFY(!FocusSchedule::windowActive(QStringLiteral("22:00"),
                                         QStringLiteral("soon"), {},
                                         at(2, 3, 0)));
}

QTEST_MAIN(TestFocusSchedule)
#include "tst_focusschedule.moc"
