//! # Módulo ICE — Interactive Connectivity Establishment
//!
//! Envia os candidatos ICE locais (gerados pelo `webrtcbin` do GStreamer)
//! para a API xCloud, completando o handshake de conectividade.
//!
//! ## Endpoint
//! `POST /v4/sessions/{session_id}/ice`
//!
//! ## Como funciona com o webrtcbin
//! O `webrtcbin` emite o sinal `"on-ice-candidate"` sempre que descobre
//! um novo candidato ICE local. Cada candidato é coletado e enviado aqui
//! em lote para a API xCloud.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use tracing::debug;

use crate::signaling::{device_info_header, IceCandidate};

/// Body para envio de candidatos ICE
#[derive(Serialize)]
struct SendIceRequest {
    candidates: Vec<IceCandidate>,
}

/// Envia os candidatos ICE locais para o servidor xCloud.
///
/// Deve ser chamado após coletar candidatos ICE do `webrtcbin` via
/// o sinal `"on-ice-candidate"`.
pub async fn send_ice_candidates(
    client: &reqwest::Client,
    auth_header: &str,
    host: &str,
    session_path: &str,
    candidates: &[IceCandidate],
) -> Result<()> {
    if candidates.is_empty() {
        debug!("Nenhum candidato ICE para enviar — pulando");
        return Ok(());
    }

    let url = format!("{}{}/ice", host, session_path);

    let body = SendIceRequest {
        candidates: candidates.to_vec(),
    };

    debug!(
        count = candidates.len(),
        "Enviando candidatos ICE para: {}",
        url
    );

    let response = client
        .post(&url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .header("x-ms-device-info", device_info_header())
        .json(&body)
        .send()
        .await
        .context("Falha ao enviar candidatos ICE")?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Falha ao enviar ICE candidates: HTTP {} — {}", status, body);
    }

    debug!("✅ {} candidatos ICE enviados com sucesso", candidates.len());
    Ok(())
}

/// Recebe candidatos ICE remotos (do servidor xCloud) e os injeta no webrtcbin.
///
/// # Integração com GStreamer
/// ```rust,ignore
/// // Pseudo-código — implementado na Fase 3 junto com o pipeline GStreamer
/// for candidate in remote_candidates {
///     webrtcbin.emit_by_name::<()>("add-ice-candidate", &[
///         &(candidate.sdp_m_line_index.unwrap_or(0) as u32),
///         &candidate.candidate.as_str(),
///     ]);
/// }
/// ```
pub async fn receive_remote_ice_candidates(
    client: &reqwest::Client,
    auth_header: &str,
    host: &str,
    session_path: &str,
) -> Result<Vec<IceCandidate>> {
    let url = format!("{}{}/ice", host, session_path);

    let response = client
        .get(&url)
        .header("Authorization", auth_header)
        .header("x-ms-device-info", device_info_header())
        .send()
        .await
        .context("Falha ao buscar candidatos ICE remotos")?;

    if !response.status().is_success() {
        // ICE pode não ter candidatos ainda — não é erro fatal
        debug!("Nenhum candidato ICE remoto disponível ainda");
        return Ok(vec![]);
    }

    #[derive(serde::Deserialize)]
    struct IceResponse {
        candidates: Vec<IceCandidate>,
    }

    let resp: IceResponse = response
        .json()
        .await
        .context("Falha ao parsear candidatos ICE remotos")?;

    debug!("{} candidatos ICE remotos recebidos", resp.candidates.len());
    Ok(resp.candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ice_candidate_serialization() {
        let candidate = IceCandidate {
            candidate: "candidate:1 1 UDP 2122260223 192.168.1.100 56789 typ host".to_string(),
            sdp_mid: Some("video".to_string()),
            sdp_m_line_index: Some(0),
        };
        let json = serde_json::to_string(&candidate).unwrap();
        assert!(json.contains("candidate:1"));
        assert!(json.contains("sdpMid"));
        assert!(json.contains("sdpMLineIndex"));
    }
}
