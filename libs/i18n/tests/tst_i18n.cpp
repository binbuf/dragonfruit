// SPDX-License-Identifier: MIT
// T-16.7 localization tests.

#include "i18n.h"

#include <QDate>
#include <QDir>
#include <QFile>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QScopedPointer>
#include <QTemporaryDir>
#include <QTest>
#include <QUrl>

namespace {

QString writeCatalog(const QTemporaryDir &dir, const QString &name, const QString &body)
{
    const QString path = dir.filePath(name);
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text))
        return QString();
    file.write(body.toUtf8());
    file.close();
    return path;
}

QString evalQmlString(QQmlEngine &engine, const QString &qml, const QString &property)
{
    QQmlComponent component(&engine);
    component.setData(qml.toUtf8(), QUrl(QStringLiteral("qrc:/tst_i18n.qml")));
    QScopedPointer<QObject> object(component.create());
    if (!object) {
        qWarning("QML error: %s", qPrintable(component.errorString()));
        return QString();
    }
    return object->property(property.toUtf8().constData()).toString();
}

const char *const kSimpleCatalog = R"(<?xml version="1.0" encoding="utf-8"?>
<TS version="2.1" language="es_ES">
<context>
    <name>Widget</name>
    <message><source>On</source><translation>Activado</translation></message>
    <message><source>Off</source><translation>Apagado</translation></message>
</context>
</TS>
)";

} // namespace

class TestI18n : public QObject
{
    Q_OBJECT

private slots:
    void parses_catalog_and_translates();
    void unfinished_and_empty_are_skipped();
    void malformed_catalog_is_rejected_and_keeps_previous();
    void loads_the_in_tree_spanish_catalog();
    void resolve_locale_honors_the_override();
    void install_translates_qsTr_and_sets_the_default_locale();
    void number_and_date_format_follow_the_installed_locale();
};

void TestI18n::parses_catalog_and_translates()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = writeCatalog(dir, QStringLiteral("dragonfruit_es.ts"),
                                      QString::fromUtf8(kSimpleCatalog));
    QVERIFY(!path.isEmpty());

    Dragonfruit::CatalogTranslator translator;
    QVERIFY(translator.loadCatalog(path));
    QCOMPARE(translator.messageCount(), 2);
    QCOMPARE(translator.loadedPath(), path);

    // Context-aware hit.
    QCOMPARE(translator.translate("Widget", "On"), QStringLiteral("Activado"));
    // Source-only fallback (context does not match).
    QCOMPARE(translator.translate("Other", "Off"), QStringLiteral("Apagado"));
    // Unknown source falls through empty so Qt keeps the source text.
    QCOMPARE(translator.translate("Widget", "Missing"), QString());
}

void TestI18n::unfinished_and_empty_are_skipped()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString body = QStringLiteral(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n"
        "<TS version=\"2.1\" language=\"es_ES\">\n"
        "<context><name>Widget</name>\n"
        "  <message><source>Done</source>"
        "<translation type=\"unfinished\"></translation></message>\n"
        "  <message><source>Blank</source><translation></translation></message>\n"
        "  <message><source>Saved</source><translation>Guardado</translation></message>\n"
        "</context></TS>\n");
    const QString path = writeCatalog(dir, QStringLiteral("dragonfruit_es.ts"), body);
    QVERIFY(!path.isEmpty());

    Dragonfruit::CatalogTranslator translator;
    QVERIFY(translator.loadCatalog(path));
    QCOMPARE(translator.messageCount(), 1);
    QCOMPARE(translator.translate("Widget", "Saved"), QStringLiteral("Guardado"));
    QCOMPARE(translator.translate("Widget", "Done"), QString());
    QCOMPARE(translator.translate("Widget", "Blank"), QString());
}

void TestI18n::malformed_catalog_is_rejected_and_keeps_previous()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString good = writeCatalog(dir, QStringLiteral("dragonfruit_es.ts"),
                                      QString::fromUtf8(kSimpleCatalog));
    const QString bad = writeCatalog(dir, QStringLiteral("broken.ts"),
                                     QStringLiteral("<TS><context>"));

    Dragonfruit::CatalogTranslator translator;
    QVERIFY(translator.loadCatalog(good));
    QVERIFY(!translator.loadCatalog(bad));
    QCOMPARE(translator.translate("Widget", "On"), QStringLiteral("Activado"));
    QCOMPARE(translator.loadedPath(), good);
}

void TestI18n::loads_the_in_tree_spanish_catalog()
{
    Dragonfruit::CatalogTranslator translator;
    QVERIFY(translator.loadForLocale(QLocale(QStringLiteral("es_ES"))));
    QVERIFY(translator.loadedPath().endsWith(QStringLiteral("dragonfruit_es.ts")));
    QVERIFY(translator.messageCount() > 0);

    // A locale with no catalog falls back to the source strings.
    Dragonfruit::CatalogTranslator none;
    QVERIFY(!none.loadForLocale(QLocale(QStringLiteral("zz_ZZ"))));
}

void TestI18n::resolve_locale_honors_the_override()
{
    const QByteArray previous = qgetenv("DRAGONFRUIT_LOCALE");
    qputenv("DRAGONFRUIT_LOCALE", "es_ES.UTF-8");
    const QLocale override = Dragonfruit::resolveLocale();
    QCOMPARE(override.language(), QLocale::Spanish);
    QCOMPARE(override.territory(), QLocale::Spain);

    if (previous.isEmpty())
        qunsetenv("DRAGONFRUIT_LOCALE");
    else
        qputenv("DRAGONFRUIT_LOCALE", previous);
}

void TestI18n::install_translates_qsTr_and_sets_the_default_locale()
{
    QCoreApplication *app = QCoreApplication::instance();
    QVERIFY(app);

    Dragonfruit::CatalogTranslator *translator =
        Dragonfruit::installTranslationsFor(*app, QLocale(QStringLiteral("es_ES")));
    QVERIFY(translator);

    // The default locale is installed for date/number formatting.
    QCOMPARE(QLocale().name(), QStringLiteral("es_ES"));

    // A real QML `qsTr()` resolves through the installed catalog.
    QQmlEngine engine;
    const QString value = evalQmlString(
        engine,
        QStringLiteral("import QtQml\nQtObject { property string label: qsTr(\"Appearance\") }\n"),
        QStringLiteral("label"));
    QCOMPARE(value, QStringLiteral("Apariencia"));

    app->removeTranslator(translator);
    delete translator;
}

void TestI18n::number_and_date_format_follow_the_installed_locale()
{
    QCoreApplication *app = QCoreApplication::instance();
    QVERIFY(app);

    Dragonfruit::CatalogTranslator *translator =
        Dragonfruit::installTranslationsFor(*app, QLocale(QStringLiteral("es_ES")));
    QVERIFY(translator);

    const QLocale expected(QStringLiteral("es_ES"));
    // Spanish uses a comma as the decimal separator.
    QVERIFY(QLocale().toString(1234.5).contains(QLatin1Char(',')));
    QCOMPARE(QLocale().toString(1234.5), expected.toString(1234.5));
    QCOMPARE(QLocale().toString(QDate(2026, 9, 28), QLocale::ShortFormat),
             expected.toString(QDate(2026, 9, 28), QLocale::ShortFormat));

    app->removeTranslator(translator);
    delete translator;
}

QTEST_GUILESS_MAIN(TestI18n)
#include "tst_i18n.moc"