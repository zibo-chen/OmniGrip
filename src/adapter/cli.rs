// ============================================================================
// CLI 协议适配器 (CLI Adapter)
// ============================================================================
// 将应用层服务映射为 CLI 子命令，支持 AI 通过命令行调用。
// 输出为 JSON 格式，便于 AI 解析。
// ============================================================================

use std::sync::Arc;

use clap::Subcommand;
use serde_json::json;

use crate::application::{
    action_service::ActionService, context_service::ContextService, ocr_service::OcrService,
    vision_service::VisionService,
};
use crate::domain::action::{ClickType, MouseButton};
use crate::domain::vision::CaptureRegion;

// ===========================================================================
// CLI 子命令定义
// ===========================================================================

#[derive(Subcommand)]
pub enum CliCommand {
    // ── 屏幕感知 (Vision) ──

    /// Get all display/monitor metadata (ID, resolution, scale factor, position)
    GetDisplays,

    /// Take a full screenshot of specified display, returns base64 JPEG
    TakeScreenshot {
        /// Display ID (from get-displays)
        #[arg(long)]
        display_id: u32,
        /// Max width limit (pixels), auto-scale if exceeded
        #[arg(long)]
        max_width: Option<u32>,
        /// Max height limit (pixels), auto-scale if exceeded
        #[arg(long)]
        max_height: Option<u32>,
        /// JPEG quality (1-100), default 80
        #[arg(long, default_value = "80")]
        quality: u8,
    },

    /// Take a screenshot of a specific screen region
    TakeScreenshotRegion {
        /// Region top-left X coordinate
        #[arg(long)]
        x: i32,
        /// Region top-left Y coordinate
        #[arg(long)]
        y: i32,
        /// Region width
        #[arg(long)]
        width: u32,
        /// Region height
        #[arg(long)]
        height: u32,
        /// Max width limit
        #[arg(long)]
        max_width: Option<u32>,
        /// Max height limit
        #[arg(long)]
        max_height: Option<u32>,
        /// JPEG quality (1-100), default 80
        #[arg(long, default_value = "80")]
        quality: u8,
    },

    // ── 鼠标操作 (Mouse) ──

    /// Move mouse cursor to absolute screen coordinates
    MouseMove {
        /// Target X coordinate
        #[arg(long)]
        x: i32,
        /// Target Y coordinate
        #[arg(long)]
        y: i32,
        /// Coordinate scale ratio (original/screenshot resolution), default 1.0
        #[arg(long, default_value = "1.0")]
        scale_ratio: f64,
    },

    /// Move mouse cursor relative to current position
    MouseMoveRelative {
        /// X offset
        #[arg(long)]
        dx: i32,
        /// Y offset
        #[arg(long)]
        dy: i32,
    },

    /// Click mouse at specified coordinates
    MouseClick {
        /// Click position X coordinate
        #[arg(long)]
        x: i32,
        /// Click position Y coordinate
        #[arg(long)]
        y: i32,
        /// Mouse button: left, right, middle (default: left)
        #[arg(long, default_value = "left")]
        button: String,
        /// Click type: click, double_click, press, release (default: click)
        #[arg(long, default_value = "click")]
        click_type: String,
        /// Coordinate scale ratio
        #[arg(long, default_value = "1.0")]
        scale_ratio: f64,
    },

    /// Drag mouse from one position to another
    MouseDrag {
        /// Start X coordinate
        #[arg(long)]
        from_x: i32,
        /// Start Y coordinate
        #[arg(long)]
        from_y: i32,
        /// End X coordinate
        #[arg(long)]
        to_x: i32,
        /// End Y coordinate
        #[arg(long)]
        to_y: i32,
        /// Mouse button: left, right, middle (default: left)
        #[arg(long, default_value = "left")]
        button: String,
        /// Coordinate scale ratio
        #[arg(long, default_value = "1.0")]
        scale_ratio: f64,
    },

    // ── 键盘操作 (Keyboard) ──

    /// Type text string via keyboard simulation
    KeyboardType {
        /// Text to type
        #[arg(long)]
        text: String,
    },

    /// Press keyboard shortcut (keys pressed in order, last key is clicked)
    KeyboardPress {
        /// Key names, e.g. "cmd c" for Cmd+C, "enter" for Enter
        keys: Vec<String>,
    },

    // ── 系统上下文 (Context) ──

    /// Get current OS type (windows/macos/linux)
    GetOsContext,

    /// Read current clipboard text content
    ClipboardRead,

    /// Write text to system clipboard
    ClipboardWrite {
        /// Text to write
        #[arg(long)]
        text: String,
    },

    /// Get currently focused window info
    GetActiveWindow,

    /// List all visible windows
    ListWindows,

    /// Bring a specific window to foreground
    FocusWindow {
        /// Window ID (from list-windows)
        #[arg(long)]
        window_id: String,
    },

    /// Get current system permission status (macOS: Accessibility + Screen Recording)
    GetPermissions,

    /// Trigger system permission requests and return the latest status snapshot
    RequestPermissions,

    // ── OCR ──

    /// Run OCR on full screen, returns text blocks with coordinates
    GetOcrData {
        /// Display ID
        #[arg(long)]
        display_id: u32,
    },

    /// Find text on screen via OCR and return center-point coordinates
    FindTextCenter {
        /// Display ID
        #[arg(long)]
        display_id: u32,
        /// Target text to find (supports fuzzy matching)
        #[arg(long)]
        target_text: String,
    },

    /// Verify action by checking if expected text appears in screen region
    ActionAssertion {
        /// Region top-left X
        #[arg(long)]
        x: i32,
        /// Region top-left Y
        #[arg(long)]
        y: i32,
        /// Region width
        #[arg(long)]
        width: u32,
        /// Region height
        #[arg(long)]
        height: u32,
        /// Expected text to find
        #[arg(long)]
        expected_text: String,
    },
}

// ===========================================================================
// CLI 执行器
// ===========================================================================

pub struct CliExecutor {
    vision: Arc<VisionService>,
    action: Arc<ActionService>,
    context: Arc<ContextService>,
    ocr: Arc<OcrService>,
}

impl CliExecutor {
    pub fn new(
        vision: Arc<VisionService>,
        action: Arc<ActionService>,
        context: Arc<ContextService>,
        ocr: Arc<OcrService>,
    ) -> Self {
        Self {
            vision,
            action,
            context,
            ocr,
        }
    }

    /// 执行 CLI 命令并将结果以 JSON 输出到 stdout
    pub async fn execute(&self, cmd: CliCommand) -> anyhow::Result<()> {
        let result = match cmd {
            // ── Vision ──
            CliCommand::GetDisplays => {
                let displays = self.vision.get_displays().await?;
                serde_json::to_value(&displays)?
            }

            CliCommand::TakeScreenshot {
                display_id,
                max_width,
                max_height,
                quality,
            } => {
                let encoded = self
                    .vision
                    .take_screenshot(display_id, max_width, max_height, quality)
                    .await?;
                json!({
                    "width": encoded.width,
                    "height": encoded.height,
                    "original_width": encoded.original_width,
                    "original_height": encoded.original_height,
                    "scale_ratio": encoded.scale_ratio,
                    "format": encoded.format,
                    "base64_data": encoded.base64_data,
                })
            }

            CliCommand::TakeScreenshotRegion {
                x,
                y,
                width,
                height,
                max_width,
                max_height,
                quality,
            } => {
                let region = CaptureRegion {
                    x,
                    y,
                    width,
                    height,
                };
                let encoded = self
                    .vision
                    .take_screenshot_region(region, max_width, max_height, quality)
                    .await?;
                json!({
                    "width": encoded.width,
                    "height": encoded.height,
                    "original_width": encoded.original_width,
                    "original_height": encoded.original_height,
                    "scale_ratio": encoded.scale_ratio,
                    "format": encoded.format,
                    "base64_data": encoded.base64_data,
                })
            }

            // ── Mouse ──
            CliCommand::MouseMove {
                x,
                y,
                scale_ratio,
            } => {
                self.action.mouse_move(x, y, scale_ratio).await?;
                json!({"status": "ok"})
            }

            CliCommand::MouseMoveRelative { dx, dy } => {
                self.action.mouse_move_relative(dx, dy).await?;
                json!({"status": "ok"})
            }

            CliCommand::MouseClick {
                x,
                y,
                button,
                click_type,
                scale_ratio,
            } => {
                let button = parse_mouse_button(Some(&button));
                let click_type = parse_click_type(Some(&click_type));
                self.action
                    .mouse_click(x, y, button, click_type, scale_ratio)
                    .await?;
                json!({"status": "ok"})
            }

            CliCommand::MouseDrag {
                from_x,
                from_y,
                to_x,
                to_y,
                button,
                scale_ratio,
            } => {
                let button = parse_mouse_button(Some(&button));
                self.action
                    .mouse_drag(from_x, from_y, to_x, to_y, button, scale_ratio)
                    .await?;
                json!({"status": "ok"})
            }

            // ── Keyboard ──
            CliCommand::KeyboardType { text } => {
                self.action.keyboard_type(text).await?;
                json!({"status": "ok"})
            }

            CliCommand::KeyboardPress { keys } => {
                self.action.keyboard_press(keys).await?;
                json!({"status": "ok"})
            }

            // ── Context ──
            CliCommand::GetOsContext => {
                let ctx = self.context.get_os_context();
                json!({"os_type": ctx.os_type})
            }

            CliCommand::ClipboardRead => {
                let text = self.context.clipboard_read().await?;
                json!({"text": text})
            }

            CliCommand::ClipboardWrite { text } => {
                self.context.clipboard_write(text).await?;
                json!({"status": "ok"})
            }

            CliCommand::GetActiveWindow => {
                let win = self.context.get_active_window().await?;
                serde_json::to_value(&win)?
            }

            CliCommand::ListWindows => {
                let windows = self.context.list_windows().await?;
                serde_json::to_value(&windows)?
            }

            CliCommand::FocusWindow { window_id } => {
                self.context.focus_window(window_id).await?;
                json!({"status": "ok"})
            }

            CliCommand::GetPermissions => {
                let status = self.context.get_permission_status().await?;
                serde_json::to_value(&status)?
            }

            CliCommand::RequestPermissions => {
                let status = self.context.request_permissions().await?;
                serde_json::to_value(&status)?
            }

            // ── OCR ──
            CliCommand::GetOcrData { display_id } => {
                let result = self.ocr.get_ocr_data(display_id).await?;
                serde_json::to_value(&result)?
            }

            CliCommand::FindTextCenter {
                display_id,
                target_text,
            } => {
                let result = self.ocr.find_text_center(display_id, target_text).await?;
                match result {
                    Some(block) => serde_json::to_value(&block)?,
                    None => json!({"found": false}),
                }
            }

            CliCommand::ActionAssertion {
                x,
                y,
                width,
                height,
                expected_text,
            } => {
                let region = CaptureRegion {
                    x,
                    y,
                    width,
                    height,
                };
                let found = self.ocr.action_assertion(region, expected_text).await?;
                json!({"matched": found})
            }
        };

        println!("{}", serde_json::to_string(&result)?);
        Ok(())
    }
}

// ===========================================================================
// 工具函数
// ===========================================================================

fn parse_mouse_button(s: Option<&str>) -> MouseButton {
    match s {
        Some("right") => MouseButton::Right,
        Some("middle") => MouseButton::Middle,
        _ => MouseButton::Left,
    }
}

fn parse_click_type(s: Option<&str>) -> ClickType {
    match s {
        Some("double_click") => ClickType::DoubleClick,
        Some("press") => ClickType::Press,
        Some("release") => ClickType::Release,
        _ => ClickType::Click,
    }
}
