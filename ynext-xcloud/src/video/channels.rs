//! # Canais MPSC — Ponte entre Sinalização (Fase 2) e Vídeo (Fase 3)
//!
//! Define os tipos e construtores dos canais Tokio MPSC que conectam
//! o módulo de sinalização WebRTC com o pipeline de vídeo GStreamer.
//!
//! ## Fluxo
//!
//! ```text
//! [Fase 2 - signaling]                    [Fase 3 - video]
//!    establish_session()
//!          │ SdpAnswer + IceCandidates
//!          └──── VideoTx (MPSC sender) ──────▶ VideoRx (MPSC receiver)
//!                                                    │
//!                                             webrtcbin.set_remote_description()
//!                                             webrtcbin.add_ice_candidate()
//! ```

use crate::signaling::IceCandidate;

/// Mensagem enviada da sinalização para o pipeline de vídeo.
/// Contém o SDP Answer da Microsoft e os candidatos ICE remotos.
#[derive(Debug)]
pub struct SignalingToVideoMessage {
    /// SDP Answer recebido da API xCloud (para injetar no `webrtcbin`)
    pub sdp_answer: String,
    /// Candidatos ICE remotos do servidor xCloud
    pub ice_candidates: Vec<IceCandidate>,
}

/// Sender do canal: usado pelo módulo de sinalização para enviar a sessão
pub type VideoTx = tokio::sync::mpsc::Sender<SignalingToVideoMessage>;

/// Receiver do canal: usado pelo pipeline GStreamer para receber a sessão
pub type VideoRx = tokio::sync::mpsc::Receiver<SignalingToVideoMessage>;

/// Cria o par (sender, receiver) do canal de vídeo com buffer de 1 mensagem.
///
/// Buffer de 1 é suficiente: apenas uma sessão é transmitida por vez.
pub fn create_video_channel() -> (VideoTx, VideoRx) {
    tokio::sync::mpsc::channel(1)
}
