// ============================================================================
// 图像处理工具 (Infrastructure Utility)
// ============================================================================
// 提供图像缩放、JPEG 编码、Base64 转换等纯函数工具。
// 被应用层 VisionService 调用，不属于领域概念。
// ============================================================================

use std::io::Cursor;

use base64::Engine;
use image::{DynamicImage, ImageEncoder, RgbaImage, codecs::jpeg::JpegEncoder, imageops::FilterType};

use crate::domain::vision::{EncodedImage, RawImage};

/// 将原始图像编码为 Base64 JPEG 格式
///
/// # Parameters
/// - `raw`: 原始 RGBA 图像数据
/// - `max_width`: 最大宽度限制 (None 则不限制)
/// - `max_height`: 最大高度限制 (None 则不限制)
/// - `quality`: JPEG 压缩质量 (1-100)
///
/// # Returns
/// 包含 Base64 编码数据和坐标缩放比例的 EncodedImage
pub fn encode_to_jpeg_base64(
    raw: &RawImage,
    max_width: Option<u32>,
    max_height: Option<u32>,
    quality: u8,
) -> anyhow::Result<EncodedImage> {
    let rgba = RgbaImage::from_raw(raw.width, raw.height, raw.pixels.clone())
        .ok_or_else(|| anyhow::anyhow!("Invalid raw image data"))?;

    let original_width = raw.width;
    let original_height = raw.height;

    let mut img = DynamicImage::ImageRgba8(rgba);

    // 按最大分辨率约束进行等比缩放
    let max_w = max_width.unwrap_or(original_width);
    let max_h = max_height.unwrap_or(original_height);

    if original_width > max_w || original_height > max_h {
        img = img.resize(max_w, max_h, FilterType::Lanczos3);
    }

    let actual_width = img.width();
    let actual_height = img.height();

    // 编码为 JPEG
    let rgb = img.to_rgb8();
    let mut buf = Cursor::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
    encoder.write_image(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;

    let base64_data = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());

    // 计算缩放比例 (原始像素 / 编码后像素)
    let scale_ratio = if actual_width > 0 {
        original_width as f64 / actual_width as f64
    } else {
        1.0
    };

    Ok(EncodedImage {
        base64_data,
        format: "jpeg".to_string(),
        width: actual_width,
        height: actual_height,
        original_width,
        original_height,
        scale_ratio,
    })
}
