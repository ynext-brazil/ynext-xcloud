//! # Módulo de Sinalização WebRTC — Fase 2
//!
//! Responsável por estabelecer a sessão de streaming com a API do Xbox Cloud Gaming
//! e integrar com o pipeline GStreamer via **`webrtcbin`**.
//!
//! ## Decisão Arquitetural Absoluta
//!
//! É **terminantemente proibido** usar `webrtc-rs` de forma isolada neste projeto.
//! Toda a sinalização WebRTC (SDP/ICE) e o decode de vídeo (H.264 via VA-API/DxVA)
//! devem ocorrer **dentro do pipeline GStreamer** para garantir Zero-Copy.
//!
//! ```text
//! API xCloud (HTTPS)                    GStreamer Pipeline
//!       │                                      │
//!       │ POST /v2/login/user (sessão)          │
//!       │ POST /v4/sessions/{id}/sdp (oferta)   │
//!       │◀── SDP answer ────────────────────────┤
//!       │                                       │
//!       └─── ICE candidates ────────────────▶ webrtcbin
//!                                               │
//!                                        rtph264depay
//!                                               │
//!                                         h264parse
//!                                               │
//!                                    vaapih264dec (Linux)
//!                                   d3d11h264dec (Windows)
//!                                               │
//!                                          glimagesink
//! ```
//!
//! ## Fluxo de Sinalização xCloud
//!
//! ```text
//! [1] POST /v2/login/user
//!     Body: { "offeringSessions": { "xhome": {} } }
//!     Header: Authorization: "XBL3.0 x=<userhash>;<xsts_token>"
//!     ─▶ Retorna: { "sessionPath": "/v4/sessions/{id}" }
//!
//! [2] GET  /v4/sessions/{id}/state  (poll até "Provisioned")
//!
//! [3] POST /v4/sessions/{id}/sdp
//!     Body: { "type": "offer", "sdp": "<SDP gerado pelo webrtcbin>" }
//!     ─▶ Retorna: { "type": "answer", "sdp": "<SDP da Microsoft>" }
//!
//! [4] POST /v4/sessions/{id}/ice
//!     Body: { "candidates": [ ... ICE candidates do webrtcbin ... ] }
//!
//! [5] webrtcbin recebe o SDP answer + ICE → conecta ao servidor xCloud
//! ```

pub mod ice;
pub mod sdp;
pub mod session;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

// ===========================================================================
// Constantes da API xCloud
// ===========================================================================

/// Host da API de sinalização xCloud (servidor de produção)
const XCLOUD_HOST: &str = "https://xhome.gssv-play-prodca.xboxlive.com";

/// Número máximo de tentativas de polling para provisionar a sessão
const MAX_PROVISION_POLLS: u32 = 30;

/// Intervalo entre polls de estado da sessão
const POLL_INTERVAL_MS: u64 = 2000;

// ===========================================================================
// Estruturas de dados — Sessão xCloud
// ===========================================================================

/// Resultado de uma sessão de streaming estabelecida com sucesso
#[derive(Debug)]
pub struct StreamingSession {
    /// ID único da sessão xCloud
    pub session_id: String,
    /// Path completo da sessão (ex: /v4/sessions/abc123)
    pub session_path: String,
    /// SDP answer recebido da Microsoft (para injetar no webrtcbin)
    pub sdp_answer: String,
    /// Candidatos ICE do servidor xCloud
    pub ice_candidates: Vec<IceCandidate>,
    /// Token de autorização XBL3.0 (reutilizado nas chamadas da sessão)
    pub auth_header: String,
}

/// Candidato ICE retornado pela API xCloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    pub candidate: String,
    #[serde(rename = "sdpMid")]
    pub sdp_mid: Option<String>,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_m_line_index: Option<u32>,
}

/// Estado de provisionamento da sessão xCloud
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SessionState {
    /// Sessão sendo criada no servidor
    Provisioning,
    /// Sessão pronta para troca de SDP
    Provisioned,
    /// Sessão expirada ou com erro
    Failed,
    /// Estado desconhecido
    #[serde(other)]
    Unknown,
}

// ===========================================================================
// Estruturas de request/response das APIs
// ===========================================================================

/// Body para criação de sessão (POST /v2/login/user)
#[derive(Serialize)]
struct LoginRequest {
    #[serde(rename = "offeringSessions")]
    offering_sessions: OfferingSessions,
}

#[derive(Serialize)]
struct OfferingSessions {
    xhome: serde_json::Value,
}

/// Resposta da criação de sessão
#[derive(Deserialize)]
struct LoginResponse {
    #[serde(rename = "sessionPath")]
    session_path: String,
}

/// Body do SDP offer (POST /v4/sessions/{id}/sdp)
#[derive(Serialize)]
pub struct SdpOfferRequest {
    #[serde(rename = "type")]
    pub sdp_type: String,
    pub sdp: String,
}

/// Resposta do SDP answer da Microsoft
#[derive(Deserialize)]
pub struct SdpAnswerResponse {
    #[serde(rename = "type")]
    pub sdp_type: String,
    pub sdp: String,
}

/// Resposta do estado da sessão (GET /v4/sessions/{id}/state)
#[derive(Deserialize)]
struct SessionStateResponse {
    state: SessionState,
    #[serde(rename = "errorDetails")]
    error_details: Option<String>,
}

/// Body para envio de candidatos ICE
#[derive(Serialize)]
struct IceCandidatesRequest {
    candidates: Vec<IceCandidate>,
}

// ===========================================================================
// Orquestrador da sinalização
// ===========================================================================

/// Establece uma sessão completa de streaming com o xCloud.
///
/// Recebe a SDP offer gerada pelo elemento `webrtcbin` do GStreamer,
/// negocia com a API da Microsoft e retorna a `StreamingSession` pronta.
///
/// # Parâmetros
/// - `auth_header`: Cabeçalho "XBL3.0 x=<userhash>;<xsts_token>" da Fase 1
/// - `sdp_offer`: SDP offer gerado pelo elemento `webrtcbin` do GStreamer
/// - `local_ice_candidates`: Candidatos ICE coletados pelo `webrtcbin`
pub async fn establish_session(
    auth_header: &str,
    sdp_offer: &str,
    local_ice_candidates: Vec<IceCandidate>,
) -> Result<StreamingSession> {
    let client = build_signaling_client()?;

    // Passo 1: Criar sessão no xCloud
    info!("📡 Criando sessão de streaming no Xbox Cloud Gaming...");
    let session_path = session::create_session(&client, auth_header, XCLOUD_HOST).await?;
    let session_id = extract_session_id(&session_path);

    debug!(session_id = %session_id, "Sessão criada");

    // Passo 2: Aguardar provisionamento da sessão
    info!("⏳ Aguardando provisionamento da sessão...");
    wait_for_provisioned(&client, auth_header, XCLOUD_HOST, &session_path).await?;

    // Passo 3: Enviar SDP offer e receber answer da Microsoft
    info!("🤝 Negociando SDP com o servidor xCloud...");
    let sdp_answer = sdp::exchange_sdp(
        &client,
        auth_header,
        XCLOUD_HOST,
        &session_path,
        sdp_offer,
    )
    .await?;

    // Passo 4: Enviar candidatos ICE locais
    info!("🧊 Enviando candidatos ICE...");
    ice::send_ice_candidates(
        &client,
        auth_header,
        XCLOUD_HOST,
        &session_path,
        &local_ice_candidates,
    )
    .await?;

    info!("✅ Sessão WebRTC estabelecida! Iniciando pipeline GStreamer...");

    Ok(StreamingSession {
        session_id,
        session_path,
        sdp_answer,
        ice_candidates: local_ice_candidates,
        auth_header: auth_header.to_string(),
    })
}

/// Aguarda o estado "Provisioned" da sessão via polling
async fn wait_for_provisioned(
    client: &reqwest::Client,
    auth_header: &str,
    host: &str,
    session_path: &str,
) -> Result<()> {
    for attempt in 0..MAX_PROVISION_POLLS {
        sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;

        let url = format!("{}{}/state", host, session_path);

        let response = client
            .get(&url)
            .header("Authorization", auth_header)
            .header("x-ms-device-info", device_info_header())
            .send()
            .await
            .context("Falha ao verificar estado da sessão")?;

        let state_resp: SessionStateResponse = response
            .json()
            .await
            .context("Falha ao parsear estado da sessão")?;

        match state_resp.state {
            SessionState::Provisioned => {
                debug!(attempt, "Sessão provisionada com sucesso");
                return Ok(());
            }
            SessionState::Provisioning => {
                debug!(attempt, "Sessão ainda sendo provisionada...");
            }
            SessionState::Failed => {
                let details = state_resp.error_details.unwrap_or_else(|| "sem detalhes".to_string());
                anyhow::bail!("❌ Sessão xCloud falhou: {}", details);
            }
            SessionState::Unknown => {
                warn!(attempt, "Estado desconhecido da sessão");
            }
        }
    }

    anyhow::bail!(
        "⏰ Timeout: sessão não foi provisionada após {} tentativas",
        MAX_PROVISION_POLLS
    )
}

/// Extrai o ID da sessão do path (ex: "/v4/sessions/abc123" → "abc123")
fn extract_session_id(session_path: &str) -> String {
    session_path
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Constrói o cliente HTTP com headers padrão do Xbox
fn build_signaling_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("XboxApp/2309.1001.3.0")
        .timeout(Duration::from_secs(30))
        .build()
        .context("Falha ao construir cliente HTTP para sinalização")
}

/// Header de informações do dispositivo exigido pela API xCloud
pub fn device_info_header() -> String {
    // Formato JSON codificado em base64 com informações do "dispositivo"
    // A Microsoft usa isso para heurísticas de qualidade de stream
    let device_info = serde_json::json!({
        "appInfo": {
            "env": {
                "clientAppId": "Microsoft.GamingApp",
                "clientAppType": "native",
                "clientAppVersion": "2309.1001.3.0",
                "clientSdkVersion": "10.0.0",
                "httpEnvironment": "prod",
                "sdkInstallId": ""
            }
        },
        "dev": {
            "hw": {
                "make": "Microsoft",
                "model": "Surface",
                "sdktype": "native"
            },
            "os": {
                "name": std::env::consts::OS,
                "ver": "10.0.22621.0"
            }
        }
    });

    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .encode(device_info.to_string())
}

// ===========================================================================
// Testes
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_id() {
        assert_eq!(
            extract_session_id("/v4/sessions/abc123def456"),
            "abc123def456"
        );
        assert_eq!(extract_session_id("/v4/sessions/xyz"), "xyz");
    }

    #[test]
    fn test_session_state_deserialization() {
        let provisioned: SessionStateResponse =
            serde_json::from_str(r#"{"state": "Provisioned"}"#).unwrap();
        assert_eq!(provisioned.state, SessionState::Provisioned);

        let provisioning: SessionStateResponse =
            serde_json::from_str(r#"{"state": "Provisioning"}"#).unwrap();
        assert_eq!(provisioning.state, SessionState::Provisioning);

        let unknown: SessionStateResponse =
            serde_json::from_str(r#"{"state": "SomeNewState"}"#).unwrap();
        assert_eq!(unknown.state, SessionState::Unknown);
    }

    #[test]
    fn test_device_info_header_is_valid_base64() {
        let header = device_info_header();
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&header)
            .expect("Deve ser base64 válido");
        let json_str = String::from_utf8(decoded).expect("Deve ser UTF-8 válido");
        let _: serde_json::Value = serde_json::from_str(&json_str)
            .expect("Deve ser JSON válido");
    }
}
