// ============================================================================
// 屏幕感知领域 (Vision Domain)
// ============================================================================
// 定义屏幕截图、显示器信息等核心能力的抽象接口和值对象。
// ============================================================================

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 值对象 (Value Objects)
// ---------------------------------------------------------------------------

/// 显示器元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    /// 显示器唯一ID
    pub id: u32,
    /// 显示器名称
    pub name: String,
    /// 分辨率宽度 (像素)
    pub width: u32,
    /// 分辨率高度 (像素)
    pub height: u32,
    /// 系统缩放比例 (如 Retina 屏为 2.0)
    pub scale_factor: f64,
    /// 在虚拟屏幕空间中的 X 偏移
    pub x: i32,
    /// 在虚拟屏幕空间中的 Y 偏移
    pub y: i32,
    /// 是否为主显示器
    pub is_primary: bool,
}

/// 截图区域 (绝对坐标)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 原始截图数据 (RGBA 像素)
pub struct RawImage {
    pub width: u32,
    pub height: u32,
    /// RGBA 格式的像素数据
    pub pixels: Vec<u8>,
}

/// 经过编码的图片 (可直接传输给 LLM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedImage {
    /// Base64 编码的图片数据
    pub base64_data: String,
    /// 图片格式 (如 "jpeg")
    pub format: String,
    /// 编码后的宽度
    pub width: u32,
    /// 编码后的高度
    pub height: u32,
    /// 原始宽度
    pub original_width: u32,
    /// 原始高度
    pub original_height: u32,
    /// 缩放比例 (original / encoded)，用于 LLM 坐标转换
    pub scale_ratio: f64,
}

// ---------------------------------------------------------------------------
// 领域 Trait (Domain Trait)
// ---------------------------------------------------------------------------

/// 屏幕捕获能力
///
/// 负责获取显示器信息和截取屏幕画面。
/// 实现者必须是 Send + Sync 以支持跨线程共享。
pub trait ScreenCapture: Send + Sync {
    /// 获取所有物理显示器的元数据
    fn get_displays(&self) -> anyhow::Result<Vec<DisplayInfo>>;

    /// 截取指定显示器的完整画面
    fn capture_display(&self, display_id: u32) -> anyhow::Result<RawImage>;

    /// 截取指定区域的画面 (绝对坐标)
    fn capture_region(&self, region: CaptureRegion) -> anyhow::Result<RawImage>;
}
