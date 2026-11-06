// SPDX-License-Identifier: MIT
// Chrome shm buffer-pool state machine (T-21.1c). No Wayland, QML, or
// compositor: the pool's reuse and busy/release transitions are pure.
#include <QtTest>

#include "chromebufferpool.h"

class TestChromeBufferPool : public QObject
{
    Q_OBJECT

private slots:
    void reusesReleasedSlot();
    void neverReusesBusySlot();
    void boundedByCapacity();
    void resizeRetiresBuffersUntilReleased();
    void accumulatesDamageForEachSlot();
    void resetClearsEverything();
};

void TestChromeBufferPool::reusesReleasedSlot()
{
    ChromeBufferPool pool(3);
    QVERIFY(pool.setSize(QSize(100, 50)));
    const int first = pool.acquire();
    QCOMPARE(first, 0);
    QCOMPARE(pool.allocations(), quint64(1));
    QVERIFY(pool.release(first));
    const int again = pool.acquire();
    QCOMPARE(again, first);
    QCOMPARE(pool.allocations(), quint64(1));
    QCOMPARE(pool.reuses(), quint64(1));
}

void TestChromeBufferPool::neverReusesBusySlot()
{
    ChromeBufferPool pool(2);
    QVERIFY(pool.setSize(QSize(100, 50)));
    const int a = pool.acquire();
    const int b = pool.acquire();
    QVERIFY(a != b);
    QCOMPARE(pool.busyCount(), 2);
    QCOMPARE(pool.acquire(), -1);
    QVERIFY(pool.release(a));
    QCOMPARE(pool.acquire(), a);
    QCOMPARE(pool.busyCount(), 2);
    QVERIFY(pool.release(b));
    QVERIFY(pool.release(a));
}

void TestChromeBufferPool::boundedByCapacity()
{
    ChromeBufferPool pool(2);
    QVERIFY(pool.setSize(QSize(16, 16)));
    QCOMPARE(pool.acquire(), 0);
    QCOMPARE(pool.acquire(), 1);
    QCOMPARE(pool.acquire(), -1);
    QCOMPARE(pool.liveCount(), 2);
    QCOMPARE(pool.allocations(), quint64(2));
}

void TestChromeBufferPool::resizeRetiresBuffersUntilReleased()
{
    ChromeBufferPool pool(2);
    QVERIFY(pool.setSize(QSize(100, 50)));
    const int a = pool.acquire();
    const int b = pool.acquire();
    QVERIFY(pool.release(a));

    // The free old-size buffer is dropped immediately; the busy one stays
    // retired but the pool still has room below its bound, so a new-size
    // buffer can be allocated without waiting.
    QVERIFY(pool.setSize(QSize(200, 100)));
    const QVector<int> dropped = pool.takeRetired();
    QCOMPARE(dropped.size(), 1);
    QCOMPARE(dropped.first(), a);
    QCOMPARE(pool.liveCount(), 1);
    QVERIFY(pool.isRetired(b));
    QVERIFY(pool.isBusy(b));

    const int fresh = pool.acquire();
    QVERIFY(fresh >= 0);
    QCOMPARE(pool.liveCount(), 2);
    QCOMPARE(pool.size(), QSize(200, 100));
    QVERIFY(!pool.isRetired(fresh));
    // The bound is reached: one retired busy slot plus the new current slot.
    QCOMPARE(pool.acquire(), -1);

    // Releasing and dropping the retired slot frees room again.
    QVERIFY(pool.release(b));
    QVERIFY(pool.isRetired(b));
    QVERIFY(pool.remove(b));
    QCOMPARE(pool.liveCount(), 1);
    QVERIFY(pool.release(fresh));
    QCOMPARE(pool.acquire(), fresh);
}

void TestChromeBufferPool::resetClearsEverything()
{
    ChromeBufferPool pool(3);
    QVERIFY(pool.setSize(QSize(10, 10)));
    pool.acquire();
    pool.acquire();
    pool.reset();
    QCOMPARE(pool.liveCount(), 0);
    QCOMPARE(pool.busyCount(), 0);
    QCOMPARE(pool.allocations(), quint64(0));
    QVERIFY(pool.size().isEmpty());
    QCOMPARE(pool.acquire(), -1);
}

void TestChromeBufferPool::accumulatesDamageForEachSlot()
{
    ChromeBufferPool pool(2);
    QVERIFY(pool.setSize(QSize(100, 50)));
    const QRect firstChange(5, 6, 2, 3);
    const QRect secondChange(8, 9, 4, 5);
    const QRect thirdChange(60, 20, 5, 6);
    const int first = pool.acquire();
    QCOMPARE(pool.damageForCommit(first, firstChange), QRect(0, 0, 100, 50));
    const int second = pool.acquire();
    QCOMPARE(pool.damageForCommit(second, secondChange), QRect(0, 0, 100, 50));

    QVERIFY(pool.release(first));
    QCOMPARE(pool.acquire(), first);
    QCOMPARE(pool.damageForCommit(first, thirdChange), secondChange.united(thirdChange));
    QVERIFY(pool.release(first));
    QCOMPARE(pool.acquire(), first);
    QCOMPARE(pool.damageForCommit(first, firstChange), firstChange);

    QVERIFY(pool.release(second));
    QCOMPARE(pool.acquire(), second);
    QCOMPARE(pool.damageForCommit(second, secondChange),
             firstChange.united(secondChange).united(thirdChange));
}

QTEST_APPLESS_MAIN(TestChromeBufferPool)
#include "tst_chromebufferpool.moc"
