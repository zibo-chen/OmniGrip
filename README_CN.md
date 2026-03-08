# OmniGrip

[![Rust](https://img.shields.io/badge/rust-1.82+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**OmniGrip** 是一个跨平台的计算机控制 MCP（Model Context Protocol）服务器，专为 LLM 驱动的 GUI 自动化设计。它通过标准化的 MCP 接口提供屏幕截图、鼠标/键盘模拟、窗口管理和 OCR 文字识别能力。

[English Documentation](README.md)

## 功能特性

### 🖥️ 屏幕感知模块 (Vision)
- **显示器检测** - 获取所有连接显示器的元数据（ID、分辨率、缩放比例、位置偏移）
- **屏幕截图** - 支持全屏或区域截图，自动缩放和 JPEG 压缩
- **坐标转换** - 自动计算缩放比例，实现精确的坐标映射

### 🖱️ 外设模拟模块 (Action)
- **鼠标控制** - 移动、点击（左键/右键/中键）、双击、拖拽等操作
- **键盘模拟** - 支持 Unicode/中文文本输入，键盘快捷键组合
- **坐标缩放** - 无缝转换压缩截图坐标与真实屏幕坐标

### 🪟 系统上下文模块 (Context)
- **窗口管理** - 列出窗口、获取活动窗口、切换窗口到前台
- **剪贴板操作** - 读取/写入系统剪贴板
- **系统检测** - 获取当前操作系统类型，用于平台感知自动化

### 📝 OCR 模块
- **全屏文字识别** - 提取屏幕上所有文字及其中心点坐标
- **文本搜索** - 在屏幕上查找特定文本，支持模糊匹配
- **操作断言** - 验证指定区域是否出现期望文本（用于自动化验证）

## 架构设计

OmniGrip 采用领域驱动设计（DDD）原则：

```
┌────────────────────────────────────┐
│      适配层 (MCP Protocol)        │  ← 协议适配
├────────────────────────────────────┤
│      应用层 (Services)            │  ← 用例编排
├────────────────────────────────────┤
│      领域层 (Traits & Types)      │  ← 核心领域抽象
├────────────────────────────────────┤
│      基础设施层 (Impls)           │  ← 技术实现
└────────────────────────────────────┘
```

**依赖规则**: Adapter → Application → Domain ← Infrastructure

## 安装

### 环境要求

- Rust 2024 Edition (1.82+)
- 平台特定依赖：
  - **macOS**: 需要授予辅助功能权限
  - **Linux**: 需要 X11 或 Wayland 支持
  - **Windows**: 无特殊要求

### 从源码构建

```bash
git clone https://github.com/yourusername/OmniGrip.git
cd OmniGrip
cargo build --release
```

编译后的二进制文件位于 `target/release/omni-grip`。

### OCR 模型配置（可选）

如需使用 OCR 功能，请下载 PP-OCRv5 模型文件并放置在以下目录之一：

- `./res/chinese_model/`
- `./models/`
- `~/.omnigrip/models/`

所需文件：
- `PP-OCRv5_mobile_det_fp16.mnn` (文字检测模型)
- `PP-OCRv5_mobile_rec_fp16.mnn` (文字识别模型)
- `ppocr_keys_v5.txt` (字符字典)

> 注意：OCR 功能是可选的。如果未找到模型文件，OmniGrip 将在无 OCR 能力的情况下启动。

## 使用方法

### 启动服务器

```bash
# 使用 stdio 传输方式运行（供 MCP 客户端调用）
./target/release/omni-grip

# 启用调试日志
RUST_LOG=debug ./target/release/omni-grip
```

### MCP 客户端配置

在你的 MCP 客户端配置中添加：

```json
{
  "mcpServers": {
    "omni-grip": {
      "command": "/path/to/omni-grip",
      "args": []
    }
  }
}
```

## MCP 工具参考

### 屏幕感知工具

| 工具 | 描述 |
|------|------|
| `get_displays` | 获取所有显示器元数据（ID、分辨率、缩放比例） |
| `take_screenshot` | 截取完整显示器画面，返回 JPEG 图片和缩放比例 |
| `take_screenshot_region` | 截取指定屏幕区域 |

### 外设模拟工具

| 工具 | 描述 |
|------|------|
| `mouse_move` | 移动鼠标到绝对坐标 |
| `mouse_move_relative` | 相对当前位置移动鼠标 |
| `mouse_click` | 在指定坐标点击（支持左键/右键/中键，单击/双击） |
| `mouse_drag` | 从 A 点拖拽到 B 点 |
| `keyboard_type` | 输入文本字符串（支持 Unicode） |
| `keyboard_press` | 按下键盘快捷键（如 ["cmd", "c"]） |

### 系统上下文工具

| 工具 | 描述 |
|------|------|
| `get_os_context` | 获取操作系统类型（windows/macos/linux） |
| `clipboard_read` | 读取剪贴板文本内容 |
| `clipboard_write` | 写入文本到剪贴板 |
| `get_active_window` | 获取当前活动窗口信息 |
| `list_windows` | 列出所有可见窗口 |
| `focus_window` | 通过 ID 将指定窗口切换到前台 |

### OCR 工具

| 工具 | 描述 |
|------|------|
| `get_ocr_data` | 全屏 OCR 扫描，返回文本及坐标 |
| `find_text_center` | 在屏幕上查找文本，返回中心点坐标 |
| `action_assertion` | 验证指定区域是否包含期望文本 |

## 工作流示例

```
1. get_displays → 获取主显示器的 display_id
2. take_screenshot(display_id=0, max_width=1000) → 获取截图 + scale_ratio
3. [LLM 分析截图，发现按钮位于 (500, 300)]
4. mouse_click(x=500, y=300, scale_ratio=1.5) → 点击转换后的坐标
5. action_assertion(region, "成功") → 验证操作结果
```

## 核心依赖

- **rmcp** - MCP 协议实现
- **xcap** - 跨平台屏幕截图
- **enigo** - 输入设备模拟
- **ocr-rs** - PP-OCR Rust 实现
- **image** - 图像处理和编码
- **tokio** - 异步运行时

## 平台支持

| 平台 | 屏幕截图 | 输入模拟 | OCR |
|------|---------|---------|-----|
| macOS | ✅ | ✅ | ✅ |
| Windows | ✅ | ✅ | ✅ |
| Linux (X11) | ✅ | ✅ | ✅ |
| Linux (Wayland) | ⚠️ 受限 | ⚠️ 受限 | ✅ |

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件。

## 贡献

欢迎贡献代码！请随时提交 Issue 和 Pull Request。

## 相关项目

- [MCP Protocol](https://modelcontextprotocol.io/) - Model Context Protocol 规范
- [rmcp](https://github.com/anthropics/rmcp) - Rust MCP 实现
- [PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR) - PP-OCR 模型来源
