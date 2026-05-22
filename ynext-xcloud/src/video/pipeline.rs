//! # Pipeline GStreamer — Fase 3
//!
//! Monta, configura e executa o pipeline de streaming de vídeo H.264 Zero-Copy
//! usando o elemento `webrtcbin` do GStreamer como núcleo WebRTC.
//!
//! ## Regras Absolutas
//!
//! - **H.264 apenas**: qualquer codec diferente no SDP é filtrado/rejeitado
//! - **VA-API no Linux**: elemento `vaapih264dec` para decode por hardware
//! - **D3D11 no Windows**: elemento `d3d11h264dec` para decode por hardware
//! - **Decode por software (avdec_h264, openh264) é proibido** em produção
//!
//! ## Estrutura do Pipeline
//!
//! ```text
//! webrtcbin (name=webrtc)
//!     └─[pad-added signal]─▶ rtph264depay
//!                                └─▶ h264parse
//!                                       └─▶ vaapih264dec   (Linux)
//!                                       └─▶ d3d11h264dec   (Windows)
//!                                                └─▶ glimagesink    (Linux)
//!                                                └─▶ d3d11videosink (Windows)
//! ```

use anyhow::{bail, Context, Result};
use gstreamer::prelude::*;
use gstreamer_webrtc::WebRTCSDPType;
use tracing::{debug, error, info, warn};

use crate::signaling::IceCandidate;
use crate::video::renderer::select_video_sink;

// ===========================================================================
// Constantes do pipeline
// ===========================================================================

/// Bundle policy do webrtcbin — agrupa todas as mídias em um único DTLS
/// para reduzir overhead de handshake (obrigatório para xCloud)
const BUNDLE_POLICY: &str = "max-bundle";

/// Número máximo de segundos aguardando o pipeline entrar em PLAYING
const PIPELINE_START_TIMEOUT_SECS: u64 = 10;

// ===========================================================================
// Estrutura do Pipeline
// ===========================================================================

/// Representa um pipeline GStreamer ativo para streaming H.264
pub struct GstreamerPipeline {
    /// O pipeline principal do GStreamer
    pipeline: gstreamer::Pipeline,
    /// Referência ao elemento webrtcbin para injeção de SDP/ICE
    webrtcbin: gstreamer::Element,
    /// ID da sessão xCloud (para logs)
    session_id: String,
    /// Canal de dados WebRTC (para input/gamepad)
    datachannel: Option<gstreamer::Object>,
}

impl GstreamerPipeline {
    /// Constrói o pipeline GStreamer sem iniciá-lo.
    ///
    /// Monta todos os elementos e conecta os sinais necessários.
    /// O pipeline só será iniciado quando `run()` for chamado.
    pub fn new(session_id: &str) -> Result<Self> {
        let pipeline = gstreamer::Pipeline::new();

        // Elemento central — núcleo WebRTC (SDP/ICE/DTLS/SRTP tudo aqui dentro)
        let webrtcbin = gstreamer::ElementFactory::make("webrtcbin")
            .name("webrtc")
            .property_from_str("bundle-policy", BUNDLE_POLICY)
            .property("latency", 0u32) // Latência mínima — crítico para gaming
            .build()
            .context(
                "Falha ao criar elemento 'webrtcbin'. Verifique se gst-plugins-bad está instalado.",
            )?;

        pipeline
            .add(&webrtcbin)
            .context("Falha ao adicionar webrtcbin ao pipeline")?;

        let pipeline_clone = pipeline.clone();
        let session_id_clone = session_id.to_string();

        // Conecta o sinal `pad-added` do webrtcbin
        // Este sinal dispara quando o webrtcbin recebe um stream de vídeo/áudio
        // e cria um pad de saída com o stream decodificado
        webrtcbin.connect("pad-added", false, move |args| {
            let pad = args[1]
                .get::<gstreamer::Pad>()
                .expect("pad-added: argumento inválido");

            let pad_name = pad.name();
            debug!(
                session_id = %session_id_clone,
                pad = %pad_name,
                "Novo pad adicionado pelo webrtcbin"
            );

            // Conecta o pad ao resto do pipeline (decode + sink)
            if let Err(e) = connect_webrtc_pad(&pipeline_clone, &pad) {
                error!("❌ Falha ao conectar pad '{}': {}", pad_name, e);
            }

            None
        });

        // Conecta o sinal de candidato ICE local gerado pelo webrtcbin
        // (usado quando for necessário enviar ICE locais para o servidor)
        webrtcbin.connect("on-ice-candidate", false, |args| {
            let sdp_mline_index = args[1].get::<u32>().unwrap_or(0);
            let candidate = args[2].get::<String>().unwrap_or_default();
            debug!(
                sdp_mline = sdp_mline_index,
                candidate = %candidate,
                "🧊 ICE candidate local gerado pelo webrtcbin"
            );
            None
        });

        info!(session_id = %session_id, "✅ Pipeline GStreamer criado");

        // Cria o DataChannel para Input ("input") antes da negociação
        let datachannel = webrtcbin.emit_by_name::<Option<gstreamer::Object>>(
            "create-data-channel",
            &[&"input", &None::<gstreamer::Structure>],
        );

        if let Some(ref dc) = datachannel {
            info!("🎮 WebRTC DataChannel 'input' criado com sucesso");
            // Conecta o sinal on-open para confirmar quando o canal estiver pronto
            dc.connect("on-open", false, |_args| {
                info!("🌐 WebRTC DataChannel 'input' ABERTO e pronto para envio");
                None
            });
            // Opcional: on-error
            dc.connect("on-error", false, |args| {
                if let Some(err) = args[1].get::<gstreamer::glib::Error>().ok() {
                    error!("❌ Erro no DataChannel 'input': {}", err);
                }
                None
            });
        } else {
            error!("❌ Falha ao criar WebRTC DataChannel 'input'");
        }

        Ok(Self {
            pipeline,
            webrtcbin,
            session_id: session_id.to_string(),
            datachannel,
        })
    }

    /// Injeta o SDP Answer e os candidatos ICE remotos no elemento `webrtcbin`.
    ///
    /// Deve ser chamado ANTES de `run()` para que o webrtcbin saiba com quem
    /// está se conectando antes de iniciar a negociação ICE.
    pub fn configure_webrtc(
        &mut self,
        sdp_answer: &str,
        ice_candidates: &[IceCandidate],
    ) -> Result<()> {
        info!(
            session_id = %self.session_id,
            "🤝 Injetando SDP Answer no webrtcbin..."
        );

        // Parseia o SDP Answer recebido da Microsoft
        let sdp_msg = gstreamer_sdp::SDPMessage::parse_buffer(sdp_answer.as_bytes())
            .context("Falha ao parsear SDP Answer da Microsoft")?;

        // Cria o WebRTCSessionDescription como "answer" (resposta)
        let answer =
            gstreamer_webrtc::WebRTCSessionDescription::new(WebRTCSDPType::Answer, sdp_msg);

        // Injeta o SDP Answer no webrtcbin como "remote description"
        // Esta chamada dispara a negociação ICE internamente
        self.webrtcbin.emit_by_name::<()>(
            "set-remote-description",
            &[&answer, &None::<gstreamer::Promise>],
        );

        info!(
            session_id = %self.session_id,
            candidates = ice_candidates.len(),
            "🧊 Injetando candidatos ICE remotos no webrtcbin..."
        );

        // Injeta os candidatos ICE remotos um a um
        for candidate in ice_candidates {
            let sdp_mid = candidate.sdp_mid.as_deref().unwrap_or("video");
            let sdp_mline_index = candidate.sdp_m_line_index.unwrap_or(0);

            self.webrtcbin.emit_by_name::<()>(
                "add-ice-candidate",
                &[&sdp_mline_index, &candidate.candidate.as_str()],
            );

            debug!(
                mid = sdp_mid,
                mline = sdp_mline_index,
                "ICE candidate remoto adicionado"
            );
        }

        info!(
            session_id = %self.session_id,
            "✅ webrtcbin configurado. Iniciando negociação ICE..."
        );

        Ok(())
    }

    /// Inicia o pipeline GStreamer e bloqueia até encerramento.
    ///
    /// Esta função é bloqueante e deve ser chamada numa thread dedicada
    /// (via `tokio::task::spawn_blocking`).
    ///
    /// O pipeline para quando:
    /// - O receiver `shutdown_rx` recebe um sinal de shutdown
    /// - O GStreamer emite um evento de EOS (End of Stream)
    /// - Ocorre um erro irrecuperável no bus
    pub fn run(
        &self,
        shutdown_rx: tokio::sync::oneshot::Receiver<()>,
        mut input_rx: tokio::sync::mpsc::Receiver<crate::input::InputReport>,
    ) -> Result<()> {
        // Inicia o pipeline (PLAYING)
        self.pipeline
            .set_state(gstreamer::State::Playing)
            .context("Falha ao iniciar pipeline GStreamer (PLAYING)")?;

        info!(
            session_id = %self.session_id,
            "▶️  Pipeline GStreamer em PLAYING — streaming iniciado!"
        );

        // Aguarda o estado PLAYING com timeout
        let state_result = self.pipeline.state(gstreamer::ClockTime::from_seconds(
            PIPELINE_START_TIMEOUT_SECS,
        ));

        if state_result.0.is_err() {
            bail!("Pipeline falhou ao atingir estado PLAYING");
        }

        // Loop principal do GStreamer — processa mensagens do bus
        let bus = self.pipeline.bus().context("Pipeline sem bus GST")?;

        // Cria um handle tokio para receber o shutdown de forma não-bloqueante
        let rt = tokio::runtime::Handle::try_current()
            .context("Nenhum runtime Tokio disponível na thread do GStreamer")?;

        let mut shutdown_rx = shutdown_rx;

        loop {
            // Verifica se há um sinal de shutdown pendente ou novos inputs
            if rt.block_on(async {
                tokio::select! {
                    biased;
                    _ = &mut shutdown_rx => true,
                    report = input_rx.recv() => {
                        if let Some(report) = report {
                            if let Some(ref dc) = self.datachannel {
                                // Serializa o report para bytes
                                let bytes = report.to_bytes();
                                // Envia os bytes pelo DataChannel
                                // `send-data` espera um glib::Bytes
                                let glib_bytes = gstreamer::glib::Bytes::from(&bytes);
                                dc.emit_by_name::<()>("send-data", &[&glib_bytes]);
                            }
                        }
                        false // Não encerra o loop
                    },
                    else => false,
                }
            }) {
                info!(session_id = %self.session_id, "🛑 Shutdown recebido — parando pipeline...");
                break;
            }

            // Processa mensagens do bus GStreamer (timeout de 100ms para não bloquear)
            let msg = bus.timed_pop(gstreamer::ClockTime::from_mseconds(100));

            if let Some(msg) = msg {
                use gstreamer::MessageView;

                match msg.view() {
                    MessageView::Eos(..) => {
                        info!(session_id = %self.session_id, "📺 EOS recebido — stream encerrado pelo servidor");
                        break;
                    }
                    MessageView::Error(err) => {
                        let gst_err = err.error();
                        let debug_info = err.debug().unwrap_or_default();
                        error!(
                            session_id = %self.session_id,
                            error = %gst_err,
                            debug = %debug_info,
                            "❌ Erro no pipeline GStreamer"
                        );
                        bail!("Erro GStreamer: {} — {}", gst_err, debug_info);
                    }
                    MessageView::Warning(warn) => {
                        let gst_warn = warn.error();
                        warn!(
                            session_id = %self.session_id,
                            warning = %gst_warn,
                            "⚠️  Warning no pipeline GStreamer"
                        );
                    }
                    MessageView::StateChanged(sc) => {
                        if msg
                            .src()
                            .map(|s| *s == *self.pipeline.upcast_ref::<gstreamer::Object>())
                            .unwrap_or(false)
                        {
                            debug!(
                                old = ?sc.old(),
                                new = ?sc.current(),
                                "Estado do pipeline alterado"
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        // Para o pipeline de forma limpa
        let _ = self.pipeline.set_state(gstreamer::State::Null);
        info!(session_id = %self.session_id, "✅ Pipeline GStreamer parado.");

        Ok(())
    }
}

// ===========================================================================
// Funções auxiliares
// ===========================================================================

/// Conecta um pad recém-criado pelo webrtcbin ao resto do pipeline de decode.
///
/// Esta função é chamada no sinal `pad-added` do webrtcbin e monta
/// dinamicamente a cadeia: rtph264depay → h264parse → vaapih264dec → sink
fn connect_webrtc_pad(pipeline: &gstreamer::Pipeline, src_pad: &gstreamer::Pad) -> Result<()> {
    // Obtém as caps do pad para verificar se é vídeo H.264
    let caps = src_pad
        .current_caps()
        .or_else(|| Some(src_pad.query_caps(None)));

    let is_video = caps
        .as_ref()
        .and_then(|c| c.structure(0))
        .map(|s| s.name().starts_with("application/x-rtp") || s.name().starts_with("video/"))
        .unwrap_or(false);

    if !is_video {
        debug!("Pad ignorado (não é vídeo): {:?}", caps);
        return Ok(());
    }

    info!("📹 Stream de vídeo detectado — montando cadeia de decode H.264...");

    // Desempacotador RTP → NAL units H.264
    let depay = gstreamer::ElementFactory::make("rtph264depay")
        .build()
        .context("Falha ao criar 'rtph264depay'. GStreamer base plugins ausente?")?;

    // Parser H.264 — extrai SPS/PPS e prepara para o decoder de hardware
    let parse = gstreamer::ElementFactory::make("h264parse")
        .build()
        .context("Falha ao criar 'h264parse'. GStreamer base plugins ausente?")?;

    // Decoder H.264 por hardware (detecta a plataforma)
    let decode = create_hw_decoder().context("Falha ao criar decoder H.264 por hardware")?;

    // Sink de vídeo (OpenGL no Linux, D3D11 no Windows)
    let sink = select_video_sink().context("Falha ao criar sink de vídeo")?;

    // Adiciona todos os elementos ao pipeline
    pipeline
        .add_many([&depay, &parse, &decode, &sink])
        .context("Falha ao adicionar elementos de decode ao pipeline")?;

    // Liga os elementos em sequência: depay → parse → decode → sink
    gstreamer::Element::link_many([&depay, &parse, &decode, &sink])
        .context("Falha ao ligar cadeia de decode H.264")?;

    // Sincroniza os novos elementos com o estado atual do pipeline
    for element in [&depay, &parse, &decode, &sink] {
        element
            .sync_state_with_parent()
            .context("Falha ao sincronizar estado do elemento")?;
    }

    // Conecta o pad do webrtcbin ao primeiro elemento da cadeia (depay)
    let depay_sink = depay
        .static_pad("sink")
        .context("rtph264depay sem pad 'sink'")?;

    src_pad
        .link(&depay_sink)
        .context("Falha ao linkar pad webrtcbin → rtph264depay")?;

    info!(
        "✅ Cadeia de decode H.264 conectada: webrtcbin → rtph264depay → h264parse → hwdec → sink"
    );

    Ok(())
}

/// Cria o decoder H.264 por hardware correto para a plataforma atual.
///
/// - **Linux**: `vaapih264dec` (VA-API — Intel/AMD/NVIDIA com driver open source)
/// - **Windows**: `d3d11h264dec` (DirectX 11 Video Acceleration)
/// - **Fallback (dev only)**: `avdec_h264` (software, apenas para debug)
fn create_hw_decoder() -> Result<gstreamer::Element> {
    // Tenta VA-API primeiro (Linux com driver vaapi instalado)
    if let Ok(el) = gstreamer::ElementFactory::make("vaapih264dec").build() {
        info!("🚀 Decoder selecionado: vaapih264dec (VA-API — Zero-Copy)");
        return Ok(el);
    }

    // Tenta D3D11 (Windows com DirectX 11)
    if let Ok(el) = gstreamer::ElementFactory::make("d3d11h264dec").build() {
        info!("🚀 Decoder selecionado: d3d11h264dec (D3D11 Video Acceleration)");
        return Ok(el);
    }

    // Tenta nvdec (NVIDIA NVDEC via CUDA — Linux/Windows com GPU NVIDIA)
    if let Ok(el) = gstreamer::ElementFactory::make("nvh264dec").build() {
        info!("🚀 Decoder selecionado: nvh264dec (NVIDIA NVDEC)");
        return Ok(el);
    }

    // Fallback: software decoder (permitido apenas em modo debug)
    #[cfg(debug_assertions)]
    if let Ok(el) = gstreamer::ElementFactory::make("avdec_h264").build() {
        warn!("⚠️  Decoder selecionado: avdec_h264 (SOFTWARE — apenas para debug!)");
        warn!("    Em produção, instale os drivers VA-API: sudo apt install gstreamer1.0-vaapi");
        return Ok(el);
    }

    bail!(
        "❌ Nenhum decoder H.264 por hardware encontrado!\n\
         Linux:   sudo apt install gstreamer1.0-vaapi\n\
         Windows: Instale o GStreamer runtime com gst-plugins-bad\n\
         Ou instale drivers de vídeo adequados para seu hardware."
    )
}

// ===========================================================================
// Testes
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gstreamer_init() {
        // Verifica que o GStreamer pode ser inicializado
        gstreamer::init().expect("GStreamer deve inicializar com sucesso");
        let version = gstreamer::version_string();
        assert!(
            version.contains("GStreamer"),
            "Versão do GStreamer deve ser válida: {}",
            version
        );
    }

    #[test]
    fn test_webrtcbin_available() {
        gstreamer::init().unwrap();
        // Verifica que o elemento webrtcbin está disponível no sistema
        let result = gstreamer::ElementFactory::make("webrtcbin").build();
        assert!(
            result.is_ok(),
            "webrtcbin deve estar disponível (instale gstreamer1.0-plugins-bad)"
        );
    }

    #[test]
    fn test_hw_decoder_or_fallback() {
        gstreamer::init().unwrap();
        // Em ambiente de CI/dev, o fallback software pode ser usado
        // Em produção, vaapih264dec ou d3d11h264dec são obrigatórios
        let result = create_hw_decoder();
        assert!(
            result.is_ok(),
            "Deve existir pelo menos um decoder H.264 disponível"
        );
    }
}
