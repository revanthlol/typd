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

## Dependency Tree

The tree below was generated from the current lockfile with:

```bash
cargo tree --no-dev-dependencies
```

```text
typd v0.1.0 (/home/rev/Documents/projects/typd)
├── cairo-rs v0.18.5
│   ├── bitflags v2.11.1
│   ├── cairo-sys-rs v0.18.2
│   │   └── libc v0.2.186
│   │   [build-dependencies]
│   │   └── system-deps v6.2.2
│   │       ├── cfg-expr v0.15.8
│   │       │   ├── smallvec v1.15.1
│   │       │   └── target-lexicon v0.12.16
│   │       ├── heck v0.5.0
│   │       ├── pkg-config v0.3.33
│   │       ├── toml v0.8.2
│   │       │   ├── serde v1.0.228
│   │       │   │   └── serde_core v1.0.228
│   │       │   ├── serde_spanned v0.6.9
│   │       │   │   └── serde v1.0.228 (*)
│   │       │   ├── toml_datetime v0.6.3
│   │       │   │   └── serde v1.0.228 (*)
│   │       │   └── toml_edit v0.20.2
│   │       │       ├── indexmap v2.14.0
│   │       │       │   ├── equivalent v1.0.2
│   │       │       │   └── hashbrown v0.17.0
│   │       │       ├── serde v1.0.228 (*)
│   │       │       ├── serde_spanned v0.6.9 (*)
│   │       │       ├── toml_datetime v0.6.3 (*)
│   │       │       └── winnow v0.5.40
│   │       └── version-compare v0.2.1
│   ├── libc v0.2.186
│   ├── once_cell v1.21.4
│   └── thiserror v1.0.69
│       └── thiserror-impl v1.0.69 (proc-macro)
│           ├── proc-macro2 v1.0.106
│           │   └── unicode-ident v1.0.24
│           ├── quote v1.0.45
│           │   └── proc-macro2 v1.0.106 (*)
│           └── syn v2.0.117
│               ├── proc-macro2 v1.0.106 (*)
│               ├── quote v1.0.45 (*)
│               └── unicode-ident v1.0.24
├── calloop v0.12.4
│   ├── bitflags v2.11.1
│   ├── log v0.4.29
│   ├── polling v3.11.0
│   │   ├── cfg-if v1.0.4
│   │   └── rustix v1.1.4
│   │       ├── bitflags v2.11.1
│   │       └── linux-raw-sys v0.12.1
│   ├── rustix v0.38.44
│   │   ├── bitflags v2.11.1
│   │   └── linux-raw-sys v0.4.15
│   ├── slab v0.4.12
│   └── thiserror v1.0.69 (*)
├── calloop-wayland-source v0.2.0
│   ├── calloop v0.12.4 (*)
│   ├── rustix v0.38.44 (*)
│   ├── wayland-backend v0.3.15
│   │   ├── downcast-rs v1.2.1
│   │   ├── rustix v1.1.4 (*)
│   │   ├── smallvec v1.15.1
│   │   └── wayland-sys v0.31.11
│   │       [build-dependencies]
│   │       └── pkg-config v0.3.33
│   │   [build-dependencies]
│   │   └── cc v1.2.61
│   │       ├── find-msvc-tools v0.1.9
│   │       └── shlex v1.3.0
│   └── wayland-client v0.31.14
│       ├── bitflags v2.11.1
│       ├── rustix v1.1.4 (*)
│       ├── wayland-backend v0.3.15 (*)
│       └── wayland-scanner v0.31.10 (proc-macro)
│           ├── proc-macro2 v1.0.106 (*)
│           ├── quick-xml v0.39.2
│           │   └── memchr v2.8.0
│           └── quote v1.0.45 (*)
├── libc v0.2.186
├── smithay-client-toolkit v0.18.1
│   ├── bitflags v2.11.1
│   ├── calloop v0.12.4 (*)
│   ├── calloop-wayland-source v0.2.0 (*)
│   ├── cursor-icon v1.2.0
│   ├── libc v0.2.186
│   ├── log v0.4.29
│   ├── memmap2 v0.9.10
│   │   └── libc v0.2.186
│   ├── rustix v0.38.44 (*)
│   ├── thiserror v1.0.69 (*)
│   ├── wayland-backend v0.3.15 (*)
│   ├── wayland-client v0.31.14 (*)
│   ├── wayland-csd-frame v0.3.0
│   │   ├── bitflags v2.11.1
│   │   ├── cursor-icon v1.2.0
│   │   └── wayland-backend v0.3.15 (*)
│   ├── wayland-cursor v0.31.14
│   │   ├── rustix v1.1.4 (*)
│   │   ├── wayland-client v0.31.14 (*)
│   │   └── xcursor v0.3.10
│   ├── wayland-protocols v0.31.2
│   │   ├── bitflags v2.11.1
│   │   ├── wayland-backend v0.3.15 (*)
│   │   ├── wayland-client v0.31.14 (*)
│   │   └── wayland-scanner v0.31.10 (proc-macro) (*)
│   ├── wayland-protocols-wlr v0.2.0
│   │   ├── bitflags v2.11.1
│   │   ├── wayland-backend v0.3.15 (*)
│   │   ├── wayland-client v0.31.14 (*)
│   │   ├── wayland-protocols v0.31.2 (*)
│   │   └── wayland-scanner v0.31.10 (proc-macro) (*)
│   ├── wayland-scanner v0.31.10 (proc-macro) (*)
│   └── xkeysym v0.2.1
├── wayland-backend v0.3.15 (*)
├── wayland-client v0.31.14 (*)
├── wayland-protocols v0.32.12
│   ├── bitflags v2.11.1
│   ├── wayland-backend v0.3.15 (*)
│   ├── wayland-client v0.31.14 (*)
│   └── wayland-scanner v0.31.10 (proc-macro) (*)
├── wayland-protocols-wlr v0.3.12
│   ├── bitflags v2.11.1
│   ├── wayland-backend v0.3.15 (*)
│   ├── wayland-client v0.31.14 (*)
│   ├── wayland-protocols v0.32.12 (*)
│   └── wayland-scanner v0.31.10 (proc-macro) (*)
├── wayland-scanner v0.31.10 (proc-macro) (*)
└── xkbcommon v0.7.0
    ├── libc v0.2.186
    ├── memmap2 v0.8.0
    │   └── libc v0.2.186
    └── xkeysym v0.2.1
```
