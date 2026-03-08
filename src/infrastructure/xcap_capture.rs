// ============================================================================
// 屏幕捕获基础设施 - xcap 实现
// ============================================================================

use crate::domain::vision::{CaptureRegion, DisplayInfo, RawImage, ScreenCapture};
use xcap::Monitor;

/// 基于 xcap 库的屏幕捕获实现
pub struct XcapCapture;

impl XcapCapture {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenCapture for XcapCapture {
    fn get_displays(&self) -> anyhow::Result<Vec<DisplayInfo>> {
        let monitors = Monitor::all()?;
        let mut displays = Vec::new();
        for m in &monitors {
            displays.push(DisplayInfo {
                id: m.id()?,
                name: m.name()?,
                width: m.width()?,
                height: m.height()?,
                scale_factor: m.scale_factor()? as f64,
                x: m.x()?,
                y: m.y()?,
                is_primary: m.is_primary()?,
            });
        }
        Ok(displays)
    }

    fn capture_display(&self, display_id: u32) -> anyhow::Result<RawImage> {
        let monitors = Monitor::all()?;
        let monitor = monitors
            .into_iter()
            .find(|m| m.id().unwrap_or(0) == display_id)
            .ok_or_else(|| anyhow::anyhow!("Display {} not found", display_id))?;

        let image = monitor.capture_image()?;
        Ok(RawImage {
            width: image.width(),
            height: image.height(),
            pixels: image.into_raw(),
        })
    }

    fn capture_region(&self, region: CaptureRegion) -> anyhow::Result<RawImage> {
        let monitors = Monitor::all()?;

        // 优先查找完全包含该区域的显示器
        let monitor = monitors
            .iter()
            .find(|m| {
                let mx = m.x().unwrap_or(0);
                let my = m.y().unwrap_or(0);
                let mw = m.width().unwrap_or(0) as i32;
                let mh = m.height().unwrap_or(0) as i32;
                region.x >= mx
                    && region.y >= my
                    && region.x + region.width as i32 <= mx + mw
                    && region.y + region.height as i32 <= my + mh
            })
            // 回退: 找到区域起始点所在的显示器
            .or_else(|| {
                monitors.iter().find(|m| {
                    let mx = m.x().unwrap_or(0);
                    let my = m.y().unwrap_or(0);
                    let mw = m.width().unwrap_or(0) as i32;
                    let mh = m.height().unwrap_or(0) as i32;
                    region.x >= mx
                        && region.x < mx + mw
                        && region.y >= my
                        && region.y < my + mh
                })
            })
            // 最终回退: 使用主显示器
            .or_else(|| monitors.iter().find(|m| m.is_primary().unwrap_or(false)))
            .or_else(|| monitors.first())
            .ok_or_else(|| anyhow::anyhow!("No display available for region capture"))?;

        let mon_x = monitor.x()?;
        let mon_y = monitor.y()?;
        let mon_w = monitor.width()? as i32;
        let mon_h = monitor.height()? as i32;

        // 将区域裁剪到显示器范围内
        let crop_x = ((region.x - mon_x).max(0)) as u32;
        let crop_y = ((region.y - mon_y).max(0)) as u32;
        let max_w = (mon_w - crop_x as i32).max(0) as u32;
        let max_h = (mon_h - crop_y as i32).max(0) as u32;
        let crop_w = region.width.min(max_w);
        let crop_h = region.height.min(max_h);

        if crop_w == 0 || crop_h == 0 {
            anyhow::bail!("Region is outside of all displays");
        }

        let image = monitor.capture_region(crop_x, crop_y, crop_w, crop_h)?;

        Ok(RawImage {
            width: image.width(),
            height: image.height(),
            pixels: image.into_raw(),
        })
    }
}
