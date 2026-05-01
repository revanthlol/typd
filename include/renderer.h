#pragma once
#include <cairo/cairo.h>
#include "virtual_kbd.h"

void renderer_draw_keyboard(cairo_t *cr, VirtualKbd *kbd);
