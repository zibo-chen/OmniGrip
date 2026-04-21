// ============================================================================
// MCP 协议适配器 (MCP Server Adapter)
// ============================================================================
// 将应用层服务映射为 MCP Tool，通过 rmcp 框架对外暴露。
// 本层是唯一依赖 rmcp 的位置，核心业务逻辑完全解耦。
// ============================================================================

use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_router,
};
use serde::Deserialize;

use crate::application::{
    action_service::ActionService, context_service::ContextService, ocr_service::OcrService,
    vision_service::VisionService,
};
use crate::domain::action::{ClickType, MouseButton};
use crate::domain::vision::CaptureRegion;

// ===========================================================================
// MCP Tool 参数定义 (使用 schemars 自动生成 JSON Schema)
// ===========================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDisplaysParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TakeScreenshotParams {
    /// 显示器 ID (从 get_displays 获取)
    pub display_id: u32,
    /// 最大宽度限制 (像素)，超过则等比缩放，建议 1000
    pub max_width: Option<u32>,
    /// 最大高度限制 (像素)，超过则等比缩放，建议 1000
    pub max_height: Option<u32>,
    /// JPEG 压缩质量 (1-100)，默认 80
    pub quality: Option<u8>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TakeScreenshotRegionParams {
    /// 区域左上角 X 坐标
    pub x: i32,
    /// 区域左上角 Y 坐标
    pub y: i32,
    /// 区域宽度
    pub width: u32,
    /// 区域高度
    pub height: u32,
    /// 最大宽度限制
    pub max_width: Option<u32>,
    /// 最大高度限制
    pub max_height: Option<u32>,
    /// JPEG 质量
    pub quality: Option<u8>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MouseMoveParams {
    /// 目标 X 坐标
    pub x: i32,
    /// 目标 Y 坐标
    pub y: i32,
    /// 坐标缩放比例 (原始/截图分辨率)，默认 1.0 表示不缩放
    pub scale_ratio: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MouseMoveRelativeParams {
    /// X 方向偏移量
    pub dx: i32,
    /// Y 方向偏移量
    pub dy: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MouseClickParams {
    /// 点击位置 X 坐标
    pub x: i32,
    /// 点击位置 Y 坐标
    pub y: i32,
    /// 鼠标按键: "left", "right", "middle"
    pub button: Option<String>,
    /// 点击类型: "click", "double_click", "press", "release"
    pub click_type: Option<String>,
    /// 坐标缩放比例
    pub scale_ratio: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MouseDragParams {
    /// 起点 X
    pub from_x: i32,
    /// 起点 Y
    pub from_y: i32,
    /// 终点 X
    pub to_x: i32,
    /// 终点 Y
    pub to_y: i32,
    /// 鼠标按键
    pub button: Option<String>,
    /// 坐标缩放比例
    pub scale_ratio: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct KeyboardTypeParams {
    /// 要输入的文本内容
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct KeyboardPressParams {
    /// 按键组合，如 ["ctrl", "c"] 或 ["enter"]
    pub keys: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClipboardWriteParams {
    /// 要写入剪贴板的文本
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPermissionsParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RequestPermissionsParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FocusWindowParams {
    /// 窗口 ID (从 list_windows 获取)
    pub window_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetOcrDataParams {
    /// 显示器 ID
    pub display_id: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindTextCenterParams {
    /// 显示器 ID
    pub display_id: u32,
    /// 要查找的目标文本
    pub target_text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ActionAssertionParams {
    /// 断言区域左上角 X
    pub x: i32,
    /// 断言区域左上角 Y
    pub y: i32,
    /// 断言区域宽度
    pub width: u32,
    /// 断言区域高度
    pub height: u32,
    /// 期望存在的文本
    pub expected_text: String,
}

// ===========================================================================
// MCP Server 主体
// ===========================================================================

/// OmniGrip MCP Server
///
/// 将四大应用服务映射为 MCP Tool，通过 stdio 与 LLM 通信。
#[derive(Clone)]
pub struct OmniGripMcpServer {
    vision: Arc<VisionService>,
    action: Arc<ActionService>,
    context: Arc<ContextService>,
    ocr: Arc<OcrService>,
    tool_router: ToolRouter<Self>,
}

impl OmniGripMcpServer {
    pub fn new(
        vision: Arc<VisionService>,
        action: Arc<ActionService>,
        context: Arc<ContextService>,
        ocr: Arc<OcrService>,
    ) -> Self {
        // 先创建不含 tool_router 的临时实例，然后补入
        Self {
            vision,
            action,
            context,
            ocr,
            tool_router: Self::tool_router(),
        }
    }
}

// ===========================================================================
// MCP Tool 实现 (通过 rmcp 宏自动注册)
// ===========================================================================

#[tool_router]
impl OmniGripMcpServer {
    // -----------------------------------------------------------------------
    // 2.1 屏幕感知模块 (Vision)
    // -----------------------------------------------------------------------

    /// 获取所有物理显示器的元数据 (ID, 分辨率, 缩放比例, 坐标偏移)
    #[tool(description = "Get all display/monitor metadata including ID, resolution, scale factor, and position offset. Call this first to get display_id for screenshots.")]
    async fn get_displays(
        &self,
        _params: Parameters<GetDisplaysParams>,
    ) -> Result<CallToolResult, McpError> {
        let displays = self
            .vision
            .get_displays()
            .await
            .map_err(|e| McpError::internal_error(format!("get_displays failed: {}", e), None))?;

        // 使用极简文本格式输出
        let text = displays
            .iter()
            .map(|d| {
                format!(
                    "id:{}|name:{}|{}x{}|scale:{}|offset:({},{})|primary:{}",
                    d.id,
                    d.name,
                    d.width,
                    d.height,
                    d.scale_factor,
                    d.x,
                    d.y,
                    d.is_primary
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    /// 截取指定显示器的完整画面，返回 JPEG Base64 图像。
    /// 支持设置最大分辨率限制实现自动缩放，返回的 scale_ratio 可用于后续坐标转换。
    #[tool(description = "Take a full screenshot of the specified display. Returns JPEG base64 image with scale_ratio for coordinate conversion. Set max_width/max_height to limit resolution (e.g. 1000).")]
    async fn take_screenshot(
        &self,
        params: Parameters<TakeScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let quality = p.quality.unwrap_or(80);

        let encoded = self
            .vision
            .take_screenshot(p.display_id, p.max_width, p.max_height, quality)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("take_screenshot failed: {}", e), None)
            })?;

        // 返回图像和元数据
        let meta = format!(
            "size:{}x{}|original:{}x{}|scale_ratio:{}",
            encoded.width, encoded.height, encoded.original_width, encoded.original_height, encoded.scale_ratio
        );

        Ok(CallToolResult::success(vec![
            Content::image(encoded.base64_data, "image/jpeg"),
            Content::text(meta),
        ]))
    }

    /// 截取屏幕的指定区域
    #[tool(description = "Take a screenshot of a specific screen region (absolute coordinates). Useful for focused verification with lower token cost.")]
    async fn take_screenshot_region(
        &self,
        params: Parameters<TakeScreenshotRegionParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let quality = p.quality.unwrap_or(80);
        let region = CaptureRegion {
            x: p.x,
            y: p.y,
            width: p.width,
            height: p.height,
        };

        let encoded = self
            .vision
            .take_screenshot_region(region, p.max_width, p.max_height, quality)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("take_screenshot_region failed: {}", e), None)
            })?;

        let meta = format!(
            "size:{}x{}|original:{}x{}|scale_ratio:{}",
            encoded.width, encoded.height, encoded.original_width, encoded.original_height, encoded.scale_ratio
        );

        Ok(CallToolResult::success(vec![
            Content::image(encoded.base64_data, "image/jpeg"),
            Content::text(meta),
        ]))
    }

    // -----------------------------------------------------------------------
    // 2.2 物理外设模拟模块 (Action)
    // -----------------------------------------------------------------------

    /// 将鼠标移动到指定绝对坐标
    #[tool(description = "Move mouse cursor to absolute screen coordinates (x, y). Use scale_ratio from screenshot metadata to convert compressed coordinates to real screen coordinates.")]
    async fn mouse_move(
        &self,
        params: Parameters<MouseMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let scale = p.scale_ratio.unwrap_or(1.0);
        self.action
            .mouse_move(p.x, p.y, scale)
            .await
            .map_err(|e| McpError::internal_error(format!("mouse_move failed: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    /// 鼠标相对移动
    #[tool(description = "Move mouse cursor relative to current position by (dx, dy) pixels.")]
    async fn mouse_move_relative(
        &self,
        params: Parameters<MouseMoveRelativeParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        self.action
            .mouse_move_relative(p.dx, p.dy)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("mouse_move_relative failed: {}", e), None)
            })?;
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    /// 在指定坐标执行鼠标点击
    #[tool(description = "Click mouse at specified coordinates. button: left/right/middle (default: left). click_type: click/double_click/press/release (default: click). Use scale_ratio for coordinate conversion.")]
    async fn mouse_click(
        &self,
        params: Parameters<MouseClickParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let button = parse_mouse_button(p.button.as_deref());
        let click_type = parse_click_type(p.click_type.as_deref());
        let scale = p.scale_ratio.unwrap_or(1.0);

        self.action
            .mouse_click(p.x, p.y, button, click_type, scale)
            .await
            .map_err(|e| McpError::internal_error(format!("mouse_click failed: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    /// 鼠标拖拽操作
    #[tool(description = "Drag mouse from (from_x, from_y) to (to_x, to_y). Simulates press at start, move, release at end.")]
    async fn mouse_drag(
        &self,
        params: Parameters<MouseDragParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let button = parse_mouse_button(p.button.as_deref());
        let scale = p.scale_ratio.unwrap_or(1.0);

        self.action
            .mouse_drag(p.from_x, p.from_y, p.to_x, p.to_y, button, scale)
            .await
            .map_err(|e| McpError::internal_error(format!("mouse_drag failed: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    /// 模拟键盘文本输入
    #[tool(description = "Type text string via keyboard simulation. Handles Unicode characters including CJK.")]
    async fn keyboard_type(
        &self,
        params: Parameters<KeyboardTypeParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        self.action
            .keyboard_type(p.text)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("keyboard_type failed: {}", e), None)
            })?;
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    /// 触发组合快捷键
    #[tool(description = "Press keyboard shortcut. Keys are pressed in order (modifiers first, last key is clicked). Examples: [\"cmd\", \"c\"] for Cmd+C, [\"enter\"] for Enter.")]
    async fn keyboard_press(
        &self,
        params: Parameters<KeyboardPressParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        self.action
            .keyboard_press(p.keys)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("keyboard_press failed: {}", e), None)
            })?;
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    // -----------------------------------------------------------------------
    // 2.3 窗口与系统上下文模块 (Context)
    // -----------------------------------------------------------------------

    /// 获取当前操作系统类型
    #[tool(description = "Get current OS type (windows/macos/linux). Use this to determine platform-specific shortcuts (e.g., Cmd vs Ctrl).")]
    async fn get_os_context(
        &self,
        _params: Parameters<GetDisplaysParams>,
    ) -> Result<CallToolResult, McpError> {
        let ctx = self.context.get_os_context();
        Ok(CallToolResult::success(vec![Content::text(ctx.os_type)]))
    }

    /// 读取剪贴板内容
    #[tool(description = "Read current clipboard text content.")]
    async fn clipboard_read(
        &self,
        _params: Parameters<GetDisplaysParams>,
    ) -> Result<CallToolResult, McpError> {
        let text = self
            .context
            .clipboard_read()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("clipboard_read failed: {}", e), None)
            })?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    /// 写入文本到剪贴板
    #[tool(description = "Write text to system clipboard. Most reliable way to transfer long text/code to local machine.")]
    async fn clipboard_write(
        &self,
        params: Parameters<ClipboardWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        self.context
            .clipboard_write(params.0.text)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("clipboard_write failed: {}", e), None)
            })?;
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    /// 获取当前焦点窗口信息
    #[tool(description = "Get the currently focused/frontmost window's title and app name.")]
    async fn get_active_window(
        &self,
        _params: Parameters<GetDisplaysParams>,
    ) -> Result<CallToolResult, McpError> {
        let win = self
            .context
            .get_active_window()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("get_active_window failed: {}", e), None)
            })?;
        let text = format!("id:{}|app:{}|title:{}", win.id, win.app_name, win.title);
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    /// 枚举所有可见窗口
    #[tool(description = "List all visible windows with their IDs, app names, and titles. Use the ID with focus_window to switch to a specific window.")]
    async fn list_windows(
        &self,
        _params: Parameters<GetDisplaysParams>,
    ) -> Result<CallToolResult, McpError> {
        let windows = self
            .context
            .list_windows()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("list_windows failed: {}", e), None)
            })?;

        let text = windows
            .iter()
            .map(|w| format!("id:{}|app:{}|title:{}", w.id, w.app_name, w.title))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    /// 将指定窗口切换到前台
    #[tool(description = "Bring a specific window to the foreground by its ID (from list_windows).")]
    async fn focus_window(
        &self,
        params: Parameters<FocusWindowParams>,
    ) -> Result<CallToolResult, McpError> {
        self.context
            .focus_window(params.0.window_id)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("focus_window failed: {}", e), None)
            })?;
        Ok(CallToolResult::success(vec![Content::text("ok")]))
    }

    /// 获取当前系统权限状态
    #[tool(description = "Get current system permission status. On macOS this reports Accessibility and Screen Recording grant state, whether a restart is needed, and whether a prompt can be triggered.")]
    async fn get_permissions(
        &self,
        _params: Parameters<GetPermissionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let status = self
            .context
            .get_permission_status()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("get_permissions failed: {}", e), None)
            })?;

        let text = serde_json::to_string(&status).map_err(|e| {
            McpError::internal_error(format!("serialize permissions failed: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    /// 主动触发系统权限申请
    #[tool(description = "Trigger system permission requests and return the latest status snapshot. On macOS this requests Accessibility and Screen Recording permissions when missing.")]
    async fn request_permissions(
        &self,
        _params: Parameters<RequestPermissionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let status = self
            .context
            .request_permissions()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("request_permissions failed: {}", e), None)
            })?;

        let text = serde_json::to_string(&status).map_err(|e| {
            McpError::internal_error(format!("serialize permissions failed: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    // -----------------------------------------------------------------------
    // 2.4 OCR 模块
    // -----------------------------------------------------------------------

    /// 全屏 OCR 扫描，返回极简文本坐标数据
    #[tool(description = "Run OCR on full screen, returns text blocks with center-point coordinates in compact format: 'text|x|y' per line. Dramatically reduces token cost compared to screenshots.")]
    async fn get_ocr_data(
        &self,
        params: Parameters<GetOcrDataParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .ocr
            .get_ocr_data(params.0.display_id)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("get_ocr_data failed: {}", e), None)
            })?;
        Ok(CallToolResult::success(vec![Content::text(
            result.to_compact_text(),
        )]))
    }

    /// 按文本检索屏幕坐标
    #[tool(description = "Find a text string on screen via OCR and return its center-point coordinates. Supports fuzzy matching. Result can be directly used with mouse_click.")]
    async fn find_text_center(
        &self,
        params: Parameters<FindTextCenterParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let result = self
            .ocr
            .find_text_center(p.display_id, p.target_text.clone())
            .await
            .map_err(|e| {
                McpError::internal_error(format!("find_text_center failed: {}", e), None)
            })?;

        match result {
            Some(block) => {
                let text = format!(
                    "found:{}|x:{}|y:{}|confidence:{}",
                    block.text, block.center_x, block.center_y, block.confidence
                );
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            None => Ok(CallToolResult::success(vec![Content::text(format!(
                "not_found:{}",
                p.target_text
            ))])),
        }
    }

    /// 操作验证断言
    #[tool(description = "Verify an action succeeded by checking if expected text appears in a screen region via OCR. Returns true/false. Use after click/type to confirm the operation worked.")]
    async fn action_assertion(
        &self,
        params: Parameters<ActionAssertionParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let region = CaptureRegion {
            x: p.x,
            y: p.y,
            width: p.width,
            height: p.height,
        };

        let found = self
            .ocr
            .action_assertion(region, p.expected_text)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("action_assertion failed: {}", e), None)
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            found.to_string(),
        )]))
    }
}

// ===========================================================================
// ServerHandler 实现
// ===========================================================================

#[rmcp::tool_handler]
impl ServerHandler for OmniGripMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "OmniGrip".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: Some("OmniGrip Computer Control".into()),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "OmniGrip is a cross-platform computer control MCP server. \
                 It provides screen capture, mouse/keyboard simulation, window management, \
                 and OCR capabilities for LLM-driven GUI automation."
                    .into(),
            ),
        }
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
