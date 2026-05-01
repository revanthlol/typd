#define _GNU_SOURCE
#include "renderer.h"
#include "layout.h"
#include <math.h>

static void draw_rounded_rect(cairo_t *cr, double x, double y, double w, double h, double r) {
    cairo_new_sub_path(cr);
    cairo_arc(cr, x + w - r, y + r, r, -M_PI / 2, 0);
    cairo_arc(cr, x + w - r, y + h - r, r, 0, M_PI / 2);
    cairo_arc(cr, x + r, y + h - r, r, M_PI / 2, M_PI);
    cairo_arc(cr, x + r, y + r, r, M_PI, 3 * M_PI / 2);
    cairo_close_path(cr);
}

static void draw_suggestion_strip(cairo_t *cr, VirtualKbd *kbd) {
    if (kbd->num_suggestions == 0) return;

    double strip_h = kbd->height * 0.15;
    double padding = 8.0;
    double sug_w = (kbd->width - (kbd->num_suggestions + 1) * padding) / kbd->num_suggestions;
    double sug_h = strip_h - padding * 2;

    for (int i = 0; i < kbd->num_suggestions; i++) {
        double x = padding + i * (sug_w + padding);
        double y = padding;

        // Suggestion capsule
        cairo_set_source_rgb(cr, 0.15, 0.15, 0.18);
        draw_rounded_rect(cr, x, y, sug_w, sug_h, sug_h / 2.0);
        cairo_fill(cr);

        // Border
        cairo_set_source_rgba(cr, 0.3, 0.3, 0.4, 0.5);
        cairo_set_line_width(cr, 1.0);
        draw_rounded_rect(cr, x, y, sug_w, sug_h, sug_h / 2.0);
        cairo_stroke(cr);

        // Text
        cairo_set_source_rgb(cr, 0.9, 0.9, 1.0);
        cairo_select_font_face(cr, "sans-serif", CAIRO_FONT_SLANT_NORMAL, CAIRO_FONT_WEIGHT_NORMAL);
        cairo_set_font_size(cr, 18.0);

        cairo_text_extents_t ext;
        cairo_text_extents(cr, kbd->suggestions[i].word, &ext);
        cairo_move_to(cr, x + (sug_w - ext.width) / 2.0 - ext.x_bearing, 
                         y + (sug_h - ext.height) / 2.0 - ext.y_bearing);
        cairo_show_text(cr, kbd->suggestions[i].word);
    }
}

void renderer_draw_keyboard(cairo_t *cr, VirtualKbd *kbd) {
    int width = kbd->width;
    int height = kbd->height;

    // Background
    cairo_set_source_rgb(cr, 0.05, 0.05, 0.07);
    cairo_paint(cr);

    // Suggestion Strip
    draw_suggestion_strip(cr, kbd);

    double key_margin = 3.0;
    double radius = 8.0;

    for (size_t i = 0; i < LAYOUT_SIZE; i++) {
        const Key *k = &qwerty_layout[i];
        double x = k->x * width + key_margin;
        double y = k->y * height + key_margin;
        double w = k->width * width - key_margin * 2;
        double h = k->height * height - key_margin * 2;

        bool pressed = (kbd->pressed_key == (int32_t)k->keycode);

        // Key body
        if (pressed) {
            cairo_set_source_rgb(cr, 0.30, 0.45, 0.85);
        } else {
            cairo_set_source_rgb(cr, 0.12, 0.12, 0.15);
        }
        draw_rounded_rect(cr, x, y, w, h, radius);
        cairo_fill(cr);

        // Border for keys
        cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.05);
        cairo_set_line_width(cr, 1.0);
        draw_rounded_rect(cr, x, y, w, h, radius);
        cairo_stroke(cr);

        // Key label
        cairo_set_source_rgb(cr, 0.9, 0.9, 0.95);
        cairo_select_font_face(cr, "sans-serif", CAIRO_FONT_SLANT_NORMAL, CAIRO_FONT_WEIGHT_BOLD);
        cairo_set_font_size(cr, 17.0);
        
        cairo_text_extents_t ext;
        cairo_text_extents(cr, k->label, &ext);
        cairo_move_to(cr, x + (w - ext.width) / 2.0 - ext.x_bearing, 
                         y + (h - ext.height) / 2.0 - ext.y_bearing);
        cairo_show_text(cr, k->label);
    }
}
