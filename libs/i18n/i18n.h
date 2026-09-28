// SPDX-License-Identifier: MIT
// Localization for the first-party stack (T-16.7).
//
// The shell and the first-party apps externalize every user-visible string
// with Qt's `qsTr()`/`tr()`. This library turns those calls into rendered
// translations: it resolves the effective locale, loads the matching catalog
// out of `translations/`, installs it on the application, and sets the default
// `QLocale` so dates, times, and numbers format for the same locale.
//
// Catalogs are Qt `.ts` XML — the same format `lupdate` emits — but are parsed
// here at runtime. The workspace deliberately does not depend on the Qt
// Linguist tools (`lupdate`/`lrelease`): the checked-in `.ts` files are the
// shipped artifact, the extraction gate keeps them in sync with the sources,
// and no binary `.qm` compile step is required. A catalog with no translation
// for a string leaves `qsTr()` returning its source text, so a partially
// translated locale is always readable.
//
// Call `installTranslations()` once, after the application object exists and
// before any QML loads.

#pragma once

#include <QCoreApplication>
#include <QLocale>
#include <QString>
#include <QStringList>
#include <QTranslator>

namespace Dragonfruit {

// A `QTranslator` that reads a Qt `.ts` catalog directly (no `lrelease`).
// Lookups are tried as (context, source) first and fall back to source text
// alone, so a catalog authored without contexts still works.
class CatalogTranslator : public QTranslator
{
    Q_OBJECT
public:
    explicit CatalogTranslator(QObject *parent = nullptr);

    // Parses `path` (a `.ts` XML file). Returns false and leaves the previous
    // catalog untouched when the file is missing or malformed.
    bool loadCatalog(const QString &path);

    // Searches `catalogSearchPaths()` for `dragonfruit_<locale>.ts` (exact
    // locale first, then its language) and loads the first hit. Returns false
    // when nothing matches, including the C/POSIX locale.
    bool loadForLocale(const QLocale &locale);

    bool isEmpty() const override;

    int messageCount() const { return m_bySource.size(); }
    QString loadedPath() const { return m_loadedPath; }

    QString translate(const char *context, const char *sourceText,
                      const char *disambiguation = nullptr, int n = -1) const override;

private:
    QHash<QString, QString> m_byContext;
    QHash<QString, QString> m_bySource;
    QString m_loadedPath;
};

// The effective locale: `DRAGONFRUIT_LOCALE` when set to a concrete locale,
// otherwise the process's system locale (which already honors LANG/LC_*).
QLocale resolveLocale();

// Directories searched for catalogs, in priority order:
// `$DRAGONFRUIT_TRANSLATIONS_DIR`, the compiled-in source tree, then the
// system install prefixes.
QStringList catalogSearchPaths();

// Installs the catalog for `locale` on `app`, sets `QLocale::setDefault`, and
// returns the installed translator (owned by `app`). Returns nullptr when no
// catalog matches, leaving the application on its source strings.
CatalogTranslator *installTranslationsFor(QCoreApplication &app, const QLocale &locale);

// `installTranslationsFor(app, resolveLocale())`.
CatalogTranslator *installTranslations(QCoreApplication &app);

} // namespace Dragonfruit