// SPDX-License-Identifier: MIT
// Files shell test runner (T-10.4a/T-10.4b). Before the QML engine loads, it
// materializes a small real directory so the view tests can list something
// through the files-core bridge, and publishes it as
// `DF_FILES_VIEW_FIXTURE`. Run headless with the offscreen platform and the
// software scene graph (see CMakeLists.txt).
#include <QDir>
#include <QFile>
#include <QTemporaryDir>
#include <QtQuickTest/quicktest.h>

#ifndef QUICK_TEST_SOURCE_DIR
#define QUICK_TEST_SOURCE_DIR "."
#endif

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
    writeFile(root + QStringLiteral("/gamma.png"), QByteArray("png"));
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
    writeFile(mutationRoot + QStringLiteral("/Rename/one.txt"), QByteArray("one"));
    writeFile(mutationRoot + QStringLiteral("/Trash/one.txt"), QByteArray("one"));
    writeFile(mutationRoot + QStringLiteral("/Revert/a.txt"), QByteArray("a"));
    writeFile(mutationRoot + QStringLiteral("/Revert/b.txt"), QByteArray("b"));
    qputenv("DF_FILES_MUTATION_FIXTURE", mutationRoot.toUtf8());

    // Keep the trash backend's store inside the test temp dir, never the real
    // user trash. The Rust worker reads this at spawn.
    QTemporaryDir dataHome(QDir::tempPath() + QStringLiteral("/df-files-t104c-data-XXXXXX"));
    if (!dataHome.isValid())
        return 2;
    qputenv("XDG_DATA_HOME", dataHome.path().toUtf8());

    return quick_test_main(argc, argv, "tst_files_shell", QUICK_TEST_SOURCE_DIR);
}