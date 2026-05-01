#pragma once
#include <wayland-client.h>
#include "wlr-layer-shell-unstable-v1-client-protocol.h"
#include "virtual-keyboard-unstable-v1-client-protocol.h"

#include "suggestions.h"

typedef struct AppState {
    struct wl_display    *display;
    struct wl_registry   *registry;
    struct wl_compositor *compositor;
    struct wl_shm        *shm;
    struct zwlr_layer_shell_v1 *layer_shell;
    struct zwp_virtual_keyboard_manager_v1 *vk_mgr;
    struct wl_seat       *seat;
    struct VirtualKbd    *kbd;
    SuggestionEngine     *engine;
} AppState;
