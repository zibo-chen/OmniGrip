// ============================================================================
// OmniGrip - 应用层 (Application Layer)
// ============================================================================
// 应用层负责编排领域能力，实现具体的用例逻辑。
// 依赖方向: Application → Domain (通过 Arc<dyn Trait> 依赖倒置)
// ============================================================================

pub mod action_service;
pub mod context_service;
pub mod ocr_service;
pub mod vision_service;
