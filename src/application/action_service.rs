// ============================================================================
// 物理外设模拟应用服务 (Action Service)
// ============================================================================
// 编排 MouseController + KeyboardController，提供带坐标变换的输入操作。
// ============================================================================

use std::sync::Arc;

use crate::domain::action::{ClickType, KeyboardController, MouseButton, MouseController, Position};

/// 物理外设模拟应用服务
///
/// 包装鼠标/键盘控制能力，并提供坐标缩放变换功能。
/// 当 LLM 基于压缩后的截图进行操作时，需要通过 scale_ratio 将压缩坐标
/// 转换为原始屏幕物理坐标。
pub struct ActionService {
    mouse: Arc<dyn MouseController>,
    keyboard: Arc<dyn KeyboardController>,
}

impl ActionService {
    pub fn new(
        mouse: Arc<dyn MouseController>,
        keyboard: Arc<dyn KeyboardController>,
    ) -> Self {
        Self { mouse, keyboard }
    }

    /// 移动鼠标到绝对坐标
    ///
    /// # Parameters
    /// - `x`, `y`: 目标坐标 (压缩坐标系)
    /// - `scale_ratio`: 坐标缩放比例 (原始/压缩)，为 1.0 则不缩放
    pub async fn mouse_move(
        &self,
        x: i32,
        y: i32,
        scale_ratio: f64,
    ) -> anyhow::Result<()> {
        let real_x = (x as f64 * scale_ratio) as i32;
        let real_y = (y as f64 * scale_ratio) as i32;
        let mouse = Arc::clone(&self.mouse);
        tokio::task::spawn_blocking(move || mouse.move_to(real_x, real_y)).await?
    }

    /// 鼠标相对移动
    pub async fn mouse_move_relative(&self, dx: i32, dy: i32) -> anyhow::Result<()> {
        let mouse = Arc::clone(&self.mouse);
        tokio::task::spawn_blocking(move || mouse.move_relative(dx, dy)).await?
    }

    /// 鼠标点击
    ///
    /// 先移动到目标位置，再执行点击操作。
    pub async fn mouse_click(
        &self,
        x: i32,
        y: i32,
        button: MouseButton,
        click_type: ClickType,
        scale_ratio: f64,
    ) -> anyhow::Result<()> {
        let real_x = (x as f64 * scale_ratio) as i32;
        let real_y = (y as f64 * scale_ratio) as i32;
        let mouse = Arc::clone(&self.mouse);
        tokio::task::spawn_blocking(move || {
            mouse.move_to(real_x, real_y)?;
            mouse.click(button, click_type)
        })
        .await?
    }

    /// 鼠标拖拽
    pub async fn mouse_drag(
        &self,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        button: MouseButton,
        scale_ratio: f64,
    ) -> anyhow::Result<()> {
        let from = Position {
            x: (from_x as f64 * scale_ratio) as i32,
            y: (from_y as f64 * scale_ratio) as i32,
        };
        let to = Position {
            x: (to_x as f64 * scale_ratio) as i32,
            y: (to_y as f64 * scale_ratio) as i32,
        };
        let mouse = Arc::clone(&self.mouse);
        tokio::task::spawn_blocking(move || mouse.drag(from, to, button)).await?
    }

    /// 键盘文本输入
    pub async fn keyboard_type(&self, text: String) -> anyhow::Result<()> {
        let keyboard = Arc::clone(&self.keyboard);
        tokio::task::spawn_blocking(move || keyboard.type_text(&text)).await?
    }

    /// 组合快捷键
    pub async fn keyboard_press(&self, keys: Vec<String>) -> anyhow::Result<()> {
        let keyboard = Arc::clone(&self.keyboard);
        tokio::task::spawn_blocking(move || keyboard.press_key(&keys)).await?
    }
}
