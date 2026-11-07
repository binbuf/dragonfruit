// SPDX-License-Identifier: MIT
// T208 development-only shell-protocol driver.
//
// Authenticates to the live compositor as the shell (using the reissued
// launch token), finds a toplevel by title substring, and sends fullscreen /
// unfullscreen. Used only to exercise the native fullscreen and direct-scanout
// row when the running shell has no synthetic-input rail.
//
// Build:
//   gcc -O2 -D_GNU_SOURCE -Ibuild/protocols -o shell-fullscreen shell-fullscreen.c \
//     build/protocols/dragonfruit-core-protocol.c \
//     build/protocols/dragonfruit-toplevel-protocol.c \
//     $(pkg-config --cflags --libs wayland-client)
//
// Usage: shell-fullscreen TOKEN TITLE_SUBSTRING [hold_seconds]

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include <wayland-client.h>

#include "dragonfruit-core-client-protocol.h"
#include "dragonfruit-toplevel-client-protocol.h"

static const struct df_toplevel_manager_listener manager_listener;

struct state {
    struct wl_display *display;
    struct wl_registry *registry;
    struct df_core *core;
    struct df_toplevel_manager *manager;
    uint32_t manager_name;
    int authenticated;
    int refused;
    const char *want;
    struct df_toplevel *target;
    char target_title[256];
};

static void core_authenticated(void *data, struct df_core *core, uint32_t version) {
    (void)core;
    struct state *s = data;
    s->authenticated = 1;
    if (s->manager_name) {
        s->manager = wl_registry_bind(s->registry, s->manager_name,
                                      &df_toplevel_manager_interface, 1);
        df_toplevel_manager_add_listener(s->manager, &manager_listener, s);
    }
    fprintf(stderr, "shell-fullscreen: authenticated lockstep=%u\n", version);
}

static void core_refused(void *data, struct df_core *core, uint32_t code,
                         const char *message) {
    (void)core;
    struct state *s = data;
    s->refused = 1;
    fprintf(stderr, "shell-fullscreen: refused code=%u %s\n", code, message ? message : "");
}

static const struct df_core_listener core_listener = {
    .authenticated = core_authenticated,
    .refused = core_refused,
};

static void top_title(void *data, struct df_toplevel *top, const char *title) {
    struct state *s = data;
    (void)top;
    fprintf(stderr, "shell-fullscreen: toplevel title=%s\n", title ? title : "");
    if (s->want && title && strstr(title, s->want)) {
        s->target = top;
        snprintf(s->target_title, sizeof(s->target_title), "%s", title);
    }
}

static void top_app_id(void *data, struct df_toplevel *top, const char *app_id) {
    (void)data;
    (void)top;
    fprintf(stderr, "shell-fullscreen: toplevel app_id=%s\n", app_id ? app_id : "");
}

static void top_state(void *data, struct df_toplevel *top, uint32_t state) {
    (void)data;
    (void)top;
    fprintf(stderr, "shell-fullscreen: toplevel state=0x%x\n", state);
}

static void top_workspace_entered(void *data, struct df_toplevel *top,
                                  struct df_workspace *workspace) {
    (void)data;
    (void)top;
    (void)workspace;
}
static void top_workspace_left(void *data, struct df_toplevel *top,
                               struct df_workspace *workspace) {
    (void)data;
    (void)top;
    (void)workspace;
}
static void top_output_entered(void *data, struct df_toplevel *top,
                               struct df_output *output) {
    (void)data;
    (void)top;
    (void)output;
}
static void top_output_left(void *data, struct df_toplevel *top,
                            struct df_output *output) {
    (void)data;
    (void)top;
    (void)output;
}
static void top_closed(void *data, struct df_toplevel *top) {
    struct state *s = data;
    if (s->target == top) s->target = NULL;
}
static void top_done(void *data, struct df_toplevel *top) { (void)data; (void)top; }

static const struct df_toplevel_listener toplevel_listener = {
    .title = top_title,
    .app_id = top_app_id,
    .state = top_state,
    .workspace_entered = top_workspace_entered,
    .workspace_left = top_workspace_left,
    .output_entered = top_output_entered,
    .output_left = top_output_left,
    .closed = top_closed,
    .done = top_done,
};

static void manager_output(void *data, struct df_toplevel_manager *m, struct df_output *o) {
    (void)data; (void)m; (void)o;
}
static void manager_workspace(void *data, struct df_toplevel_manager *m,
                              struct df_workspace *w) {
    (void)data; (void)m; (void)w;
}
static void manager_toplevel(void *data, struct df_toplevel_manager *m,
                             struct df_toplevel *t) {
    (void)m;
    struct state *s = data;
    df_toplevel_add_listener(t, &toplevel_listener, s);
}
static void manager_workspace_activated(void *data, struct df_toplevel_manager *m,
                                        struct df_workspace *w, uint32_t index) {
    (void)data; (void)m; (void)w; (void)index;
}
static void manager_focused(void *data, struct df_toplevel_manager *m,
                            struct df_toplevel *t) {
    (void)data; (void)m; (void)t;
}
static void manager_attention(void *data, struct df_toplevel_manager *m,
                              struct df_toplevel *t) {
    (void)data; (void)m; (void)t;
}
static void manager_hot_corner(void *data, struct df_toplevel_manager *m,
                               uint32_t corner, const char *output) {
    (void)data; (void)m; (void)corner; (void)output;
}
static void manager_overview_changed(void *data, struct df_toplevel_manager *m,
                                     uint32_t active, struct df_toplevel *selected) {
    (void)data; (void)m; (void)active; (void)selected;
}
static void manager_app_switcher(void *data, struct df_toplevel_manager *m,
                                 uint32_t active, const char *app_id, int32_t direction) {
    (void)data; (void)m; (void)active; (void)app_id; (void)direction;
}
static void manager_input_action(void *data, struct df_toplevel_manager *m,
                                 const char *action, const char *source, uint32_t serial) {
    (void)data; (void)m; (void)action; (void)source; (void)serial;
}
static void manager_progress(void *data, struct df_toplevel_manager *m,
                             const char *action, wl_fixed_t progress,
                             wl_fixed_t raw_progress, wl_fixed_t velocity,
                             uint32_t phase, uint32_t committed, uint32_t cancelled) {
    (void)data; (void)m; (void)action; (void)progress; (void)raw_progress;
    (void)velocity; (void)phase; (void)committed; (void)cancelled;
}
static void manager_app_accelerator(void *data, struct df_toplevel_manager *m,
                                    const char *app_id, const char *accelerator_id,
                                    const char *source, uint32_t serial) {
    (void)data; (void)m; (void)app_id; (void)accelerator_id; (void)source; (void)serial;
}
static void manager_done(void *data, struct df_toplevel_manager *m) {
    (void)data; (void)m;
}
static void manager_app_switcher_entry(void *data, struct df_toplevel_manager *m,
                                       uint32_t index, const char *app_id) {
    (void)data; (void)m; (void)index; (void)app_id;
}
static void manager_screenshot_saved(void *data, struct df_toplevel_manager *m,
                                     const char *path) {
    (void)data; (void)m; (void)path;
}
static void manager_screenshot_failed(void *data, struct df_toplevel_manager *m,
                                      const char *reason) {
    (void)data; (void)m; (void)reason;
}
static void manager_activation_result(void *data, struct df_toplevel_manager *m,
                                      const char *app_id, uint32_t found) {
    (void)data; (void)m; (void)app_id; (void)found;
}

static const struct df_toplevel_manager_listener manager_listener = {
    .output = manager_output,
    .workspace = manager_workspace,
    .toplevel = manager_toplevel,
    .workspace_activated = manager_workspace_activated,
    .focused = manager_focused,
    .attention = manager_attention,
    .hot_corner = manager_hot_corner,
    .overview_changed = manager_overview_changed,
    .app_switcher = manager_app_switcher,
    .input_action = manager_input_action,
    .progress = manager_progress,
    .app_accelerator = manager_app_accelerator,
    .done = manager_done,
    .app_switcher_entry = manager_app_switcher_entry,
    .screenshot_saved = manager_screenshot_saved,
    .screenshot_failed = manager_screenshot_failed,
    .activation_result = manager_activation_result,
};

static void registry_global(void *data, struct wl_registry *registry, uint32_t name,
                            const char *interface, uint32_t version) {
    struct state *s = data;
    if (strcmp(interface, df_core_interface.name) == 0) {
        s->core = wl_registry_bind(registry, name, &df_core_interface, 1);
        df_core_add_listener(s->core, &core_listener, s);
    } else if (strcmp(interface, df_toplevel_manager_interface.name) == 0) {
        s->manager_name = name;
    }
}

static void registry_remove(void *data, struct wl_registry *registry, uint32_t name) {
    (void)data; (void)registry; (void)name;
}

static const struct wl_registry_listener registry_listener = {
    .global = registry_global,
    .global_remove = registry_remove,
};

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s TOKEN TITLE_SUBSTRING [hold_seconds]\n", argv[0]);
        return 2;
    }
    const char *token = argv[1];
    const char *want = argv[2];
    int hold = argc > 3 ? atoi(argv[3]) : 8;
    struct state s = {0};
    s.want = want;
    s.display = wl_display_connect(NULL);
    if (!s.display) {
        fprintf(stderr, "shell-fullscreen: no display\n");
        return 1;
    }
    s.registry = wl_display_get_registry(s.display);
    wl_registry_add_listener(s.registry, &registry_listener, &s);
    wl_display_roundtrip(s.display);
    wl_display_roundtrip(s.display);
    if (!s.core) {
        fprintf(stderr, "shell-fullscreen: df_core missing\n");
        return 1;
    }
    df_core_authenticate(s.core, 1, token);
    wl_display_roundtrip(s.display);
    wl_display_roundtrip(s.display);
    if (!s.authenticated) {
        fprintf(stderr, "shell-fullscreen: not authenticated\n");
        return 1;
    }
    if (!s.manager) {
        fprintf(stderr, "shell-fullscreen: df_toplevel_manager missing\n");
        return 1;
    }
    for (int i = 0; i < 8; i++) wl_display_roundtrip(s.display);
    if (!s.target) {
        fprintf(stderr, "shell-fullscreen: no toplevel matching %s\n", want);
        return 1;
    }
    fprintf(stderr, "shell-fullscreen: fullscreen %s\n", s.target_title);
    df_toplevel_fullscreen(s.target);
    wl_display_flush(s.display);
    sleep(hold);
    fprintf(stderr, "shell-fullscreen: unfullscreen %s\n", s.target_title);
    df_toplevel_unfullscreen(s.target);
    wl_display_flush(s.display);
    sleep(2);
    wl_display_disconnect(s.display);
    return 0;
}
