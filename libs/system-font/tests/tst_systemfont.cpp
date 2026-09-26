// SPDX-License-Identifier: MIT
// Verify the system-font contract: the bundled faces register, the app
// default and its variants resolve to Inter (never a silent fallback), and
// installation is idempotent. Headless: offscreen platform.
#include "systemfont.h"

#include <QFontDatabase>
#include <QFontInfo>
#include <QGuiApplication>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQmlProperty>
#include <QScopedPointer>
#include <QTest>

class TestSystemFont : public QObject
{
    Q_OBJECT

private slots:
    void familyIsRegistered()
    {
        Dragonfruit::installSystemFont();
        QVERIFY2(QFontDatabase::families().contains(QStringLiteral("Inter")),
                 "the bundled Inter faces must register as the Inter family");
    }

    void applicationDefaultUsesInter()
    {
        Dragonfruit::installSystemFont();
        QCOMPARE(QFontInfo(QGuiApplication::font()).family(), QStringLiteral("Inter"));
    }

    void weightAndItalicVariantsResolveToInter()
    {
        Dragonfruit::installSystemFont();
        const int weights[] = {QFont::Normal, QFont::Medium, QFont::DemiBold, QFont::Bold};
        for (int weight : weights) {
            QFont font(QStringLiteral("Inter"), 13, weight);
            QCOMPARE(QFontInfo(font).family(), QStringLiteral("Inter"));
        }
        QFont italic(QStringLiteral("Inter"), 13);
        italic.setItalic(true);
        const QFontInfo info(italic);
        QCOMPARE(info.family(), QStringLiteral("Inter"));
        QVERIFY2(info.italic(), "the italic face must resolve, not fall back");
    }

    // The actual contract every first-party surface relies on: the QML
    // environment's application font (`Text` inherits it when no family is
    // named) is Inter.
    void qmlApplicationFontIsInter()
    {
        Dragonfruit::installSystemFont();
        QQmlEngine engine;
        QQmlComponent component(&engine);
        component.setData(R"(
            import QtQuick
            QtObject { property string family: Application.font.family }
        )", QUrl());
        QScopedPointer<QObject> probe(component.create());
        QVERIFY2(!probe.isNull(), qPrintable(component.errorString()));
        QCOMPARE(probe->property("family").toString(), QStringLiteral("Inter"));
    }

    void repeatInstallationIsANoOp()
    {
        Dragonfruit::installSystemFont();
        Dragonfruit::installSystemFont();
        QCOMPARE(QFontInfo(QGuiApplication::font()).family(), QStringLiteral("Inter"));
    }
};

QTEST_MAIN(TestSystemFont)
#include "tst_systemfont.moc"