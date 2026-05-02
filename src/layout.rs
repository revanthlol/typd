pub const DRAG_BAR_HEIGHT: f64 = 32.0;
pub const SUGGESTION_STRIP_HEIGHT: f64 = 38.0;
pub const SIDEBAR_WIDTH: f64 = 260.0;
pub const SIDEBAR_GAP: f64 = 8.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Key,
    Shift,
    Caps,
    Ctrl,
    Alt,
    NumLock,
    ToggleSidebar,
}

#[derive(Clone)]
pub struct KeyDef {
    pub label_lower: &'static str,
    pub label_upper: &'static str,
    pub linux_keycode: Option<u32>,
    pub width_mul: f64,
    pub action: KeyAction,
}

pub fn key(lo: &'static str, hi: &'static str, code: u32) -> KeyDef {
    KeyDef {
        label_lower: lo,
        label_upper: hi,
        linux_keycode: Some(code),
        width_mul: 1.0,
        action: KeyAction::Key,
    }
}

pub fn wide(lo: &'static str, hi: &'static str, code: u32, mul: f64) -> KeyDef {
    KeyDef {
        label_lower: lo,
        label_upper: hi,
        linux_keycode: Some(code),
        width_mul: mul,
        action: KeyAction::Key,
    }
}

pub fn shift_key(code: u32, mul: f64) -> KeyDef {
    KeyDef {
        label_lower: "Shift",
        label_upper: "Shift",
        linux_keycode: Some(code),
        width_mul: mul,
        action: KeyAction::Shift,
    }
}

pub fn caps_key(mul: f64) -> KeyDef {
    KeyDef {
        label_lower: "Caps",
        label_upper: "Caps",
        linux_keycode: Some(58),
        width_mul: mul,
        action: KeyAction::Caps,
    }
}

pub fn ctrl_key(code: u32, mul: f64) -> KeyDef {
    KeyDef {
        label_lower: "Ctrl",
        label_upper: "Ctrl",
        linux_keycode: Some(code),
        width_mul: mul,
        action: KeyAction::Ctrl,
    }
}

pub fn alt_key(code: u32, mul: f64) -> KeyDef {
    KeyDef {
        label_lower: "Alt",
        label_upper: "Alt",
        linux_keycode: Some(code),
        width_mul: mul,
        action: KeyAction::Alt,
    }
}

pub fn action_key(label: &'static str, action: KeyAction, mul: f64) -> KeyDef {
    KeyDef {
        label_lower: label,
        label_upper: label,
        linux_keycode: None,
        width_mul: mul,
        action,
    }
}

pub fn main_rows(sidebar_expanded: bool) -> Vec<Vec<KeyDef>> {
    vec![
        vec![
            wide("Esc", "Esc", 1, 1.0),
            key("F1", "F1", 59),
            key("F2", "F2", 60),
            key("F3", "F3", 61),
            key("F4", "F4", 62),
            key("F5", "F5", 63),
            key("F6", "F6", 64),
            key("F7", "F7", 65),
            key("F8", "F8", 66),
            key("F9", "F9", 67),
            key("F10", "F10", 68),
            key("F11", "F11", 87),
            key("F12", "F12", 88),
        ],
        vec![
            key("`", "~", 41),
            key("1", "!", 2),
            key("2", "@", 3),
            key("3", "#", 4),
            key("4", "$", 5),
            key("5", "%", 6),
            key("6", "^", 7),
            key("7", "&", 8),
            key("8", "*", 9),
            key("9", "(", 10),
            key("0", ")", 11),
            key("-", "_", 12),
            key("=", "+", 13),
            wide("Bksp", "Bksp", 14, 1.65),
        ],
        vec![
            wide("Tab", "Tab", 15, 1.45),
            key("q", "Q", 16),
            key("w", "W", 17),
            key("e", "E", 18),
            key("r", "R", 19),
            key("t", "T", 20),
            key("y", "Y", 21),
            key("u", "U", 22),
            key("i", "I", 23),
            key("o", "O", 24),
            key("p", "P", 25),
            key("[", "{", 26),
            key("]", "}", 27),
            wide("\\", "|", 43, 1.25),
            wide("Del", "Del", 111, 1.15),
        ],
        vec![
            caps_key(1.75),
            key("a", "A", 30),
            key("s", "S", 31),
            key("d", "D", 32),
            key("f", "F", 33),
            key("g", "G", 34),
            key("h", "H", 35),
            key("j", "J", 36),
            key("k", "K", 37),
            key("l", "L", 38),
            key(";", ":", 39),
            key("'", "\"", 40),
            wide("Enter", "Enter", 28, 2.45),
        ],
        vec![
            shift_key(42, 2.25),
            key("z", "Z", 44),
            key("x", "X", 45),
            key("c", "C", 46),
            key("v", "V", 47),
            key("b", "B", 48),
            key("n", "N", 49),
            key("m", "M", 50),
            key(",", "<", 51),
            key(".", ">", 52),
            key("/", "?", 53),
            wide("↑", "↑", 103, 1.0),
            shift_key(54, 2.0),
        ],
        vec![
            ctrl_key(29, 1.05),
            wide("Super", "Super", 125, 1.15),
            alt_key(56, 1.05),
            wide("", "", 57, 6.45),
            alt_key(100, 1.05),
            ctrl_key(97, 1.05),
            wide("←", "←", 105, 1.0),
            wide("↓", "↓", 108, 1.0),
            wide("→", "→", 106, 1.0),
            action_key(
                if sidebar_expanded { "<<" } else { ">>" },
                KeyAction::ToggleSidebar,
                1.15,
            ),
        ],
    ]
}

pub fn sidebar_rows() -> Vec<Vec<KeyDef>> {
    vec![
        vec![
            key("Home", "Home", 102),
            key("PgUp", "PgUp", 104),
            action_key("Num", KeyAction::NumLock, 1.0),
        ],
        vec![
            key("End", "End", 107),
            key("PgDn", "PgDn", 109),
            key("Ins", "Ins", 110),
        ],
        vec![
            key("PrtSc", "PrtSc", 99),
            key("Pause", "Pause", 119),
            key("Del", "Del", 111),
        ],
        vec![
            key("ScrLk", "ScrLk", 70),
            key("Menu", "Menu", 127),
            key("Help", "Help", 138),
        ],
    ]
}

#[derive(Clone)]
pub struct ComputedKey {
    pub id: usize,
    pub def: KeyDef,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub fn compute_layout(surface_w: f64, surface_h: f64, sidebar_expanded: bool) -> Vec<ComputedKey> {
    let outer_pad = (surface_w * 0.007).clamp(6.0, 12.0);
    let row_gap = (surface_h * 0.01).clamp(3.0, 7.0);
    let key_gap = (surface_w * 0.0035).clamp(3.0, 6.0);
    let top_chrome = (DRAG_BAR_HEIGHT + SUGGESTION_STRIP_HEIGHT).min(surface_h * 0.28);
    let key_area_y = top_chrome + row_gap;
    let key_area_h = (surface_h - key_area_y - outer_pad).max(180.0);
    let rows = main_rows(sidebar_expanded);
    let row_h =
        ((key_area_h - row_gap * (rows.len() as f64 - 1.0)) / rows.len() as f64).clamp(34.0, 74.0);

    let sidebar_w = if sidebar_expanded { SIDEBAR_WIDTH } else { 0.0 };
    let main_w = if sidebar_expanded {
        surface_w - outer_pad * 2.0 - sidebar_w - SIDEBAR_GAP
    } else {
        surface_w - outer_pad * 2.0
    };

    let mut result = Vec::new();
    let mut id = 0;

    for (ri, row) in rows.iter().enumerate() {
        let total_units: f64 = row.iter().map(|k| k.width_mul).sum();
        let usable_w = (main_w - key_gap * (row.len() as f64 - 1.0)).max(360.0);
        let unit_w = usable_w / total_units;
        let row_total_w = total_units * unit_w + key_gap * (row.len() as f64 - 1.0);
        let mut kx = outer_pad + (main_w - row_total_w) / 2.0;
        let ky = key_area_y + ri as f64 * (row_h + row_gap);

        for def in row {
            let kw = def.width_mul * unit_w;
            result.push(ComputedKey {
                id,
                def: def.clone(),
                x: kx,
                y: ky,
                w: kw,
                h: row_h,
            });
            id += 1;
            kx += kw + key_gap;
        }
    }

    if sidebar_expanded {
        let side_x = outer_pad + main_w + SIDEBAR_GAP;
        let side_rows = sidebar_rows();
        let side_row_h = ((key_area_h - row_gap * (side_rows.len() as f64 - 1.0))
            / side_rows.len() as f64)
            .clamp(34.0, 74.0);

        for (ri, row) in side_rows.iter().enumerate() {
            let total_units: f64 = row.iter().map(|k| k.width_mul).sum();
            let usable_w = (sidebar_w - key_gap * (row.len() as f64 - 1.0)).max(120.0);
            let unit_w = usable_w / total_units;
            let mut kx = side_x;
            let ky = key_area_y + ri as f64 * (side_row_h + row_gap);

            for def in row {
                let kw = def.width_mul * unit_w;
                result.push(ComputedKey {
                    id,
                    def: def.clone(),
                    x: kx,
                    y: ky,
                    w: kw,
                    h: side_row_h,
                });
                id += 1;
                kx += kw + key_gap;
            }
        }
    }

    result
}

pub fn sidebar_delta() -> u32 {
    (SIDEBAR_WIDTH + SIDEBAR_GAP).round() as u32
}

pub fn is_alpha_key(keycode: u32) -> bool {
    matches!(keycode, 16..=25 | 30..=38 | 44..=50)
}
