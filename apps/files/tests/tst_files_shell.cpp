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

    return quick_test_main(argc, argv, "tst_files_shell", QUICK_TEST_SOURCE_DIR);
}