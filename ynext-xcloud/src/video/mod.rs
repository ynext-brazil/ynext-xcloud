//! # Módulo de Vídeo — Fase 3
//!
//! Orquestra o pipeline GStreamer Zero-Copy para renderização de vídeo H.264
//! acelerado por hardware (VA-API no Linux, D3D11 no Windows).
//!
//! ## Pipeline GStreamer
//!
//! ```text
//! webrtcbin (SDP/ICE da Fase 2)
//!     └─▶ rtph264depay
//!              └─▶ h264parse
//!                    └─▶ vaapih264dec (Linux) | d3d11h264dec (Windows)
//!                              └─▶ glimagesink (Linux) | d3d11videosink (Windows)
//! ```
//!
//! ## Regras Absolutas
//!
//! - **PROIBIDO** usar `webrtc-rs` isolado. Apenas `webrtcbin` nativo do GStreamer.
//! - Codec: H.264 é o padrão absoluto e inegociável. AV1 apenas como opt-in futuro.
//! - Decode: VA-API (Linux) ou D3D11 (Windows). Decode por software é PROIBIDO.

pub mod channels;
pub mod pipeline;
pub mod renderer;

use anyhow::{Context, Result};
use tracing::{error, info};

use crate::signaling::StreamingSession;
use channels::{create_video_channel, SignalingToVideoMessage};
use pipeline::GstreamerPipeline;

/// Handle para controle do pipeline de vídeo em execução
pub struct PipelineHandle {
    /// Canal para enviar mensagens de controle (ex: parar pipeline)
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

/// Inicia o pipeline de vídeo GStreamer com a sessão WebRTC negociada.
///
/// Esta função:
/// 1. Cria os canais MPSC entre sinalização e vídeo
/// 2. Monta o pipeline GStreamer na thread dedicada de GLib MainLoop
/// 3. Injeta o SDP Answer e ICE Candidates da Fase 2 no `webrtcbin`
/// 4. Aguarda frames de vídeo chegarem e os renderiza via glimagesink/d3d11videosink
///
/// # Retorno
/// Retorna um `PipelineHandle` que pode ser usado para encerrar o pipeline.
pub async fn start_pipeline(session: StreamingSession) -> Result<PipelineHandle> {
    // Inicializa o GStreamer (seguro chamar múltiplas vezes)
    gstreamer::init().context("Falha ao inicializar o GStreamer")?;
    info!("✅ GStreamer {} inicializado", gstreamer::version_string());

    // Canal de shutdown para encerrar o pipeline sob demanda
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Canal MPSC: sinalização → vídeo (SDP Answer + ICE Candidates)
    let (video_tx, mut video_rx) = create_video_channel();

    // Clona os dados da sessão para mover para a thread do GLib MainLoop
    let sdp_answer = session.sdp_answer.clone();
    let ice_candidates = session.ice_candidates.clone();
    let session_id = session.session_id.clone();

    // Envia os dados da sessão para o pipeline via MPSC
    video_tx
        .send(SignalingToVideoMessage {
            sdp_answer,
            ice_candidates,
        })
        .await
        .context("Falha ao enviar sessão para o pipeline de vídeo")?;

    // Inicia o pipeline do GStreamer numa thread dedicada (GLib MainLoop é bloqueante)
    // Usamos spawn_blocking pois o GLib MainLoop não é async-friendly
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();

        match GstreamerPipeline::new(&session_id) {
            Ok(mut gst_pipeline) => {
                // Aguarda os dados de sinalização e configura o webrtcbin
                if let Some(msg) = rt.block_on(async { video_rx.recv().await }) {
                    if let Err(e) =
                        gst_pipeline.configure_webrtc(&msg.sdp_answer, &msg.ice_candidates)
                    {
                        error!("❌ Falha ao configurar webrtcbin: {}", e);
                        return;
                    }
                }

                // Inicia o pipeline e roda até shutdown ou erro
                if let Err(e) = gst_pipeline.run(shutdown_rx) {
                    error!("❌ Pipeline GStreamer encerrado com erro: {}", e);
                } else {
                    info!("🛑 Pipeline GStreamer encerrado normalmente.");
                }
            }
            Err(e) => {
                error!("❌ Falha ao criar pipeline GStreamer: {}", e);
            }
        }
    });

    Ok(PipelineHandle { shutdown_tx })
}
