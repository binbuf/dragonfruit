// SPDX-License-Identifier: MIT
//
// T207 moving-backdrop probe.
//
// A tiny override-redirect X11 panel that slides horizontally behind the
// persistent Dock. The compositor treats it as a real client surface, so the
// Dock's sampled material sees live moving content instead of a static
// wallpaper. Managed windows are clamped above the reserved Dock zone; an
// override-redirect window is the only way to put real content under it.
//
// Build: cc -O2 -o moving-panel scripts/t207-moving-panel.c -lX11
// Usage: DISPLAY=:N moving-panel [seconds] [colour]
//
// The panel starts at the bottom centre and moves between two x positions
// every 140 ms. It exits after `seconds` (default 30) or on SIGTERM/SIGINT.
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static volatile sig_atomic_t stop = 0;

static void on_signal(int signo) {
    (void)signo;
    stop = 1;
}

int main(int argc, char **argv) {
    int seconds = argc > 1 ? atoi(argv[1]) : 30;
    unsigned long colour = argc > 2 ? strtoul(argv[2], NULL, 0) : 0xffffff;
    if (seconds <= 0) seconds = 30;

    Display *display = XOpenDisplay(NULL);
    if (!display) {
        fprintf(stderr, "t207-moving-panel: cannot open DISPLAY\n");
        return 1;
    }
    int screen = DefaultScreen(display);
    int screen_w = DisplayWidth(display, screen);
    int screen_h = DisplayHeight(display, screen);
    int width = screen_w / 2;
    int height = screen_h / 8;
    if (height < 120) height = 120;
    int y = screen_h - height - 24;
    int left = screen_w / 8;
    int right = screen_w - width - screen_w / 8;
    if (right < left) right = left;

    XSetWindowAttributes attrs;
    memset(&attrs, 0, sizeof(attrs));
    attrs.override_redirect = True;
    attrs.background_pixel = colour;
    attrs.event_mask = ExposureMask;
    Window window = XCreateWindow(
        display, RootWindow(display, screen),
        left, y, (unsigned)width, (unsigned)height, 0,
        CopyFromParent, InputOutput, CopyFromParent,
        CWOverrideRedirect | CWBackPixel | CWEventMask, &attrs);
    XStoreName(display, window, "T207 moving backdrop");
    XMapWindow(display, window);
    XFlush(display);

    signal(SIGTERM, on_signal);
    signal(SIGINT, on_signal);

    int x = left;
    int direction = 1;
    int ticks = seconds * 1000 / 140;
    for (int tick = 0; tick < ticks && !stop; ++tick) {
        x += direction * 24;
        if (x <= left) { x = left; direction = 1; }
        if (x >= right) { x = right; direction = -1; }
        XMoveWindow(display, window, x, y);
        XFlush(display);
        usleep(140 * 1000);
    }
    XDestroyWindow(display, window);
    XCloseDisplay(display);
    return 0;
}
