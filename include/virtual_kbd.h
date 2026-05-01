#pragma once
#include <unistd.h>
#include <linux/input-event-codes.h>
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include "wlr-layer-shell-unstable-v1-client-protocol.h"
#include "suggestions.h"

// Forward declaration
typedef struct AppState AppState;

typedef struct VirtualKbd {
    struct zwlr_layer_shell_v1   *layer_shell;
    struct zwlr_layer_surface_v1 *layer_surface;
    struct wl_surface            *surface;
    struct wl_buffer             *buffer;
    struct zwp_virtual_keyboard_v1 *vk;
    struct wl_seat               *seat;
    struct wl_pointer            *pointer;
    int32_t  width;
    int32_t  height;
    AppState *app;
    uint8_t  *shm_data;
    size_t   shm_size;    // Stored exact mapping size
    int      configured;
    double   px, py;      // Pointer coordinates
    int32_t  pressed_key; // Keycode of currently pressed key
    bool     shift_active;

    char     current_word[64]; // MAX_WORD_LEN from suggestions.h
    Suggestion suggestions[3]; // MAX_SUGGESTIONS from suggestions.h
    int      num_suggestions;
} VirtualKbd;

bool virtual_kbd_init(VirtualKbd *kbd, AppState *app);
void virtual_kbd_bind_seat(VirtualKbd *kbd, struct wl_seat *seat);
void virtual_kbd_draw(VirtualKbd *kbd);
void virtual_kbd_destroy(VirtualKbd *kbd);
void send_key(struct zwp_virtual_keyboard_v1 *vk, uint32_t keycode, uint32_t time, struct wl_display *display);
