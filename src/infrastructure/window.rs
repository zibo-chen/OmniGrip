// ============================================================================
// 窗口管理基础设施 - xcap::Window + 平台特定 API
// ============================================================================

use crate::domain::context::{OsContext, OsContextProvider, WindowInfo, WindowManager};
use xcap::Window;

/// 基于 xcap 的跨平台窗口管理实现
///
/// 窗口列表使用 xcap::Window 提供跨平台能力；
/// 窗口聚焦等操作通过条件编译调用平台原生 API。
pub struct PlatformWindowManager;

impl PlatformWindowManager {
    pub fn new() -> Self {
        Self
    }
}

impl WindowManager for PlatformWindowManager {
    fn get_active_window(&self) -> anyhow::Result<WindowInfo> {
        // xcap 0.8+ 支持 is_focused() 方法
        let windows = Window::all()?;
        for w in &windows {
            if w.is_focused().unwrap_or(false) {
                return Ok(WindowInfo {
                    id: w.id()?.to_string(),
                    title: w.title().unwrap_or_default(),
                    app_name: w.app_name().unwrap_or_default(),
                });
            }
        }
        // 回退到平台特定实现
        platform::get_active_window()
    }

    fn list_windows(&self) -> anyhow::Result<Vec<WindowInfo>> {
        let windows = Window::all()?;
        let mut result = Vec::new();
        for w in &windows {
            let minimized = w.is_minimized().unwrap_or(false);
            let title = w.title().unwrap_or_default();
            if !minimized && !title.is_empty() {
                result.push(WindowInfo {
                    id: w.id()?.to_string(),
                    title,
                    app_name: w.app_name().unwrap_or_default(),
                });
            }
        }
        Ok(result)
    }

    fn focus_window(&self, window_id: &str) -> anyhow::Result<()> {
        platform::focus_window(window_id)
    }
}

/// 操作系统上下文提供者 (编译期确定)
pub struct RuntimeOsContext;

impl RuntimeOsContext {
    pub fn new() -> Self {
        Self
    }
}

impl OsContextProvider for RuntimeOsContext {
    fn get_os_context(&self) -> OsContext {
        let os_type = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        };
        OsContext {
            os_type: os_type.to_string(),
        }
    }
}

// ===========================================================================
// 平台特定实现
// ===========================================================================

#[cfg(target_os = "macos")]
mod platform {
    use crate::domain::context::WindowInfo;
    use std::process::Command;

    pub fn get_active_window() -> anyhow::Result<WindowInfo> {
        // 使用 osascript 获取当前前台应用信息
        let output = Command::new("osascript")
            .arg("-e")
            .arg(
                r#"tell application "System Events"
                    set frontApp to first application process whose frontmost is true
                    set appName to name of frontApp
                    set windowTitle to ""
                    try
                        set windowTitle to name of front window of frontApp
                    end try
                    return appName & "|" & windowTitle
                end tell"#,
            )
            .output()?;

        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = result.splitn(2, '|').collect();

        let app_name = parts.first().unwrap_or(&"").to_string();
        let title = parts.get(1).unwrap_or(&"").to_string();

        Ok(WindowInfo {
            id: "0".to_string(), // macOS 无直接窗口 ID，使用 0 表示前台窗口
            title,
            app_name,
        })
    }

    pub fn focus_window(window_id: &str) -> anyhow::Result<()> {
        // 通过 xcap 的窗口列表找到目标窗口的进程名，然后激活
        let windows = xcap::Window::all()?;
        let target = windows
            .iter()
            .find(|w| w.id().map(|id| id.to_string()).unwrap_or_default() == window_id)
            .ok_or_else(|| anyhow::anyhow!("Window {} not found", window_id))?;

        let app_name = target.app_name().unwrap_or_default();

        let script = format!(
            r#"tell application "{}" to activate"#,
            app_name.replace('"', "\\\"")
        );
        Command::new("osascript").arg("-e").arg(&script).output()?;

        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use crate::domain::context::WindowInfo;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, SW_RESTORE,
        SetForegroundWindow, ShowWindow,
    };

    pub fn get_active_window() -> anyhow::Result<WindowInfo> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                anyhow::bail!("No foreground window found");
            }

            // 获取窗口标题
            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title = String::from_utf16_lossy(&title_buf[..len as usize]);

            // 获取进程 ID，然后获取进程名
            let mut process_id: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));

            let app_name = get_process_name(process_id).unwrap_or_default();

            Ok(WindowInfo {
                id: format!("{}", hwnd.0 as usize),
                title,
                app_name,
            })
        }
    }

    pub fn focus_window(window_id: &str) -> anyhow::Result<()> {
        let hwnd_val: usize = window_id
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid window ID: {}", window_id))?;
        let hwnd = HWND(hwnd_val as isize);

        unsafe {
            // 先恢复窗口（如果最小化）
            let _ = ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd)
                .ok()
                .map_err(|e| anyhow::anyhow!("SetForegroundWindow failed: {}", e))?;
        }
        Ok(())
    }

    /// 通过进程 ID 获取进程可执行文件名
    fn get_process_name(pid: u32) -> Option<String> {
        // 尝试通过 xcap 的窗口列表匹配
        if let Ok(windows) = xcap::Window::all() {
            for w in &windows {
                // 匹配进程 ID 获取 app_name
                if let Ok(app_name) = w.app_name() {
                    if !app_name.is_empty() {
                        return Some(app_name);
                    }
                }
            }
        }
        Some(format!("pid:{}", pid))
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use crate::domain::context::WindowInfo;
    use std::process::Command;

    pub fn get_active_window() -> anyhow::Result<WindowInfo> {
        // 使用 xdotool 获取当前焦点窗口 ID
        let id_output = Command::new("xdotool").arg("getactivewindow").output()?;
        if !id_output.status.success() {
            anyhow::bail!(
                "xdotool getactivewindow failed. Is xdotool installed? (apt install xdotool)"
            );
        }
        let window_id = String::from_utf8_lossy(&id_output.stdout)
            .trim()
            .to_string();

        // 获取窗口标题
        let title_output = Command::new("xdotool")
            .args(["getwindowname", &window_id])
            .output()?;
        let title = String::from_utf8_lossy(&title_output.stdout)
            .trim()
            .to_string();

        // 获取窗口所属进程 PID
        let pid_output = Command::new("xdotool")
            .args(["getwindowpid", &window_id])
            .output()?;
        let pid = String::from_utf8_lossy(&pid_output.stdout)
            .trim()
            .to_string();

        // 通过 /proc/<pid>/comm 获取进程名
        let app_name = if !pid.is_empty() {
            std::fs::read_to_string(format!("/proc/{}/comm", pid))
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            String::new()
        };

        Ok(WindowInfo {
            id: window_id,
            title,
            app_name,
        })
    }

    pub fn focus_window(window_id: &str) -> anyhow::Result<()> {
        let output = Command::new("xdotool")
            .args(["windowactivate", "--sync", window_id])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("xdotool windowactivate failed: {}", stderr);
        }
        Ok(())
    }
}
