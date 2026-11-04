// SPDX-License-Identifier: MIT
// Runtime GPU-chrome capability probe (T-21.1d).
//
// One probe binds `zwp_linux_dmabuf_v1` on an existing Wayland connection,
// collects its format/modifier feedback, and queries the EGL display for the
// export extension the shell would use. It never creates a wl_buffer and never
// commits anything: T186a is a capability spike, not the transport.
#pragma once

#include <QString>
#include <QVector>

#include "gpuchromecapabilities.h"

struct wl_display;

struct GpuChromeProbeResult {
    bool displayConnected = false;
    bool dmabufGlobal = false;
    uint32_t dmabufVersion = 0;
    bool feedbackReady = false;
    int trancheCount = 0;
    QVector<GpuChromeCapabilities::FormatModifier> formats;
    quint64 mainDevice = 0;
    bool hasMainDevice = false;

    bool eglDisplayReady = false;
    QString eglVendor;
    QString eglVersion;
    GpuChromeCapabilities::EglCapabilities egl;

    GpuChromeCapabilities::FormatSelection selected;
    QString reason;

    // True when the compositor advertised dmabuf and a usable format table.
    // T186a does not create a wl_buffer, so this is "the import path exists",
    // not a measured import.
    bool compositorImportAvailable() const
    {
        return dmabufGlobal && !formats.isEmpty();
    }
    // All shell-side prerequisites for the T186b export path.
    bool gpuExportReady() const
    {
        return compositorImportAvailable() && feedbackReady
            && egl.exportReady() && selected.valid;
    }
    QString verdict() const
    {
        if (gpuExportReady())
            return QStringLiteral("gpu-ready");
        return QStringLiteral("fallback:%1").arg(
            reason.isEmpty() ? QStringLiteral("unknown") : reason);
    }
    QString summary() const;
};

class GpuChromeProbe
{
public:
    // Probe `display` without taking ownership of it.
    static GpuChromeProbeResult probeDisplay(wl_display *display);
    // Connect to WAYLAND_DISPLAY (or `socketName`), probe, then disconnect.
    static GpuChromeProbeResult probe(const QString &socketName = QString());
    // One greppable pair of lines: the verdict and the advertised formats.
    static void logResult(const GpuChromeProbeResult &result,
                          const QString &label = QString());
};
