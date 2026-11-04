// SPDX-License-Identifier: MIT
// Pure GPU-chrome capability and format selection (T-21.1d probe).
//
// This is the dependency-free half of the GPU transport probe: it parses the
// compositor's linux-dmabuf feedback table, parses EGL extension strings, and
// chooses the compositor format/modifier pair the shell should aim for. No
// Wayland, EGL, Qt GUI, or compositor state is involved, so the rules are
// unit-tested headlessly.
#pragma once

#include <QByteArray>
#include <QString>
#include <QVector>

#include <cstdint>

namespace GpuChromeCapabilities {

// DRM FourCC values used by the chrome transport. Defining the two we need
// keeps this header free of a libdrm build dependency.
constexpr uint32_t fourcc(char a, char b, char c, char d)
{
    const auto byte = [](char value) {
        return static_cast<uint32_t>(static_cast<unsigned char>(value));
    };
    return byte(a) | (byte(b) << 8) | (byte(c) << 16) | (byte(d) << 24);
}

inline constexpr uint32_t Abgr8888 = fourcc('A', 'B', '2', '4');
inline constexpr uint32_t Argb8888 = fourcc('A', 'R', '2', '4');
inline constexpr uint32_t Xbgr8888 = fourcc('X', 'B', '2', '4');
inline constexpr uint32_t Xrgb8888 = fourcc('X', 'R', '2', '4');

// DRM_FORMAT_MOD_LINEAR and DRM_FORMAT_MOD_INVALID from drm_fourcc.h.
inline constexpr uint64_t ModifierLinear = 0;
inline constexpr uint64_t ModifierInvalid = 0x00ffffffffffffffULL;

struct FormatModifier {
    uint32_t format = 0;
    uint64_t modifier = 0;

    bool operator==(const FormatModifier &other) const
    {
        return format == other.format && modifier == other.modifier;
    }
};

struct EglCapabilities {
    bool platformWayland = false;
    bool imageBase = false;
    bool imageDmaBufExport = false;
    bool imageDmaBufImport = false;
    bool imageDmaBufImportModifiers = false;

    // The shell needs EGLImage creation plus the Mesa export entry point.
    bool exportReady() const { return imageBase && imageDmaBufExport; }
};

struct FormatSelection {
    bool valid = false;
    FormatModifier pair;
    // Empty when valid; otherwise a short fallback token (`no-format`).
    QString reason;
};

// Parse a `zwp_linux_dmabuf_feedback_v1.format_table` blob. Each record is
// 16 native-endian bytes: uint32 format, 4 bytes padding, uint64 modifier.
// A malformed/empty table yields an empty vector.
QVector<FormatModifier> parseFormatTable(const QByteArray &table);

// Parse client and display EGL extension strings (space-separated). The two
// inputs are ORed: an implementation may expose an extension on either list.
EglCapabilities parseEglExtensions(const QByteArray &clientExtensions,
                                   const QByteArray &displayExtensions);

// Choose the compositor format/modifier pair to target. Preferred formats are
// tried in order; within a format, linear is preferred, then the first
// explicit modifier, then DRM_FORMAT_MOD_INVALID (implicit modifier). This is
// a capability choice only: the actual exported modifier is queried from EGL
// in T186b and must still appear in the feedback table.
FormatSelection selectFormat(const QVector<FormatModifier> &formats,
                             const QVector<uint32_t> &preferred = {Abgr8888, Argb8888});

// Human-readable tokens for probe logs.
QString formatName(uint32_t format);
QString modifierName(uint64_t modifier);
QString describe(const FormatModifier &pair);
// `limit` > 0 caps the list with a trailing `...`; 0 means all.
QString formatList(const QVector<FormatModifier> &formats, int limit = 0);

} // namespace GpuChromeCapabilities
