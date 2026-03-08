# OmniGrip

[![Rust](https://img.shields.io/badge/rust-1.82+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**OmniGrip** is a cross-platform computer control MCP (Model Context Protocol) server that enables LLM-driven GUI automation. It provides screen capture, mouse/keyboard simulation, window management, and OCR capabilities through a standardized MCP interface.

[中文文档](README_CN.md)

## Features

### 🖥️ Vision Module
- **Display Detection** - Get metadata for all connected monitors (ID, resolution, scale factor, position)
- **Screenshot Capture** - Full-screen or region-based screenshots with automatic scaling and JPEG compression
- **Coordinate Conversion** - Automatic scale ratio calculation for accurate coordinate mapping

### 🖱️ Action Module
- **Mouse Control** - Move, click (left/right/middle), double-click, drag operations
- **Keyboard Simulation** - Text typing with Unicode/CJK support, keyboard shortcuts
- **Coordinate Scaling** - Seamless coordinate conversion between compressed screenshots and real screen

### 🪟 Context Module
- **Window Management** - List windows, get active window, bring window to foreground
- **Clipboard Operations** - Read/write system clipboard
- **OS Detection** - Get current operating system type for platform-aware automation

### 📝 OCR Module
- **Full-screen OCR** - Extract all text from screen with center-point coordinates
- **Text Search** - Find specific text on screen with fuzzy matching
- **Action Verification** - Assert that expected text appears in a region (for automation validation)

## Architecture

OmniGrip follows Domain-Driven Design (DDD) principles:

```
┌────────────────────────────────────┐
│      Adapter (MCP Protocol)       │  ← Protocol adaptation
├────────────────────────────────────┤
│      Application (Services)       │  ← Use case orchestration
├────────────────────────────────────┤
│      Domain (Traits & Types)      │  ← Core domain abstractions
├────────────────────────────────────┤
│    Infrastructure (Impls)         │  ← Technical implementations
└────────────────────────────────────┘
```

## Installation

### Prerequisites

- Rust 2024 Edition (1.82+)
- Platform-specific dependencies:
  - **macOS**: Accessibility permissions required
  - **Linux**: X11 or Wayland support
  - **Windows**: No special requirements

### Build from Source

```bash
git clone https://github.com/yourusername/OmniGrip.git
cd OmniGrip
cargo build --release
```

The binary will be at `target/release/omni-grip`.

### OCR Model Setup (Optional)

For OCR features, download PP-OCRv5 model files and place them in one of these directories:

- `./res/chinese_model/`
- `./models/`
- `~/.omnigrip/models/`

Required files:
- `PP-OCRv5_mobile_det_fp16.mnn` (text detection model)
- `PP-OCRv5_mobile_rec_fp16.mnn` (text recognition model)
- `ppocr_keys_v5.txt` (character dictionary)

> Note: OCR is optional. If model files are not found, OmniGrip will start without OCR capabilities.

## Usage

### Running the Server

```bash
# Run with stdio transport (for MCP clients)
./target/release/omni-grip

# Enable debug logging
RUST_LOG=debug ./target/release/omni-grip
```

### MCP Client Configuration

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "omni-grip": {
      "command": "/path/to/omni-grip",
      "args": []
    }
  }
}
```

## MCP Tools Reference

### Vision Tools

| Tool | Description |
|------|-------------|
| `get_displays` | Get all monitor metadata (ID, resolution, scale) |
| `take_screenshot` | Capture full display as JPEG with scale ratio |
| `take_screenshot_region` | Capture specific screen region |

### Action Tools

| Tool | Description |
|------|-------------|
| `mouse_move` | Move cursor to absolute coordinates |
| `mouse_move_relative` | Move cursor by relative offset |
| `mouse_click` | Click at coordinates (left/right/middle, single/double) |
| `mouse_drag` | Drag from point A to point B |
| `keyboard_type` | Type text string (Unicode supported) |
| `keyboard_press` | Press keyboard shortcut (e.g., ["cmd", "c"]) |

### Context Tools

| Tool | Description |
|------|-------------|
| `get_os_context` | Get OS type (windows/macos/linux) |
| `clipboard_read` | Read clipboard text content |
| `clipboard_write` | Write text to clipboard |
| `get_active_window` | Get focused window info |
| `list_windows` | List all visible windows |
| `focus_window` | Bring window to foreground by ID |

### OCR Tools

| Tool | Description |
|------|-------------|
| `get_ocr_data` | Run full-screen OCR, returns text with coordinates |
| `find_text_center` | Find text on screen, return center point |
| `action_assertion` | Verify expected text in screen region |

## Example Workflow

```
1. get_displays → Get display_id for primary monitor
2. take_screenshot(display_id=0, max_width=1000) → Get screen image + scale_ratio
3. [LLM analyzes screenshot, finds button at (500, 300)]
4. mouse_click(x=500, y=300, scale_ratio=1.5) → Click converted coordinates
5. action_assertion(region, "Success") → Verify action result
```

## Dependencies

- **rmcp** - MCP protocol implementation
- **xcap** - Cross-platform screen capture
- **enigo** - Input simulation
- **ocr-rs** - PP-OCR implementation for Rust
- **image** - Image processing and encoding
- **tokio** - Async runtime

## Platform Support

| Platform | Screen Capture | Input Simulation | OCR |
|----------|---------------|------------------|-----|
| macOS    | ✅ | ✅ | ✅ |
| Windows  | ✅ | ✅ | ✅ |
| Linux (X11) | ✅ | ✅ | ✅ |
| Linux (Wayland) | ⚠️ Limited | ⚠️ Limited | ✅ |

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
