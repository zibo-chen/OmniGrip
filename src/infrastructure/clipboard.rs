// ============================================================================
// 剪贴板基础设施 - arboard 实现
// ============================================================================

use std::sync::Mutex;

use crate::domain::context::ClipboardManager;
use arboard::Clipboard;

/// 基于 arboard 库的跨平台剪贴板实现
pub struct ArboardClipboard {
    clipboard: Mutex<Clipboard>,
}

impl ArboardClipboard {
    pub fn new() -> anyhow::Result<Self> {
        let clipboard =
            Clipboard::new().map_err(|e| anyhow::anyhow!("Failed to init clipboard: {}", e))?;
        Ok(Self {
            clipboard: Mutex::new(clipboard),
        })
    }
}

impl ClipboardManager for ArboardClipboard {
    fn read_text(&self) -> anyhow::Result<String> {
        let mut cb = self.clipboard.lock().unwrap();
        cb.get_text()
            .map_err(|e| anyhow::anyhow!("Clipboard read failed: {}", e))
    }

    fn write_text(&self, text: &str) -> anyhow::Result<()> {
        let mut cb = self.clipboard.lock().unwrap();
        cb.set_text(text.to_string())
            .map_err(|e| anyhow::anyhow!("Clipboard write failed: {}", e))
    }
}
