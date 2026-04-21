// ============================================================================
// macOS 权限管理 - 辅助功能和录屏权限检查/申请
// ============================================================================

use accessibility_ng::AXUIElement;
use core_graphics::access::ScreenCaptureAccess;

use crate::domain::context::PermissionStatus;

/// 仅读取当前 macOS 权限状态，不触发系统弹窗。
pub fn get_permission_status() -> PermissionStatus {
    let accessibility_granted = AXUIElement::application_is_trusted();
    let screen_capture_access = ScreenCaptureAccess;
    let screen_recording_granted = screen_capture_access.preflight();

    build_permission_status(
        accessibility_granted,
        screen_recording_granted,
        false,
    )
}

/// 触发 macOS 系统权限申请，并返回申请后的权限状态快照。
///
/// 首次授权时，系统可能只会弹窗或打开系统设置页面，
/// 录屏权限通常需要用户授权后重新启动进程才会生效。
pub fn request_permissions() -> PermissionStatus {
    let mut prompt_triggered = false;

    let accessibility_before = AXUIElement::application_is_trusted();
    let accessibility_granted = if accessibility_before {
        true
    } else {
        prompt_triggered = true;
        AXUIElement::application_is_trusted_with_prompt()
    };

    let screen_capture_access = ScreenCaptureAccess;
    let screen_recording_before = screen_capture_access.preflight();
    let screen_recording_granted = if screen_recording_before {
        true
    } else {
        prompt_triggered = true;
        screen_capture_access.request()
    };

    build_permission_status(
        accessibility_granted,
        screen_recording_granted,
        prompt_triggered,
    )
}

fn build_permission_status(
    accessibility_granted: bool,
    screen_recording_granted: bool,
    prompt_triggered: bool,
) -> PermissionStatus {
    let mut missing_permissions = Vec::new();

    if !accessibility_granted {
        missing_permissions.push("Accessibility".to_string());
    }

    if !screen_recording_granted {
        missing_permissions.push("Screen Recording".to_string());
    }

    let restart_required = !missing_permissions.is_empty();
    let message = if missing_permissions.is_empty() {
        "All required macOS permissions are granted.".to_string()
    } else if prompt_triggered {
        format!(
            "Missing macOS permissions: {}. OmniGrip has triggered the system prompt when possible. After granting access in System Settings > Privacy & Security, restart OmniGrip.",
            missing_permissions.join(", ")
        )
    } else {
        format!(
            "Missing macOS permissions: {}. Grant access in System Settings > Privacy & Security, then restart OmniGrip.",
            missing_permissions.join(", ")
        )
    };

    PermissionStatus {
        os_type: "macos".to_string(),
        supported: true,
        accessibility_granted: Some(accessibility_granted),
        screen_recording_granted: Some(screen_recording_granted),
        missing_permissions,
        can_request: true,
        prompt_triggered,
        restart_required,
        message,
    }
}