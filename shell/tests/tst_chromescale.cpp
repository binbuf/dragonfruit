// SPDX-License-Identifier: MIT
// T-21.1b: the pure chrome scale mapping.
#include <QtTest>

#include "chromescale.h"

class TestChromeScale : public QObject
{
    Q_OBJECT

private slots:
    void oneXUsesBufferScale();
    void oneAndAHalfUsesViewport();
    void twoXUsesBufferScale();
    void roundsLogicalExtentsUp();
    void mapsLogicalRectsToBuffer();
    void pointerRoundTrips();
    void sanitizesInvalidScale();
    void exactIntegerTolerance();
};

void TestChromeScale::oneXUsesBufferScale()
{
    const ChromeScale::Geometry g = ChromeScale::geometry(QSize(1920, 125), 1.0);
    QCOMPARE(g.buffer, QSize(1920, 125));
    QCOMPARE(g.bufferScale, 1);
    QVERIFY(!g.useViewport);
    QCOMPARE(g.viewportSource, QSize());
    QCOMPARE(g.viewportDestination, QSize());
}

void TestChromeScale::oneAndAHalfUsesViewport()
{
    const ChromeScale::Geometry g = ChromeScale::geometry(QSize(1920, 125), 1.5);
    QCOMPARE(g.buffer, QSize(2880, 188));
    QCOMPARE(g.bufferScale, 1);
    QVERIFY(g.useViewport);
    QCOMPARE(g.viewportSource, QSize(2880, 188));
    QCOMPARE(g.viewportDestination, QSize(1920, 125));
}

void TestChromeScale::twoXUsesBufferScale()
{
    const ChromeScale::Geometry g = ChromeScale::geometry(QSize(1920, 125), 2.0);
    QCOMPARE(g.buffer, QSize(3840, 250));
    QCOMPARE(g.bufferScale, 2);
    QVERIFY(!g.useViewport);
}

void TestChromeScale::roundsLogicalExtentsUp()
{
    // 853 * 1.5 = 1279.5 and 125 * 1.5 = 187.5, both must cover the surface.
    const ChromeScale::Geometry g = ChromeScale::geometry(QSize(853, 125), 1.5);
    QCOMPARE(g.buffer, QSize(1280, 188));
    QCOMPARE(g.viewportDestination, QSize(853, 125));
}

void TestChromeScale::mapsLogicalRectsToBuffer()
{
    // The Dock scene is offset by its popover gutter/headroom in the offscreen
    // window; the crop follows the same scale as the QML transform.
    const QRect physical = ChromeScale::bufferRect(QRect(320, 320, 1920, 125), 1.5);
    QCOMPARE(physical, QRect(480, 480, 2880, 188));

    const QRect menu = ChromeScale::bufferRect(QRect(0, 0, 853, 28), 1.5);
    QCOMPARE(menu, QRect(0, 0, 1280, 42));
}

void TestChromeScale::pointerRoundTrips()
{
    const QPointF logical(17.25, 93.75);
    for (qreal scale : {1.0, 1.5, 2.0}) {
        const QPointF physical = ChromeScale::logicalToBuffer(logical, scale);
        const QPointF back = ChromeScale::bufferToLogical(physical, scale);
        QVERIFY2(qAbs(back.x() - logical.x()) < 1e-9,
                 qPrintable(QStringLiteral("x at %1").arg(scale)));
        QVERIFY2(qAbs(back.y() - logical.y()) < 1e-9,
                 qPrintable(QStringLiteral("y at %1").arg(scale)));
    }
    QCOMPARE(ChromeScale::logicalToBuffer(QPointF(10, 20), 1.5), QPointF(15, 30));
}

void TestChromeScale::sanitizesInvalidScale()
{
    QCOMPARE(ChromeScale::sanitize(0.0), 1.0);
    QCOMPARE(ChromeScale::sanitize(-2.0), 1.0);
    QCOMPARE(ChromeScale::sanitize(qQNaN()), 1.0);
    QCOMPARE(ChromeScale::geometry(QSize(10, 10), 0.0).buffer, QSize(10, 10));
}

void TestChromeScale::exactIntegerTolerance()
{
    QVERIFY(ChromeScale::isExactInteger(1.0));
    QVERIFY(ChromeScale::isExactInteger(2.0));
    QVERIFY(ChromeScale::isExactInteger(2.0 + 1.0 / 512.0));
    QVERIFY(!ChromeScale::isExactInteger(1.5));
    QVERIFY(!ChromeScale::isExactInteger(1.25));
}

QTEST_MAIN(TestChromeScale)
#include "tst_chromescale.moc"
