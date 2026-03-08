// ============================================================================
// 物理外设模拟领域 (Action Domain)
// ============================================================================
// 定义鼠标、键盘等 HID 设备模拟能力的抽象接口和值对象。
// ============================================================================

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 值对象 (Value Objects)
// ---------------------------------------------------------------------------

/// 屏幕坐标点
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// 鼠标按键类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// 点击行为类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClickType {
    /// 单击 (按下+释放)
    Click,
    /// 双击
    DoubleClick,
    /// 仅按下
    Press,
    /// 仅释放
    Release,
}

// ---------------------------------------------------------------------------
// 领域 Trait (Domain Traits)
// ---------------------------------------------------------------------------

/// 鼠标控制能力
///
/// 模拟鼠标的移动、点击、拖拽操作。
/// 实现者使用内部可变性 (Mutex) 以保证 &self 签名下的线程安全。
pub trait MouseController: Send + Sync {
    /// 移动鼠标到绝对坐标
    fn move_to(&self, x: i32, y: i32) -> anyhow::Result<()>;

    /// 基于当前位置进行相对移动
    fn move_relative(&self, dx: i32, dy: i32) -> anyhow::Result<()>;

    /// 执行鼠标点击 (支持多种按键和点击类型)
    fn click(&self, button: MouseButton, click_type: ClickType) -> anyhow::Result<()>;

    /// 拖拽操作: 从起点按下，移动到终点释放
    fn drag(&self, from: Position, to: Position, button: MouseButton) -> anyhow::Result<()>;
}

/// 键盘控制能力
///
/// 模拟键盘的文本输入和快捷键操作。
pub trait KeyboardController: Send + Sync {
    /// 输入一段文本 (底层处理 Unicode 映射)
    fn type_text(&self, text: &str) -> anyhow::Result<()>;

    /// 触发组合快捷键 (如 ["ctrl", "c"] 表示 Ctrl+C)
    fn press_key(&self, keys: &[String]) -> anyhow::Result<()>;
}
