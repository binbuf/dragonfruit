// SPDX-License-Identifier: MIT
// The Files command-line location argument (T-10.6b). The Dock launches Files
// with `trash://` as a positional argument, which main.cpp maps onto the
// `DF_FILES_START_URI` seam; this checks the parser independently of QML.
#include <QtTest>

#include "FilesArguments.h"

class FilesArgumentsTest : public QObject
{
    Q_OBJECT

private slots:
    void picks_the_first_non_option_argument()
    {
        const QStringList args{QStringLiteral("dragonfruit-files"),
                               QStringLiteral("trash://")};
        QCOMPARE(filesLocationArgument(args), QStringLiteral("trash://"));
    }

    void skips_leading_options()
    {
        const QStringList args{QStringLiteral("dragonfruit-files"),
                               QStringLiteral("-platform"), QStringLiteral("offscreen"),
                               QStringLiteral("file:///home/tester")};
        QCOMPARE(filesLocationArgument(args), QStringLiteral("file:///home/tester"));
    }

    void empty_when_there_is_no_argument()
    {
        QCOMPARE(filesLocationArgument({QStringLiteral("dragonfruit-files")}),
                 QString());
        QCOMPARE(filesLocationArgument({QStringLiteral("dragonfruit-files"),
                                        QStringLiteral("--help")}),
                 QString());
        QCOMPARE(filesLocationArgument({}), QString());
    }
};

QTEST_APPLESS_MAIN(FilesArgumentsTest)

#include "tst_files_arguments.moc"