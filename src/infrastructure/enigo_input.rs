// ============================================================================
// HID 输入模拟基础设施 - enigo 实现
// ============================================================================

use std::sync::Mutex;

use crate::domain::action::{
    ClickType, KeyboardController, MouseButton, MouseController, Position,
};
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

/// 基于 enigo 库的鼠标/键盘输入模拟实现
///
/// 内部使用 Mutex 保证线程安全，允许通过 &self 调用可变操作。
/// Enigo 内部持有平台相关的裸指针 (如 macOS 的 CGEventSource)，
/// 但通过 Mutex 保证了同一时刻只有一个线程访问，因此安全。
pub struct EnigoInput {
    enigo: Mutex<Enigo>,
}

// SAFETY: EnigoInput 通过 Mutex 保证了线程安全性。
// Enigo 的裸指针 (CGEventSource 等) 在 Mutex 保护下不会被并发访问。
unsafe impl Send for EnigoInput {}
unsafe impl Sync for EnigoInput {}

impl EnigoInput {
    pub fn new() -> anyhow::Result<Self> {
        Self::new_with_prompt(true)
    }

    pub fn new_with_prompt(prompt_for_permissions: bool) -> anyhow::Result<Self> {
        let enigo = Enigo::new(&build_enigo_settings(prompt_for_permissions))
            .map_err(enrich_enigo_init_error)?;
        Ok(Self {
            enigo: Mutex::new(enigo),
        })
    }
}

#[cfg(target_os = "macos")]
fn build_enigo_settings(prompt_for_permissions: bool) -> Settings {
    let mut settings = Settings::default();
    settings.open_prompt_to_get_permissions = prompt_for_permissions;
    settings
}

#[cfg(not(target_os = "macos"))]
fn build_enigo_settings(_prompt_for_permissions: bool) -> Settings {
    Settings::default()
}

fn enrich_enigo_init_error(error: enigo::NewConError) -> anyhow::Error {
    if cfg!(target_os = "macos") {
        anyhow::anyhow!(
            "Failed to initialize Enigo: {}. On macOS, grant OmniGrip access in System Settings > Privacy & Security > Accessibility, then restart the process.",
            error
        )
    } else {
        anyhow::anyhow!("Failed to initialize Enigo: {}", error)
    }
}

impl MouseController for EnigoInput {
    fn move_to(&self, x: i32, y: i32) -> anyhow::Result<()> {
        let mut enigo = self.enigo.lock().unwrap();
        enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("Mouse move failed: {:?}", e))
    }

    fn move_relative(&self, dx: i32, dy: i32) -> anyhow::Result<()> {
        let mut enigo = self.enigo.lock().unwrap();
        enigo
            .move_mouse(dx, dy, Coordinate::Rel)
            .map_err(|e| anyhow::anyhow!("Mouse relative move failed: {:?}", e))
    }

    fn click(&self, button: MouseButton, click_type: ClickType) -> anyhow::Result<()> {
        let mut enigo = self.enigo.lock().unwrap();
        let btn = to_enigo_button(button);

        match click_type {
            ClickType::Click => {
                enigo
                    .button(btn, Direction::Click)
                    .map_err(|e| anyhow::anyhow!("Click failed: {:?}", e))?;
            }
            ClickType::DoubleClick => {
                enigo
                    .button(btn, Direction::Click)
                    .map_err(|e| anyhow::anyhow!("Double click (1st) failed: {:?}", e))?;
                enigo
                    .button(btn, Direction::Click)
                    .map_err(|e| anyhow::anyhow!("Double click (2nd) failed: {:?}", e))?;
            }
            ClickType::Press => {
                enigo
                    .button(btn, Direction::Press)
                    .map_err(|e| anyhow::anyhow!("Mouse press failed: {:?}", e))?;
            }
            ClickType::Release => {
                enigo
                    .button(btn, Direction::Release)
                    .map_err(|e| anyhow::anyhow!("Mouse release failed: {:?}", e))?;
            }
        }
        Ok(())
    }

    fn drag(&self, from: Position, to: Position, button: MouseButton) -> anyhow::Result<()> {
        let mut enigo = self.enigo.lock().unwrap();
        let btn = to_enigo_button(button);

        enigo
            .move_mouse(from.x, from.y, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("Drag: move to start failed: {:?}", e))?;
        enigo
            .button(btn, Direction::Press)
            .map_err(|e| anyhow::anyhow!("Drag: press failed: {:?}", e))?;
        enigo
            .move_mouse(to.x, to.y, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("Drag: move to end failed: {:?}", e))?;
        enigo
            .button(btn, Direction::Release)
            .map_err(|e| anyhow::anyhow!("Drag: release failed: {:?}", e))?;

        Ok(())
    }
}

impl KeyboardController for EnigoInput {
    fn type_text(&self, text: &str) -> anyhow::Result<()> {
        let mut enigo = self.enigo.lock().unwrap();
        enigo
            .text(text)
            .map_err(|e| anyhow::anyhow!("Text input failed: {:?}", e))
    }

    fn press_key(&self, keys: &[String]) -> anyhow::Result<()> {
        let mut enigo = self.enigo.lock().unwrap();
        let parsed: Vec<Key> = keys
            .iter()
            .map(|k| parse_key(k))
            .collect::<anyhow::Result<Vec<_>>>()?;

        if parsed.is_empty() {
            return Ok(());
        }

        // 按下所有修饰键 (除最后一个键外)
        let (modifiers, last) = parsed.split_at(parsed.len().saturating_sub(1));

        for key in modifiers {
            enigo
                .key(*key, Direction::Press)
                .map_err(|e| anyhow::anyhow!("Key press failed: {:?}", e))?;
        }

        // 敲击最后一个键
        if let Some(key) = last.first() {
            enigo
                .key(*key, Direction::Click)
                .map_err(|e| anyhow::anyhow!("Key click failed: {:?}", e))?;
        }

        // 反序释放所有修饰键
        for key in modifiers.iter().rev() {
            enigo
                .key(*key, Direction::Release)
                .map_err(|e| anyhow::anyhow!("Key release failed: {:?}", e))?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

fn to_enigo_button(button: MouseButton) -> Button {
    match button {
        MouseButton::Left => Button::Left,
        MouseButton::Right => Button::Right,
        MouseButton::Middle => Button::Middle,
    }
}

/// 将字符串形式的按键名解析为 enigo Key 枚举
fn parse_key(key_name: &str) -> anyhow::Result<Key> {
    match key_name.to_lowercase().as_str() {
        "enter" | "return" => Ok(Key::Return),
        "tab" => Ok(Key::Tab),
        "space" => Ok(Key::Space),
        "backspace" => Ok(Key::Backspace),
        "delete" => Ok(Key::Delete),
        "escape" | "esc" => Ok(Key::Escape),
        "up" => Ok(Key::UpArrow),
        "down" => Ok(Key::DownArrow),
        "left" => Ok(Key::LeftArrow),
        "right" => Ok(Key::RightArrow),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" => Ok(Key::PageUp),
        "pagedown" => Ok(Key::PageDown),
        "shift" => Ok(Key::Shift),
        "ctrl" | "control" => Ok(Key::Control),
        "alt" | "option" => Ok(Key::Alt),
        "cmd" | "command" | "meta" | "super" | "win" => Ok(Key::Meta),
        "capslock" => Ok(Key::CapsLock),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        s if s.len() == 1 => Ok(Key::Unicode(s.chars().next().unwrap())),
        other => Err(anyhow::anyhow!("Unknown key: {}", other)),
    }
}
