project('typd', 'c', version: '0.1.0', default_options: ['c_std=c11', 'warning_level=2'])

cc = meson.get_compiler('c')

# Dependencies
wayland_client  = dependency('wayland-client')
wayland_protos  = dependency('wayland-protocols')
cairo           = dependency('cairo')
xkbcommon       = dependency('xkbcommon')

# wayland-scanner
scanner         = find_program('wayland-scanner')

# Protocol sources — generate both header and glue C from each XML
proto_dir = meson.current_source_dir() / 'protocols'

# Helper to generate a protocol source + header pair
layer_shell_xml = proto_dir / 'wlr-layer-shell-unstable-v1.xml'
layer_shell_h   = custom_target('layer-shell-header',
  output  : 'wlr-layer-shell-unstable-v1-client-protocol.h',
  input   : layer_shell_xml,
  command : [scanner, 'client-header', '@INPUT@', '@OUTPUT@'])
layer_shell_c   = custom_target('layer-shell-code',
  output  : 'wlr-layer-shell-unstable-v1-protocol.c',
  input   : layer_shell_xml,
  command : [scanner, 'private-code', '@INPUT@', '@OUTPUT@'])

xdg_shell_xml = proto_dir / 'xdg-shell.xml'
xdg_shell_h   = custom_target('xdg-shell-header',
  output  : 'xdg-shell-client-protocol.h',
  input   : xdg_shell_xml,
  command : [scanner, 'client-header', '@INPUT@', '@OUTPUT@'])
xdg_shell_c   = custom_target('xdg-shell-code',
  output  : 'xdg-shell-protocol.c',
  input   : xdg_shell_xml,
  command : [scanner, 'private-code', '@INPUT@', '@OUTPUT@'])

# Sources
typd_src = [
  'src/main.c',
  'src/virtual_kbd.c',
  'src/renderer.c',
  layer_shell_h,
  layer_shell_c,
  xdg_shell_h,
  xdg_shell_c,
]

typd_inc = include_directories('include')

executable('typd',
  typd_src,
  include_directories : typd_inc,
  dependencies        : [wayland_client, cairo, xkbcommon],
  install             : true
)
