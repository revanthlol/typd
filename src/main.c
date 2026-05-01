#include <stdio.h>
#include <string.h>
#include <signal.h>
#include <wayland-client.h>
#include <unistd.h>
#include <linux/input-event-codes.h>
#include "common.h"
#include "virtual_kbd.h"

static volatile int running = 1;
static void handle_sig(int s) { (void)s; running = 0; }

static void registry_handle_global(void *data, struct wl_registry *registry,
                                  uint32_t name, const char *interface, uint32_t version) {
    AppState *app = data;

    fprintf(stderr, "registry: %s v%u\n", interface, version);
    if (strcmp(interface, wl_compositor_interface.name) == 0) {
        app->compositor = wl_registry_bind(registry, name, &wl_compositor_interface, version < 4 ? version : 4);
    } else if (strcmp(interface, wl_shm_interface.name) == 0) {
        app->shm = wl_registry_bind(registry, name, &wl_shm_interface, 1);
    } else if (strcmp(interface, zwlr_layer_shell_v1_interface.name) == 0) {
        app->layer_shell = wl_registry_bind(registry, name, &zwlr_layer_shell_v1_interface, 1);
    } else if (strcmp(interface, zwp_virtual_keyboard_manager_v1_interface.name) == 0) {
        app->vk_mgr = wl_registry_bind(registry, name, &zwp_virtual_keyboard_manager_v1_interface, 1);
    } else if (strcmp(interface, wl_seat_interface.name) == 0) {
        app->seat = wl_registry_bind(registry, name, &wl_seat_interface, version < 7 ? version : 7);
        if (app->kbd) {
            virtual_kbd_bind_seat(app->kbd, app->seat);
        }
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

    app.kbd = &kbd;
    app.registry = wl_display_get_registry(app.display);
    wl_registry_add_listener(app.registry, &registry_listener, &app);

    // Initial roundtrips to get globals
    wl_display_roundtrip(app.display);
    wl_display_roundtrip(app.display);

    if (!app.compositor || !app.shm || !app.layer_shell || !app.vk_mgr || !app.seat) {
        fprintf(stderr, "Missing critical Wayland globals (compositor: %p, shm: %p, layer_shell: %p, vk_mgr: %p, seat: %p)\n",
                (void*)app.compositor, (void*)app.shm, (void*)app.layer_shell, (void*)app.vk_mgr, (void*)app.seat);
        if (app.registry) wl_registry_destroy(app.registry);
        wl_display_flush(app.display);
        wl_display_disconnect(app.display);
        return 1;
    }

    app.engine = suggestions_engine_create("data/words.freq");
    if (!app.engine) {
        fprintf(stderr, "Failed to initialize suggestion engine\n");
    }

    if (!virtual_kbd_init(&kbd, &app)) {
        fprintf(stderr, "Failed to initialize virtual keyboard\n");
        if (app.registry) wl_registry_destroy(app.registry);
        wl_display_flush(app.display);
        wl_display_disconnect(app.display);
        return 1;
    }

    // Process initial events (like seat capabilities)
    wl_display_roundtrip(app.display);
    wl_display_roundtrip(app.display);

    // DEBUG: Auto-inject 'A' after 2 seconds to test if it works without clicking
    fprintf(stderr, "DEBUG: waiting 2s then injecting 'A'...\n");
    sleep(2);
    send_key(kbd.vk, KEY_A, 0, app.display);
    wl_display_flush(app.display);
    fprintf(stderr, "DEBUG: 'A' injected.\n");

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
    if (app.engine) suggestions_engine_destroy(app.engine);
    if (app.registry) wl_registry_destroy(app.registry);
    wl_display_flush(app.display);
    wl_display_disconnect(app.display);

    return 0;
}
