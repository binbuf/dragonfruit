// SPDX-License-Identifier: MIT
// Files shell test runner (T-10.4a/T-10.4b). Before the QML engine loads, it
// materializes a small real directory so the view tests can list something
// through the files-core bridge, and publishes it as
// `DF_FILES_VIEW_FIXTURE`. Run headless with the offscreen platform and the
// software scene graph (see CMakeLists.txt).
#include <QDir>
#include <QFile>
#include <QImage>
#include <QQmlEngine>
#include <QTemporaryDir>
#include <QtQuickTest/quicktest.h>

#include "FilesThumbnailProvider.h"

#ifndef QUICK_TEST_SOURCE_DIR
#define QUICK_TEST_SOURCE_DIR "."
#endif

// The QML engine asks for `image://files-thumbnail/…` sources; the production
// entry point registers the provider, so the test engine must too.
class FilesTestSetup : public QObject
{
    Q_OBJECT

public slots:
    void qmlEngineAvailable(QQmlEngine *engine)
    {
        engine->addImageProvider(QStringLiteral("files-thumbnail"),
                                 new FilesThumbnailProvider);
    }
};

static void writeFile(const QString &path, const QByteArray &contents)
{
    QFile file(path);
    if (file.open(QIODevice::WriteOnly))
        file.write(contents);
}

int main(int argc, char **argv)
{
    // Outlives the tests: quick_test_main runs them before returning.
    QTemporaryDir fixture(QDir::tempPath() + QStringLiteral("/df-files-t104b-XXXXXX"));
    if (!fixture.isValid())
        return 2;
    const QString root = fixture.path();
    QDir(root).mkpath(QStringLiteral("Folder"));
    QDir(root).mkpath(QStringLiteral("Nested/Nested Folder"));
    writeFile(root + QStringLiteral("/alpha.txt"), QByteArray("alpha"));
    writeFile(root + QStringLiteral("/beta.bin"), QByteArray(2048, 'x'));
    // A real (tiny) PNG so the thumbnail provider has something to decode.
    QImage gamma(16, 16, QImage::Format_ARGB32_Premultiplied);
    gamma.fill(Qt::red);
    gamma.save(root + QStringLiteral("/gamma.png"), "PNG");
    // A folder-metadata dotfile, like the one a desktop creates on a fresh
    // `~/Desktop`; hidden entries are not listed by default.
    writeFile(root + QStringLiteral("/.directory"),
              QByteArray("[Desktop Entry]\nIcon=user-desktop\n"));
    qputenv("DF_FILES_VIEW_FIXTURE", root.toUtf8());

    // A separate tree for the T-10.4c optimistic/mutation tests, so a rename
    // or a move to Trash never disturbs the read-only view fixture above.
    QTemporaryDir mutations(QDir::tempPath() + QStringLiteral("/df-files-t104c-XXXXXX"));
    if (!mutations.isValid())
        return 2;
    const QString mutationRoot = mutations.path();
    QDir(mutationRoot).mkpath(QStringLiteral("Rename"));
    QDir(mutationRoot).mkpath(QStringLiteral("Trash"));
    QDir(mutationRoot).mkpath(QStringLiteral("New"));
    QDir(mutationRoot).mkpath(QStringLiteral("Revert"));
    QDir(mutationRoot).mkpath(QStringLiteral("Empty"));
    QDir(mutationRoot).mkpath(QStringLiteral("Info"));
    writeFile(mutationRoot + QStringLiteral("/Rename/one.txt"), QByteArray("one"));
    writeFile(mutationRoot + QStringLiteral("/Trash/one.txt"), QByteArray("one"));
    writeFile(mutationRoot + QStringLiteral("/Revert/a.txt"), QByteArray("a"));
    writeFile(mutationRoot + QStringLiteral("/Revert/b.txt"), QByteArray("b"));
    writeFile(mutationRoot + QStringLiteral("/Empty/x.txt"), QByteArray("x"));
    writeFile(mutationRoot + QStringLiteral("/Empty/y.txt"), QByteArray("y"));
    writeFile(mutationRoot + QStringLiteral("/Info/item.txt"), QByteArray("item"));
    qputenv("DF_FILES_MUTATION_FIXTURE", mutationRoot.toUtf8());

    // Keep the trash backend's store inside the test temp dir, never the real
    // user trash. The Rust worker reads this at spawn.
    QTemporaryDir dataHome(QDir::tempPath() + QStringLiteral("/df-files-t104c-data-XXXXXX"));
    if (!dataHome.isValid())
        return 2;
    qputenv("XDG_DATA_HOME", dataHome.path().toUtf8());

    // The T-10.5 windowed-rendering check lists a synthetic 100k directory
    // through the same bridge. The Rust side fabricates the entries; only a
    // `/synthetic` location is gated to it, so the fixtures above stay real.
    qputenv("DF_FILES_SYNTHETIC_COUNT", "100000");
    qputenv("DF_FILES_SYNTHETIC_BATCH", "512");

    // Keep thumbnail generation inside the test temp dir, never the real
    // user cache.
    QTemporaryDir cacheHome(QDir::tempPath() + QStringLiteral("/df-files-t104b-cache-XXXXXX"));
    if (!cacheHome.isValid())
        return 2;
    qputenv("XDG_CACHE_HOME", cacheHome.path().toUtf8());

    // T206: the Files shell mirrors the desktop appearance/accessibility keys
    // onto its own Theme/Glass. The mock client keeps the headless test off
    // the session bus and serves the documented schema defaults.
    qputenv("DF_SETTINGS_FIXTURE", "1");

    FilesTestSetup setup;
    return quick_test_main_with_setup(argc, argv, "tst_files_shell",
                                      QUICK_TEST_SOURCE_DIR, &setup);
}

#include "tst_files_shell.moc"
