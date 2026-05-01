pub use ::wayland_backend;
pub use ::wayland_client;
pub use ::wayland_client::protocol::wl_seat;

#[allow(dead_code, non_camel_case_types, unused_imports)]
pub mod generated {
    pub use super::wayland_backend;
    pub use super::wayland_client;
    pub use super::wl_seat;

    pub mod __interfaces {
        pub use super::wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/virtual-keyboard-unstable-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/virtual-keyboard-unstable-v1.xml");
}
pub use generated::*;
