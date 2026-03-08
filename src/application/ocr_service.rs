// ============================================================================
// OCR 应用服务 (OCR Service)
// ============================================================================
// 编排 OcrEngine + ScreenCapture，提供 OCR 扫描、文本搜索、操作断言等高级功能。
// ============================================================================

use std::sync::Arc;

use crate::domain::ocr::{OcrEngine, OcrResult, TextBlock};
use crate::domain::vision::{CaptureRegion, ScreenCapture};

/// OCR 应用服务
///
/// 整合屏幕捕获和 OCR 引擎，提供全屏扫描、文本定位、操作验证等能力。
pub struct OcrService {
    ocr_engine: Arc<dyn OcrEngine>,
    capture: Arc<dyn ScreenCapture>,
}

impl OcrService {
    pub fn new(ocr_engine: Arc<dyn OcrEngine>, capture: Arc<dyn ScreenCapture>) -> Self {
        Self {
            ocr_engine,
            capture,
        }
    }

    /// 对指定显示器执行全屏 OCR 扫描
    ///
    /// 返回极简格式的文本块列表 (文本|X|Y)
    pub async fn get_ocr_data(&self, display_id: u32) -> anyhow::Result<OcrResult> {
        let capture = Arc::clone(&self.capture);
        let engine = Arc::clone(&self.ocr_engine);
        tokio::task::spawn_blocking(move || {
            let raw = capture.capture_display(display_id)?;
            engine.recognize(&raw)
        })
        .await?
    }

    /// 对指定区域执行 OCR 扫描
    pub async fn get_ocr_data_region(&self, region: CaptureRegion) -> anyhow::Result<OcrResult> {
        let capture = Arc::clone(&self.capture);
        let engine = Arc::clone(&self.ocr_engine);
        tokio::task::spawn_blocking(move || {
            let raw = capture.capture_region(region)?;
            engine.recognize(&raw)
        })
        .await?
    }

    /// 按文本精确检索坐标
    ///
    /// 在指定显示器上执行 OCR，模糊匹配目标文本，返回命中的中心点坐标。
    /// 使用 Levenshtein 距离进行模糊匹配。
    pub async fn find_text_center(
        &self,
        display_id: u32,
        target_text: String,
    ) -> anyhow::Result<Option<TextBlock>> {
        let capture = Arc::clone(&self.capture);
        let engine = Arc::clone(&self.ocr_engine);
        tokio::task::spawn_blocking(move || {
            let raw = capture.capture_display(display_id)?;
            let result = engine.recognize(&raw)?;
            Ok(find_best_match(&result, &target_text))
        })
        .await?
    }

    /// 操作状态断言
    ///
    /// 在指定区域执行 OCR，检测指定文本是否存在。
    /// 可用于验证点击或输入操作是否成功。
    pub async fn action_assertion(
        &self,
        region: CaptureRegion,
        expected_text: String,
    ) -> anyhow::Result<bool> {
        let capture = Arc::clone(&self.capture);
        let engine = Arc::clone(&self.ocr_engine);
        tokio::task::spawn_blocking(move || {
            let raw = capture.capture_region(region)?;
            let result = engine.recognize(&raw)?;
            let found = result
                .blocks
                .iter()
                .any(|b| b.text.contains(&expected_text));
            Ok(found)
        })
        .await?
    }
}

/// 在 OCR 结果中模糊匹配目标文本，返回最佳匹配
fn find_best_match(result: &OcrResult, target: &str) -> Option<TextBlock> {
    let target_lower = target.to_lowercase();

    // 1. 先尝试精确包含匹配
    if let Some(block) = result
        .blocks
        .iter()
        .find(|b| b.text.to_lowercase().contains(&target_lower))
    {
        return Some(block.clone());
    }

    // 2. 回退到模糊匹配 (Normalized Levenshtein >= 0.6)
    let mut best: Option<(f64, &TextBlock)> = None;
    for block in &result.blocks {
        let similarity = strsim::normalized_levenshtein(&block.text.to_lowercase(), &target_lower);
        if similarity >= 0.6 {
            if best.is_none() || similarity > best.unwrap().0 {
                best = Some((similarity, block));
            }
        }
    }

    best.map(|(_, block)| block.clone())
}
