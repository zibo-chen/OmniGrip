// ============================================================================
// 屏幕文本识别领域 (OCR Domain)
// ============================================================================
// 定义 OCR 扫描、文本定位等能力的抽象接口和值对象。
// 依赖 vision 领域的 RawImage 类型作为输入。
// ============================================================================

use serde::{Deserialize, Serialize};

use super::vision::RawImage;

// ---------------------------------------------------------------------------
// 值对象 (Value Objects)
// ---------------------------------------------------------------------------

/// OCR 识别出的文本块
///
/// 按照需求，抛弃传统的 (x, y, w, h) 边界框，仅保留中心点坐标。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    /// 识别出的文本内容
    pub text: String,
    /// 文本区域中心点 X 坐标
    pub center_x: i32,
    /// 文本区域中心点 Y 坐标
    pub center_y: i32,
    /// 识别置信度 (0.0 ~ 1.0)
    pub confidence: f32,
}

/// OCR 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub blocks: Vec<TextBlock>,
}

impl OcrResult {
    /// 将 OCR 结果转换为极简的管道分隔文本格式
    ///
    /// 格式: `文本内容|X坐标|Y坐标`，每行一个文本块。
    /// 用于极致压缩传输体积，节省 LLM 的上下文空间。
    pub fn to_compact_text(&self) -> String {
        self.blocks
            .iter()
            .map(|b| format!("{}|{}|{}", b.text, b.center_x, b.center_y))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ---------------------------------------------------------------------------
// 领域 Trait (Domain Trait)
// ---------------------------------------------------------------------------

/// OCR 引擎能力
///
/// 对原始图像数据执行文字识别，返回文本块列表。
pub trait OcrEngine: Send + Sync {
    /// 识别图像中的文字
    fn recognize(&self, image: &RawImage) -> anyhow::Result<OcrResult>;
}
