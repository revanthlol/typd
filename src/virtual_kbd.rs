use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
    shell::wlr_layer::{Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use wayland_client;
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface::WlSurface},
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1, wp_cursor_shape_manager_v1,
};

use crate::vkbd_proto;

use vkbd_proto::{zwp_virtual_keyboard_manager_v1, zwp_virtual_keyboard_v1};

use std::os::unix::io::AsFd;
use std::os::unix::io::FromRawFd;
use std::time::{Duration, Instant};

use crate::layout::{self, ComputedKey, KeyAction};
use crate::renderer;

pub const SURFACE_WIDTH: u32 = 1280;
pub const SURFACE_HEIGHT: u32 = 480;
pub const DRAG_BAR_HEIGHT: f64 = 32.0; // Slightly taller for better touch/click
const MIN_WIDTH: f64 = 600.0;
const MIN_HEIGHT: f64 = 220.0;
const INITIAL_REPEAT_DELAY: Duration = Duration::from_millis(430);
const REPEAT_INTERVAL: Duration = Duration::from_millis(55);
const HOVER_FADE: Duration = Duration::from_millis(120);

#[derive(Clone)]
pub struct HeldKey {
    pub key_id: usize,
    pub keycode: u32,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub next_repeat: Instant,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ResizeEdge {
    None,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub struct TypdApp {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell: LayerShell,
    pub shm: Shm,
    pub seat_state: SeatState,
    pub wl_surface: Option<WlSurface>,
    pub layer_surface: Option<LayerSurface>,
    pub pool: Option<SlotPool>,
    pub width: u32,
    pub height: u32,
    pub configured: bool,
    pub running: bool,

    // Redraw throttling
    pub frame_pending: bool,

    // Cursor shape
    pub cursor_shape_manager: Option<wp_cursor_shape_manager_v1::WpCursorShapeManagerV1>,
    pub cursor_shape_device: Option<wp_cursor_shape_device_v1::WpCursorShapeDeviceV1>,

    // Seat and Pointer
    pub seat: Option<wl_seat::WlSeat>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_pos: (f64, f64),
    pub pointer_enter_serial: u32,

    // Dragging state
    pub dragging: bool,
    pub drag_start_ptr: (f64, f64),
    pub drag_start_margin: (i32, i32),
    pub margin_x: i32,
    pub margin_y: i32,

    // Resize state
    pub resize_edge: ResizeEdge,
    pub resize_start_ptr: (f64, f64), // Screen-space coords
    pub resize_start_size: (u32, u32),
    pub resize_start_margin: (i32, i32),

    // Virtual keyboard protocol
    pub vkbd_manager: Option<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1>,
    pub vkbd: Option<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1>,
    pub keymap_uploaded: bool,
    pub vkbd_error_logged: bool,

    // Keyboard state
    pub shift_active: bool,
    pub caps_active: bool,
    pub ctrl_active: bool,
    pub alt_active: bool,
    pub num_active: bool,
    pub sidebar_expanded: bool,
    pub keys: Vec<ComputedKey>,
    pub hover_key_id: Option<usize>,
    pub hover_key_started: Option<Instant>,
    pub active_key_id: Option<usize>,
    pub active_key_until: Option<Instant>,
    pub held_key: Option<HeldKey>,
}

delegate_compositor!(TypdApp);
delegate_output!(TypdApp);
delegate_shm!(TypdApp);
delegate_layer!(TypdApp);
delegate_registry!(TypdApp);
delegate_seat!(TypdApp);

impl CompositorHandler for TypdApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _time: u32,
    ) {
        self.frame_pending = false;
        if self.configured {
            self.draw(qh);
        }
    }
}

impl OutputHandler for TypdApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ShmHandler for TypdApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl SeatHandler for TypdApp {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        if self.seat.is_none() {
            self.seat = Some(seat);
            if !self.keymap_uploaded {
                self.upload_keymap(qh);
                let _ = conn.flush();
            }
        }
    }
    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = seat.get_pointer(qh, ());
            if let Some(manager) = &self.cursor_shape_manager {
                self.cursor_shape_device = Some(manager.get_pointer(&pointer, qh, ()));
            }
            self.pointer = Some(pointer);
        }
    }
    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            self.pointer = None;
            self.cursor_shape_device = None;
        }
    }
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        if self.seat.as_ref() == Some(&seat) {
            self.seat = None;
            self.pointer = None;
            self.cursor_shape_device = None;
        }
    }
}

impl ProvidesRegistryState for TypdApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

impl LayerShellHandler for TypdApp {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.running = false;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let width = if configure.new_size.0 == 0 {
            self.width
        } else {
            configure.new_size.0
        };
        let height = if configure.new_size.1 == 0 {
            self.height
        } else {
            configure.new_size.1
        };
        self.width = width;
        self.height = height;
        self.configured = true;

        if !self.keymap_uploaded {
            self.upload_keymap(qh);
            let _ = conn.flush();
        }
        self.keys =
            layout::compute_layout(self.width as f64, self.height as f64, self.sidebar_expanded);

        if !self.frame_pending {
            self.draw(qh);
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for TypdApp {
    fn event(
        app: &mut Self,
        _pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                serial,
                surface_x,
                surface_y,
                ..
            } => {
                app.pointer_enter_serial = serial;
                app.pointer_pos = (surface_x, surface_y);
                app.update_hover(qh);
                app.update_cursor();
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                app.pointer_pos = (surface_x, surface_y);
                app.update_hover(qh);
                app.update_cursor();

                if app.resize_edge != ResizeEdge::None {
                    // 1. Current screen position (Margin is Bottom-Left anchor)
                    let cur_screen_x = app.margin_x as f64 + surface_x;
                    let cur_screen_y = app.margin_y as f64 + (app.height as f64 - surface_y);

                    // 2. Start screen position
                    let start_screen_x = app.resize_start_ptr.0;
                    let start_screen_y = app.resize_start_ptr.1;

                    // 3. Deltas in screen space
                    let dx = cur_screen_x - start_screen_x;
                    let dy = cur_screen_y - start_screen_y;

                    let sw = app.resize_start_size.0 as f64;
                    let sh = app.resize_start_size.1 as f64;
                    let smx = app.resize_start_margin.0 as f64;
                    let smy = app.resize_start_margin.1 as f64;
                    let ar = sw / sh;

                    let mut new_w = sw;
                    let mut new_h = sh;

                    // Calculate "raw" size change based on edge
                    match app.resize_edge {
                        ResizeEdge::Right => {
                            new_w = sw + dx;
                            new_h = new_w / ar;
                        }
                        ResizeEdge::Left => {
                            new_w = sw - dx;
                            new_h = new_w / ar;
                        }
                        ResizeEdge::Top => {
                            new_h = sh + dy;
                            new_w = new_h * ar;
                        }
                        ResizeEdge::Bottom => {
                            new_h = sh - dy;
                            new_w = new_h * ar;
                        }
                        ResizeEdge::TopRight => {
                            let d = dx.max(dy);
                            new_w = sw + d;
                            new_h = new_w / ar;
                        }
                        ResizeEdge::TopLeft => {
                            let d = (-dx).max(dy);
                            new_w = sw + d;
                            new_h = new_w / ar;
                        }
                        ResizeEdge::BottomRight => {
                            let d = dx.max(-dy);
                            new_w = sw + d;
                            new_h = new_w / ar;
                        }
                        ResizeEdge::BottomLeft => {
                            let d = (-dx).max(-dy);
                            new_w = sw + d;
                            new_h = new_w / ar;
                        }
                        _ => {}
                    }

                    // Apply constraints
                    if new_w < MIN_WIDTH {
                        new_w = MIN_WIDTH;
                        new_h = new_w / ar;
                    }
                    if new_h < MIN_HEIGHT {
                        new_h = MIN_HEIGHT;
                        new_w = new_h * ar;
                    }

                    // Anchoring logic: maintain fixed point
                    let mut new_mx = smx;
                    let mut new_my = smy;

                    match app.resize_edge {
                        ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => {
                            // Anchor RIGHT edge
                            new_mx = (smx + sw) - new_w;
                        }
                        _ => {}
                    }
                    match app.resize_edge {
                        ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => {
                            // Anchor TOP edge
                            new_my = (smy + sh) - new_h;
                        }
                        _ => {}
                    }

                    app.width = new_w.round() as u32;
                    app.height = new_h.round() as u32;
                    app.margin_x = new_mx.round() as i32;
                    app.margin_y = new_my.round() as i32;

                    app.keys = layout::compute_layout(
                        app.width as f64,
                        app.height as f64,
                        app.sidebar_expanded,
                    );

                    if let Some(ls) = &app.layer_surface {
                        ls.set_size(app.width, app.height);
                        ls.set_margin(0, 0, app.margin_y, app.margin_x);
                        if let Some(surface) = &app.wl_surface {
                            surface.commit();
                        }
                    }
                } else if app.dragging {
                    let cur_screen_x = app.margin_x as f64 + surface_x;
                    let cur_screen_y = app.margin_y as f64 + (app.height as f64 - surface_y);

                    let dx = cur_screen_x - app.drag_start_ptr.0;
                    let dy = cur_screen_y - app.drag_start_ptr.1;

                    app.margin_x = (app.drag_start_margin.0 as f64 + dx).round() as i32;
                    app.margin_y = (app.drag_start_margin.1 as f64 + dy).round() as i32;

                    if let Some(ls) = &app.layer_surface {
                        ls.set_margin(0, 0, app.margin_y, app.margin_x);
                        if let Some(surface) = &app.wl_surface {
                            surface.commit();
                        }
                    }
                }
            }
            wl_pointer::Event::Button { button, state, .. } => {
                use wl_pointer::ButtonState;
                let (px, py) = app.pointer_pos;

                match state {
                    WEnum::Value(ButtonState::Pressed) => {
                        if button == 272 {
                            let w = app.width as f64;
                            // 1. Close button
                            if px >= w - 32.0 && py <= 32.0 {
                                app.running = false;
                                return;
                            }

                            // 2. Edge resize
                            let edge = app.hit_edge(px, py);
                            if edge != ResizeEdge::None {
                                app.resize_edge = edge;
                                // Capture start screen-space coords
                                let sx = app.margin_x as f64 + px;
                                let sy = app.margin_y as f64 + (app.height as f64 - py);
                                app.resize_start_ptr = (sx, sy);
                                app.resize_start_size = (app.width, app.height);
                                app.resize_start_margin = (app.margin_x, app.margin_y);
                                return;
                            }

                            // 3. Title bar drag
                            if py <= DRAG_BAR_HEIGHT {
                                app.dragging = true;
                                let sx = app.margin_x as f64 + px;
                                let sy = app.margin_y as f64 + (app.height as f64 - py);
                                app.drag_start_ptr = (sx, sy);
                                app.drag_start_margin = (app.margin_x, app.margin_y);
                                return;
                            }

                            // 4. Key hit test
                            if !app.keys.is_empty() {
                                let mut hit_key = None;
                                for key in &app.keys {
                                    if px >= key.x
                                        && px <= key.x + key.w
                                        && py >= key.y
                                        && py <= key.y + key.h
                                    {
                                        hit_key = Some(key.clone());
                                        break;
                                    }
                                }

                                if let Some(key) = hit_key {
                                    app.press_key(key, conn, qh);
                                    app.draw(qh);
                                    return;
                                }
                            }
                        }
                    }
                    WEnum::Value(ButtonState::Released) => {
                        app.dragging = false;
                        app.resize_edge = ResizeEdge::None;
                        app.held_key = None;
                    }
                    _ => {}
                }
            }
            wl_pointer::Event::Leave { .. } => {
                app.dragging = false;
                app.resize_edge = ResizeEdge::None;
                app.held_key = None;
                app.hover_key_id = None;
            }
            _ => {}
        }
    }
}

impl Dispatch<wp_cursor_shape_manager_v1::WpCursorShapeManagerV1, ()> for TypdApp {
    fn event(
        _: &mut Self,
        _: &wp_cursor_shape_manager_v1::WpCursorShapeManagerV1,
        _: wp_cursor_shape_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_cursor_shape_device_v1::WpCursorShapeDeviceV1, ()> for TypdApp {
    fn event(
        _: &mut Self,
        _: &wp_cursor_shape_device_v1::WpCursorShapeDeviceV1,
        _: wp_cursor_shape_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, ()> for TypdApp {
    fn event(
        _: &mut Self,
        _: &zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
        _: zwp_virtual_keyboard_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1, ()> for TypdApp {
    fn event(
        _: &mut Self,
        _: &zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
        _: zwp_virtual_keyboard_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl TypdApp {
    pub fn new(globals: &GlobalList, qh: &QueueHandle<Self>) -> Self {
        let compositor_state = CompositorState::bind(globals, qh).unwrap();
        let layer_shell = LayerShell::bind(globals, qh).unwrap();
        let shm = Shm::bind(globals, qh).unwrap();
        let seat_state = SeatState::new(globals, qh);
        let initial_seat = seat_state.seats().next();

        let cursor_shape_manager = globals
            .bind::<wp_cursor_shape_manager_v1::WpCursorShapeManagerV1, _, _>(qh, 1..=1, ())
            .ok();

        let vkbd_manager = globals
            .bind::<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, _, _>(
                qh,
                1..=1,
                (),
            )
            .ok();

        let wl_surface = compositor_state.create_surface(qh);

        let layer_surface = layer_shell.create_layer_surface(
            qh,
            wl_surface.clone(),
            Layer::Top,
            Some("typd"),
            None,
        );

        use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity};
        layer_surface.set_anchor(Anchor::BOTTOM | Anchor::LEFT);
        layer_surface.set_size(SURFACE_WIDTH, SURFACE_HEIGHT);
        layer_surface.set_exclusive_zone(0);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

        let initial_margin_x = 320;
        let initial_margin_y = 60;
        layer_surface.set_margin(0, 0, initial_margin_y, initial_margin_x);
        wl_surface.commit();

        let pool = SlotPool::new(1920 * 1080 * 4, &shm).unwrap();

        let mut app = Self {
            registry_state: RegistryState::new(globals),
            output_state: OutputState::new(globals, qh),
            compositor_state,
            layer_shell,
            shm,
            seat_state,
            wl_surface: Some(wl_surface),
            layer_surface: Some(layer_surface),
            pool: Some(pool),
            width: SURFACE_WIDTH,
            height: SURFACE_HEIGHT,
            configured: false,
            running: true,
            frame_pending: false,

            cursor_shape_manager,
            cursor_shape_device: None,

            seat: initial_seat,
            pointer: None,
            pointer_pos: (0.0, 0.0),
            pointer_enter_serial: 0,

            dragging: false,
            drag_start_ptr: (0.0, 0.0),
            drag_start_margin: (0, 0),
            margin_x: initial_margin_x,
            margin_y: initial_margin_y,

            resize_edge: ResizeEdge::None,
            resize_start_ptr: (0.0, 0.0),
            resize_start_size: (SURFACE_WIDTH, SURFACE_HEIGHT),
            resize_start_margin: (0, 0),

            vkbd_manager,
            vkbd: None,
            keymap_uploaded: false,
            vkbd_error_logged: false,

            shift_active: false,
            caps_active: false,
            ctrl_active: false,
            alt_active: false,
            num_active: false,
            sidebar_expanded: false,
            keys: Vec::new(),
            hover_key_id: None,
            hover_key_started: None,
            active_key_id: None,
            active_key_until: None,
            held_key: None,
        };

        app.upload_keymap(qh);
        app
    }

    fn upload_keymap(&mut self, qh: &QueueHandle<Self>) {
        use std::io::Write;
        use xkbcommon::xkb;

        let seat = match &self.seat {
            Some(s) => s.clone(),
            None => return,
        };
        let manager: zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1 =
            match self.vkbd_manager.as_ref() {
                Some(m) => m.clone(),
                None => {
                    self.log_vkbd_unavailable(
                        "compositor did not advertise zwp_virtual_keyboard_manager_v1",
                    );
                    return;
                }
            };

        if self.vkbd.is_none() {
            self.vkbd = Some(manager.create_virtual_keyboard(&seat, qh, ()));
        }

        let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap =
            xkb::Keymap::new_from_names(&ctx, "", "", "us", "", None, xkb::KEYMAP_COMPILE_NO_FLAGS)
                .expect("xkb keymap failed");

        let keymap_str = keymap.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1);
        let keymap_bytes = keymap_str.as_bytes();
        let len = keymap_bytes.len();

        let fd = unsafe {
            libc::memfd_create(
                b"typd-keymap\0".as_ptr() as *const libc::c_char,
                libc::MFD_CLOEXEC,
            )
        };
        if fd < 0 {
            self.log_vkbd_unavailable("memfd_create failed for xkb keymap");
            return;
        }

        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        file.write_all(keymap_bytes).unwrap();

        use std::os::unix::io::IntoRawFd;
        let fd2 = file.into_raw_fd();
        let owned_fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd2) };

        self.vkbd.as_ref().unwrap().keymap(
            wl_keyboard::KeymapFormat::XkbV1.into(),
            owned_fd.as_fd(),
            len as u32,
        );

        self.keymap_uploaded = true;
    }

    fn press_key(&mut self, key: ComputedKey, conn: &Connection, qh: &QueueHandle<Self>) {
        self.active_key_id = Some(key.id);
        self.active_key_until = Some(Instant::now() + Duration::from_millis(110));

        match key.def.action {
            KeyAction::ToggleSidebar => {
                let expanding = !self.sidebar_expanded;
                self.sidebar_expanded = expanding;
                let delta = layout::sidebar_delta();
                self.width = if expanding {
                    self.width.saturating_add(delta)
                } else {
                    self.width.saturating_sub(delta).max(MIN_WIDTH as u32)
                };
                self.keys = layout::compute_layout(
                    self.width as f64,
                    self.height as f64,
                    self.sidebar_expanded,
                );
                self.hover_key_id = None;
                if let Some(ls) = &self.layer_surface {
                    ls.set_size(self.width, self.height);
                    ls.set_margin(0, 0, self.margin_y, self.margin_x);
                    if let Some(surface) = &self.wl_surface {
                        surface.commit();
                    }
                }
            }
            KeyAction::Shift => {
                self.shift_active = !self.shift_active;
            }
            KeyAction::Caps => {
                self.caps_active = !self.caps_active;
            }
            KeyAction::Ctrl => {
                self.ctrl_active = !self.ctrl_active;
            }
            KeyAction::Alt => {
                self.alt_active = !self.alt_active;
            }
            KeyAction::NumLock => {
                self.num_active = !self.num_active;
                if let Some(keycode) = key.def.linux_keycode {
                    self.inject_key(keycode, self.shift_active, self.ctrl_active, self.alt_active, conn);
                }
            }
            KeyAction::Key => {
                let Some(keycode) = key.def.linux_keycode else {
                    return;
                };

                let shift = self.effective_shift_for_key(keycode);
                self.inject_key(keycode, shift, self.ctrl_active, self.alt_active, conn);
                if self.shift_active && !Self::is_modifier_key(keycode) {
                    self.shift_active = false;
                }

                if Self::is_repeatable_key(keycode) {
                    self.held_key = Some(HeldKey {
                        key_id: key.id,
                        keycode,
                        shift,
                        ctrl: self.ctrl_active,
                        alt: self.alt_active,
                        next_repeat: Instant::now() + INITIAL_REPEAT_DELAY,
                    });
                }
            }
        }

        self.draw(qh);
    }

    fn inject_key(&mut self, keycode: u32, shift: bool, ctrl: bool, alt: bool, conn: &Connection) {
        if !self.keymap_uploaded {
            self.log_vkbd_unavailable("keymap has not been uploaded");
            return;
        }
        let vkbd: &zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1 = match &self.vkbd {
            Some(v) => v,
            None => {
                self.log_vkbd_unavailable("virtual keyboard object was not created");
                return;
            }
        };
        let t = Self::now_ms();

        let mods = Self::modifier_mask(shift, ctrl, alt);
        vkbd.modifiers(mods, 0, 0, 0);

        vkbd.key(t, keycode, 1); // pressed
        vkbd.key(t + 8, keycode, 0); // released

        if mods != 0 {
            vkbd.modifiers(0, 0, 0, 0);
        }
        if let Err(err) = conn.flush() {
            eprintln!("typd: failed to flush virtual keyboard events: {err}");
        }
    }

    pub fn tick(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        let now = Instant::now();
        if self.active_key_until.is_some_and(|until| now >= until) {
            self.active_key_id = None;
            self.active_key_until = None;
            self.draw(qh);
        }

        let Some(held) = self.held_key.clone() else {
            return;
        };
        if now < held.next_repeat {
            return;
        }

        self.active_key_id = Some(held.key_id);
        self.active_key_until = Some(now + Duration::from_millis(80));
        self.inject_key(held.keycode, held.shift, held.ctrl, held.alt, conn);

        if let Some(current) = &mut self.held_key {
            current.next_repeat = now + REPEAT_INTERVAL;
        }
        self.draw(qh);
    }

    fn is_repeatable_key(keycode: u32) -> bool {
        matches!(
            keycode,
            2..=53 | 57 | 111 | 103 | 105 | 106 | 108 | 104 | 109
        )
    }

    fn is_modifier_key(keycode: u32) -> bool {
        matches!(keycode, 29 | 42 | 54 | 56 | 97 | 100 | 125)
    }

    fn modifier_mask(shift: bool, ctrl: bool, alt: bool) -> u32 {
        (if shift { 1 } else { 0 }) | (if ctrl { 4 } else { 0 }) | (if alt { 8 } else { 0 })
    }

    fn effective_shift_for_key(&self, keycode: u32) -> bool {
        if layout::is_alpha_key(keycode) {
            self.shift_active ^ self.caps_active
        } else {
            self.shift_active
        }
    }

    fn update_hover(&mut self, qh: &QueueHandle<Self>) {
        let (px, py) = self.pointer_pos;
        let hovered = self
            .keys
            .iter()
            .find(|key| px >= key.x && px <= key.x + key.w && py >= key.y && py <= key.y + key.h)
            .map(|key| key.id);

        if self.hover_key_id != hovered {
            self.hover_key_id = hovered;
            self.draw(qh);
        }
    }

    fn vkbd_ready(&self) -> bool {
        self.vkbd_manager.is_some() && self.vkbd.is_some() && self.keymap_uploaded
    }

    fn log_vkbd_unavailable(&mut self, reason: &str) {
        if !self.vkbd_error_logged {
            eprintln!("typd: virtual keyboard unavailable: {reason}");
            eprintln!("typd: on wlroots compositors this usually requires the virtual-keyboard protocol and compositor policy permission");
            self.vkbd_error_logged = true;
        }
    }

    fn now_ms() -> u32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        (d.as_secs() * 1000 + d.subsec_millis() as u64) as u32
    }

    fn hit_edge(&self, px: f64, py: f64) -> ResizeEdge {
        let w = self.width as f64;
        let h = self.height as f64;
        let t = 12.0; // Thicker hit area

        let on_left = px <= t;
        let on_right = px >= w - t;
        let on_top = py <= t;
        let on_bottom = py >= h - t;

        match (on_left, on_right, on_top, on_bottom) {
            (true, false, true, false) => ResizeEdge::TopLeft,
            (false, true, true, false) => ResizeEdge::TopRight,
            (true, false, false, true) => ResizeEdge::BottomLeft,
            (false, true, false, true) => ResizeEdge::BottomRight,
            (true, false, false, false) => ResizeEdge::Left,
            (false, true, false, false) => ResizeEdge::Right,
            (false, false, true, false) => ResizeEdge::Top,
            (false, false, false, true) => ResizeEdge::Bottom,
            _ => ResizeEdge::None,
        }
    }

    fn update_cursor(&self) {
        if let Some(device) = &self.cursor_shape_device {
            use wp_cursor_shape_device_v1::Shape;
            let (px, py) = self.pointer_pos;
            let edge = self.hit_edge(px, py);

            let shape = if self.resize_edge != ResizeEdge::None || edge != ResizeEdge::None {
                let active_edge = if self.resize_edge != ResizeEdge::None {
                    self.resize_edge
                } else {
                    edge
                };
                match active_edge {
                    ResizeEdge::Left => Shape::WResize,
                    ResizeEdge::Right => Shape::EResize,
                    ResizeEdge::Top => Shape::NResize,
                    ResizeEdge::Bottom => Shape::SResize,
                    ResizeEdge::TopLeft => Shape::NwResize,
                    ResizeEdge::TopRight => Shape::NeResize,
                    ResizeEdge::BottomLeft => Shape::SwResize,
                    ResizeEdge::BottomRight => Shape::SeResize,
                    ResizeEdge::None => Shape::Default,
                }
            } else if py <= DRAG_BAR_HEIGHT {
                if px >= self.width as f64 - 32.0 {
                    Shape::Pointer
                } else {
                    Shape::Grab
                }
            } else {
                Shape::Default
            };

            device.set_shape(self.pointer_enter_serial, shape.into());
        }
    }

    pub fn draw(&mut self, qh: &QueueHandle<Self>) {
        if self.frame_pending {
            return;
        }

        let width = self.width as i32;
        let height = self.height as i32;
        let stride = width * 4;

        let pool = self.pool.as_mut().unwrap();
        let (buffer, canvas) =
            match pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888) {
                Ok(b) => b,
                Err(_) => return,
            };

        let cairo_surface = unsafe {
            match cairo::ImageSurface::create_for_data_unsafe(
                canvas.as_mut_ptr(),
                cairo::Format::ARgb32,
                width,
                height,
                stride,
            ) {
                Ok(s) => s,
                Err(_) => return,
            }
        };
        let cr = match cairo::Context::new(&cairo_surface) {
            Ok(c) => c,
            Err(_) => return,
        };

        if self
            .active_key_until
            .is_some_and(|until| Instant::now() >= until)
        {
            self.active_key_id = None;
            self.active_key_until = None;
        }

        renderer::draw_keyboard(
            &cr,
            width,
            height,
            &self.keys,
            self.shift_active,
            self.caps_active,
            self.ctrl_active,
            self.alt_active,
            self.num_active,
            self.hover_key_id,
            self.active_key_id,
            self.vkbd_ready(),
        );

        drop(cr);
        drop(cairo_surface);

        let wl_surface = self.wl_surface.as_ref().unwrap();

        wl_surface.frame(qh, wl_surface.clone());
        self.frame_pending = true;

        let _ = buffer.attach_to(wl_surface);
        wl_surface.damage_buffer(0, 0, width, height);
        wl_surface.commit();
    }
}
