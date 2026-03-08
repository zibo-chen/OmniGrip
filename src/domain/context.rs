// ============================================================================
// 窗口与系统上下文领域 (Context Domain)
// ============================================================================
// 定义剪贴板、窗口管理、OS 上下文等系统级信息获取能力的抽象接口。
// ============================================================================

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 值对象 (Value Objects)
// ---------------------------------------------------------------------------

/// 操作系统上下文信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsContext {
    /// 操作系统类型: "windows", "macos", "linux"
    pub os_type: String,
}

/// 窗口信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    /// 窗口唯一 ID
    pub id: String,
    /// 窗口标题
    pub title: String,
    /// 所属应用进程名
    pub app_name: String,
}

// ---------------------------------------------------------------------------
// 领域 Trait (Domain Traits)
// ---------------------------------------------------------------------------

/// 剪贴板管理能力
///
/// 读写系统剪贴板的纯文本内容。
pub trait ClipboardManager: Send + Sync {
    /// 读取剪贴板文本
    fn read_text(&self) -> anyhow::Result<String>;

    /// 写入文本到剪贴板
    fn write_text(&self, text: &str) -> anyhow::Result<()>;
}

/// 窗口管理能力
///
/// 获取窗口列表、焦点窗口、切换窗口前台等操作。
pub trait WindowManager: Send + Sync {
    /// 获取当前焦点窗口信息
    fn get_active_window(&self) -> anyhow::Result<WindowInfo>;

    /// 枚举所有可见窗口
    fn list_windows(&self) -> anyhow::Result<Vec<WindowInfo>>;

    /// 将指定窗口切换到前台
    fn focus_window(&self, window_id: &str) -> anyhow::Result<()>;
}

/// 操作系统上下文提供者
///
/// 提供当前运行的操作系统类型信息。
pub trait OsContextProvider: Send + Sync {
    fn get_os_context(&self) -> OsContext;
}
