// ============================================================================
// OmniGrip - 程序入口 (Composition Root)
// ============================================================================
// 组装所有 DDD 层级，构造依赖图。
// 支持两种运行模式：MCP Server (默认) 和 CLI 命令行。
// ============================================================================

use std::sync::Arc;

use clap::{Parser, Subcommand};
use tracing;

use omni_grip::adapter::cli::{CliCommand, CliExecutor};
use omni_grip::adapter::mcp_server::OmniGripMcpServer;
use omni_grip::application::{
    action_service::ActionService, context_service::ContextService, ocr_service::OcrService,
    vision_service::VisionService,
};
use omni_grip::domain::{
    action::{ClickType, KeyboardController, MouseButton, MouseController, Position},
    context::{PermissionManager, PermissionStatus},
};
use omni_grip::infrastructure::{
    clipboard::ArboardClipboard,
    enigo_input::EnigoInput,
    permissions::PlatformPermissionManager,
    window::{PlatformWindowManager, RuntimeOsContext},
    xcap_capture::XcapCapture,
};

// ===========================================================================
// 顶层 CLI 定义
// ===========================================================================

#[derive(Parser)]
#[command(
    name = "omni-grip",
    version,
    about = "Cross-platform computer control for LLM-driven GUI automation"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<TopCommand>,
}

#[derive(Subcommand)]
enum TopCommand {
    /// Start MCP server on stdio (default mode)
    Mcp,

    #[command(flatten)]
    Cli(CliCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // MCP 模式下日志写入 stderr（stdout 用于 MCP 通信）
    // CLI 模式下日志也写入 stderr（stdout 用于 JSON 输出）
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .finish(),
    )?;

    let permission_mgr = Arc::new(PlatformPermissionManager::new());
    let permission_status = if should_auto_request_permissions(cli.command.as_ref()) {
        let status = permission_mgr.request_permissions()?;
        log_permission_status(&status);
        status
    } else {
        permission_mgr.get_permission_status()?
    };

    // -----------------------------------------------------------------------
    // 1. 构造基础设施层 (Infrastructure)
    // -----------------------------------------------------------------------
    let capture = Arc::new(XcapCapture::new());
    let (mouse_input, keyboard_input) = build_input_controllers(&permission_status)?;
    let clipboard = Arc::new(ArboardClipboard::new()?);
    let window_mgr = Arc::new(PlatformWindowManager::new());
    let os_ctx = Arc::new(RuntimeOsContext::new());

    let ocr_engine: Arc<dyn omni_grip::domain::ocr::OcrEngine> = match try_create_ocr_engine() {
        Ok(engine) => {
            tracing::info!("OCR engine initialized successfully");
            Arc::new(engine)
        }
        Err(e) => {
            tracing::warn!(
                "OCR engine not available: {}. OCR features will return errors.",
                e
            );
            Arc::new(NoopOcrEngine)
        }
    };

    // -----------------------------------------------------------------------
    // 2. 构造应用层 (Application Services)
    // -----------------------------------------------------------------------
    let vision_svc = Arc::new(VisionService::new(capture.clone()));
    let action_svc = Arc::new(ActionService::new(mouse_input, keyboard_input));
    let context_svc = Arc::new(ContextService::new(
        clipboard,
        window_mgr,
        os_ctx,
        permission_mgr,
    ));
    let ocr_svc = Arc::new(OcrService::new(ocr_engine, capture));

    // -----------------------------------------------------------------------
    // 3. 根据模式启动
    // -----------------------------------------------------------------------
    match cli.command {
        None | Some(TopCommand::Mcp) => {
            // MCP Server 模式
            tracing::info!("OmniGrip MCP Server starting...");
            let server = OmniGripMcpServer::new(vision_svc, action_svc, context_svc, ocr_svc);

            use rmcp::ServiceExt;
            let transport = rmcp::transport::io::stdio();
            let service = server.serve(transport).await?;
            service.waiting().await?;

            tracing::info!("OmniGrip MCP Server stopped");
        }
        Some(TopCommand::Cli(cmd)) => {
            // CLI 命令模式
            let executor = CliExecutor::new(vision_svc, action_svc, context_svc, ocr_svc);
            executor.execute(cmd).await?;
        }
    }

    Ok(())
}

fn should_auto_request_permissions(command: Option<&TopCommand>) -> bool {
    !matches!(
        command,
        Some(TopCommand::Cli(CliCommand::GetPermissions | CliCommand::RequestPermissions))
    )
}

fn log_permission_status(status: &PermissionStatus) {
    if !status.supported {
        return;
    }

    if status.missing_permissions.is_empty() {
        tracing::info!("All required macOS permissions are already granted");
    } else if status.prompt_triggered {
        tracing::warn!("{}", status.message);
    } else {
        tracing::info!("{}", status.message);
    }
}

fn build_input_controllers(
    permission_status: &PermissionStatus,
) -> anyhow::Result<(Arc<dyn MouseController>, Arc<dyn KeyboardController>)> {
    let accessibility_missing = cfg!(target_os = "macos")
        && matches!(permission_status.accessibility_granted, Some(false));

    if accessibility_missing {
        tracing::warn!(
            "Input simulation is disabled until macOS Accessibility permission is granted"
        );
        return Ok(disabled_input_controllers(permission_status.message.clone()));
    }

    match EnigoInput::new_with_prompt(false) {
        Ok(input) => {
            let input = Arc::new(input);
            Ok((
                input.clone() as Arc<dyn MouseController>,
                input as Arc<dyn KeyboardController>,
            ))
        }
        Err(error) => {
            if cfg!(target_os = "macos") {
                tracing::warn!(
                    "Input backend initialization failed; starting with input disabled: {}",
                    error
                );
                Ok(disabled_input_controllers(error.to_string()))
            } else {
                Err(error)
            }
        }
    }
}

fn disabled_input_controllers(
    reason: String,
) -> (Arc<dyn MouseController>, Arc<dyn KeyboardController>) {
    let input = Arc::new(NoopInput::new(reason));
    (
        input.clone() as Arc<dyn MouseController>,
        input as Arc<dyn KeyboardController>,
    )
}

/// 尝试创建 OCR 引擎
///
/// 优先使用编译时嵌入的模型文件（零配置），
/// 若失败则回退到文件系统查找。
fn try_create_ocr_engine() -> anyhow::Result<omni_grip::infrastructure::ocr_engine::OcrRsEngine> {
    // ── 1. 使用编译时嵌入的模型数据 (推荐，开箱即用) ──
    static DET_MODEL: &[u8] = include_bytes!("../res/chinese_model/PP-OCRv5_mobile_det_fp16.mnn");
    static REC_MODEL: &[u8] = include_bytes!("../res/chinese_model/PP-OCRv5_mobile_rec_fp16.mnn");
    static CHARSET: &[u8] = include_bytes!("../res/chinese_model/ppocr_keys_v5.txt");

    match omni_grip::infrastructure::ocr_engine::OcrRsEngine::from_bytes(
        DET_MODEL, REC_MODEL, CHARSET,
    ) {
        Ok(engine) => {
            tracing::info!("OCR engine initialized from embedded model files");
            return Ok(engine);
        }
        Err(e) => {
            tracing::warn!(
                "Failed to init OCR from embedded data: {}, trying file system...",
                e
            );
        }
    }

    // ── 2. 回退：从文件系统查找模型文件 ──
    let model_patterns: &[(&str, &str, &str)] = &[
        // PP-OCRv5 模型 (优先)
        (
            "PP-OCRv5_mobile_det_fp16.mnn",
            "PP-OCRv5_mobile_rec_fp16.mnn",
            "ppocr_keys_v5.txt",
        ),
        // 通用命名
        ("det_model.mnn", "rec_model.mnn", "ppocr_keys.txt"),
    ];

    // 在多个位置查找模型文件
    let search_dirs = [
        "res/chinese_model",
        "models",
        "res/models",
        "~/.omnigrip/models",
        "/usr/local/share/omnigrip/models",
    ];

    for dir in &search_dirs {
        let dir = shellexpand::tilde(dir).to_string();
        for (det_name, rec_name, charset_name) in model_patterns {
            let det = format!("{}/{}", dir, det_name);
            let rec = format!("{}/{}", dir, rec_name);
            let charset = format!("{}/{}", dir, charset_name);

            if std::path::Path::new(&det).exists()
                && std::path::Path::new(&rec).exists()
                && std::path::Path::new(&charset).exists()
            {
                tracing::info!("Found OCR model files in: {}", dir);
                return omni_grip::infrastructure::ocr_engine::OcrRsEngine::new(det, rec, charset);
            }
        }
    }

    anyhow::bail!(
        "OCR model files not found. Searched directories: {:?}. \
         Place PP-OCRv5_mobile_det_fp16.mnn, PP-OCRv5_mobile_rec_fp16.mnn, ppocr_keys_v5.txt \
         in ./res/chinese_model/ or ./models/",
        search_dirs
    )
}

/// 空 OCR 引擎 (当模型文件不可用时的降级实现)
struct NoopOcrEngine;

impl omni_grip::domain::ocr::OcrEngine for NoopOcrEngine {
    fn recognize(
        &self,
        _image: &omni_grip::domain::vision::RawImage,
    ) -> anyhow::Result<omni_grip::domain::ocr::OcrResult> {
        anyhow::bail!("OCR engine not available. Please install model files in ./models/")
    }
}

/// 空输入实现（当输入能力不可用时的降级实现）
struct NoopInput {
    reason: String,
}

impl NoopInput {
    fn new(reason: String) -> Self {
        Self { reason }
    }

    fn error(&self) -> anyhow::Error {
        anyhow::anyhow!(
            "Input simulation is unavailable: {}. Grant macOS Accessibility permission and restart OmniGrip, or run the get_permissions/request_permissions tools to inspect and trigger the permission flow.",
            self.reason
        )
    }
}

impl MouseController for NoopInput {
    fn move_to(&self, _x: i32, _y: i32) -> anyhow::Result<()> {
        Err(self.error())
    }

    fn move_relative(&self, _dx: i32, _dy: i32) -> anyhow::Result<()> {
        Err(self.error())
    }

    fn click(&self, _button: MouseButton, _click_type: ClickType) -> anyhow::Result<()> {
        Err(self.error())
    }

    fn drag(&self, _from: Position, _to: Position, _button: MouseButton) -> anyhow::Result<()> {
        Err(self.error())
    }
}

impl KeyboardController for NoopInput {
    fn type_text(&self, _text: &str) -> anyhow::Result<()> {
        Err(self.error())
    }

    fn press_key(&self, _keys: &[String]) -> anyhow::Result<()> {
        Err(self.error())
    }
}
