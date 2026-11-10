#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""GPU regression tests for the compositor's actual liquid-glass shader.

Run with python3 tests/glass/test_glass_shader.py. Uses a private surfaceless
EGL/GLES context, ctypes and the shader embedded in blur.rs; no shell session,
screenshots or third-party Python packages are needed. Skips explicitly when
surfaceless EGL is unavailable. Run on hardware as well as software GL: some
drivers implement mediump as FP32 and cannot reproduce the FP16 regression.
"""

import ctypes as c
import math
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
GLASS = re.search(
    r'pub const GLASS_SHADER: &str = r#"(.*?)"#;',
    (ROOT / "compositor/src/window/blur.rs").read_text(), re.S,
).group(1)

# Same texture-coordinate contract as Smithay: the vertex shader maps each
# drawn source slice into the entire texture's normalized coordinate space.
VERTEX = """#version 100
attribute vec2 position;
uniform vec4 source_rect;
varying vec2 v_coords;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_coords = source_rect.xy + (position * 0.5 + 0.5) * source_rect.zw;
}
"""


class GPU:
    def __init__(self):
        self.display = self.context = self.surface = None
        try:
            self.egl = c.CDLL("libEGL.so.1")
            self.gl = c.CDLL("libGLESv2.so.2")
        except OSError as error:
            raise unittest.SkipTest(str(error)) from error
        get_proc = self.bind(self.egl, "eglGetProcAddress", c.c_void_p, c.c_char_p)
        address = get_proc(b"eglGetPlatformDisplayEXT")
        if not address:
            raise unittest.SkipTest("EGL platform display extension unavailable")
        get_display = c.CFUNCTYPE(c.c_void_p, c.c_uint, c.c_void_p,
                                 c.POINTER(c.c_int))(address)
        self.display = get_display(0x31DD, None, None)  # EGL_PLATFORM_SURFACELESS_MESA
        major, minor = c.c_int(), c.c_int()
        if not self.e("eglInitialize", c.c_uint, c.c_void_p,
                      c.POINTER(c.c_int), c.POINTER(c.c_int))(
                          self.display, c.byref(major), c.byref(minor)):
            raise unittest.SkipTest("surfaceless EGL initialization unavailable")
        try:
            if not self.e("eglBindAPI", c.c_uint, c.c_uint)(0x30A0):
                raise unittest.SkipTest("OpenGL ES unavailable")
            config, count = c.c_void_p(), c.c_int()
            attrs = (c.c_int * 13)(0x3040, 4, 0x3033, 1, 0x3024, 8,
                                  0x3023, 8, 0x3022, 8, 0x3021, 8, 0x3038)
            choose = self.e("eglChooseConfig", c.c_uint, c.c_void_p,
                            c.POINTER(c.c_int), c.POINTER(c.c_void_p),
                            c.c_int, c.POINTER(c.c_int))
            if not choose(self.display, attrs, c.byref(config), 1, c.byref(count)) or not count.value:
                raise unittest.SkipTest("RGBA8 GLES pbuffer config unavailable")
            self.context = self.e("eglCreateContext", c.c_void_p, c.c_void_p,
                                  c.c_void_p, c.c_void_p, c.POINTER(c.c_int))(
                self.display, config, None, (c.c_int * 3)(0x3098, 2, 0x3038))
            self.surface = self.e("eglCreatePbufferSurface", c.c_void_p, c.c_void_p,
                                  c.c_void_p, c.POINTER(c.c_int))(
                self.display, config, (c.c_int * 5)(0x3057, 1, 0x3056, 1, 0x3038))
            if not self.context or not self.surface or not self.e(
                "eglMakeCurrent", c.c_uint, *([c.c_void_p] * 4)
            )(self.display, self.surface, self.surface, self.context):
                raise unittest.SkipTest("GLES context unavailable")
        except BaseException:
            self.close()
            raise
        self.programs = {}
        self.vertices = (c.c_float * 8)(-1, -1, 1, -1, -1, 1, 1, 1)
        self.vertex = self.shader(0x8B31, VERTEX)
        self.input_texture = self.texture()
        self.output_texture = self.texture()
        self.framebuffer = c.c_uint()
        self.g("glGenFramebuffers", None, c.c_int, c.POINTER(c.c_uint))(
            1, c.byref(self.framebuffer))

    @staticmethod
    def bind(library, name, result, *arguments):
        function = getattr(library, name)
        function.restype, function.argtypes = result, list(arguments)
        return function

    def e(self, name, result, *arguments):
        return self.bind(self.egl, name, result, *arguments)

    def g(self, name, result, *arguments):
        return self.bind(self.gl, name, result, *arguments)

    def shader(self, kind, text):
        shader = self.g("glCreateShader", c.c_uint, c.c_uint)(kind)
        source = c.c_char_p(text.encode())
        self.g("glShaderSource", None, c.c_uint, c.c_int,
               c.POINTER(c.c_char_p), c.POINTER(c.c_int))(shader, 1, c.byref(source), None)
        self.g("glCompileShader", None, c.c_uint)(shader)
        ok = c.c_int()
        self.g("glGetShaderiv", None, c.c_uint, c.c_uint, c.POINTER(c.c_int))(
            shader, 0x8B81, c.byref(ok))
        if not ok.value:
            log = c.create_string_buffer(16384)
            self.g("glGetShaderInfoLog", None, c.c_uint, c.c_int,
                   c.POINTER(c.c_int), c.c_char_p)(shader, len(log), None, log)
            raise AssertionError(log.value.decode())
        return shader

    def program(self, defines):
        if defines in self.programs:
            return self.programs[defines]
        fragment = self.shader(0x8B30, GLASS.replace("//_DEFINES_", defines))
        program = self.g("glCreateProgram", c.c_uint)()
        attach = self.g("glAttachShader", None, c.c_uint, c.c_uint)
        attach(program, self.vertex)
        attach(program, fragment)
        self.g("glLinkProgram", None, c.c_uint)(program)
        self.g("glDeleteShader", None, c.c_uint)(fragment)
        ok = c.c_int()
        self.g("glGetProgramiv", None, c.c_uint, c.c_uint, c.POINTER(c.c_int))(
            program, 0x8B82, c.byref(ok))
        if not ok.value:
            log = c.create_string_buffer(16384)
            self.g("glGetProgramInfoLog", None, c.c_uint, c.c_int,
                   c.POINTER(c.c_int), c.c_char_p)(program, len(log), None, log)
            raise AssertionError(log.value.decode())
        self.programs[defines] = program
        return program

    def texture(self):
        texture = c.c_uint()
        self.g("glGenTextures", None, c.c_int, c.POINTER(c.c_uint))(1, c.byref(texture))
        self.g("glBindTexture", None, c.c_uint, c.c_uint)(0x0DE1, texture.value)
        param = self.g("glTexParameteri", None, c.c_uint, c.c_uint, c.c_int)
        for key, value in ((0x2801, 0x2601), (0x2800, 0x2601),
                           (0x2802, 0x812F), (0x2803, 0x812F)):
            param(0x0DE1, key, value)
        return texture.value

    def uniform(self, name, *values):
        location = self.g("glGetUniformLocation", c.c_int, c.c_uint, c.c_char_p)(
            self.current_program, name.encode())
        self.g(f"glUniform{len(values)}f", None, c.c_int,
               *([c.c_float] * len(values)))(location, *values)

    def render(self, width, height, scale, *, rows=False, dark=False,
               peak=9.6, solid=False, phase=0, defines="", flat_band=True):
        self.current_program = self.program(defines)
        self.g("glUseProgram", None, c.c_uint)(self.current_program)
        bind_texture = self.g("glBindTexture", None, c.c_uint, c.c_uint)
        upload = self.g("glTexImage2D", None, c.c_uint, c.c_int, c.c_int,
                        c.c_int, c.c_int, c.c_int, c.c_uint, c.c_uint, c.c_void_p)
        bind_texture(0x0DE1, self.output_texture)
        upload(0x0DE1, 0, 0x1908, width, height, 0, 0x1908, 0x1401, None)
        self.g("glBindFramebuffer", None, c.c_uint, c.c_uint)(0x8D40, self.framebuffer.value)
        self.g("glFramebufferTexture2D", None, c.c_uint, c.c_uint, c.c_uint,
               c.c_uint, c.c_int)(0x8D40, 0x8CE0, 0x0DE1, self.output_texture, 0)
        assert self.g("glCheckFramebufferStatus", c.c_uint, c.c_uint)(0x8D40) == 0x8CD5
        bind_texture(0x0DE1, self.input_texture)
        # RGB vary independently along both axes, so a stretched row, clamped
        # sample or stale backdrop cannot masquerade as a correct blur sample.
        x_values = [round(127 + 90 * math.sin(2 * math.pi * (x + phase) / (32 * scale)))
                    for x in range(width)]
        y_values = [round(127 + 70 * math.sin(2 * math.pi * (y + phase) / (24 * scale)))
                    for y in range(height)]
        pixels = bytearray(width * height * 4)
        for y in range(height):
            for x in range(width):
                i = (y * width + x) * 4
                pixels[i:i + 4] = bytes((127, 127, 127, 255) if solid
                                        else (x_values[x], y_values[y], 127, 255))
        data = (c.c_ubyte * len(pixels)).from_buffer(pixels)
        upload(0x0DE1, 0, 0x1908, width, height, 0, 0x1908, 0x1401, data)
        alpha, tone = (.149, .207) if dark else (.253, .604)
        scalars = dict(alpha=1, glass_radius=27 * scale, refraction=13.6 * scale,
                       specular=.397, specular_width=scale, tint_amount=alpha,
                       lens_falloff=24 * scale, tint_scale=1, flat_band=float(flat_band),
                       flat_alpha=alpha, rim_peak=.397, rim_half_width=scale,
                       rim_tail=6 * scale, corner_peak=peak * scale,
                       corner_distance=15 * scale, corner_tail=50 * scale, tint=0)
        for name, value in scalars.items():
            self.uniform(name, value)
        self.uniform("glass_size", width, height)
        self.uniform("tint_color", tone, tone, tone)
        self.uniform("panel_uv_offset", 0, 0)
        self.uniform("panel_uv_scale", 1, 1)
        self.uniform("uv_per_px", 1 / width, 1 / height)
        sampler = self.g("glGetUniformLocation", c.c_int, c.c_uint, c.c_char_p)(
            self.current_program, b"tex")
        self.g("glUniform1i", None, c.c_int, c.c_int)(sampler, 0)
        position = self.g("glGetAttribLocation", c.c_int, c.c_uint, c.c_char_p)(
            self.current_program, b"position")
        self.g("glEnableVertexAttribArray", None, c.c_uint)(position)
        self.g("glVertexAttribPointer", None, c.c_uint, c.c_int, c.c_uint,
               c.c_uint, c.c_int, c.c_void_p)(position, 2, 0x1406, 0, 0, self.vertices)
        for y in range(height) if rows else [0]:
            h = 1 if rows else height
            self.g("glViewport", None, *([c.c_int] * 4))(0, y, width, h)
            self.uniform("source_rect", 0, y / height, 1, h / height)
            self.g("glDrawArrays", None, c.c_uint, c.c_int, c.c_int)(5, 0, 4)
        out = (c.c_ubyte * len(pixels))()
        self.g("glReadPixels", None, *([c.c_int] * 4), c.c_uint, c.c_uint,
               c.c_void_p)(0, 0, width, height, 0x1908, 0x1401, out)
        assert self.g("glGetError", c.c_uint)() == 0
        return bytes(out), pixels

    def close(self):
        if self.display:
            self.e("eglMakeCurrent", c.c_uint, *([c.c_void_p] * 4))(
                self.display, None, None, None)
            if self.surface:
                self.e("eglDestroySurface", c.c_uint, c.c_void_p, c.c_void_p)(
                    self.display, self.surface)
            if self.context:
                self.e("eglDestroyContext", c.c_uint, c.c_void_p, c.c_void_p)(
                    self.display, self.context)
            self.e("eglTerminate", c.c_uint, c.c_void_p)(self.display)
            self.display = None


class GlassShaderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.gpu = GPU()

    @classmethod
    def tearDownClass(cls):
        cls.gpu.close()

    def test_middle_transmits_backdrop_at_all_scales_and_widths(self):
        for scale in (1, 1.5, 2):
            for logical_width in (483, 1400):
                for dark in (False, True):
                    with self.subTest(scale=scale, width=logical_width, dark=dark):
                        w, h = round(logical_width * scale), round(78 * scale)
                        out, source = self.gpu.render(w, h, scale, dark=dark)
                        alpha, tone = (.149, .207) if dark else (.253, .604)
                        values = []
                        for x in range(w // 2 - 50, w // 2 + 50):
                            i = (h // 2 * w + x) * 4
                            values.append(out[i])
                            for channel in (0, 1):
                                expected = source[i + channel] * (1 - alpha) + tone * 255 * alpha
                                self.assertLessEqual(abs(out[i + channel] - expected), 2)
                        self.assertGreater(max(values) - min(values), 80)

    def test_span_partition_does_not_change_sampling(self):
        for scale in (1, 1.5, 2):
            with self.subTest(scale=scale):
                w, h = round(483 * scale), round(78 * scale)
                full, _ = self.gpu.render(w, h, scale)
                rows, _ = self.gpu.render(w, h, scale, rows=True)
                self.assertLessEqual(max(abs(a - b) for a, b in zip(full, rows)), 1)

    def test_corner_lens_is_local_and_retained(self):
        w, h, scale = 966, 156, 2
        lens, _ = self.gpu.render(w, h, scale)
        plain, _ = self.gpu.render(w, h, scale, peak=0)
        # Outside both corner fields, the lens is exactly an identity map.
        for y in range(h):
            a, b = (y * w + 200) * 4, (y * w + w - 200) * 4
            self.assertEqual(lens[a:b], plain[a:b])
        self.assertGreater(max(abs(a - b) for a, b in zip(lens, plain)), 20)

    def test_uniform_rim_peak_on_every_edge(self):
        # T235a: the reference plate has one specular rim on the whole
        # perimeter. The first covered pixel must reach the measured +0.397 on
        # 50 % grey at the top, bottom, left and right, at every output scale.
        for scale in (1, 1.5, 2):
            w, h = round(483 * scale), round(78 * scale)
            out, _ = self.gpu.render(w, h, scale, solid=True)
            alpha, tone = .253, .604
            interior = 127 * (1 - alpha) + tone * 255 * alpha

            def value(x, y):
                return out[(y * w + x) * 4]

            with self.subTest(scale=scale):
                for name, (x, y) in {
                    "top": (w // 2, 0),
                    "bottom": (w // 2, h - 1),
                    "left": (0, h // 2),
                    "right": (w - 1, h // 2),
                }.items():
                    delta = (value(x, y) - interior) / 255.0
                    self.assertLessEqual(
                        abs(delta - 0.397), 0.02,
                        f"{name} first-pixel delta {delta:.3f} at {scale}x")

    def test_rim_half_ten_percent_and_tail_match_the_measured_lut(self):
        for scale in (1, 1.5, 2):
            w, h = round(483 * scale), round(78 * scale)
            out, _ = self.gpu.render(w, h, scale, solid=True)
            alpha, tone = .253, .604
            interior = 127 * (1 - alpha) + tone * 255 * alpha
            cx = w // 2
            rows = [out[(y * w + cx) * 4] for y in range(h // 2)]
            delta = [(value - interior) / 255.0 for value in rows]
            peak = delta[0]

            def crossing(fraction):
                target = peak * fraction
                for index in range(1, len(delta)):
                    if delta[index] <= target:
                        previous = delta[index - 1]
                        span = previous - delta[index]
                        t = (previous - target) / span if span > 1e-9 else 0.0
                        # Sample centres are (index + 0.5) physical px inside.
                        return (index - 1 + t + 0.5) / scale
                return None

            with self.subTest(scale=scale):
                half = crossing(0.5)
                ten = crossing(0.1)
                self.assertIsNotNone(half)
                self.assertIsNotNone(ten)
                self.assertLessEqual(abs(half - 1.0), 0.25,
                                     f"half amplitude {half:.2f} pt")
                self.assertLessEqual(abs(ten - 2.5), 0.5,
                                     f"10 % point {ten:.2f} pt")
                # The profile is gone by the measured 6 pt tail.
                tail_row = int(6 * scale) + 1
                self.assertLess(max(delta[tail_row:]), 0.01,
                                "the rim tail must end by 6 pt")

    def test_corner_arc_rim_is_continuous(self):
        # The rim must not fade to a dark side through the corner arc. Sample
        # the first covered pixel around the top-left corner and require it to
        # stay a bright rim at every angle.
        w, h, scale = 966, 156, 2
        out, _ = self.gpu.render(w, h, scale, solid=True)
        alpha, tone = .253, .604
        interior = 127 * (1 - alpha) + tone * 255 * alpha
        radius = 27 * scale
        values = []
        for degrees in range(90, 181, 10):
            angle = math.radians(degrees)
            nx, ny = math.cos(angle), -math.sin(angle)
            x = radius + radius * nx - nx * 0.5
            y = radius - radius * math.sin(angle) - ny * 0.5
            value = out[(round(y) * w + round(x)) * 4]
            values.append((value - interior) / 255.0)
        self.assertGreaterEqual(min(values), 0.4 * 0.397,
                                f"the corner rim darkened: {values}")
        # No monotonic bright-to-dark fade: the left side is as bright as the
        # top, so the arc cannot end in a darker vertical outline.
        self.assertLessEqual(abs(values[-1] - values[0]), 0.08)

    def test_flat_edge_and_flat_side_lens_are_identity(self):
        # The corner gate is exactly zero on a flat top/bottom edge and on a
        # flat side, so the lens cannot warp the end caps.
        w, h, scale = 966, 156, 2
        lens, _ = self.gpu.render(w, h, scale)
        plain, _ = self.gpu.render(w, h, scale, peak=0)
        for name, (x, y) in {
            "top": (w // 2, 2),
            "bottom": (w // 2, h - 3),
            "left": (2, h // 2),
            "right": (w - 3, h // 2),
        }.items():
            with self.subTest(edge=name):
                a = (y * w + x) * 4
                self.assertEqual(lens[a:a + 3], plain[a:a + 3],
                                 f"{name} lens is not an identity map")

    def test_corner_lens_still_displaces_the_corner(self):
        w, h, scale = 966, 156, 2
        lens, _ = self.gpu.render(w, h, scale)
        plain, _ = self.gpu.render(w, h, scale, peak=0)
        # 15 pt inside the rim on the 45-degree diagonal (top-left corner).
        radius = 27 * scale
        inside = 15 * scale
        point = radius - radius / math.sqrt(2) + inside / math.sqrt(2)
        x = y = round(point)
        a = (y * w + x) * 4
        self.assertGreater(
            max(abs(lens[a + c] - plain[a + c]) for c in range(3)), 20,
            "the corner lens must still magnify the corner")

    def test_horizontal_rim_has_consistent_normal(self):
        out, _ = self.gpu.render(966, 156, 2, solid=True)
        values = [out[x * 4] for x in range(100, 866)]
        self.assertLessEqual(max(values) - min(values), 1)

    def test_changed_backdrop_updates_middle(self):
        before, _ = self.gpu.render(725, 117, 1.5)
        after, _ = self.gpu.render(725, 117, 1.5, phase=12)
        a, b = (58 * 725 + 300) * 4, (58 * 725 + 425) * 4
        self.assertGreater(sum(abs(x - y) for x, y in zip(before[a:b:4], after[a:b:4])) / 125, 30)

    def test_alpha_and_debug_program_variants(self):
        baseline, _ = self.gpu.render(725, 117, 1.5)
        for defines in ("#define NO_ALPHA", "#define DEBUG_FLAGS",
                        "#define NO_ALPHA\n#define DEBUG_FLAGS"):
            with self.subTest(defines=defines):
                out, _ = self.gpu.render(725, 117, 1.5, defines=defines)
                self.assertEqual(baseline, out)

    def test_legacy_material_still_transmits_backdrop(self):
        for scale in (1, 1.5, 2):
            with self.subTest(scale=scale):
                w, h = round(483 * scale), round(78 * scale)
                out, _ = self.gpu.render(w, h, scale, flat_band=False)
                middle = [out[(h // 2 * w + x) * 4]
                          for x in range(w // 2 - 50, w // 2 + 50)]
                self.assertGreater(max(middle) - min(middle), 100)

    def test_external_texture_program_variants_compile(self):
        extensions = self.gpu.g("glGetString", c.c_char_p, c.c_uint)(0x1F03)
        if b"GL_OES_EGL_image_external" not in extensions.split():
            self.skipTest("external EGL image extension unavailable")
        for extra in ("", "\n#define NO_ALPHA", "\n#define DEBUG_FLAGS",
                      "\n#define NO_ALPHA\n#define DEBUG_FLAGS"):
            with self.subTest(defines=extra):
                self.gpu.program("#define EXTERNAL" + extra)


if __name__ == "__main__":
    unittest.main(verbosity=2)
