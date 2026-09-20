// SPDX-License-Identifier: MIT
// Interim persistence for the Dock's `dock.pinned` key (T-10, section 19).
//
// settingsd (T-15) owns desktop-settings persistence; until it lands the Dock
// writes the same key under `$XDG_CONFIG_HOME/dragonfruit/settings.json` in
// the eventual `org.dragonfruit.Settings1` named-key shape:
//
//   { "schema": 1, "keys": { "dock.pinned": ["org.dragonfruit.Files.desktop"] } }
//
// The file is read/merged (unknown keys and future top-level fields are
// preserved) so T-15 can adopt it without a migration. Delete this class when
// T-15's settings API is available.
#pragma once

#include <QJsonObject>
#include <QString>
#include <QStringList>

class DesktopEntryIndex;

class DockPins
{
public:
    explicit DockPins(const QString &filePath = defaultFilePath());

    static QString defaultFilePath();

    // Load the key from disk. A missing file leaves the pin set empty (the
    // caller then seeds the defaults). Returns false on a malformed file;
    // `lastError()` explains, and the in-memory set stays empty.
    bool load();
    bool save();
    bool fileExists() const;

    QStringList ids() const { return m_ids; }
    bool contains(const QString &id) const { return m_ids.contains(id); }
    bool isEmpty() const { return m_ids.isEmpty(); }
    void setIds(const QStringList &ids);

    // Returns true when the set changed.
    bool add(const QString &id);
    bool remove(const QString &id);
    bool move(int from, int to);

    // The default pin set (T-10 section 19): Files, Settings, Terminal,
    // Browser, resolved against the installed `.desktop` corpus by trying a
    // small candidate list per slot and taking the first that resolves.
    // Unresolved slots are skipped so the Dock never opens with broken
    // default tiles.
    static QStringList resolveDefaultPins(const DesktopEntryIndex &index);

    QString lastError() const { return m_error; }

private:
    QString m_filePath;
    QStringList m_ids;
    QString m_error;
};
