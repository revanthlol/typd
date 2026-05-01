#pragma once
#include "common.h"
#include "wlr-layer-shell-unstable-v1-client-protocol.h"

typedef struct {
    struct zwlr_layer_shell_v1   *layer_shell;
    struct zwlr_layer_surface_v1 *layer_surface;
    struct wl_surface            *surface;
    struct wl_buffer             *buffer;
    int32_t  width;
    int32_t  height;
    AppState *app;       // pointer back to app state
    uint8_t *shm_data;    // mmap'd shm pool data
    int      configured;  // set to 1 when layer_surface sends configure
} VirtualKbd;

void virtual_kbd_init(VirtualKbd *kbd, AppState *app);
void virtual_kbd_draw(VirtualKbd *kbd);
void virtual_kbd_destroy(VirtualKbd *kbd);
void virtual_kbd_ack_configure(VirtualKbd *kbd, uint32_t serial);
