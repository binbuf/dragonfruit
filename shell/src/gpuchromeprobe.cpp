// SPDX-License-Identifier: MIT
#include "gpuchromeprobe.h"

#include <QDebug>

#include <algorithm>
#include <cstring>

#include <sys/mman.h>
#include <sys/sysmacros.h>
#include <unistd.h>

#include <wayland-client.h>

#include "linux-dmabuf-unstable-v1-client-protocol.h"

#include <EGL/egl.h>
#include <EGL/eglext.h>

namespace {

struct ProbeState {
    GpuChromeProbeResult result;
    zwp_linux_dmabuf_v1 *dmabuf = nullptr;
    zwp_linux_dmabuf_feedback_v1 *feedback = nullptr;
    uint32_t lastFormat = 0;
};

QByteArray arrayToByteArray(const wl_array *array)
{
    if (!array || !array->data || array->size == 0)
        return QByteArray();
    return QByteArray(static_cast<const char *>(array->data),
                      static_cast<int>(array->size));
}

quint64 deviceFromArray(const wl_array *array)
{
    if (!array || !array->data || array->size == 0)
        return 0;
    quint64 device = 0;
    std::memcpy(&device, array->data, qMin<size_t>(array->size, sizeof(device)));
    return device;
}

void dedupeFormats(QVector<GpuChromeCapabilities::FormatModifier> *formats)
{
    QVector<GpuChromeCapabilities::FormatModifier> unique;
    unique.reserve(formats->size());
    for (const auto &pair : *formats) {
        bool seen = false;
        for (const auto &existing : unique) {
            if (existing == pair) {
                seen = true;
                break;
            }
        }
        if (!seen)
            unique.append(pair);
    }
    *formats = unique;
}

// --- zwp_linux_dmabuf_v1 (version 3 fallback) ------------------------------

void onDmabufFormat(void *data, zwp_linux_dmabuf_v1 *, uint32_t format)
{
    auto *state = static_cast<ProbeState *>(data);
    state->lastFormat = format;
    // v3 sends modifier events after the format. Record the implicit pair too;
    // an explicit modifier later wins in selectFormat(). Deduped at the end.
    state->result.formats.append({format, GpuChromeCapabilities::ModifierInvalid});
}

void onDmabufModifier(void *data, zwp_linux_dmabuf_v1 *, uint32_t format,
                      uint32_t modifierHi, uint32_t modifierLo)
{
    auto *state = static_cast<ProbeState *>(data);
    const uint64_t modifier = (uint64_t(modifierHi) << 32) | modifierLo;
    state->result.formats.append({format, modifier});
}

const zwp_linux_dmabuf_v1_listener kDmabufListener = {
    onDmabufFormat,
    onDmabufModifier,
};

// --- zwp_linux_dmabuf_feedback_v1 -----------------------------------------

void onFeedbackDone(void *data, zwp_linux_dmabuf_feedback_v1 *)
{
    auto *state = static_cast<ProbeState *>(data);
    state->result.feedbackReady = true;
}

void onFeedbackFormatTable(void *data, zwp_linux_dmabuf_feedback_v1 *, int32_t fd,
                           uint32_t size)
{
    auto *state = static_cast<ProbeState *>(data);
    if (fd >= 0 && size > 0) {
        void *mapped = ::mmap(nullptr, size, PROT_READ, MAP_PRIVATE, fd, 0);
        if (mapped != MAP_FAILED) {
            const QByteArray table(static_cast<const char *>(mapped),
                                   static_cast<int>(size));
            state->result.formats = GpuChromeCapabilities::parseFormatTable(table);
            ::munmap(mapped, size);
        }
    }
    if (fd >= 0)
        ::close(fd);
}

void onFeedbackMainDevice(void *data, zwp_linux_dmabuf_feedback_v1 *,
                          wl_array *device)
{
    auto *state = static_cast<ProbeState *>(data);
    state->result.mainDevice = deviceFromArray(device);
    state->result.hasMainDevice = state->result.mainDevice != 0;
}

void onFeedbackTrancheDone(void *data, zwp_linux_dmabuf_feedback_v1 *)
{
    auto *state = static_cast<ProbeState *>(data);
    ++state->result.trancheCount;
}

void onFeedbackTrancheTargetDevice(void *, zwp_linux_dmabuf_feedback_v1 *,
                                   wl_array *)
{
}

void onFeedbackTrancheFormats(void *, zwp_linux_dmabuf_feedback_v1 *, wl_array *)
{
    // The format table already carries every pair; the tranche only orders
    // preference. T186b may use it to refine the choice, but the capability
    // verdict does not need the ordering.
}

void onFeedbackTrancheFlags(void *, zwp_linux_dmabuf_feedback_v1 *, uint32_t)
{
}

const zwp_linux_dmabuf_feedback_v1_listener kFeedbackListener = {
    onFeedbackDone,
    onFeedbackFormatTable,
    onFeedbackMainDevice,
    onFeedbackTrancheDone,
    onFeedbackTrancheTargetDevice,
    onFeedbackTrancheFormats,
    onFeedbackTrancheFlags,
};

// --- registry --------------------------------------------------------------

void onRegistryGlobal(void *data, wl_registry *registry, uint32_t name,
                      const char *interface, uint32_t version)
{
    auto *state = static_cast<ProbeState *>(data);
    if (std::strcmp(interface, zwp_linux_dmabuf_v1_interface.name) != 0)
        return;
    state->result.dmabufGlobal = true;
    state->result.dmabufVersion = version;
    const uint32_t bindVersion = std::min(version, 5u);
    state->dmabuf = static_cast<zwp_linux_dmabuf_v1 *>(
        wl_registry_bind(registry, name, &zwp_linux_dmabuf_v1_interface, bindVersion));
    if (!state->dmabuf)
        return;
    zwp_linux_dmabuf_v1_add_listener(state->dmabuf, &kDmabufListener, state);
    if (bindVersion >= ZWP_LINUX_DMABUF_V1_GET_DEFAULT_FEEDBACK_SINCE_VERSION) {
        state->feedback = zwp_linux_dmabuf_v1_get_default_feedback(state->dmabuf);
        if (state->feedback)
            zwp_linux_dmabuf_feedback_v1_add_listener(state->feedback,
                                                      &kFeedbackListener, state);
    }
}

void onRegistryGlobalRemove(void *, wl_registry *, uint32_t)
{
}

const wl_registry_listener kRegistryListener = {
    onRegistryGlobal,
    onRegistryGlobalRemove,
};

QByteArray eglExtensions(EGLDisplay display)
{
    const char *extensions = eglQueryString(display, EGL_EXTENSIONS);
    return extensions ? QByteArray(extensions) : QByteArray();
}

void probeEgl(ProbeState *state, wl_display *display)
{
    using GetPlatformDisplay = EGLDisplay (*)(EGLenum, void *, const EGLint *);
    auto getPlatformDisplay = reinterpret_cast<GetPlatformDisplay>(
        eglGetProcAddress("eglGetPlatformDisplay"));
    if (!getPlatformDisplay)
        getPlatformDisplay = reinterpret_cast<GetPlatformDisplay>(
            eglGetProcAddress("eglGetPlatformDisplayEXT"));
    if (!getPlatformDisplay)
        return;

    EGLDisplay eglDisplay = getPlatformDisplay(EGL_PLATFORM_WAYLAND_KHR, display, nullptr);
    if (eglDisplay == EGL_NO_DISPLAY)
        return;
    EGLint major = 0;
    EGLint minor = 0;
    if (!eglInitialize(eglDisplay, &major, &minor))
        return;
    state->result.eglDisplayReady = true;

    const char *vendor = eglQueryString(eglDisplay, EGL_VENDOR);
    const char *version = eglQueryString(eglDisplay, EGL_VERSION);
    state->result.eglVendor = vendor ? QString::fromLatin1(vendor) : QString();
    state->result.eglVersion = version ? QString::fromLatin1(version) : QString();

    const QByteArray client = eglExtensions(EGL_NO_DISPLAY);
    const QByteArray displayExtensions = eglExtensions(eglDisplay);
    state->result.egl = GpuChromeCapabilities::parseEglExtensions(client, displayExtensions);

    // Do not eglTerminate(): the EGL display can be shared with another user
    // of the same wl_display (Qt's Wayland QPA in the smoke rail), and
    // terminating it here would drop that user's display. The probe runs once
    // per process, so the reference is process-lifetime.
}

void setReason(GpuChromeProbeResult *result)
{
    if (!result->displayConnected) {
        result->reason = QStringLiteral("no-wayland-display");
    } else if (!result->dmabufGlobal) {
        result->reason = QStringLiteral("no-dmabuf");
    } else if (!result->feedbackReady) {
        result->reason = QStringLiteral("no-feedback");
    } else if (!result->eglDisplayReady) {
        result->reason = QStringLiteral("no-egl-display");
    } else if (!result->egl.exportReady()) {
        result->reason = QStringLiteral("no-egl-export");
    } else if (!result->selected.valid) {
        result->reason = result->selected.reason.isEmpty()
            ? QStringLiteral("no-format")
            : result->selected.reason;
    } else {
        result->reason.clear();
    }
}

} // namespace

QString GpuChromeProbeResult::summary() const
{
    QStringList formats = GpuChromeCapabilities::formatList(this->formats, 8).split(
        QLatin1Char(','));
    const QString formatText = formats.isEmpty() || (formats.size() == 1 && formats.first().isEmpty())
        ? QStringLiteral("none")
        : formats.join(QLatin1Char(','));
    return QStringLiteral(
               "dmabuf_global=%1 version=%2 feedback=%3 tranches=%4 formats=%5 "
               "main_device=%6 egl_display=%7 egl_vendor=%8 egl_export=%9 "
               "egl_import=%10 egl_modifiers=%11 selected=%12 "
               "compositor_import=%13 verdict=%14 reason=%15")
        .arg(dmabufGlobal ? QStringLiteral("yes") : QStringLiteral("no"))
        .arg(dmabufVersion)
        .arg(feedbackReady ? QStringLiteral("yes") : QStringLiteral("no"))
        .arg(trancheCount)
        .arg(formatText)
        .arg(hasMainDevice
                 ? QStringLiteral("%1:%2")
                       .arg(::major(static_cast<dev_t>(mainDevice)))
                       .arg(::minor(static_cast<dev_t>(mainDevice)))
                 : QStringLiteral("none"))
        .arg(eglDisplayReady ? QStringLiteral("yes") : QStringLiteral("no"))
        .arg(eglVendor.isEmpty() ? QStringLiteral("none") : eglVendor)
        .arg(egl.imageDmaBufExport ? QStringLiteral("yes") : QStringLiteral("no"))
        .arg(egl.imageDmaBufImport ? QStringLiteral("yes") : QStringLiteral("no"))
        .arg(egl.imageDmaBufImportModifiers ? QStringLiteral("yes") : QStringLiteral("no"))
        .arg(selected.valid ? GpuChromeCapabilities::describe(selected.pair)
                            : QStringLiteral("none"))
        .arg(compositorImportAvailable() ? QStringLiteral("advertised")
                                         : QStringLiteral("absent"))
        .arg(verdict())
        .arg(reason.isEmpty() ? QStringLiteral("none") : reason);
}

GpuChromeProbeResult GpuChromeProbe::probeDisplay(wl_display *display)
{
    GpuChromeProbeResult result;
    if (!display) {
        result.reason = QStringLiteral("no-wayland-display");
        return result;
    }
    result.displayConnected = true;

    ProbeState state;
    state.result = result;
    wl_registry *registry = wl_display_get_registry(display);
    if (!registry) {
        result.reason = QStringLiteral("no-registry");
        return result;
    }
    wl_registry_add_listener(registry, &kRegistryListener, &state);

    // First roundtrip: globals and the dmabuf bind. Second: the feedback
    // table and done event. A third flush is cheap and covers compositors that
    // send the table across two dispatch batches.
    if (wl_display_roundtrip(display) < 0) {
        wl_registry_destroy(registry);
        result.reason = QStringLiteral("roundtrip-failed");
        return result;
    }
    if (wl_display_roundtrip(display) < 0) {
        wl_registry_destroy(registry);
        result.reason = QStringLiteral("roundtrip-failed");
        return result;
    }
    if (state.feedback)
        wl_display_roundtrip(display);

    if (!state.result.feedbackReady && !state.result.formats.isEmpty())
        state.result.feedbackReady = true;
    dedupeFormats(&state.result.formats);
    state.result.selected =
        GpuChromeCapabilities::selectFormat(state.result.formats);
    setReason(&state.result);

    if (state.feedback)
        zwp_linux_dmabuf_feedback_v1_destroy(state.feedback);
    if (state.dmabuf)
        zwp_linux_dmabuf_v1_destroy(state.dmabuf);
    wl_registry_destroy(registry);
    wl_display_flush(display);

    result = state.result;
    if (result.displayConnected)
        probeEgl(&state, display);
    result.eglDisplayReady = state.result.eglDisplayReady;
    result.eglVendor = state.result.eglVendor;
    result.eglVersion = state.result.eglVersion;
    result.egl = state.result.egl;
    setReason(&result);
    return result;
}

GpuChromeProbeResult GpuChromeProbe::probe(const QString &socketName)
{
    const QByteArray name = socketName.toUtf8();
    wl_display *display = wl_display_connect(socketName.isEmpty() ? nullptr : name.constData());
    if (!display) {
        GpuChromeProbeResult result;
        result.reason = QStringLiteral("no-wayland-display");
        return result;
    }
    GpuChromeProbeResult result = probeDisplay(display);
    wl_display_disconnect(display);
    return result;
}

void GpuChromeProbe::logResult(const GpuChromeProbeResult &result,
                               const QString &label)
{
    const QString prefix = label.isEmpty() ? QStringLiteral("chrome gpu probe")
                                           : QStringLiteral("chrome gpu probe (%1)").arg(label);
    qInfo().noquote() << QStringLiteral("%1: %2").arg(prefix, result.summary());
    qInfo().noquote() << QStringLiteral("%1 formats: %2").arg(
        prefix,
        result.formats.isEmpty()
            ? QStringLiteral("none")
            : GpuChromeCapabilities::formatList(result.formats, 16));
}
