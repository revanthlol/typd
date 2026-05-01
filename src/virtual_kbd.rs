use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::wlr_layer::{
        Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_output, wl_surface::WlSurface},
    Connection, QueueHandle,
};
use crate::renderer;

pub const SURFACE_HEIGHT: i32 = 378;   // 35% of 1080p per PRD

pub struct TypdApp {
    pub registry_state:   RegistryState,
    pub output_state:     OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell:      LayerShell,
    pub shm:              Shm,
    pub wl_surface:       Option<WlSurface>,
    pub layer_surface:    Option<LayerSurface>,
    pub pool:             Option<SlotPool>,
    pub width:            u32,
    pub height:           u32,
    pub configured:       bool,
    pub running:          bool,
}

delegate_compositor!(TypdApp);
delegate_output!(TypdApp);
delegate_shm!(TypdApp);
delegate_layer!(TypdApp);
delegate_registry!(TypdApp);

impl CompositorHandler for TypdApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {}

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: wl_output::Transform,
    ) {}

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _time: u32,
    ) {}
}

impl OutputHandler for TypdApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

impl ShmHandler for TypdApp {
    fn shm_state(&mut self) -> &mut Shm { &mut self.shm }
}

impl ProvidesRegistryState for TypdApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

impl LayerShellHandler for TypdApp {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.running = false;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let width  = if configure.new_size.0 == 0 { 1920 } else { configure.new_size.0 };
        let height = if configure.new_size.1 == 0 { SURFACE_HEIGHT as u32 } else { configure.new_size.1 };
        self.width  = width;
        self.height = height;
        self.configured = true;
        self.draw(qh);
    }
}

impl TypdApp {
    pub fn new(globals: &GlobalList, qh: &QueueHandle<Self>) -> Self {
        let compositor_state = CompositorState::bind(globals, qh).unwrap();
        let layer_shell      = LayerShell::bind(globals, qh).unwrap();
        let shm              = Shm::bind(globals, qh).unwrap();

        let wl_surface = compositor_state.create_surface(qh);

        let layer_surface = layer_shell.create_layer_surface(
            qh,
            wl_surface.clone(),
            Layer::Top,
            Some("typd"),
            None,
        );

        use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity};
        layer_surface.set_anchor(Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer_surface.set_size(0, SURFACE_HEIGHT as u32);
        layer_surface.set_exclusive_zone(0);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        wl_surface.commit();

        let pool = SlotPool::new(1920 * SURFACE_HEIGHT as usize * 4, &shm).unwrap();

        Self {
            registry_state:   RegistryState::new(globals),
            output_state:     OutputState::new(globals, qh),
            compositor_state,
            layer_shell,
            shm,
            wl_surface:       Some(wl_surface),
            layer_surface:    Some(layer_surface),
            pool:             Some(pool),
            width:            1920,
            height:           SURFACE_HEIGHT as u32,
            configured:       false,
            running:          true,
        }
    }

    pub fn draw(&mut self, _qh: &QueueHandle<Self>) {
        let width  = self.width  as i32;
        let height = self.height as i32;
        let stride = width * 4;

        let pool = self.pool.as_mut().unwrap();
        let (buffer, canvas) = pool
            .create_buffer(width, height, stride, wayland_client::protocol::wl_shm::Format::Argb8888)
            .unwrap();

        let cairo_surface = unsafe {
            cairo::ImageSurface::create_for_data_unsafe(
                canvas.as_mut_ptr(),
                cairo::Format::ARgb32,
                width,
                height,
                stride,
            ).unwrap()
        };
        let cr = cairo::Context::new(&cairo_surface).unwrap();
        renderer::draw_placeholder(&cr, width, height);
        drop(cr);
        drop(cairo_surface);

        let wl_surface = self.wl_surface.as_ref().unwrap();
        buffer.attach_to(wl_surface).unwrap();
        wl_surface.damage_buffer(0, 0, width, height);
        wl_surface.commit();
    }
}
