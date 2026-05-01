#pragma once
#include <wayland-client.h>

typedef struct {
    struct wl_display    *display;
    struct wl_registry   *registry;
    struct wl_compositor *compositor;
    struct wl_shm        *shm;
    struct zwlr_layer_shell_v1 *layer_shell;
} AppState;
