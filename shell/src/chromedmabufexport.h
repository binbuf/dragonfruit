// SPDX-License-Identifier: MIT
// EGL dmabuf export for the GPU chrome producer (T-21.1d).
//
// Wraps the EGLImage-from-GL-texture + `eglExportDMABUFImageMESA` pair behind
// one small object. The exporter does not own the GL texture: the caller keeps
// the FBO alive for as long as the exported frame is committed.
#pragma once

#include <QSize>
#include <QString>

#include "chromedmabuf.h"

#include <EGL/egl.h>
#include <EGL/eglext.h>

class ChromeDmabufExporter
{
public:
    // `display`/`context` are the EGL display and context of the current Qt
    // GL context. The required extensions and entry points are resolved here;
    // on failure `reason` is a short fallback token and `ready()` stays false.
    bool initialize(EGLDisplay display, EGLContext context,
                    QString *reason = nullptr);
    bool ready() const { return m_ready; }
    QString failureReason() const { return m_reason; }

    // Create an EGLImage from `texture`, export plane 0, and destroy the
    // image. The returned frame's fd is owned by the caller (close it with
    // `closeFrame`). Only single-plane exports are supported in T186b.
    ChromeDmabufFrame exportTexture(unsigned int texture, const QSize &size,
                                    QString *reason = nullptr);

    static void closeFrame(ChromeDmabufFrame *frame);

private:
    EGLDisplay m_display = EGL_NO_DISPLAY;
    EGLContext m_context = EGL_NO_CONTEXT;
    PFNEGLCREATEIMAGEKHRPROC m_createImage = nullptr;
    PFNEGLDESTROYIMAGEKHRPROC m_destroyImage = nullptr;
    PFNEGLEXPORTDMABUFIMAGEQUERYMESAPROC m_query = nullptr;
    PFNEGLEXPORTDMABUFIMAGEMESAPROC m_export = nullptr;
    bool m_ready = false;
    QString m_reason;
};
