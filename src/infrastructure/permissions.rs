// ============================================================================
// 系统权限管理基础设施 - 平台特定实现
// ============================================================================

use crate::domain::context::{PermissionManager, PermissionStatus};

/// 平台权限管理器
pub struct PlatformPermissionManager;

impl PlatformPermissionManager {
    pub fn new() -> Self {
        Self
    }
}

impl PermissionManager for PlatformPermissionManager {
    fn get_permission_status(&self) -> anyhow::Result<PermissionStatus> {
        platform::get_permission_status()
    }

    fn request_permissions(&self) -> anyhow::Result<PermissionStatus> {
        platform::request_permissions()
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use crate::domain::context::PermissionStatus;

    pub fn get_permission_status() -> anyhow::Result<PermissionStatus> {
        Ok(crate::infrastructure::macos_permissions::get_permission_status())
    }

    pub fn request_permissions() -> anyhow::Result<PermissionStatus> {
        Ok(crate::infrastructure::macos_permissions::request_permissions())
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use crate::domain::context::PermissionStatus;

    pub fn get_permission_status() -> anyhow::Result<PermissionStatus> {
        Ok(unsupported_status(false))
    }

    pub fn request_permissions() -> anyhow::Result<PermissionStatus> {
        Ok(unsupported_status(false))
    }

    fn unsupported_status(prompt_triggered: bool) -> PermissionStatus {
        PermissionStatus {
            os_type: current_os().to_string(),
            supported: false,
            accessibility_granted: None,
            screen_recording_granted: None,
            missing_permissions: Vec::new(),
            can_request: false,
            prompt_triggered,
            restart_required: false,
            message: "Permission management is currently only implemented on macOS.".to_string(),
        }
    }

    fn current_os() -> &'static str {
        if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        }
    }
}
