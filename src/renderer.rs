use std::f64::consts::PI;

const UI_FONT: &str = "EB Garamond";

pub fn draw_keyboard(
    cr: &cairo::Context,
    width: i32,
    height: i32,
    keys: &[crate::layout::ComputedKey],
    shift: bool,
    caps: bool,
    ctrl: bool,
    alt: bool,
    hover_key_id: Option<usize>,
    active_key_id: Option<usize>,
    vkbd_ready: bool,
) {
    // 1. Clear to transparent
    cr.set_operator(cairo::Operator::Source);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.paint().unwrap();
    cr.set_operator(cairo::Operator::Over);

    let w = width as f64;
    let h = height as f64;
    let r = (w.min(h) * 0.035).clamp(8.0, 18.0);

    // 2. Rounded background with restrained depth
    let grad = cairo::LinearGradient::new(0.0, 0.0, 0.0, h);
    grad.add_color_stop_rgba(0.0, 0.12, 0.13, 0.15, 0.98);
    grad.add_color_stop_rgba(1.0, 0.055, 0.058, 0.066, 0.98);

    rounded_path(cr, 0.0, 0.0, w, h, r);
    cr.set_source(&grad).unwrap();
    cr.fill().unwrap();

    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12);
    cr.set_line_width(1.0);
    rounded_path(cr, 1.0, 1.0, w - 2.0, h - 2.0, r - 1.0);
    cr.stroke().unwrap();

    // Subtle inner highlight
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.05);
    cr.set_line_width(1.0);
    rounded_path(cr, 2.5, 2.5, w - 5.0, h - 5.0, r - 2.5);
    cr.stroke().unwrap();

    cr.save().unwrap();
    rounded_path(cr, 0.0, 0.0, w, h, r);
    cr.clip();
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.045);
    cr.rectangle(0.0, 0.0, w, crate::layout::DRAG_BAR_HEIGHT);
    cr.fill().unwrap();

    cr.set_source_rgba(1.0, 1.0, 1.0, 0.28);
    cr.set_line_width(1.6);
    let gx = w / 2.0 - 22.0;
    let gy = crate::layout::DRAG_BAR_HEIGHT / 2.0;
    cr.move_to(gx, gy);
    cr.line_to(gx + 44.0, gy);
    cr.stroke().unwrap();

    let close_size = 20.0;
    let close_x = w - close_size - 10.0;
    let close_y = (crate::layout::DRAG_BAR_HEIGHT - close_size) / 2.0;
    cr.set_source_rgba(0.82, 0.18, 0.18, 0.92);
    rounded_path(cr, close_x, close_y, close_size, close_size, 5.0);
    cr.fill().unwrap();
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.set_line_width(1.6);
    let (cx, cy, p) = (close_x + close_size / 2.0, close_y + close_size / 2.0, 4.2);
    cr.move_to(cx - p, cy - p);
    cr.line_to(cx + p, cy + p);
    cr.stroke().unwrap();
    cr.move_to(cx + p, cy - p);
    cr.line_to(cx - p, cy + p);
    cr.stroke().unwrap();
    cr.restore().unwrap();

    // 5. Suggestion strip placeholder
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.25);
    cr.rectangle(
        0.0,
        crate::layout::DRAG_BAR_HEIGHT,
        w,
        crate::layout::SUGGESTION_STRIP_HEIGHT,
    );
    cr.fill().unwrap();
    if !vkbd_ready {
        cr.set_source_rgba(1.0, 0.78, 0.26, 0.92);
        cr.select_font_face(UI_FONT, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size((w * 0.018).clamp(11.0, 14.0));
        cr.move_to(16.0, crate::layout::DRAG_BAR_HEIGHT + 27.0);
        cr.show_text("virtual keyboard unavailable").unwrap();
    }

    // 6. Keys
    if keys.is_empty() {
        cr.set_source_rgb(0.5, 0.5, 0.7);
        cr.select_font_face(UI_FONT, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(18.0);
        let text = "typd v0.1";
        let ext = cr.text_extents(text).unwrap();
        cr.move_to(w / 2.0 - ext.width() / 2.0, h / 2.0);
        cr.show_text(text).unwrap();
        return;
    }

    for key in keys {
        let effective_shift = if let Some(code) = key.def.linux_keycode {
            if crate::layout::is_alpha_key(code) {
                shift ^ caps
            } else {
                shift
            }
        } else {
            shift
        };
        let label = if effective_shift {
            key.def.label_upper
        } else {
            key.def.label_lower
        };

        let is_shift = key.def.action == crate::layout::KeyAction::Shift;
        let is_caps = key.def.action == crate::layout::KeyAction::Caps;
        let is_ctrl = key.def.action == crate::layout::KeyAction::Ctrl;
        let is_alt = key.def.action == crate::layout::KeyAction::Alt;
        let is_space = key.def.linux_keycode == Some(57);
        let is_special = key.def.action != crate::layout::KeyAction::Key
            || matches!(
                key.def.linux_keycode,
                Some(1 | 14 | 15 | 28 | 29 | 56 | 97 | 100 | 103 | 105 | 106 | 108 | 111 | 125)
            )
            || is_shift
            || is_space;
        let is_active = active_key_id == Some(key.id);
        let is_hovered = hover_key_id == Some(key.id);

        if is_active {
            cr.set_source_rgba(0.18, 0.48, 0.82, 1.0);
        } else if is_shift && shift {
            cr.set_source_rgba(0.17, 0.42, 0.72, 1.0);
        } else if is_caps && caps {
            cr.set_source_rgba(0.18, 0.54, 0.45, 1.0);
        } else if is_ctrl && ctrl {
            cr.set_source_rgba(0.2, 0.43, 0.72, 1.0);
        } else if is_alt && alt {
            cr.set_source_rgba(0.35, 0.38, 0.68, 1.0);
        } else if is_hovered {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.16);
        } else if is_special {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.105);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.07);
        }
        let key_radius = (key.h * 0.18).clamp(6.0, 10.0);
        rounded_path(cr, key.x, key.y, key.w, key.h, key_radius);
        cr.fill().unwrap();

        // Key border (subtle)
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.05);
        cr.set_line_width(1.0);
        rounded_path(
            cr,
            key.x + 0.5,
            key.y + 0.5,
            key.w - 1.0,
            key.h - 1.0,
            key_radius,
        );
        cr.stroke().unwrap();

        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
        if draw_arrow_label(cr, label, key.x, key.y, key.w, key.h) {
            continue;
        }
        draw_shift_hint(cr, label, key, effective_shift);
        draw_centered_label(cr, label, key.x, key.y, key.w, key.h);
    }
}

fn draw_shift_hint(
    cr: &cairo::Context,
    label: &str,
    key: &crate::layout::ComputedKey,
    effective_shift: bool,
) {
    if key.def.label_lower == key.def.label_upper {
        return;
    }
    let hint = if effective_shift {
        key.def.label_lower
    } else {
        key.def.label_upper
    };
    if hint == label || matches!(hint, "←" | "→" | "↑" | "↓") {
        return;
    }

    cr.save().unwrap();
    cr.select_font_face(UI_FONT, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size((key.h * 0.21).clamp(8.0, 12.0));
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.58);
    let ext = cr.text_extents(hint).unwrap();
    cr.move_to(
        key.x + key.w - ext.width() - ext.x_bearing() - 7.0,
        key.y + 7.0 - ext.y_bearing(),
    );
    cr.show_text(hint).unwrap();
    cr.restore().unwrap();
}

fn draw_centered_label(cr: &cairo::Context, label: &str, x: f64, y: f64, w: f64, h: f64) {
    cr.select_font_face(UI_FONT, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    let mut font_size = (h * 0.34).clamp(12.0, 20.0);
    loop {
        cr.set_font_size(font_size);
        let ext = cr.text_extents(label).unwrap();
        if ext.width() <= w - 12.0 || font_size <= 10.0 {
            cr.move_to(
                x + w / 2.0 - (ext.x_bearing() + ext.width() / 2.0),
                y + h / 2.0 - (ext.y_bearing() + ext.height() / 2.0),
            );
            cr.show_text(label).unwrap();
            break;
        }
        font_size -= 0.5;
    }
}

fn draw_arrow_label(cr: &cairo::Context, label: &str, x: f64, y: f64, w: f64, h: f64) -> bool {
    let dir = match label {
        "←" => (-1.0, 0.0),
        "→" => (1.0, 0.0),
        "↑" => (0.0, -1.0),
        "↓" => (0.0, 1.0),
        _ => return false,
    };

    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let len = (w.min(h) * 0.28).clamp(9.0, 18.0);
    let head = (len * 0.42).clamp(5.0, 8.0);
    let (dx, dy) = dir;
    let sx = cx - dx * len * 0.45;
    let sy = cy - dy * len * 0.45;
    let ex = cx + dx * len * 0.45;
    let ey = cy + dy * len * 0.45;

    cr.set_line_width(2.0);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.move_to(sx, sy);
    cr.line_to(ex, ey);
    cr.stroke().unwrap();

    let px = -dy;
    let py = dx;
    cr.move_to(ex, ey);
    cr.line_to(
        ex - dx * head + px * head * 0.65,
        ey - dy * head + py * head * 0.65,
    );
    cr.stroke().unwrap();
    cr.move_to(ex, ey);
    cr.line_to(
        ex - dx * head - px * head * 0.65,
        ey - dy * head - py * head * 0.65,
    );
    cr.stroke().unwrap();
    true
}

pub fn rounded_path(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    if r <= 0.0 {
        cr.rectangle(x, y, w, h);
        return;
    }
    cr.new_sub_path();
    cr.arc(x + r, y + r, r, PI, 1.5 * PI);
    cr.arc(x + w - r, y + r, r, 1.5 * PI, 2.0 * PI);
    cr.arc(x + w - r, y + h - r, r, 0.0, 0.5 * PI);
    cr.arc(x + r, y + h - r, r, 0.5 * PI, PI);
    cr.close_path();
}
