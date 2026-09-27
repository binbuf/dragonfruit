// SPDX-License-Identifier: MIT
// The Add Application picker's pure list model (T-14.7e, ADR 0090).
//
// The Dock never scans `.desktop` directories itself (T-14.7 retired the
// interim resolver); the picker is a consumer of the app-index corpus the
// shell already holds in `DesktopEntryIndex`, joined with `dock.pinned`. This
// header has no QML or Wayland types, so the list/filter/toggle logic is
// headless-testable.
#pragma once

#include "desktopentry.h"

#include <QList>
#include <QString>
#include <QStringList>

// One row of the picker list.
struct AppPickerRow {
    QString desktopId; // the app-index record id, e.g. "org.gnome.Calculator.desktop"
    QString name;
    QString iconPath; // themed icon file from app-index (may be empty)
    bool pinned = false;
};

// Build the picker list from the app-index corpus and the persisted pin set.
// Records that are `noDisplay` or not launchable (`DesktopEntryIndex::
// isLaunchable`) are dropped; records are de-duplicated by desktop id (first
// wins); the survivors are sorted by localized name (`localeAwareCompare`)
// with a stable desktop-id tiebreak; each is tagged `pinned`. A non-empty
// `query` keeps only rows whose name or id contains it case-insensitively
// (leading/trailing whitespace is ignored). Pure.
QList<AppPickerRow> buildAppPickerList(const QList<DesktopEntry> &entries,
                                       const QStringList &pinnedIds,
                                       const QString &query = QString());

// Toggle Dock membership: `pinnedIds` with `desktopId` appended when `pinned`
// is true or every occurrence removed otherwise. The id is never duplicated,
// so the result is the canonical `dock.pinned` value the settings writer
// persists. Pure and unit-tested.
QStringList toggleAppPickerPin(const QStringList &pinnedIds, const QString &desktopId,
                               bool pinned);