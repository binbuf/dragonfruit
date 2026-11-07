// SPDX-License-Identifier: MIT
// T208 development-only native KMS readback.
//
// Duplicates the running compositor's DRM fd with pidfd_getfd, finds the
// active CRTC's primary plane, imports the scanout dma-buf through EGL, and
// writes a PPM. This is the T191b capture method: no screencopy, no portal
// grant, no nested window. Primary plane only (hardware cursor and overlay
// planes are not included).
//
// Build:
//   gcc -O2 -D_GNU_SOURCE -o kms-readback kms-readback.c \
//     $(pkg-config --cflags --libs libdrm egl glesv2)
//
// Usage:
//   kms-readback --info
//   kms-readback --capture out.ppm [--pid PID]

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <unistd.h>

#include <xf86drm.h>
#include <xf86drmMode.h>
#include <drm_fourcc.h>

#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES2/gl2.h>
#include <GLES2/gl2ext.h>

#ifndef __NR_pidfd_open
#define __NR_pidfd_open 434
#endif
#ifndef __NR_pidfd_getfd
#define __NR_pidfd_getfd 438
#endif

static void die(const char *what) {
    fprintf(stderr, "kms-readback: %s: %s\n", what, strerror(errno));
    exit(1);
}

static pid_t compositor_pid(void) {
    const char *env = getenv("DF_COMPOSITOR_PID");
    if (env && *env) return (pid_t)atoi(env);
    FILE *p = popen("systemctl --user show -p MainPID --value dragonfruit-compositor.service 2>/dev/null", "r");
    if (p) {
        char buf[64] = {0};
        if (fgets(buf, sizeof(buf), p)) {
            pclose(p);
            pid_t pid = (pid_t)atoi(buf);
            if (pid > 0) return pid;
        } else {
            pclose(p);
        }
    }
    p = popen("pgrep -f 'dragonfruit-compositor --backend drm' | head -1", "r");
    if (!p) return 0;
    char buf[64] = {0};
    if (!fgets(buf, sizeof(buf), p)) { pclose(p); return 0; }
    pclose(p);
    return (pid_t)atoi(buf);
}

static int drm_fd_for_pid(pid_t pid) {
    char dir[64];
    snprintf(dir, sizeof(dir), "/proc/%d/fd", pid);
    DIR *d = opendir(dir);
    if (!d) die("opendir /proc/<pid>/fd");
    int fdnum = -1;
    struct dirent *e;
    while ((e = readdir(d))) {
        if (e->d_name[0] == '.') continue;
        char link[256], target[512];
        snprintf(link, sizeof(link), "%s/%s", dir, e->d_name);
        ssize_t n = readlink(link, target, sizeof(target) - 1);
        if (n <= 0) continue;
        target[n] = 0;
        if (strncmp(target, "/dev/dri/card", 13) == 0) {
            fdnum = atoi(e->d_name);
            break;
        }
    }
    closedir(d);
    if (fdnum < 0) {
        fprintf(stderr, "kms-readback: no /dev/dri/card fd in pid %d\n", pid);
        exit(1);
    }
    int pidfd = (int)syscall(__NR_pidfd_open, pid, 0);
    if (pidfd < 0) die("pidfd_open");
    int fd = (int)syscall(__NR_pidfd_getfd, pidfd, fdnum, 0);
    close(pidfd);
    if (fd < 0) die("pidfd_getfd");
    return fd;
}

static const char *fourcc_str(uint32_t f, char out[8]) {
    out[0] = f & 0xff; out[1] = (f >> 8) & 0xff;
    out[2] = (f >> 16) & 0xff; out[3] = (f >> 24) & 0xff; out[4] = 0;
    return out;
}

static int plane_type(int fd, uint32_t plane_id) {
    drmModeObjectProperties *props =
        drmModeObjectGetProperties(fd, plane_id, DRM_MODE_OBJECT_PLANE);
    if (!props) return -1;
    int type = -1;
    for (uint32_t i = 0; i < props->count_props; i++) {
        drmModePropertyRes *p = drmModeGetProperty(fd, props->props[i]);
        if (!p) continue;
        if (strcmp(p->name, "type") == 0) type = (int)props->prop_values[i];
        drmModeFreeProperty(p);
    }
    drmModeFreeObjectProperties(props);
    return type;
}

typedef struct {
    uint32_t plane_id, crtc_id, fb_id;
    drmModeFB2 *fb;
    drmModeCrtc *crtc;
    int crtc_index;
    int type;
} Scanout;

static int find_scanout(int fd, Scanout *out) {
    memset(out, 0, sizeof(*out));
    drmModePlaneRes *pres = drmModeGetPlaneResources(fd);
    if (!pres) return -1;
    int found = 0;
    for (uint32_t i = 0; i < pres->count_planes; i++) {
        drmModePlane *p = drmModeGetPlane(fd, pres->planes[i]);
        if (!p) continue;
        if (p->fb_id == 0 || p->crtc_id == 0) { drmModeFreePlane(p); continue; }
        int type = plane_type(fd, p->plane_id);
        if (type != DRM_PLANE_TYPE_PRIMARY) { drmModeFreePlane(p); continue; }
        drmModeCrtc *crtc = drmModeGetCrtc(fd, p->crtc_id);
        if (!crtc || !crtc->mode_valid) { drmModeFreeCrtc(crtc); drmModeFreePlane(p); continue; }
        drmModeFB2 *fb = drmModeGetFB2(fd, p->fb_id);
        if (!fb) { drmModeFreeCrtc(crtc); drmModeFreePlane(p); continue; }
        out->plane_id = p->plane_id;
        out->crtc_id = p->crtc_id;
        out->fb_id = p->fb_id;
        out->fb = fb;
        out->crtc = crtc;
        out->type = type;
        found = 1;
        drmModeFreePlane(p);
        break;
    }
    drmModeFreePlaneResources(pres);
    return found ? 0 : -1;
}

static void print_info(int fd, const Scanout *s) {
    char fcc[8];
    printf("primary plane=%u crtc=%u fb=%u type=%d\n", s->plane_id, s->crtc_id,
           s->fb_id, s->type);
    printf("crtc mode=%s %ux%u@%uHz clock=%u\n", s->crtc->mode.name,
           s->crtc->mode.hdisplay, s->crtc->mode.vdisplay,
           s->crtc->mode.vrefresh, s->crtc->mode.clock);
    printf("fb %ux%u format=%s (0x%08x) flags=0x%x\n", s->fb->width, s->fb->height,
           fourcc_str(s->fb->pixel_format, fcc), s->fb->pixel_format, s->fb->flags);
    for (int i = 0; i < 4; i++) {
        if (s->fb->handles[i] == 0) continue;
        printf("plane%d handle=%u pitch=%u offset=%u modifier=0x%016llx\n", i,
               s->fb->handles[i], s->fb->pitches[i], s->fb->offsets[i],
               (unsigned long long)s->fb->modifier);
    }
    drmModeRes *res = drmModeGetResources(fd);
    if (res) {
        for (int i = 0; i < res->count_connectors; i++) {
            drmModeConnector *c = drmModeGetConnector(fd, res->connectors[i]);
            if (!c) continue;
            if (c->connection == DRM_MODE_CONNECTED && c->encoder_id) {
                printf("connector=%u-%u connected encoder=%u modes=%d\n",
                       c->connector_type, c->connector_type_id, c->encoder_id,
                       c->count_modes);
                for (int m = 0; m < c->count_modes && m < 3; m++)
                    printf("  mode %s %ux%u@%u\n", c->modes[m].name,
                           c->modes[m].hdisplay, c->modes[m].vdisplay,
                           c->modes[m].vrefresh);
            }
            drmModeFreeConnector(c);
        }
        drmModeFreeResources(res);
    }
}

static GLuint compile(GLenum type, const char *src) {
    GLuint s = glCreateShader(type);
    glShaderSource(s, 1, &src, NULL);
    glCompileShader(s);
    GLint ok = 0;
    glGetShaderiv(s, GL_COMPILE_STATUS, &ok);
    if (!ok) {
        char log[1024];
        glGetShaderInfoLog(s, sizeof(log), NULL, log);
        fprintf(stderr, "kms-readback: shader: %s\n", log);
        exit(1);
    }
    return s;
}

static int capture(int fd, const Scanout *s, const char *path) {
    char fcc[8];
    fprintf(stderr, "kms-readback: capturing fb %ux%u %s crtc %s\n", s->fb->width,
            s->fb->height, fourcc_str(s->fb->pixel_format, fcc), s->crtc->mode.name);

    int dmabuf[4] = {-1, -1, -1, -1};
    for (int i = 0; i < 4; i++) {
        if (s->fb->handles[i] == 0) continue;
        if (drmPrimeHandleToFD(fd, s->fb->handles[i], DRM_CLOEXEC | DRM_RDWR,
                               &dmabuf[i]) != 0) {
            fprintf(stderr, "kms-readback: drmPrimeHandleToFD plane%d: %s\n", i,
                    strerror(errno));
            return -1;
        }
    }

    PFNEGLGETPLATFORMDISPLAYEXTPROC get_platform_display =
        (PFNEGLGETPLATFORMDISPLAYEXTPROC)eglGetProcAddress("eglGetPlatformDisplayEXT");
    EGLDisplay dpy = EGL_NO_DISPLAY;
    if (get_platform_display) {
        dpy = get_platform_display(EGL_PLATFORM_SURFACELESS_MESA, EGL_DEFAULT_DISPLAY, NULL);
    }
    if (dpy == EGL_NO_DISPLAY) dpy = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    if (dpy == EGL_NO_DISPLAY) {
        fprintf(stderr, "kms-readback: no EGL display (0x%x)\n", eglGetError());
        return -1;
    }
    EGLint major, minor;
    if (!eglInitialize(dpy, &major, &minor)) {
        fprintf(stderr, "kms-readback: eglInitialize failed (0x%x)\n", eglGetError());
        return -1;
    }
    if (!eglBindAPI(EGL_OPENGL_ES_API)) {
        fprintf(stderr, "kms-readback: eglBindAPI failed (0x%x)\n", eglGetError());
        return -1;
    }
    EGLint cfg_attribs[] = {
        EGL_SURFACE_TYPE, EGL_PBUFFER_BIT,
        EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
        EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8, EGL_ALPHA_SIZE, 8,
        EGL_NONE};
    EGLConfig cfg;
    EGLint ncfg = 0;
    if (!eglChooseConfig(dpy, cfg_attribs, &cfg, 1, &ncfg) || ncfg < 1) {
        fprintf(stderr, "kms-readback: eglChooseConfig failed (0x%x)\n", eglGetError());
        return -1;
    }
    EGLint ctx_attribs[] = {EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE};
    EGLContext ctx = eglCreateContext(dpy, cfg, EGL_NO_CONTEXT, ctx_attribs);
    if (ctx == EGL_NO_CONTEXT) {
        fprintf(stderr, "kms-readback: eglCreateContext failed (0x%x)\n", eglGetError());
        return -1;
    }
    EGLSurface surf = EGL_NO_SURFACE;
    if (!eglMakeCurrent(dpy, EGL_NO_SURFACE, EGL_NO_SURFACE, ctx)) {
        EGLint pb[] = {EGL_WIDTH, 1, EGL_HEIGHT, 1, EGL_NONE};
        surf = eglCreatePbufferSurface(dpy, cfg, pb);
        if (surf == EGL_NO_SURFACE || !eglMakeCurrent(dpy, surf, surf, ctx)) {
            fprintf(stderr, "kms-readback: eglMakeCurrent failed (0x%x)\n", eglGetError());
            return -1;
        }
    }

    PFNEGLCREATEIMAGEKHRPROC create_image =
        (PFNEGLCREATEIMAGEKHRPROC)eglGetProcAddress("eglCreateImageKHR");
    PFNGLEGLIMAGETARGETTEXTURE2DOESPROC image_target =
        (PFNGLEGLIMAGETARGETTEXTURE2DOESPROC)eglGetProcAddress("glEGLImageTargetTexture2DOES");
    if (!create_image || !image_target) {
        fprintf(stderr, "kms-readback: EGL image entry points unavailable\n");
        return -1;
    }

    EGLint attribs[64];
    int n = 0;
    attribs[n++] = EGL_WIDTH; attribs[n++] = (EGLint)s->fb->width;
    attribs[n++] = EGL_HEIGHT; attribs[n++] = (EGLint)s->fb->height;
    attribs[n++] = EGL_LINUX_DRM_FOURCC_EXT; attribs[n++] = (EGLint)s->fb->pixel_format;
    static const EGLint plane_fd[3] = {EGL_DMA_BUF_PLANE0_FD_EXT,
                                       EGL_DMA_BUF_PLANE1_FD_EXT,
                                       EGL_DMA_BUF_PLANE2_FD_EXT};
    static const EGLint plane_off[3] = {EGL_DMA_BUF_PLANE0_OFFSET_EXT,
                                        EGL_DMA_BUF_PLANE1_OFFSET_EXT,
                                        EGL_DMA_BUF_PLANE2_OFFSET_EXT};
    static const EGLint plane_pitch[3] = {EGL_DMA_BUF_PLANE0_PITCH_EXT,
                                          EGL_DMA_BUF_PLANE1_PITCH_EXT,
                                          EGL_DMA_BUF_PLANE2_PITCH_EXT};
    static const EGLint plane_mod_lo[3] = {EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT,
                                           EGL_DMA_BUF_PLANE1_MODIFIER_LO_EXT,
                                           EGL_DMA_BUF_PLANE2_MODIFIER_LO_EXT};
    static const EGLint plane_mod_hi[3] = {EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT,
                                           EGL_DMA_BUF_PLANE1_MODIFIER_HI_EXT,
                                           EGL_DMA_BUF_PLANE2_MODIFIER_HI_EXT};
    for (int i = 0; i < 3; i++) {
        if (dmabuf[i] < 0) continue;
        attribs[n++] = plane_fd[i]; attribs[n++] = dmabuf[i];
        attribs[n++] = plane_off[i]; attribs[n++] = (EGLint)s->fb->offsets[i];
        attribs[n++] = plane_pitch[i]; attribs[n++] = (EGLint)s->fb->pitches[i];
        if (s->fb->modifier != DRM_FORMAT_MOD_INVALID) {
            attribs[n++] = plane_mod_lo[i];
            attribs[n++] = (EGLint)(s->fb->modifier & 0xffffffffu);
            attribs[n++] = plane_mod_hi[i];
            attribs[n++] = (EGLint)(s->fb->modifier >> 32);
        }
    }
    attribs[n++] = EGL_NONE;
    EGLImageKHR img = create_image(dpy, EGL_NO_CONTEXT, EGL_LINUX_DMA_BUF_EXT,
                                   (EGLClientBuffer)NULL, attribs);
    if (img == EGL_NO_IMAGE_KHR) {
        fprintf(stderr, "kms-readback: eglCreateImageKHR failed (0x%x)\n", eglGetError());
        return -1;
    }

    GLuint tex = 0;
    glGenTextures(1, &tex);
    glBindTexture(GL_TEXTURE_2D, tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    image_target(GL_TEXTURE_2D, img);
    if (glGetError() != GL_NO_ERROR) {
        fprintf(stderr, "kms-readback: glEGLImageTargetTexture2DOES failed\n");
        return -1;
    }

    GLuint fbo = 0, outtex = 0;
    glGenFramebuffers(1, &fbo);
    glBindFramebuffer(GL_FRAMEBUFFER, fbo);
    glGenTextures(1, &outtex);
    glBindTexture(GL_TEXTURE_2D, outtex);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, (GLsizei)s->fb->width,
                 (GLsizei)s->fb->height, 0, GL_RGBA, GL_UNSIGNED_BYTE, NULL);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, outtex, 0);
    if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE) {
        fprintf(stderr, "kms-readback: output framebuffer incomplete\n");
        return -1;
    }

    const char *vs_src =
        "attribute vec2 pos; varying vec2 uv;"
        "void main(){ uv = pos * 0.5 + 0.5; gl_Position = vec4(pos, 0.0, 1.0); }";
    const char *fs_src =
        "precision mediump float; varying vec2 uv; uniform sampler2D tex;"
        "void main(){ gl_FragColor = texture2D(tex, uv); }";
    GLuint vs = compile(GL_VERTEX_SHADER, vs_src);
    GLuint fs = compile(GL_FRAGMENT_SHADER, fs_src);
    GLuint prog = glCreateProgram();
    glAttachShader(prog, vs);
    glAttachShader(prog, fs);
    glBindAttribLocation(prog, 0, "pos");
    glLinkProgram(prog);
    GLint linked = 0;
    glGetProgramiv(prog, GL_LINK_STATUS, &linked);
    if (!linked) {
        char log[1024];
        glGetProgramInfoLog(prog, sizeof(log), NULL, log);
        fprintf(stderr, "kms-readback: link: %s\n", log);
        return -1;
    }
    glUseProgram(prog);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_2D, tex);
    glUniform1i(glGetUniformLocation(prog, "tex"), 0);
    static const GLfloat verts[] = {-1, -1, 1, -1, -1, 1, 1, 1};
    glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 0, verts);
    glEnableVertexAttribArray(0);
    glViewport(0, 0, (GLsizei)s->fb->width, (GLsizei)s->fb->height);
    glClearColor(0, 0, 0, 1);
    glClear(GL_COLOR_BUFFER_BIT);
    glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    glFinish();
    GLenum err = glGetError();
    if (err != GL_NO_ERROR) {
        fprintf(stderr, "kms-readback: draw error 0x%x\n", err);
        return -1;
    }

    size_t stride = (size_t)s->fb->width * 4;
    unsigned char *pixels = malloc(stride * s->fb->height);
    if (!pixels) die("malloc");
    glReadPixels(0, 0, (GLsizei)s->fb->width, (GLsizei)s->fb->height, GL_RGBA,
                 GL_UNSIGNED_BYTE, pixels);
    err = glGetError();
    if (err != GL_NO_ERROR) {
        fprintf(stderr, "kms-readback: glReadPixels error 0x%x\n", err);
        return -1;
    }

    FILE *f = fopen(path, "wb");
    if (!f) die("fopen output");
    fprintf(f, "P6\n%u %u\n255\n", s->fb->width, s->fb->height);
    for (int y = 0; y < (int)s->fb->height; y++) {
        unsigned char *row = pixels + (size_t)y * stride;
        for (uint32_t x = 0; x < s->fb->width; x++) fwrite(row + x * 4, 1, 3, f);
    }
    fclose(f);
    free(pixels);
    fprintf(stderr, "kms-readback: wrote %s\n", path);
    return 0;
}

int main(int argc, char **argv) {
    int info = 0;
    const char *out = NULL;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--info") == 0) info = 1;
        else if (strcmp(argv[i], "--capture") == 0 && i + 1 < argc) out = argv[++i];
        else if (strcmp(argv[i], "--pid") == 0 && i + 1 < argc) {
            char buf[32];
            snprintf(buf, sizeof(buf), "%s", argv[++i]);
            setenv("DF_COMPOSITOR_PID", buf, 1);
        } else {
            fprintf(stderr, "usage: %s --info | --capture out.ppm [--pid PID]\n", argv[0]);
            return 2;
        }
    }
    if (!info && !out) {
        fprintf(stderr, "usage: %s --info | --capture out.ppm [--pid PID]\n", argv[0]);
        return 2;
    }
    pid_t pid = compositor_pid();
    if (pid <= 0) {
        fprintf(stderr, "kms-readback: no compositor pid\n");
        return 1;
    }
    int fd = drm_fd_for_pid(pid);
    Scanout s;
    if (find_scanout(fd, &s) != 0) {
        fprintf(stderr, "kms-readback: no active primary plane with a framebuffer\n");
        return 1;
    }
    if (info) print_info(fd, &s);
    if (out) {
        if (capture(fd, &s, out) != 0) return 1;
    }
    drmModeFreeFB2(s.fb);
    drmModeFreeCrtc(s.crtc);
    close(fd);
    return 0;
}
