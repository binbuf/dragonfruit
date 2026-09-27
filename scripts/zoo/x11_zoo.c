/* SPDX-License-Identifier: MIT
 *
 * A minimal raw X11 client for the T-14.6a strange-app zoo. It is the
 * stand-in for X11 apps that cannot be installed in the zoo environment
 * (Steam, a generic X11 app): it maps a plain X11 window and lets the caller
 * pin the exact `WM_CLASS` pair and title, so the app-index identity path is
 * exercised with the real values. Build:
 *
 *   gcc -o x11_zoo x11_zoo.c $(pkg-config --cflags --libs x11)
 *
 * Usage:
 *
 *   DISPLAY=<xwayland> ./x11_zoo --title "Steam" --instance steam --class Steam
 */
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char **argv) {
    const char *title = "X11 Zoo";
    const char *instance = "x11zoo";
    const char *klass = "X11Zoo";
    int lifetime_ms = 120000;
    for (int i = 1; i < argc - 1; i++) {
        if (strcmp(argv[i], "--title") == 0) {
            title = argv[++i];
        } else if (strcmp(argv[i], "--instance") == 0) {
            instance = argv[++i];
        } else if (strcmp(argv[i], "--class") == 0) {
            klass = argv[++i];
        } else if (strcmp(argv[i], "--lifetime-ms") == 0) {
            lifetime_ms = atoi(argv[++i]);
        }
    }
    Display *d = XOpenDisplay(NULL);
    if (!d) {
        return 1;
    }
    int s = DefaultScreen(d);
    Window w = XCreateSimpleWindow(d, RootWindow(d, s), 40, 40, 380, 220, 1,
        BlackPixel(d, s), WhitePixel(d, s));
    XStoreName(d, w, title);
    XClassHint hint;
    hint.res_name = (char *)instance;
    hint.res_class = (char *)klass;
    XSetClassHint(d, w, &hint);
    Atom wm_delete = XInternAtom(d, "WM_DELETE_WINDOW", False);
    XSetWMProtocols(d, w, &wm_delete, 1);
    XMapWindow(d, w);
    XFlush(d);
    for (int elapsed = 0; elapsed < lifetime_ms; elapsed += 100) {
        XEvent e;
        while (XPending(d)) {
            XNextEvent(d, &e);
            if (e.type == ClientMessage) {
                XCloseDisplay(d);
                return 0;
            }
        }
        usleep(100000);
    }
    XCloseDisplay(d);
    return 0;
}