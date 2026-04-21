// ============================================================================
// 窗口与系统上下文应用服务 (Context Service)
// ============================================================================

use std::sync::Arc;

use crate::domain::context::{
    ClipboardManager, OsContext, OsContextProvider, PermissionManager, PermissionStatus,
    WindowInfo, WindowManager,
};

/// 系统上下文应用服务
///
/// 整合剪贴板、窗口管理、OS 上下文等系统级操作。
pub struct ContextService {
    clipboard: Arc<dyn ClipboardManager>,
    window_mgr: Arc<dyn WindowManager>,
    os_ctx: Arc<dyn OsContextProvider>,
    permissions: Arc<dyn PermissionManager>,
}

impl ContextService {
    pub fn new(
        clipboard: Arc<dyn ClipboardManager>,
        window_mgr: Arc<dyn WindowManager>,
        os_ctx: Arc<dyn OsContextProvider>,
        permissions: Arc<dyn PermissionManager>,
    ) -> Self {
        Self {
            clipboard,
            window_mgr,
            os_ctx,
            permissions,
        }
    }

    /// 获取当前操作系统类型
    pub fn get_os_context(&self) -> OsContext {
        self.os_ctx.get_os_context()
    }

    /// 获取当前系统权限状态
    pub async fn get_permission_status(&self) -> anyhow::Result<PermissionStatus> {
        let permissions = Arc::clone(&self.permissions);
        tokio::task::spawn_blocking(move || permissions.get_permission_status()).await?
    }

    /// 主动触发系统权限申请
    pub async fn request_permissions(&self) -> anyhow::Result<PermissionStatus> {
        let permissions = Arc::clone(&self.permissions);
        tokio::task::spawn_blocking(move || permissions.request_permissions()).await?
    }

    /// 读取剪贴板文本
    pub async fn clipboard_read(&self) -> anyhow::Result<String> {
        let cb = Arc::clone(&self.clipboard);
        tokio::task::spawn_blocking(move || cb.read_text()).await?
    }

    /// 写入文本到剪贴板
    pub async fn clipboard_write(&self, text: String) -> anyhow::Result<()> {
        let cb = Arc::clone(&self.clipboard);
        tokio::task::spawn_blocking(move || cb.write_text(&text)).await?
    }

    /// 获取当前焦点窗口
    pub async fn get_active_window(&self) -> anyhow::Result<WindowInfo> {
        let wm = Arc::clone(&self.window_mgr);
        tokio::task::spawn_blocking(move || wm.get_active_window()).await?
    }

    /// 枚举所有可见窗口
    pub async fn list_windows(&self) -> anyhow::Result<Vec<WindowInfo>> {
        let wm = Arc::clone(&self.window_mgr);
        tokio::task::spawn_blocking(move || wm.list_windows()).await?
    }

    /// 将指定窗口切换到前台
    pub async fn focus_window(&self, window_id: String) -> anyhow::Result<()> {
        let wm = Arc::clone(&self.window_mgr);
        tokio::task::spawn_blocking(move || wm.focus_window(&window_id)).await?
    }
}
