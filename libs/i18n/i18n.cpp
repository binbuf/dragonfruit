// SPDX-License-Identifier: MIT
#include "i18n.h"

#include <QByteArray>
#include <QFile>
#include <QFileInfo>
#include <QHash>
#include <QXmlStreamReader>

namespace {

constexpr QChar kContextSeparator(0x1f);

// Qt `.ts` catalogs name the language as `es_ES`; `QLocale::name()` uses the
// same separator, so a locale's exact name and its bare language are the two
// candidate files.
QStringList candidateFileNames(const QLocale &locale)
{
    const QString name = locale.name();
    QStringList names;
    names << QStringLiteral("dragonfruit_%1.ts").arg(name);
    const QString language = name.section(QLatin1Char('_'), 0, 0);
    if (language != name)
        names << QStringLiteral("dragonfruit_%1.ts").arg(language);
    return names;
}

} // namespace

namespace Dragonfruit {

CatalogTranslator::CatalogTranslator(QObject *parent)
    : QTranslator(parent)
{
}

bool CatalogTranslator::loadCatalog(const QString &path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text))
        return false;

    QXmlStreamReader xml(&file);
    if (!xml.readNextStartElement() || xml.name() != QLatin1String("TS"))
        return false;

    QHash<QString, QString> byContext;
    QHash<QString, QString> bySource;

    while (xml.readNextStartElement()) {
        if (xml.name() != QLatin1String("context")) {
            xml.skipCurrentElement();
            continue;
        }
        QString context;
        while (xml.readNextStartElement()) {
            if (xml.name() == QLatin1String("name")) {
                context = xml.readElementText();
            } else if (xml.name() == QLatin1String("message")) {
                QString source;
                QString translation;
                bool unfinished = false;
                bool hasTranslation = false;
                while (xml.readNextStartElement()) {
                    if (xml.name() == QLatin1String("source")) {
                        source = xml.readElementText();
                    } else if (xml.name() == QLatin1String("translation")) {
                        unfinished = xml.attributes().value(QLatin1String("type"))
                                     == QLatin1String("unfinished");
                        hasTranslation = true;
                        translation = xml.readElementText();
                    } else {
                        xml.skipCurrentElement();
                    }
                }
                if (!hasTranslation || unfinished || translation.isEmpty()
                    || source.isEmpty()) {
                    continue;
                }
                byContext.insert(context + kContextSeparator + source, translation);
                if (!bySource.contains(source))
                    bySource.insert(source, translation);
            } else {
                xml.skipCurrentElement();
            }
        }
    }

    if (xml.hasError() || bySource.isEmpty())
        return false;

    m_byContext = byContext;
    m_bySource = bySource;
    m_loadedPath = path;
    return true;
}

bool CatalogTranslator::loadForLocale(const QLocale &locale)
{
    const QString name = locale.name();
    if (name.isEmpty() || name == QLatin1String("C") || name == QLatin1String("POSIX"))
        return false;

    const QStringList fileNames = candidateFileNames(locale);
    const QStringList directories = catalogSearchPaths();
    for (const QString &directory : directories) {
        for (const QString &fileName : fileNames) {
            const QString path = directory + QLatin1Char('/') + fileName;
            if (QFileInfo::exists(path) && loadCatalog(path))
                return true;
        }
    }
    return false;
}

bool CatalogTranslator::isEmpty() const
{
    return m_bySource.isEmpty();
}

QString CatalogTranslator::translate(const char *context, const char *sourceText,
                                     const char * /*disambiguation*/, int /*n*/) const
{
    if (sourceText == nullptr)
        return QString();

    const QString source = QString::fromUtf8(sourceText);
    if (context != nullptr) {
        const auto it = m_byContext.constFind(QString::fromUtf8(context)
                                              + kContextSeparator + source);
        if (it != m_byContext.constEnd())
            return it.value();
    }
    const auto it = m_bySource.constFind(source);
    if (it != m_bySource.constEnd())
        return it.value();
    return QString();
}

QLocale resolveLocale()
{
    const QByteArray override = qgetenv("DRAGONFRUIT_LOCALE").trimmed();
    if (!override.isEmpty()) {
        const QLocale requested(QString::fromLocal8Bit(override));
        const QString name = requested.name();
        if (!name.isEmpty() && name != QLatin1String("C")
            && name != QLatin1String("POSIX")) {
            return requested;
        }
    }
    return QLocale::system();
}

QStringList catalogSearchPaths()
{
    QStringList paths;
    const QByteArray override = qgetenv("DRAGONFRUIT_TRANSLATIONS_DIR").trimmed();
    if (!override.isEmpty())
        paths << QString::fromLocal8Bit(override);
#ifdef DF_TRANSLATIONS_DIR
    paths << QStringLiteral(DF_TRANSLATIONS_DIR);
#endif
    if (QCoreApplication::instance()) {
        const QString appDir = QCoreApplication::instance()->applicationDirPath();
        paths << appDir + QStringLiteral("/../share/dragonfruit/translations");
    }
    paths << QStringLiteral("/usr/local/share/dragonfruit/translations");
    paths << QStringLiteral("/usr/share/dragonfruit/translations");
    paths.removeDuplicates();
    return paths;
}

CatalogTranslator *installTranslationsFor(QCoreApplication &app, const QLocale &locale)
{
    QLocale::setDefault(locale);

    auto *translator = new CatalogTranslator(&app);
    if (!translator->loadForLocale(locale)) {
        delete translator;
        return nullptr;
    }
    app.installTranslator(translator);
    return translator;
}

CatalogTranslator *installTranslations(QCoreApplication &app)
{
    return installTranslationsFor(app, resolveLocale());
}

} // namespace Dragonfruit