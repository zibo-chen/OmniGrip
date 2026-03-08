// ============================================================================
// 屏幕感知应用服务 (Vision Service)
// ============================================================================
// 编排 ScreenCapture + ImageProcessor，提供截图+编码+缩放的完整流程。
// ============================================================================

use std::sync::Arc;

use crate::domain::vision::{CaptureRegion, DisplayInfo, EncodedImage, ScreenCapture};
use crate::infrastructure::image_proc;

/// 屏幕感知应用服务
///
/// 封装截图→缩放→编码的完整管线，对外暴露异步接口供协议适配层调用。
pub struct VisionService {
    capture: Arc<dyn ScreenCapture>,
}

impl VisionService {
    pub fn new(capture: Arc<dyn ScreenCapture>) -> Self {
        Self { capture }
    }

    /// 获取所有显示器信息
    pub async fn get_displays(&self) -> anyhow::Result<Vec<DisplayInfo>> {
        let capture = Arc::clone(&self.capture);
        tokio::task::spawn_blocking(move || capture.get_displays()).await?
    }

    /// 截取指定显示器并编码为 JPEG Base64
    ///
    /// # Parameters
    /// - `display_id`: 显示器 ID
    /// - `max_width`: 最大宽度限制 (如 1000)
    /// - `max_height`: 最大高度限制 (如 1000)
    /// - `quality`: JPEG 质量 (1-100, 推荐 80)
    pub async fn take_screenshot(
        &self,
        display_id: u32,
        max_width: Option<u32>,
        max_height: Option<u32>,
        quality: u8,
    ) -> anyhow::Result<EncodedImage> {
        let capture = Arc::clone(&self.capture);
        tokio::task::spawn_blocking(move || {
            let raw = capture.capture_display(display_id)?;
            image_proc::encode_to_jpeg_base64(&raw, max_width, max_height, quality)
        })
        .await?
    }

    /// 截取指定区域并编码为 JPEG Base64
    pub async fn take_screenshot_region(
        &self,
        region: CaptureRegion,
        max_width: Option<u32>,
        max_height: Option<u32>,
        quality: u8,
    ) -> anyhow::Result<EncodedImage> {
        let capture = Arc::clone(&self.capture);
        tokio::task::spawn_blocking(move || {
            let raw = capture.capture_region(region)?;
            image_proc::encode_to_jpeg_base64(&raw, max_width, max_height, quality)
        })
        .await?
    }
}
