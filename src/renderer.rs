use cairo::Context;

pub fn draw_placeholder(cr: &Context, width: i32, height: i32) {
    // Background
    cr.set_source_rgb(0.10, 0.10, 0.12);
    cr.paint().unwrap();

    // Keyboard body
    let margin = 12.0;
    cr.set_source_rgb(0.16, 0.16, 0.20);
    cr.rectangle(
        margin,
        margin,
        (width as f64) - margin * 2.0,
        (height as f64) - margin * 2.0,
    );
    cr.fill().unwrap();

    // Centered label
    cr.set_source_rgb(0.55, 0.55, 0.70);
    cr.select_font_face("monospace",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal);
    cr.set_font_size(18.0);
    let ext = cr.text_extents("typd v0.1").unwrap();
    cr.move_to(
        (width as f64 - ext.width()) / 2.0,
        (height as f64 + ext.height()) / 2.0,
    );
    cr.show_text("typd v0.1").unwrap();
}
