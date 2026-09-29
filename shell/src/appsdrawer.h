// SPDX-License-Identifier: MIT
// The Applications drawer's pure list model (T-19.2).
//
// The drawer is the shell's launch surface: every launchable app the app-index
// corpus knows about, listed alphabetically, with a category filter and a
// search query. It mirrors `buildAppPickerList` (T-14.7e) but deliberately
// drops the pin semantics: the drawer starts apps, it does not manage Dock
// membership. This header has no QML or Wayland types, so the list/filter
// logic is headless-testable.
#pragma once

#include "desktopentry.h"

#include <QList>
#include <QString>
#include <QStringList>

// One tile of the drawer grid.
struct AppsDrawerRow {
    QString desktopId;      // the app-index record id, e.g. "org.example.Calc.desktop"
    QString name;           // localized display name (falls back to the id)
    QString iconPath;       // themed icon file from app-index (may be empty)
    QStringList categories; // canonical pill keys this app belongs to, in pill order
};

// The canonical category pill keys, in display order. `"all"` is the implicit
// first pill and is not a data category; the view adds it. Every other key is
// produced by `appsDrawerCategoriesFor`.
QStringList appsDrawerCategoryKeys();

// The canonical pill key for one freedesktop `Categories` entry, or an empty
// string when the entry maps to no pill. The one mapping table lives in
// `appsdrawer.cpp` (`kCategoryMap`); unknown categories and the GTK/Qt
// implementation categories map to nothing so they never grow a pill.
QString appsDrawerCategoryFor(const QString &freedesktopCategory);

// Map one freedesktop category string (possibly a semicolon-separated list)
// to the distinct canonical pill keys it belongs to, in `appsDrawerCategoryKeys`
// order. Unknown entries are dropped. Pure.
QStringList appsDrawerCategoriesFor(const QString &freedesktopCategoryList);

// The distinct categories present across `entries`, in pill order. Rows that
// are not launchable or are `noDisplay` are ignored. Pure.
QStringList appsDrawerPresentCategories(const QList<DesktopEntry> &entries);

// Build the drawer list from the app-index corpus. Records that are
// `noDisplay` or not launchable (`DesktopEntryIndex::isLaunchable`) are
// dropped; records are de-duplicated by desktop id (first wins); each surviving
// row carries its mapped category keys; the survivors are sorted by localized
// name (`localeAwareCompare`) with a stable desktop-id tiebreak. `category` is
// a canonical pillar key (`"all"` or empty means every category); a non-empty
// `query` keeps only rows whose name or id contains it case-insensitively
// (leading/trailing whitespace is ignored). Pure.
QList<AppsDrawerRow> buildAppsDrawerList(const QList<DesktopEntry> &entries,
                                         const QString &category = QString(),
                                         const QString &query = QString());