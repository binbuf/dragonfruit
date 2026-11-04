// SPDX-License-Identifier: MIT
#include "chromedmabufexport.h"

#include <QByteArray>
#include <QList>
#include <QtGlobal>

#include <unistd.h>

namespace {

bool hasExtension(const char *extensions, const char *name)
{
    if (!extensions || !name)
        return false;
    const QByteArray list(extensions);
    const QList<QByteArray> tokens = list.split(' ');
    for (const QByteArray &token : tokens) {
        if (token == name)
            return true;
    }
    return false;
}

} // namespace

bool ChromeDmabufExporter::initialize(EGLDisplay display, EGLContext context,
                                      QString *reason)
{
    if (m_ready)
        return true;
    m_reason.clear();
    if (display == EGL_NO_DISPLAY) {
        m_reason = QStringLiteral("no-egl-display");
        if (reason)
            *reason = m_reason;
        return false;
    }
    m_display = display;
    m_context = context;

    const char *extensions = eglQueryString(display, EGL_EXTENSIONS);
    const char *clientExtensions = eglQueryString(EGL_NO_DISPLAY, EGL_EXTENSIONS);
    auto has = [&](const char *name) {
        return hasExtension(extensions, name) || hasExtension(clientExtensions, name);
    };
    if (!has("EGL_KHR_image_base") || !has("EGL_KHR_gl_texture_2D_image")
        || !has("EGL_MESA_image_dma_buf_export")) {
        m_reason = QStringLiteral("no-egl-export");
        if (reason)
            *reason = m_reason;
        return false;
    }

    m_createImage = reinterpret_cast<PFNEGLCREATEIMAGEKHRPROC>(
        eglGetProcAddress("eglCreateImageKHR"));
    m_destroyImage = reinterpret_cast<PFNEGLDESTROYIMAGEKHRPROC>(
        eglGetProcAddress("eglDestroyImageKHR"));
    m_query = reinterpret_cast<PFNEGLEXPORTDMABUFIMAGEQUERYMESAPROC>(
        eglGetProcAddress("eglExportDMABUFImageQueryMESA"));
    m_export = reinterpret_cast<PFNEGLEXPORTDMABUFIMAGEMESAPROC>(
        eglGetProcAddress("eglExportDMABUFImageMESA"));
    if (!m_createImage || !m_destroyImage || !m_query || !m_export) {
        m_reason = QStringLiteral("no-egl-export");
        if (reason)
            *reason = m_reason;
        return false;
    }

    m_ready = true;
    if (reason)
        reason->clear();
    return true;
}

ChromeDmabufFrame ChromeDmabufExporter::exportTexture(unsigned int texture,
                                                      const QSize &size,
                                                      QString *reason)
{
    ChromeDmabufFrame frame;
    if (!m_ready) {
        if (reason)
            *reason = m_reason.isEmpty() ? QStringLiteral("no-egl-export") : m_reason;
        return frame;
    }
    if (texture == 0 || size.isEmpty()) {
        if (reason)
            *reason = QStringLiteral("bad-texture");
        return frame;
    }

    const EGLint imageAttributes[] = {
        EGL_IMAGE_PRESERVED_KHR, EGL_TRUE,
        EGL_NONE,
    };
    EGLImageKHR image = m_createImage(
        m_display, m_context, EGL_GL_TEXTURE_2D_KHR,
        reinterpret_cast<EGLClientBuffer>(static_cast<quintptr>(texture)),
        imageAttributes);
    if (image == EGL_NO_IMAGE_KHR) {
        if (reason)
            *reason = QStringLiteral("egl-image-failed");
        return frame;
    }

    int fourcc = 0;
    int planes = 0;
    EGLuint64KHR modifiers[4] = {0, 0, 0, 0};
    if (!m_query(m_display, image, &fourcc, &planes, modifiers) || planes != 1) {
        m_destroyImage(m_display, image);
        if (reason)
            *reason = planes > 1 ? QStringLiteral("multi-plane")
                                 : QStringLiteral("export-query-failed");
        return frame;
    }

    int fds[4] = {-1, -1, -1, -1};
    EGLint strides[4] = {0, 0, 0, 0};
    EGLint offsets[4] = {0, 0, 0, 0};
    if (!m_export(m_display, image, fds, strides, offsets)) {
        m_destroyImage(m_display, image);
        if (reason)
            *reason = QStringLiteral("export-failed");
        return frame;
    }
    m_destroyImage(m_display, image);

    if (fds[0] < 0 || strides[0] <= 0) {
        for (int fd : fds) {
            if (fd >= 0)
                ::close(fd);
        }
        if (reason)
            *reason = QStringLiteral("bad-export");
        return frame;
    }
    // The protocol only carries one plane in T186b; close any extra fds the
    // driver happened to return rather than leaking them.
    for (int i = 1; i < 4; ++i) {
        if (fds[i] >= 0)
            ::close(fds[i]);
    }

    frame.fd = fds[0];
    frame.size = size;
    frame.format = static_cast<uint32_t>(fourcc);
    frame.modifier = modifiers[0];
    frame.stride = static_cast<uint32_t>(strides[0]);
    frame.offset = static_cast<uint32_t>(offsets[0]);
    frame.yInverted = true;
    if (reason)
        reason->clear();
    return frame;
}

void ChromeDmabufExporter::closeFrame(ChromeDmabufFrame *frame)
{
    if (!frame)
        return;
    if (frame->fd >= 0)
        ::close(frame->fd);
    *frame = ChromeDmabufFrame();
}
