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
use wayland_protocols::wp::input_method::zv1::client::{
    zwp_input_method_context_v1, zwp_input_method_v1,
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
const MIN_WIDTH: f64 = 900.0;
const MIN_HEIGHT: f64 = 300.0;
const MAX_WIDTH: f64 = SURFACE_WIDTH as f64;
const MAX_HEIGHT: f64 = SURFACE_HEIGHT as f64;
const INITIAL_REPEAT_DELAY: Duration = Duration::from_millis(430);
const REPEAT_INTERVAL: Duration = Duration::from_millis(55);

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
    #[allow(dead_code)]
    pub compositor_state: CompositorState,
    #[allow(dead_code)]
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
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    #[allow(dead_code)]
    pub input_method: Option<zwp_input_method_v1::ZwpInputMethodV1>,
    pub input_method_context: Option<zwp_input_method_context_v1::ZwpInputMethodContextV1>,
    pub ime_keyboard: Option<wl_keyboard::WlKeyboard>,
    pub pointer_pos: (f64, f64),
    pub pointer_enter_serial: u32,

    // Dragging state
    pub dragging: bool,
    // Screen-space anchor captured at drag start: (margin_x_at_start, margin_y_at_start, surface_x_at_start, surface_y_at_start)
    pub drag_anchor: (i32, i32, f64, f64),
    pub drag_last_ptr: (f64, f64),
    pub margin_x: i32,
    pub margin_y: i32,

    // Resize state
    pub resize_edge: ResizeEdge,
    pub resize_start_ptr: (f64, f64), // Screen-space coords
    pub resize_start_size: (u32, u32),
    pub resize_start_margin: (i32, i32),
    pub resize_last_ptr: (f64, f64),

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
    pub physical_shift_active: bool,
    pub physical_caps_active: bool,
    pub physical_ctrl_active: bool,
    pub physical_alt_active: bool,
    pub physical_num_active: bool,
    pub physical_pressed_key_ids: Vec<usize>,
    pub physical_active_key_id: Option<usize>,

    // Sidebar Animation
    pub sidebar_expanded: bool,     // Current logical state
    pub sidebar_anim_progress: f32, // 0.0 to 1.0
    pub sidebar_anim_speed: f32,
    pub last_frame_time: Instant,
    pub sidebar_toggle_cooldown: Option<Instant>, // debounce rapid clicks

    pub keys: Vec<ComputedKey>,
    pub hover_key_id: Option<usize>,
    pub active_key_id: Option<usize>,

    // Sidebar Surface
    pub sidebar_wl_surface: Option<WlSurface>,
    pub sidebar_layer_surface: Option<LayerSurface>,
    pub sidebar_keys: Vec<ComputedKey>,
    pub sidebar_hover_key_id: Option<usize>,
    pub sidebar_active_key_id: Option<usize>,

    pub active_key_until: Option<Instant>,
    pub held_key: Option<HeldKey>,

    // Pending updates for throttling
    pub pending_margin: Option<(i32, i32)>,
    pub pending_size: Option<(u32, u32)>,
    pub pending_sidebar_margin: Option<(i32, i32)>,
    pub pending_sidebar_size: Option<(u32, u32)>,

    pub active_pointer_surface: Option<WlSurface>,

    // Surface Synchronization & State Guards
    pub sidebar_configured: bool,
    pub sidebar_visible: bool,
    pub sidebar_animating: bool,
    // Tracks whether we have unmapped the sidebar and need a new configure before re-mapping
    pub sidebar_needs_configure: bool,
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
        } else if capability == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = Some(seat.get_keyboard(qh, ()));
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
        } else if capability == Capability::Keyboard {
            self.keyboard = None;
            self.clear_physical_keyboard_state();
        }
    }
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        if self.seat.as_ref() == Some(&seat) {
            self.seat = None;
            self.pointer = None;
            self.keyboard = None;
            self.ime_keyboard = None;
            self.input_method_context = None;
            self.cursor_shape_device = None;
            self.clear_physical_keyboard_state();
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
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        if let Some(main_ls) = &self.layer_surface {
            if main_ls == layer {
                if configure.new_size.0 != 0 {
                    self.width = configure.new_size.0;
                }
                if configure.new_size.1 != 0 {
                    self.height = configure.new_size.1;
                }
                self.configured = true;

                if !self.keymap_uploaded {
                    self.upload_keymap(qh);
                    let _ = conn.flush();
                }

                self.keys = layout::compute_main_layout(self.width as f64, self.height as f64);
                self.sidebar_keys =
                    layout::compute_sidebar_layout(self.height as f64, self.keys.len());

                // Trigger initial draw
                self.draw(qh);
                return;
            }
        }

        if let Some(sidebar_ls) = &self.sidebar_layer_surface {
            if sidebar_ls == layer {
                // Sidebar configuration received — safe to map with a buffer now.
                self.sidebar_configured = true;
                self.sidebar_needs_configure = false;
                self.sidebar_keys =
                    layout::compute_sidebar_layout(self.height as f64, self.keys.len());

                if self.sidebar_anim_progress > 0.001 {
                    self.draw(qh);
                }
            }
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
                surface,
                ..
            } => {
                app.pointer_enter_serial = serial;
                app.pointer_pos = (surface_x, surface_y);
                app.active_pointer_surface = Some(surface);
                if app.dragging {
                    app.drag_last_ptr = (surface_x, surface_y);
                }
                if app.resize_edge != ResizeEdge::None {
                    app.resize_last_ptr = (surface_x, surface_y);
                }
                app.update_hover(qh);
                app.update_cursor();
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                app.pointer_pos = (surface_x, surface_y);

                if app.resize_edge != ResizeEdge::None {
                    let (last_x, last_y) = app.resize_last_ptr;
                    let dx = surface_x - last_x;
                    let dy = last_y - surface_y;
                    app.resize_last_ptr = (surface_x, surface_y);

                    let ar = app.width as f64 / app.height as f64;
                    let mut new_w = app.width as f64;
                    let mut new_h = app.height as f64;
                    let mut new_mx = app.margin_x as f64;
                    let mut new_my = app.margin_y as f64;

                    match app.resize_edge {
                        ResizeEdge::Right => {
                            new_w += dx;
                            new_h = new_w / ar;
                        }
                        ResizeEdge::Left => {
                            new_w -= dx;
                            new_h = new_w / ar;
                            new_mx += dx;
                        }
                        ResizeEdge::Top => {
                            new_h += dy;
                            new_w = new_h * ar;
                        }
                        ResizeEdge::Bottom => {
                            new_h -= dy;
                            new_w = new_h * ar;
                            new_my += dy;
                        }
                        ResizeEdge::TopRight => {
                            let d = dx.max(dy);
                            new_w += d;
                            new_h = new_w / ar;
                        }
                        ResizeEdge::TopLeft => {
                            let d = (-dx).max(dy);
                            new_w += d;
                            new_h = new_w / ar;
                            new_mx += dx;
                        }
                        ResizeEdge::BottomRight => {
                            let d = dx.max(-dy);
                            new_w += d;
                            new_h = new_w / ar;
                            new_my += dy;
                        }
                        ResizeEdge::BottomLeft => {
                            let d = (-dx).max(-dy);
                            new_w += d;
                            new_h = new_w / ar;
                            new_mx += dx;
                            new_my += dy;
                        }
                        _ => {}
                    }

                    new_w = new_w.max(1.0);
                    new_h = new_h.max(1.0);
                    let min_scale = (MIN_WIDTH / new_w).max(MIN_HEIGHT / new_h).max(1.0);
                    let max_scale = (MAX_WIDTH / new_w).min(MAX_HEIGHT / new_h).min(1.0);
                    let scale = min_scale.min(max_scale);
                    new_w *= scale;
                    new_h *= scale;
                    new_w = new_w.clamp(MIN_WIDTH, MAX_WIDTH);
                    new_h = new_h.clamp(MIN_HEIGHT, MAX_HEIGHT);
                    if new_w >= MAX_WIDTH {
                        new_w = MAX_WIDTH;
                        new_h = (new_w / ar).min(MAX_HEIGHT);
                    }
                    if new_h >= MAX_HEIGHT {
                        new_h = MAX_HEIGHT;
                        new_w = (new_h * ar).min(MAX_WIDTH);
                    }

                    app.width = new_w.round() as u32;
                    app.height = new_h.round() as u32;
                    app.margin_x = new_mx.round() as i32;
                    app.margin_y = new_my.round() as i32;
                    app.pending_size = Some((app.width, app.height));
                    app.pending_margin = Some((app.margin_x, app.margin_y));
                    app.pending_sidebar_size = Some((
                        (layout::SIDEBAR_WIDTH * app.sidebar_anim_progress as f64).max(1.0) as u32,
                        app.height,
                    ));
                    app.pending_sidebar_margin = Some((app.sidebar_margin_x(), app.margin_y));

                    app.keys = layout::compute_main_layout(app.width as f64, app.height as f64);
                    app.sidebar_keys =
                        layout::compute_sidebar_layout(app.height as f64, app.keys.len());
                    app.draw(qh);
                } else if app.dragging {
                    let (last_x, last_y) = app.drag_last_ptr;
                    let dx = surface_x - last_x;
                    let dy = last_y - surface_y;
                    app.drag_last_ptr = (surface_x, surface_y);

                    let new_mx = app.margin_x + dx.round() as i32;
                    let new_my = app.margin_y + dy.round() as i32;

                    if new_mx != app.margin_x || new_my != app.margin_y {
                        app.margin_x = new_mx;
                        app.margin_y = new_my;
                        app.pending_margin = Some((app.margin_x, app.margin_y));

                        // Sidebar follows the main keyboard
                        let sb_width = (layout::SIDEBAR_WIDTH * app.sidebar_anim_progress as f64)
                            .max(1.0) as u32;
                        app.pending_sidebar_margin = Some((app.sidebar_margin_x(), app.margin_y));
                        app.pending_sidebar_size = Some((sb_width, app.height));

                        app.draw(qh);
                    }
                } else {
                    app.update_hover(qh);
                    app.update_cursor();
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
                                app.resize_last_ptr = (px, py);
                                return;
                            }

                            // 3. Title bar drag
                            if py <= DRAG_BAR_HEIGHT {
                                app.dragging = true;
                                // Anchor: capture start margin AND start surface coords.
                                // Delta motion is computed as (current_surface - start_surface)
                                // which is immune to compositor lag in applying margin changes.
                                app.drag_anchor = (app.margin_x, app.margin_y, px, py);
                                app.drag_last_ptr = (px, py);
                                return;
                            }

                            // 4. Key hit test
                            let mut hit_key = None;
                            let (target_keys, _is_sidebar) = if app.active_pointer_surface.as_ref()
                                == app.sidebar_wl_surface.as_ref()
                            {
                                (&app.sidebar_keys, true)
                            } else {
                                (&app.keys, false)
                            };

                            for key in target_keys {
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
                app.sidebar_hover_key_id = None;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for TypdApp {
    fn event(
        app: &mut Self,
        _keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if app.ime_keyboard.is_some() && app.ime_keyboard.as_ref() != Some(_keyboard) {
            return;
        }
        match event {
            wl_keyboard::Event::Enter { .. } => {}
            wl_keyboard::Event::Leave { .. } => {
                app.clear_physical_keyboard_state();
                app.physical_shift_active = false;
                app.physical_ctrl_active = false;
                app.physical_alt_active = false;
            }
            wl_keyboard::Event::Key {
                serial,
                time,
                key,
                state,
            } => {
                let keycode = key + 8;
                let pressed = matches!(state, WEnum::Value(wl_keyboard::KeyState::Pressed));
                let is_grab = app.ime_keyboard.as_ref() == Some(_keyboard);

                if let Some(id) = app.key_id_for_keycode(keycode) {
                    if pressed {
                        app.mark_physical_key_pressed(id);
                    } else {
                        app.mark_physical_key_released(id);
                    }
                }

                if is_grab {
                    if let Some(context) = &app.input_method_context {
                        let state_u32 = if pressed { 1 } else { 0 };
                        context.key(serial, time, key, state_u32);
                    }
                }

                match keycode {
                    42 | 54 => app.physical_shift_active = pressed,
                    29 | 97 => app.physical_ctrl_active = pressed,
                    56 | 100 => app.physical_alt_active = pressed,
                    58 => {
                        if pressed {
                            app.physical_caps_active = !app.physical_caps_active;
                        }
                    }
                    69 => {
                        if pressed {
                            app.physical_num_active = !app.physical_num_active;
                        }
                    }
                    _ => {}
                }

                app.draw(qh);
            }
            wl_keyboard::Event::Modifiers {
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                let is_grab = app.ime_keyboard.as_ref() == Some(_keyboard);
                if is_grab {
                    if let Some(context) = &app.input_method_context {
                        context.modifiers(serial, mods_depressed, mods_latched, mods_locked, group);
                    }
                }
                let depressed = mods_depressed | mods_latched;
                app.physical_shift_active = depressed & 0x01 != 0;
                app.physical_ctrl_active = depressed & 0x04 != 0;
                app.physical_alt_active = depressed & 0x08 != 0;
                app.physical_caps_active = mods_locked & 0x02 != 0;
                app.physical_num_active = mods_locked & 0x10 != 0;
                app.draw(qh);
            }
            _ => {}
        }
    }
}

impl Dispatch<zwp_input_method_v1::ZwpInputMethodV1, ()> for TypdApp {
    fn event(
        app: &mut Self,
        input_method: &zwp_input_method_v1::ZwpInputMethodV1,
        event: zwp_input_method_v1::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_input_method_v1::Event::Activate { id } => {
                let context = id.clone();
                app.input_method_context = Some(context);
                let keyboard = id.grab_keyboard(qh, ());
                app.ime_keyboard = Some(keyboard);
                app.draw(qh);
            }
            zwp_input_method_v1::Event::Deactivate { context } => {
                if app
                    .input_method_context
                    .as_ref()
                    .is_some_and(|current| current == &context)
                {
                    app.input_method_context = None;
                    app.ime_keyboard = None;
                    app.clear_physical_keyboard_state();
                    app.draw(qh);
                }
            }
            _ => {
                let _ = input_method;
            }
        }
    }
}

impl Dispatch<zwp_input_method_context_v1::ZwpInputMethodContextV1, ()> for TypdApp {
    fn event(
        _app: &mut Self,
        _context: &zwp_input_method_context_v1::ZwpInputMethodContextV1,
        _event: zwp_input_method_context_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
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
    fn sidebar_margin_x(&self) -> i32 {
        self.margin_x
            .saturating_add(self.width as i32)
            .saturating_add(layout::SIDEBAR_GAP.round() as i32)
    }

    fn mark_physical_key_pressed(&mut self, key_id: usize) {
        if !self.physical_pressed_key_ids.contains(&key_id) {
            self.physical_pressed_key_ids.push(key_id);
        }
        self.physical_active_key_id = Some(key_id);
    }

    fn mark_physical_key_released(&mut self, key_id: usize) {
        self.physical_pressed_key_ids.retain(|id| *id != key_id);
        if self.physical_active_key_id == Some(key_id) {
            self.physical_active_key_id = self.physical_pressed_key_ids.last().copied();
        }
    }

    fn clear_physical_keyboard_state(&mut self) {
        self.physical_pressed_key_ids.clear();
        self.physical_active_key_id = None;
        self.physical_shift_active = false;
        self.physical_caps_active = false;
        self.physical_ctrl_active = false;
        self.physical_alt_active = false;
        self.physical_num_active = false;
    }

    fn key_id_for_keycode(&self, keycode: u32) -> Option<usize> {
        self.keys
            .iter()
            .chain(self.sidebar_keys.iter())
            .find(|key| key.def.linux_keycode == Some(keycode))
            .map(|key| key.id)
    }
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

        let input_method = globals
            .bind::<zwp_input_method_v1::ZwpInputMethodV1, _, _>(qh, 1..=1, ())
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
        layer_surface.set_exclusive_zone(-1); // -1 means no exclusive zone, fixing protocol error
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

        let initial_margin_x = 320;
        let initial_margin_y = 60;
        layer_surface.set_margin(0, 0, initial_margin_y, initial_margin_x);
        wl_surface.commit();

        // Sidebar surface
        let sidebar_wl_surface = compositor_state.create_surface(qh);
        let sidebar_layer_surface = layer_shell.create_layer_surface(
            qh,
            sidebar_wl_surface.clone(),
            Layer::Top,
            Some("typd-sidebar"),
            None,
        );
        sidebar_layer_surface.set_anchor(Anchor::BOTTOM | Anchor::LEFT);
        sidebar_layer_surface.set_size(layout::SIDEBAR_WIDTH as u32, SURFACE_HEIGHT);
        sidebar_layer_surface.set_exclusive_zone(-1); // -1 means no exclusive zone
        sidebar_layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        sidebar_layer_surface.set_margin(
            0,
            0,
            initial_margin_y,
            initial_margin_x
                .saturating_add(SURFACE_WIDTH as i32)
                .saturating_add(layout::SIDEBAR_GAP.round() as i32),
        );
        sidebar_wl_surface.commit();

        let pool = SlotPool::new(1920 * 1080 * 4 * 2, &shm).unwrap(); // Larger pool for two surfaces

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
            input_method,
            input_method_context: None,
            ime_keyboard: None,
            pointer_pos: (0.0, 0.0),
            pointer_enter_serial: 0,

            dragging: false,
            drag_anchor: (0, 0, 0.0, 0.0),
            drag_last_ptr: (0.0, 0.0),
            margin_x: initial_margin_x,
            margin_y: initial_margin_y,

            resize_edge: ResizeEdge::None,
            resize_start_ptr: (0.0, 0.0),
            resize_start_size: (SURFACE_WIDTH, SURFACE_HEIGHT),
            resize_start_margin: (0, 0),
            resize_last_ptr: (0.0, 0.0),

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
            sidebar_anim_progress: 0.0,
            sidebar_anim_speed: 7.2,
            last_frame_time: Instant::now(),
            sidebar_toggle_cooldown: None,

            keys: Vec::new(),
            hover_key_id: None,
            active_key_id: None,

            sidebar_wl_surface: Some(sidebar_wl_surface),
            sidebar_layer_surface: Some(sidebar_layer_surface),
            sidebar_keys: Vec::new(),
            sidebar_hover_key_id: None,
            sidebar_active_key_id: None,

            active_key_until: None,
            held_key: None,

            pending_margin: None,
            pending_size: None,
            pending_sidebar_margin: None,
            pending_sidebar_size: None,

            active_pointer_surface: None,

            keyboard: None,
            physical_shift_active: false,
            physical_caps_active: false,
            physical_ctrl_active: false,
            physical_alt_active: false,
            physical_num_active: false,
            physical_pressed_key_ids: Vec::new(),
            physical_active_key_id: None,

            sidebar_configured: false,
            sidebar_visible: false,
            sidebar_animating: false,
            sidebar_needs_configure: true, // must wait for first configure before mapping
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

    fn press_key(&mut self, key: layout::ComputedKey, conn: &Connection, qh: &QueueHandle<Self>) {
        if key.def.action == layout::KeyAction::ToggleSidebar {
            // Debounce: ignore clicks within 200ms of last toggle to prevent
            // rapid cycling that causes protocol errors on re-map.
            let now = Instant::now();
            if let Some(cooldown_until) = self.sidebar_toggle_cooldown {
                if now < cooldown_until {
                    return;
                }
            }
            self.sidebar_toggle_cooldown = Some(now + Duration::from_millis(200));
            self.sidebar_expanded = !self.sidebar_expanded;
            self.sidebar_animating = true;
            return;
        }

        let is_sidebar = key.id >= self.keys.len();
        if is_sidebar {
            self.sidebar_active_key_id = Some(key.id);
        } else {
            self.active_key_id = Some(key.id);
        }
        self.active_key_until = Some(Instant::now() + Duration::from_millis(100));

        match key.def.action {
            KeyAction::ToggleSidebar => {}
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
                    self.inject_key(
                        keycode,
                        self.shift_active,
                        self.ctrl_active,
                        self.alt_active,
                        conn,
                    );
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
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        // 1. Handle sidebar animation with easing
        let target = if self.sidebar_expanded { 1.0 } else { 0.0 };
        let diff = target - self.sidebar_anim_progress;

        if diff.abs() > 0.001 {
            self.sidebar_animating = true;
            // Use basic easing: move 15% of the remaining distance per frame, or at least anim_speed * dt
            let easing_step = diff * 12.0 * dt;
            let min_step = self.sidebar_anim_speed * dt;

            if diff > 0.0 {
                self.sidebar_anim_progress =
                    (self.sidebar_anim_progress + easing_step.max(min_step)).min(1.0);
            } else {
                self.sidebar_anim_progress =
                    (self.sidebar_anim_progress + easing_step.min(-min_step)).max(0.0);
            }

            // Update sidebar surface properties
            let sb_width =
                (layout::SIDEBAR_WIDTH * self.sidebar_anim_progress as f64).max(1.0) as u32;
            self.pending_sidebar_size = Some((sb_width, self.height));
            self.pending_sidebar_margin = Some((self.sidebar_margin_x(), self.margin_y));

            self.draw(qh);
        } else if self.sidebar_animating {
            // Snap to target if very close
            self.sidebar_anim_progress = target;
            self.sidebar_animating = false;
            self.draw(qh);
        }

        // 2. Handle key release/repeat
        if self.active_key_until.is_some_and(|until| now >= until) {
            self.active_key_id = None;
            self.active_key_until = None;
            self.draw(qh);
        }

        if let Some(held) = self.held_key.clone() {
            if now >= held.next_repeat {
                self.active_key_id = Some(held.key_id);
                self.active_key_until = Some(now + Duration::from_millis(80));
                self.inject_key(held.keycode, held.shift, held.ctrl, held.alt, conn);

                if let Some(current) = &mut self.held_key {
                    current.next_repeat = now + REPEAT_INTERVAL;
                }
                self.draw(qh);
            }
        }
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

        let mut hovered_main = None;
        let mut hovered_sidebar = None;

        if self.active_pointer_surface.as_ref() == self.sidebar_wl_surface.as_ref() {
            hovered_sidebar = self
                .sidebar_keys
                .iter()
                .find(|key| {
                    px >= key.x && px <= key.x + key.w && py >= key.y && py <= key.y + key.h
                })
                .map(|key| key.id);
        } else {
            hovered_main = self
                .keys
                .iter()
                .find(|key| {
                    px >= key.x && px <= key.x + key.w && py >= key.y && py <= key.y + key.h
                })
                .map(|key| key.id);
        }

        let changed =
            self.hover_key_id != hovered_main || self.sidebar_hover_key_id != hovered_sidebar;
        if changed {
            self.hover_key_id = hovered_main;
            self.sidebar_hover_key_id = hovered_sidebar;
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
        self.commit_pending_geometry(false);
        self.commit_pending_geometry(true);

        if self.configured {
            self.draw_surface(qh, false, false);
        }

        if self.sidebar_configured {
            let hidden = self.sidebar_anim_progress <= 0.01;
            if !hidden || self.sidebar_visible || self.sidebar_animating {
                self.draw_surface(qh, true, hidden);
                self.sidebar_visible = !hidden;
            }
        }
    }

    fn commit_pending_geometry(&mut self, is_sidebar: bool) {
        let (layer_surface, wl_surface, pending_size, pending_margin) = if is_sidebar {
            (
                self.sidebar_layer_surface.as_ref(),
                self.sidebar_wl_surface.as_ref(),
                &mut self.pending_sidebar_size,
                &mut self.pending_sidebar_margin,
            )
        } else {
            (
                self.layer_surface.as_ref(),
                self.wl_surface.as_ref(),
                &mut self.pending_size,
                &mut self.pending_margin,
            )
        };

        let mut changed = false;
        if let Some(ls) = layer_surface {
            if let Some((w, h)) = pending_size.take() {
                ls.set_size(w, h);
                changed = true;
            }
            if let Some((mx, my)) = pending_margin.take() {
                ls.set_margin(0, 0, my, mx);
                changed = true;
            }
        }

        if changed {
            if let Some(surface) = wl_surface {
                surface.commit();
            }
        }
    }

    fn draw_surface(&mut self, _qh: &QueueHandle<Self>, is_sidebar: bool, hidden: bool) {
        let vk_ready = self.vkbd_ready();
        let shift = self.shift_active || self.physical_shift_active;
        let caps = self.caps_active || self.physical_caps_active;
        let ctrl = self.ctrl_active || self.physical_ctrl_active;
        let alt = self.alt_active || self.physical_alt_active;
        let num = self.num_active || self.physical_num_active;

        let (
            width,
            height,
            wl_surface,
            _layer_surface,
            keys,
            hover_id,
            active_id,
        ) = if is_sidebar {
            let sb_width = (layout::SIDEBAR_WIDTH * self.sidebar_anim_progress as f64) as u32;
            (
                sb_width.max(1) as i32,
                self.height as i32,
                self.sidebar_wl_surface.as_ref().unwrap(),
                self.sidebar_layer_surface.as_ref(),
                &self.sidebar_keys,
                self.sidebar_hover_key_id,
                self.sidebar_active_key_id,
            )
        } else {
            (
                self.width as i32,
                self.height as i32,
                self.wl_surface.as_ref().unwrap(),
                self.layer_surface.as_ref(),
                &self.keys,
                self.hover_key_id,
                self.active_key_id,
            )
        };

        if width <= 0 || height <= 0 {
            return;
        }

        let stride = width * 4;
        let format = if self.shm.formats().contains(&wl_shm::Format::Xrgb8888) {
            wl_shm::Format::Xrgb8888
        } else if self.shm.formats().contains(&wl_shm::Format::Argb8888) {
            wl_shm::Format::Argb8888
        } else {
            return;
        };
        let cairo_format = match format {
            wl_shm::Format::Xrgb8888 => cairo::Format::Rgb24,
            _ => cairo::Format::ARgb32,
        };
        let pool = self.pool.as_mut().unwrap();
        let (buffer, canvas) =
            match pool.create_buffer(width, height, stride, format) {
                Ok(b) => b,
                Err(_) => return,
            };

        let cairo_surface = unsafe {
            match cairo::ImageSurface::create_for_data_unsafe(
                canvas.as_mut_ptr(),
                cairo_format,
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

        renderer::draw_keyboard(
            &cr,
            width,
            height,
            keys,
            shift,
            caps,
            ctrl,
            alt,
            num,
            hover_id,
            active_id,
            &self.physical_pressed_key_ids,
            vk_ready,
            hidden,
        );

        drop(cr);
        drop(cairo_surface);

        let _ = buffer.attach_to(wl_surface);
        wl_surface.damage_buffer(0, 0, width, height);
        wl_surface.commit();
    }
}
