// ============================================================================
// OCR 引擎基础设施 - ocr-rs 实现
// ============================================================================

use std::path::Path;

use crate::domain::ocr::{OcrEngine, OcrResult, TextBlock};
use crate::domain::vision::RawImage;

/// 基于 ocr-rs (PaddleOCR + MNN) 的 OCR 引擎实现
pub struct OcrRsEngine {
    engine: ocr_rs::OcrEngine,
}

impl OcrRsEngine {
    /// 从模型文件路径创建 OCR 引擎
    ///
    /// # Parameters
    /// - `det_model_path`: 检测模型文件路径 (det_model.mnn)
    /// - `rec_model_path`: 识别模型文件路径 (rec_model.mnn)
    /// - `charset_path`: 字符集文件路径 (ppocr_keys.txt)
    pub fn new(
        det_model_path: impl AsRef<Path>,
        rec_model_path: impl AsRef<Path>,
        charset_path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let config = ocr_rs::OcrEngineConfig::fast();
        let engine =
            ocr_rs::OcrEngine::new(det_model_path, rec_model_path, charset_path, Some(config))?;
        Ok(Self { engine })
    }

    /// 从内存中的模型数据创建 OCR 引擎
    pub fn from_bytes(
        det_model_bytes: &[u8],
        rec_model_bytes: &[u8],
        charset_bytes: &[u8],
    ) -> anyhow::Result<Self> {
        let config = ocr_rs::OcrEngineConfig::fast();
        let engine = ocr_rs::OcrEngine::from_bytes(
            det_model_bytes,
            rec_model_bytes,
            charset_bytes,
            Some(config),
        )?;
        Ok(Self { engine })
    }
}

impl OcrEngine for OcrRsEngine {
    fn recognize(&self, image: &RawImage) -> anyhow::Result<OcrResult> {
        // 将 RawImage (RGBA) 转换为 image::DynamicImage
        let rgba = image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone())
            .ok_or_else(|| anyhow::anyhow!("Invalid raw image data"))?;
        let dynamic = image::DynamicImage::ImageRgba8(rgba);

        // 执行 OCR 识别
        let results = self.engine.recognize(&dynamic)?;

        // 转换为领域模型: 计算每个文本块的中心点坐标
        let blocks = results
            .into_iter()
            .map(|r| {
                let rect = &r.bbox.rect;
                let center_x = rect.left() + (rect.width() as i32) / 2;
                let center_y = rect.top() + (rect.height() as i32) / 2;
                TextBlock {
                    text: r.text,
                    center_x,
                    center_y,
                    confidence: r.confidence,
                }
            })
            .collect();

        Ok(OcrResult { blocks })
    }
}
