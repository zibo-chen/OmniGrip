---
name: omnigrip-computer-control
description: "Control computer GUI via OmniGrip. Use when: automating desktop tasks, clicking buttons, typing text, navigating menus, filling forms, opening apps, or any GUI interaction. Covers CLI-first invocation, OCR-first element location (saves tokens), screenshot fallback, mouse/keyboard control, window management, and action verification."
argument-hint: "Describe the GUI task you want to perform"
---

# OmniGrip Computer Control

LLM-driven desktop GUI automation via OmniGrip.

**Project:** https://github.com/zibo-chen/OmniGrip

## Core Principles

1. **CLI first** — If a shell/terminal tool is available, invoke `omni-grip` as a CLI command. This is the most efficient mode (no MCP round-trips, JSON output directly parseable).
2. **MCP fallback** — Use MCP tools only when a terminal is unavailable.
3. **OCR over screenshot** — Prefer OCR commands to locate text elements; fall back to screenshots only for icon/graphical targets.
4. **Verify actions** — After each key operation, confirm the result with `action-assertion` or OCR.
5. **OCR coordinates are real** — Coordinates from OCR are native screen coordinates. Pass `--scale-ratio 1.0` (or omit it) for mouse commands.

---

## Installation & Updates

Download the latest pre-compiled binary for your platform from the GitHub Releases page:

```
https://github.com/zibo-chen/OmniGrip/releases/latest
```

| Platform | Binary filename |
|----------|----------------|
| macOS (Apple Silicon) | `omni-grip-aarch64-apple-darwin` |
| macOS (Intel) | `omni-grip-x86_64-apple-darwin` |
| Linux (x86_64) | `omni-grip-x86_64-unknown-linux-gnu` |
| Windows (x86_64) | `omni-grip-x86_64-pc-windows-msvc.exe` |

After downloading, make it executable (macOS/Linux):

```bash
chmod +x omni-grip-*
mv omni-grip-* /usr/local/bin/omni-grip
```

To check the installed version:

```bash
omni-grip --version
```

Compare with the latest release tag at https://github.com/zibo-chen/OmniGrip/releases to determine if an update is needed.

---

## Mode 1 (Preferred): CLI Mode

When a terminal/shell is available, run `omni-grip <subcommand> [args]` directly. Output is JSON on stdout.

### CLI Command Reference

```bash
# ── Environment ──
omni-grip get-os-context                       # → {"os":"macos"|"windows"|"linux"}
omni-grip get-displays                         # → [{id, width, height, scale_factor, ...}]

# ── Screenshot ──
omni-grip take-screenshot \
  --display-id 0 \
  --max-width 1280 --max-height 800 \
  --quality 80                                 # → {base64, scale_ratio, width, height}

omni-grip take-screenshot-region \
  --x 0 --y 0 --width 500 --height 200        # → {base64, scale_ratio, ...}

# ── OCR ──
omni-grip get-ocr-data --display-id 0         # → ["text|x|y", ...]
omni-grip find-text-center \
  --display-id 0 \
  --target-text "Save"                         # → "found:Save|x:500|y:300|confidence:0.95"

omni-grip action-assertion \
  --x 0 --y 0 --width 500 --height 100 \
  --expected-text "Saved"                      # → true / false

# ── Mouse ──
omni-grip mouse-click --x 500 --y 300                        # left click
omni-grip mouse-click --x 500 --y 300 --button right         # right click
omni-grip mouse-click --x 500 --y 300 --click-type double_click
omni-grip mouse-click --x 640 --y 400 --scale-ratio 1.5      # from screenshot coords
omni-grip mouse-move --x 500 --y 300
omni-grip mouse-move-relative --dx 10 --dy -20
omni-grip mouse-drag --from-x 100 --from-y 100 --to-x 400 --to-y 400

# ── Keyboard ──
omni-grip keyboard-type --text "Hello World"
omni-grip keyboard-press cmd s                  # Cmd+S (macOS save)
omni-grip keyboard-press ctrl s                 # Ctrl+S (Windows/Linux save)
omni-grip keyboard-press enter
omni-grip keyboard-press cmd space              # Spotlight

# ── Clipboard ──
omni-grip clipboard-read
omni-grip clipboard-write --text "paste me"

# ── Window Management ──
omni-grip get-active-window
omni-grip list-windows
omni-grip focus-window --window-id "12345"
```

### CLI Workflow Example

```bash
# 1. Check OS for correct modifier key
omni-grip get-os-context
# → {"os":"macos"}  → use "cmd"

# 2. Find "Save" button via OCR (no screenshot needed)
omni-grip find-text-center --display-id 0 --target-text "Save"
# → "found:Save|x:854|y:612|confidence:0.97"

# 3. Click it
omni-grip mouse-click --x 854 --y 612

# 4. Verify
omni-grip action-assertion --x 0 --y 0 --width 1920 --height 50 --expected-text "Saved"
# → true
```

---

## Mode 2 (Fallback): MCP Tools

Use these when a terminal is not available (the MCP server is running via stdio).

### Tool Reference

| Category | Tool | Purpose |
|----------|------|---------|
| Vision | `get_displays` | Get display IDs (required on first call) |
| Vision | `take_screenshot` | Full-screen JPEG + scale_ratio |
| Vision | `take_screenshot_region` | Region screenshot |
| OCR | `get_ocr_data` | Full-screen OCR → `text\|x\|y` per line |
| OCR | `find_text_center` | Find text, return center coordinates |
| OCR | `action_assertion` | Assert text exists in region |
| Mouse | `mouse_click` | Click at coordinates |
| Mouse | `mouse_move` | Move cursor |
| Mouse | `mouse_move_relative` | Relative cursor move |
| Mouse | `mouse_drag` | Drag operation |
| Keyboard | `keyboard_type` | Type text (Unicode/CJK supported) |
| Keyboard | `keyboard_press` | Press shortcuts, e.g. `["cmd","c"]` |
| Window | `list_windows` | List all windows |
| Window | `focus_window` | Bring window to foreground |
| Window | `get_active_window` | Get current active window |
| System | `get_os_context` | Get OS type |
| Clipboard | `clipboard_read` / `clipboard_write` | Read/write clipboard |

---

## Element Location Decision Tree

```
Need to locate a UI element?
├── Text target (button label, menu item, link text)
│   ├── Know exact text → find_text_center  (most efficient)
│   └── Unsure of text  → get_ocr_data → pick closest match
├── Icon / graphical / non-text element
│   └── take_screenshot → visual analysis → get coordinates
└── Need overall layout
    ├── get_ocr_data first (cheap, text structure)
    └── Still unclear → take_screenshot (expensive but complete)
```

## Coordinate Rules

| Source | Coordinates | How to use |
|--------|-------------|-----------|
| OCR tools | Native screen coords | Use directly, `scale_ratio=1.0` |
| `take_screenshot` | Compressed coords | Must pass returned `scale_ratio` |

---

## Common Task Templates

### Open an Application (macOS)

```bash
omni-grip keyboard-press cmd space       # Spotlight
omni-grip keyboard-type --text "Safari"
omni-grip keyboard-press enter
# wait for app to launch (one conversation turn is usually enough)
omni-grip get-active-window              # confirm app is in foreground
```

### Click a Text Button

```bash
omni-grip find-text-center --display-id 0 --target-text "Submit"
# → "found:Submit|x:712|y:480|confidence:0.98"
omni-grip mouse-click --x 712 --y 480
```

### Type into a Text Field

```bash
omni-grip mouse-click --x 300 --y 200   # click the input field first
omni-grip keyboard-type --text "my input text"
```

### Paste Long Text (more reliable than keyboard-type for long content)

```bash
omni-grip clipboard-write --text "very long text..."
omni-grip keyboard-press cmd v           # macOS
# omni-grip keyboard-press ctrl v       # Windows/Linux
```

### Navigate a Menu

```bash
omni-grip find-text-center --display-id 0 --target-text "File"
omni-grip mouse-click --x <x> --y <y>
# wait for menu to open
omni-grip find-text-center --display-id 0 --target-text "Save As"
omni-grip mouse-click --x <x> --y <y>
```

### Verify Action Result

```bash
# Assert text appeared in the top status bar region
omni-grip action-assertion --x 0 --y 0 --width 1920 --height 60 --expected-text "Saved"
```

### Scroll

```bash
omni-grip keyboard-press pagedown   # scroll down
omni-grip keyboard-press pageup     # scroll up
omni-grip keyboard-press end        # jump to bottom
omni-grip keyboard-press home       # jump to top
```

### Switch Window

```bash
omni-grip list-windows
omni-grip focus-window --window-id "12345"
```

---

## Tips

- **Fuzzy matching**: `find-text-center` uses Levenshtein distance — an approximate text is fine.
- **CJK input**: `keyboard-type` supports full Unicode including Chinese/Japanese/Korean.
- **Long text**: Use `clipboard-write` + paste shortcut to avoid dropped characters.
- **Multi-monitor**: Coordinates use a global coordinate space; secondary displays may have x/y offsets.
- **OCR unavailable**: If model files are missing, all OCR commands return errors — fall back to screenshot mode entirely.
- **Timing**: UI needs time to respond after actions, but one conversation turn provides natural delay — explicit sleeps are usually unnecessary.
