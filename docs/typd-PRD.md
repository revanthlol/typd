# typd — Product Requirements Document

**Version:** 0.1  
**Status:** Draft  
**Author:** Revanth  
**Last Updated:** May 2025

---

## 1. Executive Summary

**typd** is a lightweight, Wayland-native virtual keyboard and intelligent text suggestion system for Linux. It provides:

- A virtual on-screen keyboard rendered via `wlr-layer-shell`, for touchscreen / accessibility use
- A floating suggestion popup that appears near the cursor during physical keyboard input
- Context-aware suggestion toggling based on app type and file context
- A shared suggestion engine (Trie + BK-Tree) across both modes

It is designed to be minimal, fast, dependency-light, and distro-agnostic. It targets Wayland compositors with wlroots-based protocol support (Hyprland, Sway, river, etc.).

---

## 2. Problem Statement

Linux lacks a well-designed, lightweight input assistance system. Existing options are:

| Tool | Problem |
|------|---------|
| `onboard` | GTK3, bloated, X11-only |
| `wvkbd` | Wayland virtual keyboard only, zero suggestions |
| `fcitx5` | IME framework, not a virtual keyboard, heavy |
| `ibus` | Same — IME only, not keyboard UI |

No existing tool combines a virtual keyboard + floating inline suggestions + smart context detection in a single, minimal binary.

---

## 3. Goals

- **G1:** Functional virtual keyboard overlay on Wayland (wlroots compositors)
- **G2:** Floating suggestion popup near cursor for physical keyboard use
- **G3:** Shared suggestion engine: prefix autocomplete + fuzzy correction
- **G4:** Context-aware enable/disable (terminals off, prose on, code editor smart detection)
- **G5:** Single binary, minimal runtime dependencies
- **G6:** User-configurable via a plain text rules file
- **G7:** Zero crashes, graceful degradation for unsupported apps

---

## 4. Non-Goals

- X11 support (explicitly out of scope for v1)
- AI/LLM-based suggestions
- Cloud sync or any network functionality
- Multi-language IME / CJK input
- Mobile / Android port
- GTK/Qt theming integration

---

## 5. Target Users

- Linux users with touchscreens (tablets, convertibles) running Wayland
- Users who want mobile-style text correction on desktop
- Accessibility users who need on-screen keyboard support
- Developers who want a showable, technically deep portfolio project

---

## 6. System Architecture

### 6.1 Components

```
typd (single binary)
│
├── input_method.c       zwp_input_method_v2
│     ├── Keyboard grab (zwp_input_method_keyboard_grab_v2)
│     ├── Surrounding text reader
│     └── Commit string (insert selected suggestion)
│
├── virtual_kbd.c        zwp_virtual_keyboard_v1 + wlr-layer-shell
│     ├── Layer shell overlay (bottom anchor)
│     ├── Keyboard UI rendering (via renderer.c)
│     └── Suggestion strip (inside keyboard UI)
│
├── popup.c              zwp_input_popup_surface_v2
│     ├── Floating suggestion box (compositor-positioned at cursor)
│     └── Click + shortcut selection
│
├── suggestions.c        Shared engine
│     ├── Trie (prefix autocomplete, O(k) lookup)
│     ├── BK-Tree (fuzzy correction, edit distance ≤ 2)
│     └── Bigram table (next-word suggestion between words) [v2]
│
├── context_detect.c     Smart enable/disable logic
│     ├── Layer 1: app_id lookup
│     ├── Layer 2: window title / extension parse
│     ├── Layer 3: content heuristic scorer
│     └── Layer 4: user rules file override
│
├── renderer.c           Cairo-based drawing
│     ├── Virtual keyboard surface
│     └── Popup surface
│
├── config.c             Config + rules parser
│     ├── ~/.config/typd/config.toml
│     └── ~/.config/typd/rules.conf
│
└── data/
      └── words.freq     Prebuilt word frequency list (~50k words)
```

### 6.2 Wayland Protocols Used

| Protocol | Purpose |
|----------|---------|
| `wlr-layer-shell-unstable-v1` | Render keyboard as overlay layer |
| `zwp_virtual_keyboard_v1` | Inject keystrokes from virtual keyboard |
| `zwp_input_method_v2` | Physical keyboard input method (suggestions) |
| `zwp_input_method_keyboard_grab_v2` | Intercept shortcuts (Tab, 1/2/3) |
| `zwp_input_popup_surface_v2` | Position floating popup at cursor |
| `zwp_text_input_v3` | Read surrounding text in virtual keyboard mode |
| `xdg_output_v1` | Multi-monitor awareness |

### 6.3 Suggestion Engine

**Prefix autocomplete (Trie)**
- Built at startup from `words.freq`
- Input: current partial word
- Output: top-N completions sorted by frequency
- Complexity: O(prefix_length) lookup

**Fuzzy correction (BK-Tree)**
- Triggered on word completion (spacebar pressed)
- Computes Levenshtein distance ≤ 2 from typed word
- Corrects typos silently or presents alternatives
- Complexity: O(log n) average

**Content heuristic scorer (context_detect.c)**
- Reads surrounding text (up to 300 chars)
- Scores based on code token density
- Threshold ≥ 4 → disable suggestions

---

## 7. Context Detection Specification

### 7.1 Detection Layers (evaluated in order)

**Layer 4 (user rules) evaluated first — always wins.**

```
Layer 4 → User rules file (~/.config/typd/rules.conf)
Layer 1 → app_id lookup table (hardcoded)
Layer 2 → Window title extension parse
Layer 3 → Content heuristic scorer
```

### 7.2 Layer 1 — app_id Table

| Category | app_ids | Suggestions |
|----------|---------|-------------|
| Terminals | `foot`, `kitty`, `alacritty`, `wezterm`, `contour`, `rio`, `blackbox`, `ghostty` | OFF |
| Browsers | `firefox`, `chromium`, `google-chrome`, `brave-browser`, `org.gnome.Epiphany` | ON |
| Doc apps | `libreoffice`, `org.libreoffice.*`, `apostrophe`, `gedit`, `gnome-text-editor`, `obsidian`, `logseq` | ON |
| Code editors | `code`, `codium`, `zed`, `helix`, `kate`, `lapce`, `sublime_text`, `neovide` | → Layer 2 |
| Unknown | anything not in table | → Layer 3 |

### 7.3 Layer 2 — Extension Parse

Parse pattern: `(filename)(\.ext)(\s*[—\-–]\s*.+)?$` from window title.

| Extensions | Suggestions |
|------------|-------------|
| `.txt`, `.md`, `.rst`, `.tex`, `.org`, `.typ`, `.wiki`, `.adoc` | ON |
| `.py`, `.js`, `.ts`, `.jsx`, `.tsx`, `.c`, `.cpp`, `.h`, `.rs`, `.go`, `.sh`, `.bash`, `.zsh`, `.fish`, `.json`, `.yaml`, `.toml`, `.lua`, `.rb`, `.java`, `.kt`, `.cs`, `.php`, `.html`, `.css`, `.scss`, `.xml`, `.sql` | OFF |
| Unknown / no extension | OFF (safe default) |

### 7.4 Layer 3 — Content Heuristic

```c
int score = 0;
score += count_occurrences(text, "{");
score += count_occurrences(text, "}");
score += count_occurrences(text, "()");
score += strstr(text, "import ")  ? 2 : 0;
score += strstr(text, "def ")     ? 2 : 0;
score += strstr(text, "fn ")      ? 1 : 0;
score += strstr(text, "=>")       ? 1 : 0;
score += strstr(text, "//")       ? 1 : 0;
score += strstr(text, "#!")       ? 2 : 0;
score += strstr(text, "const ")   ? 1 : 0;
score += strstr(text, "let ")     ? 1 : 0;

return (score >= 4) ? SUGGESTIONS_OFF : SUGGESTIONS_ON;
```

### 7.5 Layer 4 — User Rules File

File: `~/.config/typd/rules.conf`

```ini
# Syntax: app:<app_id> [ext:<extension>] = on|off
app:obsidian     = on
app:zed          = off
app:code ext:.md = on
app:code ext:.py = off
```

- Parsed at startup
- Hot-reloaded on `SIGHUP` (no restart needed)
- Malformed lines are skipped with a warning to stderr

---

## 8. UI Specification

### 8.1 Virtual Keyboard Layout

```
┌──────────────────────────────────────────────────────┐
│  [hello]   [help]   [held]                           │  ← suggestion strip
├──────────────────────────────────────────────────────┤
│  q   w   e   r   t   y   u   i   o   p   ⌫          │
│    a   s   d   f   g   h   j   k   l   ↵            │
│  ⇧   z   x   c   v   b   n   m   ,   .   ⇧          │
│  123    [              space              ]    ✓      │
└──────────────────────────────────────────────────────┘
```

**Layers:**
- Layer 0: lowercase QWERTY (default)
- Layer 1: uppercase (Shift)
- Layer 2: numbers + basic symbols
- Layer 3: special symbols

**Interactions:**
- Tap → key press
- Long-press (500ms) → alternate character popup
- Swipe up on key → alternate character (optional, v2)
- Suggestion tap → commit word

### 8.2 Floating Suggestion Popup (Physical Keyboard Mode)

```
         ┌──────────────────────────┐
         │  [1] hello  [2] help  [3] held │
         └──────────────────────────┘
              ▲ cursor here
```

- Compositor-positioned via `zwp_input_popup_surface_v2`
- Appears 150ms after last keystroke (debounced)
- Selection: `Tab` cycles, `1/2/3` selects directly, `Esc` dismisses
- Click also selects
- Disappears on word commit (space/punctuation) or Esc

### 8.3 Rendering

- Cairo for all drawing (keyboard UI + popup)
- Dirty-rect redraws only (don't redraw entire keyboard per keypress)
- Configurable theme via config: background color, key color, accent, font

---

## 9. Configuration

File: `~/.config/typd/config.toml`

```toml
[general]
suggestion_delay_ms = 150    # debounce before showing popup
max_suggestions = 3          # number of suggestions shown
fuzzy_correction = true      # enable BK-tree correction
word_list = "default"        # path or "default" for bundled list

[keyboard]
height_percent = 35          # % of screen height
layout = "qwerty"            # layout file name (in layouts/)
theme = "dark"               # "dark" | "light" | path to theme file

[theme.dark]
bg = "#1a1a1a"
key_bg = "#2a2a2a"
key_fg = "#ffffff"
key_border = "#3a3a3a"
accent = "#4a9eff"
suggestion_bg = "#1a1a1a"
suggestion_fg = "#ffffff"

[popup]
position = "above_cursor"    # always above, compositor decides exact position
```

---

## 10. Runtime Dependencies

| Library | Purpose | Reason chosen |
|---------|---------|---------------|
| `libwayland-client` | Wayland IPC | Required |
| `libcairo` | 2D drawing | Lightweight, no toolkit |
| `libxkbcommon` | Keyboard layout handling | Standard on all Wayland distros |
| `wayland-protocols` | Protocol headers | Build-time only |

**No GTK. No Qt. No Electron. No Python runtime.**

Binary size target: < 2MB stripped.
RAM usage target: < 15MB idle (keyboard hidden), < 20MB active.

---

## 11. Error Handling & Graceful Degradation

| Situation | Behavior |
|-----------|---------|
| App doesn't support `text-input-v3` | Disable suggestions silently, keyboard still works |
| App doesn't support `input-method-v2` | Physical keyboard suggestions disabled, log warning |
| `input-popup-surface-v2` not supported by compositor | Fallback: place popup at fixed bottom-of-screen position |
| Malformed rules.conf line | Skip line, print warning to stderr |
| words.freq missing | Disable suggestions entirely, log error |
| Compositor kills layer shell surface | Attempt reconnect once, then exit cleanly |

---

## 12. Supported Environments

| Compositor | Virtual KB | Popup | Status |
|------------|-----------|-------|--------|
| Hyprland | ✅ | ✅ | Primary target |
| Sway | ✅ | ✅ | Supported |
| river | ✅ | ⚠️ (no popup surface) | Partial |
| labwc | ✅ | ⚠️ | Partial |
| KWin (Wayland) | ✅ | ❓ | Untested |
| X11 (any) | ❌ | ❌ | Out of scope v1 |

---

## 13. Build System

- **Build tool:** `meson` + `ninja`
- **Language:** C (C11)
- **Compiler targets:** gcc, clang
- **Protocol headers:** generated via `wayland-scanner` at build time

```
meson setup build
ninja -C build
ninja -C build install
```

Installs to `/usr/local/bin/typd` by default. Systemd user service file included for autostart.

---

## 14. Versioning & Milestones

| Version | Milestone |
|---------|-----------|
| v0.1 | Layer shell window renders, static keyboard drawable |
| v0.2 | Key tap injects characters via virtual-keyboard protocol |
| v0.3 | Suggestion strip on keyboard UI, Trie engine working |
| v0.4 | text-input-v3 integration, live suggestions while typing |
| v0.5 | Physical keyboard mode: input-method-v2, floating popup |
| v0.6 | BK-Tree fuzzy correction |
| v0.7 | Context detection (all 4 layers) |
| v0.8 | Config file, themes, rules.conf |
| v0.9 | Multi-monitor, edge cases, stability |
| v1.0 | Packaged release, README, AUR package |

---

## 15. Out of Scope (Future / v2)

- Bigram next-word prediction model
- Swipe typing (gesture-based input)
- RTL language support
- Custom layout editor GUI
- X11 fallback via XTest
- Emoji picker integration
