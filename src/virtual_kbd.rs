use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm, delegate_seat,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::wlr_layer::{
        Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
    seat::{SeatHandler, SeatState, Capability},
};
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_output, wl_surface::WlSurface, wl_seat, wl_pointer},
    Connection, QueueHandle, Dispatch, WEnum,
};
use wayland_protocols::wp::cursor_shape::v1::client::{wp_cursor_shape_manager_v1, wp_cursor_shape_device_v1};
use crate::renderer;

pub const SURFACE_WIDTH:  u32 = 1200;
pub const SURFACE_HEIGHT: u32 = 440;
pub const DRAG_BAR_HEIGHT: f64 = 28.0;
const MIN_WIDTH:  f64 = 400.0;
const MIN_HEIGHT: f64 = 200.0;

#[derive(Clone, Copy, PartialEq)]
pub enum ResizeEdge {
    None,
    Left,
    Right,
    Bottom,
    BottomLeft,
    BottomRight,
}

pub struct TypdApp {
    pub registry_state:   RegistryState,
    pub output_state:     OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell:      LayerShell,
    pub shm:              Shm,
    pub seat_state:       SeatState,
    pub wl_surface:       Option<WlSurface>,
    pub layer_surface:    Option<LayerSurface>,
    pub pool:             Option<SlotPool>,
    pub width:            u32,
    pub height:           u32,
    pub configured:       bool,
    pub running:          bool,

    // Redraw throttling
    pub frame_pending:    bool,

    // Cursor shape
    pub cursor_shape_manager: Option<wp_cursor_shape_manager_v1::WpCursorShapeManagerV1>,
    pub cursor_shape_device:  Option<wp_cursor_shape_device_v1::WpCursorShapeDeviceV1>,

    // Seat and Pointer
    pub seat:             Option<wl_seat::WlSeat>,
    pub pointer:          Option<wl_pointer::WlPointer>,
    pub pointer_pos:      (f64, f64),
    pub pointer_enter_serial: u32,

    // Dragging state
    pub dragging:         bool,
    pub drag_start_ptr:   (f64, f64),
    pub drag_start_margin: (i32, i32),
    pub margin_x:         i32,
    pub margin_y:         i32,

    // Resize state
    pub resize_edge:        ResizeEdge,
    pub resize_start_ptr:   (f64, f64),
    pub resize_start_size:  (u32, u32),
    pub resize_start_margin:(i32, i32),
}

delegate_compositor!(TypdApp);
delegate_output!(TypdApp);
delegate_shm!(TypdApp);
delegate_layer!(TypdApp);
delegate_registry!(TypdApp);
delegate_seat!(TypdApp);

impl CompositorHandler for TypdApp {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _new_factor: i32) {}
    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _new_transform: wl_output::Transform) {}
    fn frame(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _surface: &WlSurface, _time: u32) {
        self.frame_pending = false;
        if self.configured {
            self.draw(qh);
        }
    }
}

impl OutputHandler for TypdApp {
    fn output_state(&mut self) -> &mut OutputState { &mut self.output_state }
    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

impl ShmHandler for TypdApp {
    fn shm_state(&mut self) -> &mut Shm { &mut self.shm }
}

impl SeatHandler for TypdApp {
    fn seat_state(&mut self) -> &mut SeatState { &mut self.seat_state }
    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        if self.seat.is_none() {
            self.seat = Some(seat);
        }
    }
    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = seat.get_pointer(qh, ());
            if let Some(manager) = &self.cursor_shape_manager {
                self.cursor_shape_device = Some(manager.get_pointer(&pointer, qh, ()));
            }
            self.pointer = Some(pointer);
        }
    }
    fn remove_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat, capability: Capability) {
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
    fn registry(&mut self) -> &mut RegistryState { &mut self.registry_state }
    registry_handlers![OutputState, SeatState];
}

impl LayerShellHandler for TypdApp {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.running = false;
    }

    fn configure(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _layer: &LayerSurface, configure: LayerSurfaceConfigure, _serial: u32) {
        let width  = if configure.new_size.0 == 0 { SURFACE_WIDTH } else { configure.new_size.0 };
        let height = if configure.new_size.1 == 0 { SURFACE_HEIGHT } else { configure.new_size.1 };
        self.width  = width;
        self.height = height;
        self.configured = true;
        
        if !self.frame_pending {
            self.draw(qh);
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for TypdApp {
    fn event(app: &mut Self, _pointer: &wl_pointer::WlPointer, event: wl_pointer::Event, _data: &(), _conn: &Connection, _qh: &QueueHandle<Self>) {
        match event {
            wl_pointer::Event::Enter { serial, surface_x, surface_y, .. } => {
                app.pointer_enter_serial = serial;
                app.pointer_pos = (surface_x, surface_y);
                app.update_cursor();
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                app.pointer_pos = (surface_x, surface_y);
                app.update_cursor();

                if app.resize_edge != ResizeEdge::None {
                    let dx = surface_x - app.resize_start_ptr.0;
                    let dy = surface_y - app.resize_start_ptr.1;

                    let start_w  = app.resize_start_size.0 as f64;
                    let start_h  = app.resize_start_size.1 as f64;
                    let start_mx = app.resize_start_margin.0 as f64;
                    let start_my = app.resize_start_margin.1 as f64;

                    let mut new_w  = start_w;
                    let mut new_h  = start_h;
                    let mut new_mx = start_mx;
                    let mut new_my = start_my;

                    match app.resize_edge {
                        ResizeEdge::Right => {
                            new_w = start_w + dx;
                        }
                        ResizeEdge::Left => {
                            new_w  = start_w - dx;
                            new_mx = start_mx + dx;
                        }
                        ResizeEdge::Bottom => {
                            new_h  = start_h + dy;
                        }
                        ResizeEdge::BottomRight => {
                            new_w = start_w + dx;
                            new_h = start_h + dy;
                        }
                        ResizeEdge::BottomLeft => {
                            new_w  = start_w - dx;
                            new_h  = start_h + dy;
                            new_mx = start_mx + dx;
                        }
                        ResizeEdge::None => unreachable!(),
                    }

                    new_w  = new_w.max(MIN_WIDTH);
                    new_h  = new_h.max(MIN_HEIGHT);
                    new_mx = new_mx.max(0.0);
                    new_my = new_my.max(0.0);

                    app.width    = new_w.round()  as u32;
                    app.height   = new_h.round()  as u32;
                    app.margin_x = new_mx.round() as i32;
                    app.margin_y = new_my.round() as i32;

                    if let Some(ls) = &app.layer_surface {
                        ls.set_size(app.width, app.height);
                        ls.set_margin(0, 0, app.margin_y, app.margin_x);
                        if let Some(surface) = &app.wl_surface {
                            surface.commit();
                        }
                    }
                    return;
                }

                if app.dragging {
                    let dx = (surface_x - app.drag_start_ptr.0) as i32;
                    let dy = (surface_y - app.drag_start_ptr.1) as i32;

                    app.margin_x = app.drag_start_margin.0 + dx;
                    app.margin_y = app.drag_start_margin.1 - dy;

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
                            if px >= w - 32.0 && py <= 28.0 {
                                app.running = false;
                                return;
                            }
                            
                            // 2. Edge resize
                            let edge = app.hit_edge(px, py);
                            if edge != ResizeEdge::None {
                                app.resize_edge         = edge;
                                app.resize_start_ptr    = (px, py);
                                app.resize_start_size   = (app.width, app.height);
                                app.resize_start_margin = (app.margin_x, app.margin_y);
                                return;
                            }

                            // 3. Title bar drag
                            if py <= DRAG_BAR_HEIGHT {
                                app.dragging = true;
                                app.drag_start_ptr = (px, py);
                                app.drag_start_margin = (app.margin_x, app.margin_y);
                            }
                        }
                    }
                    WEnum::Value(ButtonState::Released) => {
                        app.dragging = false;
                        app.resize_edge = ResizeEdge::None;
                    }
                    _ => {}
                }
            }
            wl_pointer::Event::Leave { .. } => {
                app.dragging = false;
                app.resize_edge = ResizeEdge::None;
            }
            _ => {}
        }
    }
}

impl Dispatch<wp_cursor_shape_manager_v1::WpCursorShapeManagerV1, ()> for TypdApp {
    fn event(_: &mut Self, _: &wp_cursor_shape_manager_v1::WpCursorShapeManagerV1, _: wp_cursor_shape_manager_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wp_cursor_shape_device_v1::WpCursorShapeDeviceV1, ()> for TypdApp {
    fn event(_: &mut Self, _: &wp_cursor_shape_device_v1::WpCursorShapeDeviceV1, _: wp_cursor_shape_device_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl TypdApp {
    pub fn new(globals: &GlobalList, qh: &QueueHandle<Self>) -> Self {
        let compositor_state = CompositorState::bind(globals, qh).unwrap();
        let layer_shell      = LayerShell::bind(globals, qh).unwrap();
        let shm              = Shm::bind(globals, qh).unwrap();
        let seat_state       = SeatState::new(globals, qh);
        
        let cursor_shape_manager = globals.bind::<wp_cursor_shape_manager_v1::WpCursorShapeManagerV1, _, _>(qh, 1..=1, ()).ok();

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

        let initial_margin_x = 360;
        let initial_margin_y = 24;
        layer_surface.set_margin(0, 0, initial_margin_y, initial_margin_x);
        wl_surface.commit();

        let pool = SlotPool::new(1920 * 1080 * 4, &shm).unwrap();

        Self {
            registry_state:   RegistryState::new(globals),
            output_state:     OutputState::new(globals, qh),
            compositor_state,
            layer_shell,
            shm,
            seat_state,
            wl_surface:       Some(wl_surface),
            layer_surface:    Some(layer_surface),
            pool:             Some(pool),
            width:            SURFACE_WIDTH,
            height:           SURFACE_HEIGHT,
            configured:       false,
            running:          true,
            frame_pending:    false,

            cursor_shape_manager,
            cursor_shape_device: None,

            seat:             None,
            pointer:          None,
            pointer_pos:      (0.0, 0.0),
            pointer_enter_serial: 0,

            dragging:         false,
            drag_start_ptr:   (0.0, 0.0),
            drag_start_margin: (0, 0),
            margin_x:         initial_margin_x,
            margin_y:         initial_margin_y,

            resize_edge:        ResizeEdge::None,
            resize_start_ptr:   (0.0, 0.0),
            resize_start_size:  (SURFACE_WIDTH, SURFACE_HEIGHT),
            resize_start_margin:(0, 0),
        }
    }

    fn hit_edge(&self, px: f64, py: f64) -> ResizeEdge {
        let w = self.width as f64;
        let h = self.height as f64;
        let t = 8.0;

        let on_left   = px <= t;
        let on_right  = px >= w - t;
        let on_bottom = py >= h - t;

        match (on_left, on_right, on_bottom) {
            (true,  false, true)  => ResizeEdge::BottomLeft,
            (false, true,  true)  => ResizeEdge::BottomRight,
            (true,  false, false) => ResizeEdge::Left,
            (false, true,  false) => ResizeEdge::Right,
            (false, false, true)  => ResizeEdge::Bottom,
            _                     => ResizeEdge::None,
        }
    }

    fn update_cursor(&self) {
        if let Some(device) = &self.cursor_shape_device {
            use wp_cursor_shape_device_v1::Shape;
            let (px, py) = self.pointer_pos;
            let edge = self.hit_edge(px, py);
            
            let shape = if self.resize_edge != ResizeEdge::None || edge != ResizeEdge::None {
                let active_edge = if self.resize_edge != ResizeEdge::None { self.resize_edge } else { edge };
                match active_edge {
                    ResizeEdge::Left   => Shape::WResize,
                    ResizeEdge::Right  => Shape::EResize,
                    ResizeEdge::Bottom => Shape::SResize,
                    ResizeEdge::BottomLeft  => Shape::SwResize,
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

            device.set_shape(self.pointer_enter_serial, shape);
        }
    }

    pub fn draw(&mut self, qh: &QueueHandle<Self>) {
        if self.frame_pending {
            return;
        }

        let width  = self.width  as i32;
        let height = self.height as i32;
        let stride = width * 4;

        let pool = self.pool.as_mut().unwrap();
        let (buffer, canvas) = match pool.create_buffer(width, height, stride, wayland_client::protocol::wl_shm::Format::Argb8888) {
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
        
        renderer::draw_placeholder(&cr, width, height);
        
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
