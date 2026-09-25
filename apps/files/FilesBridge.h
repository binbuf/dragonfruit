// SPDX-License-Identifier: MIT
// The Files app's platform location provider, exposed to QML as the `Files`
// singleton (T-10.4a).
//
// The window chrome never resolves paths itself: `Files.favorites` and
// `Files.volumes` are the real XDG user directories and mounted block
// volumes, and `Files.breadcrumb(uri)` is the one path-bar builder. The
// browsing model (history and per-location view state) lives in QML
// (`FilesBrowser`); the directory listing itself is T-10.4b's files-core
// bridge and does not cross this seam.
//
// `DF_FILES_FIXTURE` swaps in a deterministic set of locations (fixed paths,
// one volume) for headless QML tests that must not depend on the host's
// mounts, the same way `DF_SETTINGS_FIXTURE` does for Settings.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>

class FilesBridge : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Files)
    QML_SINGLETON

    // The user's home as a `file://` URI (also the root of the Favorites).
    Q_PROPERTY(QString homeUri READ homeUri CONSTANT)
    // The machine root `/` as a `file://` URI (the Computer location).
    Q_PROPERTY(QString computerUri READ computerUri CONSTANT)
    // The freedesktop home trash as the `trash://` URI.
    Q_PROPERTY(QString trashUri READ trashUri CONSTANT)
    // The account's short name, used as the Home row label.
    Q_PROPERTY(QString userName READ userName CONSTANT)
    // Favorites: `{ id, label, icon, uri }`, in display order, only
    // directories that exist. Home is always present.
    Q_PROPERTY(QVariantList favorites READ favorites CONSTANT)
    // Mounted block volumes: `{ id, label, icon, uri }`. The root volume is
    // not listed (Computer covers it); pseudo/network filesystems are
    // skipped until the GVfs volume monitor (T-10.6) owns them.
    Q_PROPERTY(QVariantList volumes READ volumes CONSTANT)
    // True when `DF_FILES_FIXTURE` selected the deterministic location set.
    Q_PROPERTY(bool fixture READ fixture CONSTANT)
    // The location a window opens on: `DF_FILES_START_URI` when set, else
    // empty (the shell falls back to Home).
    Q_PROPERTY(QString startUri READ startUri CONSTANT)
    // A real directory the headless tests point the views at, from
    // `DF_FILES_VIEW_FIXTURE`; empty unless the test runner set it.
    Q_PROPERTY(QString viewFixtureUri READ viewFixtureUri CONSTANT)
    // A writable tree for the T-10.4c optimistic-operation tests, from
    // `DF_FILES_MUTATION_FIXTURE`; empty unless the test runner set it.
    Q_PROPERTY(QString mutationFixtureUri READ mutationFixtureUri CONSTANT)
    // `DF_FILES_START_MENU`: `item` or `background` opens that context menu
    // once the listing settles. A capture seam, sibling of `DF_FILES_START_VIEW`
    // (nested synthetic pointer events cannot hold a modifier or right-click a
    // Qt client surface reliably).
    Q_PROPERTY(QString startMenu READ startMenu CONSTANT)
    // `DF_FILES_START_RENAME=1` enters inline rename on the first row once the
    // listing settles.
    Q_PROPERTY(bool startRename READ startRename CONSTANT)
    // `DF_FILES_START_SELECT=N` selects the first N rows once the listing
    // settles (multi-select capture).
    Q_PROPERTY(int startSelect READ startSelect CONSTANT)
    // `DF_FILES_START_EMPTY_TRASH=1` opens the Empty Trash confirmation once a
    // non-empty Trash listing settles (T-10.6b capture seam).
    Q_PROPERTY(bool startEmptyTrash READ startEmptyTrash CONSTANT)
    // The view a fresh window opens in, from `DF_FILES_START_VIEW` (`list`
    // or `icon`); empty means the shell default. A capture seam, sibling of
    // `DF_FILES_START_URI`.
    Q_PROPERTY(QString startView READ startView CONSTANT)

public:
    explicit FilesBridge(QObject *parent = nullptr);

    QString homeUri() const { return m_homeUri; }
    QString computerUri() const { return QStringLiteral("file:///"); }
    QString trashUri() const { return QStringLiteral("trash://"); }
    QString userName() const { return m_userName; }
    QVariantList favorites() const { return m_favorites; }
    QVariantList volumes() const { return m_volumes; }
    bool fixture() const { return m_fixture; }
    QString startUri() const { return m_startUri; }
    QString viewFixtureUri() const { return m_viewFixtureUri; }
    QString mutationFixtureUri() const { return m_mutationFixtureUri; }
    QString startView() const { return m_startView; }
    QString startMenu() const { return m_startMenu; }
    bool startRename() const { return m_startRename; }
    int startSelect() const { return m_startSelect; }
    bool startEmptyTrash() const { return m_startEmptyTrash; }

    // The path bar breadcrumb as `{ label, uri }` from the machine down to the
    // location. `file://` paths are segmented; the home directory collapses
    // into the user-name row (`Computer > <user> > Desktop`).
    Q_INVOKABLE QVariantList breadcrumb(const QString &uri) const;
    // The location's own display name (the window title). `/` is "Computer",
    // `trash://` is "Trash".
    Q_INVOKABLE QString displayName(const QString &uri) const;
    // True for a `file://` path or the `trash://` scheme (the two locations
    // this seam can open; other schemes are not advertised).
    Q_INVOKABLE bool isBrowsable(const QString &uri) const;

private:
    void buildLocations();

    bool m_fixture = false;
    QString m_homeUri;
    QString m_homePath;
    QString m_userName;
    QString m_startUri;
    QString m_viewFixtureUri;
    QString m_mutationFixtureUri;
    QString m_startView;
    QString m_startMenu;
    bool m_startRename = false;
    int m_startSelect = 0;
    bool m_startEmptyTrash = false;
    QVariantList m_favorites;
    QVariantList m_volumes;
};