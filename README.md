# typd

`typd` is a Wayland virtual keyboard with a built-in on-screen layout, draggable window chrome, resize handling, and visual key feedback for both on-screen clicks and physical keyboard events when the compositor delivers them to the app.

## What It Does

- Renders a full virtual keyboard as a Wayland layer-surface.
- Supports mouse interaction on keys, drag bar, resize edges, and the close button.
- Shows active key states directly on the keyboard UI.
- Includes a collapsible sidebar with navigation and utility keys.
- Uses `wl_shm` for drawing and `xkbcommon` for keymap handling.
- Integrates with Wayland virtual keyboard and input-method paths where the compositor supports them.

## Requirements

- Linux
- A Wayland session
- A compositor that supports the Wayland globals typd uses, especially `wl_compositor`, `wl_shm`, `wl_seat`, `zwlr_layer_shell_v1`, and `zwp_virtual_keyboard_v1`
- Optional cursor-shape support for pointer feedback

If a compositor does not provide a given protocol, typd will usually keep running with reduced functionality rather than hard-failing.

## Build

```bash
cargo build
```

For an optimized binary:

```bash
cargo build --release
```

## Run

```bash
cargo run
```

Or run the compiled binary directly:

```bash
./target/debug/typd
```

## Controls

- Click a key to activate it.
- Drag the top bar to move the keyboard.
- Drag the edges or corners to resize it.
- Click the red circular button in the top-right to close the app.
- Use the sidebar toggle key to expand or collapse the sidebar.

## Project Layout

- `src/main.rs`: Wayland connection and event loop.
- `src/virtual_kbd.rs`: window state, input handling, rendering flow, keyboard interaction.
- `src/renderer.rs`: Cairo drawing for the main keyboard UI.
- `src/layout.rs`: key definitions and layout calculation.
- `src/config.rs`: configuration constants.
- `src/input_method.rs`: input-method related helpers.
- `src/popup.rs`: popup-related helpers.
- `src/vkbd_proto.rs`: generated virtual-keyboard protocol bindings.
- `protocols/`: local protocol XML files used for code generation.
- `docs/`: design and planning notes.
- `contrib/typd.service`: systemd service example.

## Dependencies

Direct Rust dependencies from `Cargo.toml`:

- `wayland-client = 0.31`
- `wayland-backend = 0.3`
- `wayland-protocols-wlr = 0.3` with `client`
- `wayland-protocols = 0.32` with `client`, `unstable`, `staging`
- `smithay-client-toolkit = 0.18` with `calloop`, `calloop-wayland-source`
- `calloop = 0.12`
- `calloop-wayland-source = 0.2`
- `cairo-rs = 0.18`
- `xkbcommon = 0.7`
- `libc = 0.2`
- `wayland-scanner = 0.31`

## Notes

- typd is a Wayland-only application.
- The app is intended to run as a floating keyboard surface rather than a normal desktop window.
- Some compositor capabilities are optional, so behavior can vary depending on the Wayland environment.
