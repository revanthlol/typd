use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;
use wayland_client::{globals::registry_queue_init, Connection};

mod renderer;
mod virtual_kbd;
mod suggestions;
mod layout;
mod input_method;
mod popup;
mod context_detect;
mod config;

use virtual_kbd::TypdApp;

fn main() {
    let conn = Connection::connect_to_env()
        .expect("Failed to connect to Wayland display. Is WAYLAND_DISPLAY set?");

    let (globals, event_queue) = registry_queue_init::<TypdApp>(&conn)
        .expect("Failed to init Wayland registry");

    let qh = event_queue.handle();

    let mut app = TypdApp::new(&globals, &qh);

    let mut event_loop: EventLoop<TypdApp> =
        EventLoop::try_new().expect("Failed to create event loop");

    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .expect("Failed to insert Wayland source");

    eprintln!("typd: surface initialized {}x{}", app.width, app.height);

    while app.running {
        event_loop
            .dispatch(Some(std::time::Duration::from_millis(16)), &mut app)
            .unwrap();
    }
}