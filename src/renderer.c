#include "renderer.h"

void renderer_draw_keyboard_placeholder(cairo_t *cr, int width, int height) {
    // Background
    cairo_set_source_rgb(cr, 0.10, 0.10, 0.12);
    cairo_paint(cr);

    // Keyboard body rectangle
    double margin = 12.0;
    cairo_set_source_rgb(cr, 0.16, 0.16, 0.20);
    cairo_rectangle(cr, margin, margin, (double)width - margin*2, (double)height - margin*2);
    cairo_fill(cr);

    // Center label
    cairo_set_source_rgb(cr, 0.55, 0.55, 0.70);
    cairo_select_font_face(cr, "monospace", CAIRO_FONT_SLANT_NORMAL, CAIRO_FONT_WEIGHT_NORMAL);
    cairo_set_font_size(cr, 18.0);
    cairo_text_extents_t ext;
    cairo_text_extents(cr, "typd v0.1", &ext);
    cairo_move_to(cr, ((double)width - ext.width) / 2.0, ((double)height + ext.height) / 2.0);
    cairo_show_text(cr, "typd v0.1");
}
