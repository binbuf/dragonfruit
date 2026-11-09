// SPDX-License-Identifier: MIT
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickWindow>
#include <QtTest>

#include "chromeframeproducer.h"

class TestChromeFrameProducer : public QObject
{
    Q_OBJECT
private slots:
    void hiddenSceneRendersAfterGrowingAndShrinking()
    {
        ChromeFrameProducer producer;
        QVERIFY(producer.renderControlActive());
        QVERIFY(producer.softwareSceneGraph());
        QQmlEngine engine;
        QQmlComponent component(&engine);
        component.setData("import QtQuick\nRectangle { anchors.fill: parent; color: 'red' }",
                          QUrl());
        QScopedPointer<QObject> object(component.create());
        QVERIFY2(object, qPrintable(component.errorString()));
        auto *item = qobject_cast<QQuickItem *>(object.data());
        QVERIFY(item);
        item->setParentItem(producer.window()->contentItem());

        // Ordinary Dock strip -> overlay headroom -> strip again. Every pixel
        // must follow the new scene bounds without exposing the hidden window.
        for (const QSize size : {QSize(240, 78), QSize(880, 398), QSize(240, 78)}) {
            producer.resize(size);
            const QImage image = producer.renderFrame();
            QVERIFY(!image.isNull());
            QCOMPARE(producer.window()->contentItem()->size(), QSizeF(size));
            QCOMPARE(image.pixelColor(0, 0), QColor(Qt::red));
            QCOMPARE(image.pixelColor(image.width() - 1, image.height() - 1),
                     QColor(Qt::red));
            QVERIFY(!producer.window()->isVisible());
        }
    }
};

QTEST_MAIN(TestChromeFrameProducer)
#include "tst_chromeframeproducer.moc"
