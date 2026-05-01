#define _GNU_SOURCE
#include "virtual_kbd.h"
#include "renderer.h"
#include "common.h"
#include <stdio.h>
#include <ctype.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/mman.h>
#include <linux/input-event-codes.h>

#define KEYCODE_OFFSET 8

static uint32_t char_to_keycode(char c) {
    if (c >= 'a' && c <= 'z') {
        static const uint32_t map[] = {
            KEY_A, KEY_B, KEY_C, KEY_D, KEY_E, KEY_F, KEY_G, KEY_H, KEY_I, KEY_J,
            KEY_K, KEY_L, KEY_M, KEY_N, KEY_O, KEY_P, KEY_Q, KEY_R, KEY_S, KEY_T,
            KEY_U, KEY_V, KEY_W, KEY_X, KEY_Y, KEY_Z
        };
        return map[c - 'a'];
    }
    if (c == ' ') return KEY_SPACE;
    return 0;
}

void send_key(struct zwp_virtual_keyboard_v1 *vk, uint32_t keycode, uint32_t time, struct wl_display *display) {
    if (!vk) return;
    uint32_t vk_code = keycode + KEYCODE_OFFSET;
    fprintf(stderr, "send_key: evdev %u -> vk_code %u\n", keycode, vk_code);
    zwp_virtual_keyboard_v1_modifiers(vk, 0, 0, 0, 0);
    zwp_virtual_keyboard_v1_key(vk, time, vk_code, WL_KEYBOARD_KEY_STATE_PRESSED);
    zwp_virtual_keyboard_v1_key(vk, time + 1, vk_code, WL_KEYBOARD_KEY_STATE_RELEASED);
    wl_display_flush(display);
}
#include <sys/types.h>
#include <fcntl.h>
#include <errno.h>
#include <time.h>
#include <xkbcommon/xkbcommon.h>
#include "layout.h"

static int create_shm_file(off_t size) {
    int fd = memfd_create("typd-shm", MFD_CLOEXEC);
    if (fd < 0) {
        return -1;
    }
    if (ftruncate(fd, size) < 0) {
        close(fd);
        return -1;
    }
    return fd;
}

static void layer_surface_configure(void *data, struct zwlr_layer_surface_v1 *surface,
                                   uint32_t serial, uint32_t width, uint32_t height) {
    VirtualKbd *kbd = data;
    (void)height;
    kbd->width = (int32_t)width;
    
    zwlr_layer_surface_v1_ack_configure(surface, serial);

    // Cleanup old resources before reallocating (e.g. on resize)
    if (kbd->buffer) {
        wl_buffer_destroy(kbd->buffer);
        kbd->buffer = NULL;
    }
    if (kbd->shm_data && kbd->shm_data != MAP_FAILED) {
        munmap(kbd->shm_data, kbd->shm_size);
        kbd->shm_data = NULL;
    }
    kbd->shm_size = 0;

    size_t size = (size_t)kbd->width * kbd->height * 4;
    int fd = create_shm_file((off_t)size);
    if (fd < 0) {
        fprintf(stderr, "Failed to create SHM file: %s\n", strerror(errno));
        return;
    }

    kbd->shm_data = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (kbd->shm_data == MAP_FAILED) {
        fprintf(stderr, "mmap failed: %s\n", strerror(errno));
        close(fd);
        return;
    }
    kbd->shm_size = size;

    struct wl_shm_pool *pool = wl_shm_create_pool(kbd->app->shm, fd, (int32_t)size);
    kbd->buffer = wl_shm_pool_create_buffer(pool, 0, kbd->width, kbd->height, kbd->width * 4, WL_SHM_FORMAT_ARGB8888);
    wl_shm_pool_destroy(pool);
    close(fd);

    printf("Surface configured: %dx%d\n", kbd->width, kbd->height);
    virtual_kbd_draw(kbd);
    kbd->configured = 1;
}

static void layer_surface_closed(void *data, struct zwlr_layer_surface_v1 *surface) {
    (void)surface;
    VirtualKbd *kbd = data;
    fprintf(stderr, "layer surface closed\n");
    virtual_kbd_destroy(kbd);
    wl_display_disconnect(kbd->app->display);
    exit(0);
}

static const struct zwlr_layer_surface_v1_listener layer_surface_listener = {
    .configure = layer_surface_configure,
    .closed = layer_surface_closed,
};

static void pointer_handle_enter(void *data, struct wl_pointer *pointer, uint32_t serial,
                                struct wl_surface *surface, wl_fixed_t sx, wl_fixed_t sy) {
    (void)pointer; (void)serial; (void)surface;
    VirtualKbd *kbd = data;
    kbd->px = wl_fixed_to_double(sx);
    kbd->py = wl_fixed_to_double(sy);
}

static void pointer_handle_leave(void *data, struct wl_pointer *pointer, uint32_t serial,
                                struct wl_surface *surface) {
    (void)pointer; (void)serial; (void)surface;
    VirtualKbd *kbd = data;
    kbd->px = -1;
    kbd->py = -1;
}

static void pointer_handle_motion(void *data, struct wl_pointer *pointer, uint32_t time,
                                 wl_fixed_t sx, wl_fixed_t sy) {
    (void)pointer; (void)time;
    VirtualKbd *kbd = data;
    kbd->px = wl_fixed_to_double(sx);
    kbd->py = wl_fixed_to_double(sy);
    fprintf(stderr, "pointer motion: %.1f %.1f\n", kbd->px, kbd->py);
}

static void pointer_handle_button(void *data, struct wl_pointer *pointer, uint32_t serial,
                                 uint32_t time, uint32_t button, uint32_t state) {
    (void)pointer; (void)serial; (void)time;
    VirtualKbd *kbd = data;
    if (button != 0x110) return; // Left click
    fprintf(stderr, "button: %u state: %u\n", button, state);

    if (state == WL_POINTER_BUTTON_STATE_PRESSED) {
        // Check suggestion strip first
        double strip_h = kbd->height * 0.15;
        if (kbd->py < strip_h && kbd->num_suggestions > 0) {
            double padding = 8.0;
            double sug_w = (kbd->width - (kbd->num_suggestions + 1) * padding) / kbd->num_suggestions;
            int idx = (int)((kbd->px - padding) / (sug_w + padding));
            if (idx >= 0 && idx < kbd->num_suggestions) {
                const char *word = kbd->suggestions[idx].word;
                size_t typed_len = strlen(kbd->current_word);
                
                // 1. Backspace current word
                for (size_t i = 0; i < typed_len; i++) {
                    send_key(kbd->vk, KEY_BACKSPACE, time, kbd->app->display);
                }
                
                // 2. Type suggestion
                for (size_t i = 0; word[i]; i++) {
                    send_key(kbd->vk, char_to_keycode(word[i]), time, kbd->app->display);
                }
                
                // 3. Add space
                send_key(kbd->vk, KEY_SPACE, time, kbd->app->display);
                
                // 4. Reset
                kbd->current_word[0] = '\0';
                kbd->num_suggestions = 0;
                virtual_kbd_draw(kbd);
                return;
            }
        }

        for (size_t i = 0; i < LAYOUT_SIZE; i++) {
            const Key *k = &qwerty_layout[i];
            double x = k->x * kbd->width;
            double y = k->y * kbd->height;
            double w = k->width * kbd->width;
            double h = k->height * kbd->height;

            fprintf(stderr, "checking key %s at (%.0f,%.0f,%.0f,%.0f) vs click (%.0f,%.0f)\n",
                    k->label, x, y, w, h, kbd->px, kbd->py);
            if (kbd->px >= x && kbd->px <= x + w && kbd->py >= y && kbd->py <= y + h) {
                kbd->pressed_key = (int32_t)k->keycode;
                uint32_t vk_code = k->keycode + KEYCODE_OFFSET;
                fprintf(stderr, "injecting evdev %u -> vk_code %u PRESSED\n", k->keycode, vk_code);
                zwp_virtual_keyboard_v1_modifiers(kbd->vk, 0, 0, 0, 0);
                zwp_virtual_keyboard_v1_key(kbd->vk, time, vk_code, WL_KEYBOARD_KEY_STATE_PRESSED);
                wl_display_flush(kbd->app->display);

                // Word tracking logic
                if ((k->keycode >= KEY_Q && k->keycode <= KEY_P) ||
                    (k->keycode >= KEY_A && k->keycode <= KEY_L) ||
                    (k->keycode >= KEY_Z && k->keycode <= KEY_M)) {
                    size_t len = strlen(kbd->current_word);
                    if (len < MAX_WORD_LEN - 1) {
                        kbd->current_word[len] = (char)tolower(k->label[0]);
                        kbd->current_word[len + 1] = '\0';
                    }
                } else if (k->keycode == KEY_BACKSPACE) {
                    size_t len = strlen(kbd->current_word);
                    if (len > 0) {
                        kbd->current_word[len - 1] = '\0';
                    }
                } else if (k->keycode == KEY_SPACE || k->keycode == KEY_ENTER) {
                    kbd->current_word[0] = '\0';
                }

                // Query suggestions
                if (kbd->app->engine) {
                    kbd->num_suggestions = suggestions_prefix(kbd->app->engine, kbd->current_word, kbd->suggestions);
                }

                virtual_kbd_draw(kbd);
                break;
            }
        }
    } else {
        if (kbd->pressed_key != -1) {
            uint32_t vk_code = (uint32_t)kbd->pressed_key + KEYCODE_OFFSET;
            fprintf(stderr, "injecting evdev %u -> vk_code %u RELEASED\n", (uint32_t)kbd->pressed_key, vk_code);
            zwp_virtual_keyboard_v1_key(kbd->vk, time, vk_code, WL_KEYBOARD_KEY_STATE_RELEASED);
            wl_display_flush(kbd->app->display);
            kbd->pressed_key = -1;
            virtual_kbd_draw(kbd);
        }
    }
}

static void pointer_handle_axis(void *data, struct wl_pointer *pointer, uint32_t time,
                               uint32_t axis, wl_fixed_t value) {
    (void)data; (void)pointer; (void)time; (void)axis; (void)value;
}

static void pointer_handle_frame(void *data, struct wl_pointer *pointer) {
    (void)data; (void)pointer;
}

static void pointer_handle_axis_source(void *data, struct wl_pointer *pointer, uint32_t axis_source) {
    (void)data; (void)pointer; (void)axis_source;
}

static void pointer_handle_axis_stop(void *data, struct wl_pointer *pointer, uint32_t time, uint32_t axis) {
    (void)data; (void)pointer; (void)time; (void)axis;
}

static void pointer_handle_axis_discrete(void *data, struct wl_pointer *pointer, uint32_t axis, int32_t discrete) {
    (void)data; (void)pointer; (void)axis; (void)discrete;
}

static void pointer_handle_axis_value120(void *data, struct wl_pointer *pointer, uint32_t axis, int32_t value120) {
    (void)data; (void)pointer; (void)axis; (void)value120;
}

static void pointer_handle_axis_relative_direction(void *data, struct wl_pointer *pointer, uint32_t axis, uint32_t direction) {
    (void)data; (void)pointer; (void)axis; (void)direction;
}

static const struct wl_pointer_listener pointer_listener = {
    .enter = pointer_handle_enter,
    .leave = pointer_handle_leave,
    .motion = pointer_handle_motion,
    .button = pointer_handle_button,
    .axis = pointer_handle_axis,
    .frame = pointer_handle_frame,
    .axis_source = pointer_handle_axis_source,
    .axis_stop = pointer_handle_axis_stop,
    .axis_discrete = pointer_handle_axis_discrete,
    .axis_value120 = pointer_handle_axis_value120,
    .axis_relative_direction = pointer_handle_axis_relative_direction,
};

static void seat_handle_capabilities(void *data, struct wl_seat *seat, uint32_t capabilities) {
    VirtualKbd *kbd = data;
    fprintf(stderr, "seat capabilities: %u\n", capabilities);
    if (capabilities & WL_SEAT_CAPABILITY_POINTER) {
        kbd->pointer = wl_seat_get_pointer(seat);
        wl_pointer_add_listener(kbd->pointer, &pointer_listener, kbd);
    }
}

static void seat_handle_name(void *data, struct wl_seat *seat, const char *name) {
    (void)data; (void)seat; (void)name;
}

static const struct wl_seat_listener seat_listener = {
    .capabilities = seat_handle_capabilities,
    .name = seat_handle_name,
};

void virtual_kbd_bind_seat(VirtualKbd *kbd, struct wl_seat *seat) {
    kbd->seat = seat;
    wl_seat_add_listener(seat, &seat_listener, kbd);
}

bool virtual_kbd_init(VirtualKbd *kbd, AppState *app) {
    kbd->app = app;
    kbd->height = (1080 * 35) / 100; // 378px
    kbd->shm_data = MAP_FAILED; // Initialize to MAP_FAILED
    kbd->pressed_key = -1;
    kbd->shift_active = false;
    kbd->current_word[0] = '\0';
    kbd->num_suggestions = 0;

    kbd->surface = wl_compositor_create_surface(app->compositor);
    if (!kbd->surface) {
        fprintf(stderr, "Failed to create wl_surface\n");
        return false;
    }

    kbd->layer_surface = zwlr_layer_shell_v1_get_layer_surface(app->layer_shell, kbd->surface, NULL, 
                                                               ZWLR_LAYER_SHELL_V1_LAYER_TOP, "typd");
    if (!kbd->layer_surface) {
        fprintf(stderr, "Failed to create zwlr_layer_surface_v1\n");
        wl_surface_destroy(kbd->surface);
        kbd->surface = NULL;
        return false;
    }

    // Seat and Pointer
    kbd->seat = app->seat;

    // Virtual Keyboard
    kbd->vk = zwp_virtual_keyboard_manager_v1_create_virtual_keyboard(app->vk_mgr, kbd->seat);
    if (!kbd->vk) {
        fprintf(stderr, "Failed to create virtual keyboard\n");
    } else {
        fprintf(stderr, "Virtual keyboard created successfully\n");
    }
    
    // Keymap setup
    struct xkb_context *ctx = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
    if (!ctx) {
        fprintf(stderr, "Failed to create xkb_context\n");
        return false;
    }
    struct xkb_rule_names names = { .layout = "us" };
    struct xkb_keymap *keymap = xkb_keymap_new_from_names(ctx, &names, XKB_KEYMAP_COMPILE_NO_FLAGS);
    if (!keymap) {
        fprintf(stderr, "Failed to create xkb_keymap for layout 'us'\n");
        xkb_context_unref(ctx);
        return false;
    }
    char *keymap_str = xkb_keymap_get_as_string(keymap, XKB_KEYMAP_FORMAT_TEXT_V1);
    if (!keymap_str) {
        fprintf(stderr, "Failed to get keymap as string\n");
        xkb_keymap_unref(keymap);
        xkb_context_unref(ctx);
        return false;
    }
    size_t keymap_len = strlen(keymap_str) + 1;
    fprintf(stderr, "Sending keymap (%zu bytes)...\n", keymap_len);

    int fd = create_shm_file((off_t)keymap_len);
    if (fd >= 0) {
        void *ptr = mmap(NULL, keymap_len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
        if (ptr != MAP_FAILED) {
            memcpy(ptr, keymap_str, keymap_len);
            munmap(ptr, keymap_len);
            zwp_virtual_keyboard_v1_keymap(kbd->vk, WL_KEYBOARD_KEYMAP_FORMAT_XKB_V1, fd, (uint32_t)keymap_len);
            zwp_virtual_keyboard_v1_modifiers(kbd->vk, 0, 0, 0, 0);
            wl_display_flush(app->display);
            fprintf(stderr, "Keymap sent successfully\n");
        } else {
            fprintf(stderr, "mmap for keymap failed: %s\n", strerror(errno));
        }
        close(fd);
    } else {
        fprintf(stderr, "Failed to create shm file for keymap: %s\n", strerror(errno));
    }
    free(keymap_str);
    xkb_keymap_unref(keymap);
    xkb_context_unref(ctx);

    zwlr_layer_surface_v1_set_anchor(kbd->layer_surface, 
                                     ZWLR_LAYER_SURFACE_V1_ANCHOR_BOTTOM | 
                                     ZWLR_LAYER_SURFACE_V1_ANCHOR_LEFT | 
                                     ZWLR_LAYER_SURFACE_V1_ANCHOR_RIGHT);
    zwlr_layer_surface_v1_set_size(kbd->layer_surface, 0, (uint32_t)kbd->height);
    zwlr_layer_surface_v1_set_exclusive_zone(kbd->layer_surface, 0);
    zwlr_layer_surface_v1_set_keyboard_interactivity(kbd->layer_surface, ZWLR_LAYER_SURFACE_V1_KEYBOARD_INTERACTIVITY_NONE);

    zwlr_layer_surface_v1_add_listener(kbd->layer_surface, &layer_surface_listener, kbd);
    wl_surface_commit(kbd->surface);
    
    return true;
}

void virtual_kbd_draw(VirtualKbd *kbd) {
    if (kbd->shm_data == MAP_FAILED || !kbd->shm_data) return;

    cairo_surface_t *s = cairo_image_surface_create_for_data(kbd->shm_data, CAIRO_FORMAT_ARGB32, kbd->width, kbd->height, kbd->width * 4);
    if (cairo_surface_status(s) != CAIRO_STATUS_SUCCESS) {
        cairo_surface_destroy(s);
        return;
    }
    
    cairo_t *cr = cairo_create(s);
    if (cairo_status(cr) != CAIRO_STATUS_SUCCESS) {
        cairo_destroy(cr);
        cairo_surface_destroy(s);
        return;
    }

    renderer_draw_keyboard(cr, kbd);

    cairo_destroy(cr);
    cairo_surface_destroy(s);

    wl_surface_attach(kbd->surface, kbd->buffer, 0, 0);
    wl_surface_damage_buffer(kbd->surface, 0, 0, kbd->width, kbd->height);
    wl_surface_commit(kbd->surface);
}

void virtual_kbd_destroy(VirtualKbd *kbd) {
    if (kbd->pointer) {
        wl_pointer_release(kbd->pointer);
        kbd->pointer = NULL;
    }
    if (kbd->vk) {
        zwp_virtual_keyboard_v1_destroy(kbd->vk);
        kbd->vk = NULL;
    }
    if (kbd->layer_surface) {
        zwlr_layer_surface_v1_destroy(kbd->layer_surface);
        kbd->layer_surface = NULL;
    }
    if (kbd->surface) {
        wl_surface_destroy(kbd->surface);
        kbd->surface = NULL;
    }
    if (kbd->buffer) {
        wl_buffer_destroy(kbd->buffer);
        kbd->buffer = NULL;
    }
    if (kbd->shm_data && kbd->shm_data != MAP_FAILED) {
        munmap(kbd->shm_data, kbd->shm_size);
        kbd->shm_data = MAP_FAILED;
    }
    kbd->shm_size = 0;
}
