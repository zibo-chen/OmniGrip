// ============================================================================
// OmniGrip - 程序入口 (Composition Root)
// ============================================================================
// 组装所有 DDD 层级，构造依赖图并启动 MCP Server。
// ============================================================================

use std::sync::Arc;

use tracing;

use omni_grip::adapter::mcp_server::OmniGripMcpServer;
use omni_grip::application::{
    action_service::ActionService, context_service::ContextService, ocr_service::OcrService,
    vision_service::VisionService,
};
use omni_grip::infrastructure::{
    clipboard::ArboardClipboard,
    enigo_input::EnigoInput,
    // ocr_engine::OcrRsEngine,  // OCR 需要模型文件，按需启用
    window::{PlatformWindowManager, RuntimeOsContext},
    xcap_capture::XcapCapture,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(std::io::stderr) // MCP 使用 stdout 通信，日志写入 stderr
            .finish(),
    )?;

    tracing::info!("OmniGrip MCP Server starting...");

    // -----------------------------------------------------------------------
    // 1. 构造基础设施层 (Infrastructure)
    // -----------------------------------------------------------------------
    let capture = Arc::new(XcapCapture::new());
    let input = Arc::new(EnigoInput::new()?);
    let clipboard = Arc::new(ArboardClipboard::new()?);
    let window_mgr = Arc::new(PlatformWindowManager::new());
    let os_ctx = Arc::new(RuntimeOsContext::new());

    // OCR 引擎 (需要 PaddleOCR 模型文件)
    // 如果模型文件不存在，使用空的 OCR 实现
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
    let action_svc = Arc::new(ActionService::new(
        input.clone() as Arc<dyn omni_grip::domain::action::MouseController>,
        input as Arc<dyn omni_grip::domain::action::KeyboardController>,
    ));
    let context_svc = Arc::new(ContextService::new(clipboard, window_mgr, os_ctx));
    let ocr_svc = Arc::new(OcrService::new(ocr_engine, capture));

    // -----------------------------------------------------------------------
    // 3. 构造协议适配层并启动 MCP Server
    // -----------------------------------------------------------------------
    let server = OmniGripMcpServer::new(vision_svc, action_svc, context_svc, ocr_svc);

    tracing::info!("OmniGrip MCP Server running on stdio");

    // 通过 stdin/stdout 启动 MCP 服务
    use rmcp::ServiceExt;
    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;

    // 等待客户端断开连接
    service.waiting().await?;

    tracing::info!("OmniGrip MCP Server stopped");
    Ok(())
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
