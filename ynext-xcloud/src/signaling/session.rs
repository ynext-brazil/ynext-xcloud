//! # Módulo de Sessão xCloud
//!
//! Gerencia a criação e o ciclo de vida de uma sessão de streaming.
//!
//! ## Endpoint
//! `POST https://xhome.gssv-play-prodca.xboxlive.com/v2/login/user`

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::signaling::device_info_header;

/// Corpo da requisição de login (criação de sessão)
#[derive(Serialize)]
struct CreateSessionRequest {
    #[serde(rename = "offeringSessions")]
    offering_sessions: serde_json::Value,
}

/// Resposta da criação de sessão
#[derive(Deserialize)]
struct CreateSessionResponse {
    #[serde(rename = "sessionPath")]
    session_path: Option<String>,
    // Campos de erro
    message: Option<String>,
}

/// Cria uma nova sessão de streaming no servidor xCloud.
///
/// Retorna o `session_path` (ex: `/v4/sessions/abc123`) usado em todas
/// as chamadas subsequentes.
pub async fn create_session(
    client: &reqwest::Client,
    auth_header: &str,
    host: &str,
) -> Result<String> {
    let url = format!("{}/v2/login/user", host);

    let body = CreateSessionRequest {
        offering_sessions: serde_json::json!({
            "xhome": {}
        }),
    };

    debug!("Criando sessão xCloud em: {}", url);

    let response = client
        .post(&url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-ms-device-info", device_info_header())
        .json(&body)
        .send()
        .await
        .context("Falha ao enviar requisição de criação de sessão")?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!(
            "Falha ao criar sessão xCloud: HTTP {} — {}",
            status,
            body
        );
    }

    let resp: CreateSessionResponse = response
        .json()
        .await
        .context("Falha ao parsear resposta de criação de sessão")?;

    resp.session_path.ok_or_else(|| {
        let msg = resp.message.unwrap_or_else(|| "sem sessionPath na resposta".to_string());
        anyhow::anyhow!("xCloud não retornou sessionPath: {}", msg)
    })
}
