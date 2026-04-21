// ============================================================================
// OmniGrip - 基础设施层 (Infrastructure Layer)
// ============================================================================
// 基础设施层负责实现领域层定义的 Trait，对接具体的外部库和操作系统 API。
// 依赖方向: Infrastructure → Domain (实现领域 Trait)
// ============================================================================

pub mod clipboard;
pub mod enigo_input;
pub mod image_proc;
#[cfg(target_os = "macos")]
mod macos_permissions;
pub mod ocr_engine;
pub mod permissions;
pub mod window;
pub mod xcap_capture;
