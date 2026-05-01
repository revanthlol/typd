#include <stdio.h>
#include <string.h>
#include <signal.h>
#include <wayland-client.h>
#include "common.h"
#include "virtual_kbd.h"
#include "wlr-layer-shell-unstable-v1-client-protocol.h"

static volatile int running = 1;
static void handle_sig(int s) { (void)s; running = 0; }

static void registry_handle_global(void *data, struct wl_registry *registry,
                                  uint32_t name, const char *interface, uint32_t version) {
    AppState *app = data;

    if (strcmp(interface, wl_compositor_interface.name) == 0) {
        app->compositor = wl_registry_bind(registry, name, &wl_compositor_interface, version < 4 ? version : 4);
    } else if (strcmp(interface, wl_shm_interface.name) == 0) {
        app->shm = wl_registry_bind(registry, name, &wl_shm_interface, 1);
    } else if (strcmp(interface, zwlr_layer_shell_v1_interface.name) == 0) {
        app->layer_shell = wl_registry_bind(registry, name, &zwlr_layer_shell_v1_interface, 1);
    }
}

static void registry_handle_global_remove(void *data, struct wl_registry *registry, uint32_t name) {
    (void)data; (void)registry; (void)name;
}

static const struct wl_registry_listener registry_listener = {
    .global = registry_handle_global,
    .global_remove = registry_handle_global_remove,
};

int main(int argc, char **argv) {
    (void)argc; (void)argv;
    AppState app = {0};
    VirtualKbd kbd = {0};

    app.display = wl_display_connect(NULL);
    if (!app.display) {
        fprintf(stderr, "Failed to connect to Wayland display\n");
        return 1;
    }

    app.registry = wl_display_get_registry(app.display);
    wl_registry_add_listener(app.registry, &registry_listener, &app);

    // Initial roundtrips to get globals
    wl_display_roundtrip(app.display);
    wl_display_roundtrip(app.display);

    if (!app.compositor || !app.shm || !app.layer_shell) {
        fprintf(stderr, "Missing critical Wayland globals (compositor: %p, shm: %p, layer_shell: %p)\n",
                (void*)app.compositor, (void*)app.shm, (void*)app.layer_shell);
        return 1;
    }

    virtual_kbd_init(&kbd, &app);

    signal(SIGINT, handle_sig);
    signal(SIGTERM, handle_sig);

    // Wait for the configure event
    while (running && wl_display_dispatch(app.display) != -1 && !kbd.configured) {
        // Wait for first configure
    }

    // Main loop
    while (running && wl_display_dispatch(app.display) != -1) {
        // Handle events
    }

    virtual_kbd_destroy(&kbd);
    wl_display_disconnect(app.display);

    return 0;
}
