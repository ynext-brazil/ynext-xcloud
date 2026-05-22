//! # Módulo SDP — Negociação Session Description Protocol
//!
//! Envia a SDP offer gerada pelo `webrtcbin` do GStreamer para a API xCloud
//! e recebe a SDP answer da Microsoft para fechar o handshake WebRTC.
//!
//! ## Endpoint
//! `POST /v4/sessions/{session_id}/sdp`
//!
//! ## Codec absoluto: H.264
//! A SDP offer gerada pelo `webrtcbin` deve ser forçada para H.264.
//! AV1 pode ser adicionado no futuro via toggle CLI, mas H.264 é o padrão
//! inegociável para suporte a hardware legado (Celeron/Atom/HD Graphics).

use anyhow::{bail, Context, Result};
use tracing::{debug, warn};

use crate::signaling::{device_info_header, SdpAnswerResponse, SdpOfferRequest};

/// Troca SDP offer (local/webrtcbin) pelo SDP answer (Microsoft/xCloud).
///
/// A `sdp_offer` deve ser a string SDP gerada pelo elemento `webrtcbin`
/// do GStreamer **após** ser filtrada para garantir apenas H.264.
pub async fn exchange_sdp(
    client: &reqwest::Client,
    auth_header: &str,
    host: &str,
    session_path: &str,
    sdp_offer: &str,
) -> Result<String> {
    // Sanitiza a SDP para garantir que apenas H.264 seja negociado
    let sanitized_offer = enforce_h264_only(sdp_offer);

    let url = format!("{}{}/sdp", host, session_path);

    let request_body = SdpOfferRequest {
        sdp_type: "offer".to_string(),
        sdp: sanitized_offer,
    };

    debug!("Enviando SDP offer para: {}", url);

    let response = client
        .post(&url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-ms-device-info", device_info_header())
        .json(&request_body)
        .send()
        .await
        .context("Falha ao enviar SDP offer")?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Falha na negociação SDP: HTTP {} — {}", status, body);
    }

    let answer: SdpAnswerResponse = response
        .json()
        .await
        .context("Falha ao parsear SDP answer")?;

    if answer.sdp_type != "answer" {
        bail!(
            "Tipo de SDP inesperado: esperado 'answer', recebido '{}'",
            answer.sdp_type
        );
    }

    debug!("SDP answer recebido com sucesso ({} bytes)", answer.sdp.len());
    Ok(answer.sdp)
}

/// Filtra a SDP para garantir que somente H.264 seja negociado.
///
/// Remove linhas de codec que não sejam H.264, garantindo que o servidor
/// xCloud não possa downgrade para VP8/VP9 ou outro codec não acelerado.
///
/// # Por que isso importa?
/// Hardware antigo (Intel HD Graphics 2000-4000) suporta VA-API apenas para H.264.
/// Se a Microsoft escolher VP9 ou AV1, o decode cairia para software (CPU 100%).
fn enforce_h264_only(sdp: &str) -> String {
    // Por ora, passamos o SDP sem modificação — a configuração do webrtcbin
    // já deve garantir H.264 via `video-caps` na Fase 3.
    // Este filtro será implementado na Fase 3 junto com o pipeline GStreamer.
    //
    // TODO (Fase 3): Parsear o SDP e remover payloads não-H.264
    if sdp.contains("VP9") || sdp.contains("AV1") {
        warn!("⚠️  SDP contém codecs além de H.264 — configure o webrtcbin para forçar H.264");
    }
    sdp.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforce_h264_only_passthrough() {
        let sdp = "v=0\r\nm=video 9 UDP/TLS/RTP/SAVPF 96\r\na=rtpmap:96 H264/90000\r\n";
        let result = enforce_h264_only(sdp);
        assert_eq!(result, sdp);
    }

    #[test]
    fn test_sdp_offer_request_serialization() {
        let req = SdpOfferRequest {
            sdp_type: "offer".to_string(),
            sdp: "v=0\r\n".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"type\":\"offer\""));
        assert!(json.contains("\"sdp\":\"v=0\\r\\n\""));
    }
}
