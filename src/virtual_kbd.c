#define _GNU_SOURCE
#include "virtual_kbd.h"
#include "renderer.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/types.h>
#include <fcntl.h>

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
    // Note: In Phase 1, we use the requested height (378), but the compositor might tell us something else.
    // However, since we requested 378 and anchor BOTTOM|LEFT|RIGHT with width 0, 
    // the width here will be the screen width.
    
    zwlr_layer_surface_v1_ack_configure(surface, serial);
    printf("Surface configured: %dx%d\n", kbd->width, kbd->height);

    size_t size = (size_t)kbd->width * kbd->height * 4;
    int fd = create_shm_file((off_t)size);
    if (fd < 0) {
        fprintf(stderr, "Failed to create SHM file\n");
        return;
    }

    kbd->shm_data = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (kbd->shm_data == MAP_FAILED) {
        close(fd);
        return;
    }

    struct wl_shm_pool *pool = wl_shm_create_pool(kbd->app->shm, fd, (int32_t)size);
    kbd->buffer = wl_shm_pool_create_buffer(pool, 0, kbd->width, kbd->height, kbd->width * 4, WL_SHM_FORMAT_ARGB8888);
    wl_shm_pool_destroy(pool);
    close(fd);

    virtual_kbd_draw(kbd);
    kbd->configured = 1;
}

static void layer_surface_closed(void *data, struct zwlr_layer_surface_v1 *surface) {
    (void)surface;
    VirtualKbd *kbd = data;
    fprintf(stderr, "layer surface closed\n");
    wl_display_disconnect(kbd->app->display);
    exit(0);
}

static const struct zwlr_layer_surface_v1_listener layer_surface_listener = {
    .configure = layer_surface_configure,
    .closed = layer_surface_closed,
};

void virtual_kbd_init(VirtualKbd *kbd, AppState *app) {
    memset(kbd, 0, sizeof(VirtualKbd));
    kbd->app = app;
    kbd->height = (1080 * 35) / 100; // 378px

    kbd->surface = wl_compositor_create_surface(app->compositor);
    kbd->layer_surface = zwlr_layer_shell_v1_get_layer_surface(app->layer_shell, kbd->surface, NULL, 
                                                              ZWLR_LAYER_SHELL_V1_LAYER_TOP, "typd");

    zwlr_layer_surface_v1_set_anchor(kbd->layer_surface, 
                                     ZWLR_LAYER_SURFACE_V1_ANCHOR_BOTTOM | 
                                     ZWLR_LAYER_SURFACE_V1_ANCHOR_LEFT | 
                                     ZWLR_LAYER_SURFACE_V1_ANCHOR_RIGHT);
    zwlr_layer_surface_v1_set_size(kbd->layer_surface, 0, (uint32_t)kbd->height);
    zwlr_layer_surface_v1_set_exclusive_zone(kbd->layer_surface, 0);
    zwlr_layer_surface_v1_set_keyboard_interactivity(kbd->layer_surface, ZWLR_LAYER_SURFACE_V1_KEYBOARD_INTERACTIVITY_NONE);

    zwlr_layer_surface_v1_add_listener(kbd->layer_surface, &layer_surface_listener, kbd);
    wl_surface_commit(kbd->surface);
}

void virtual_kbd_draw(VirtualKbd *kbd) {
    if (!kbd->shm_data) return;

    cairo_surface_t *s = cairo_image_surface_create_for_data(kbd->shm_data, CAIRO_FORMAT_ARGB32, kbd->width, kbd->height, kbd->width * 4);
    cairo_t *cr = cairo_create(s);

    renderer_draw_keyboard_placeholder(cr, kbd->width, kbd->height);

    cairo_destroy(cr);
    cairo_surface_destroy(s);

    wl_surface_attach(kbd->surface, kbd->buffer, 0, 0);
    wl_surface_damage_buffer(kbd->surface, 0, 0, kbd->width, kbd->height);
    wl_surface_commit(kbd->surface);
}

void virtual_kbd_destroy(VirtualKbd *kbd) {
    if (kbd->layer_surface) zwlr_layer_surface_v1_destroy(kbd->layer_surface);
    if (kbd->surface) wl_surface_destroy(kbd->surface);
    if (kbd->buffer) wl_buffer_destroy(kbd->buffer);
    if (kbd->shm_data) munmap(kbd->shm_data, (size_t)kbd->width * kbd->height * 4);
}
