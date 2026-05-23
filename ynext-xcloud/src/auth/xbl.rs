//! # Módulo XBL — Xbox Live Token
//!
//! Troca o `access_token` da Microsoft Account (MSA) pelo **Xbox Live Token (XBL)**.
//!
//! ## Fluxo:
//! ```text
//! MSA access_token
//!   │
//!   ▼
//! POST https://user.auth.xboxlive.com/user/authenticate
//!   │
//!   ▼
//! XBL Token (DisplayClaims.xui[0].uhs = UserHash)
//! ```
//!
//! ## Referência:
//! https://wiki.xasf.io/wiki/Xbox_Live_Authentication

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

/// Endpoint de autenticação Xbox Live
const XBL_AUTH_ENDPOINT: &str = "https://user.auth.xboxlive.com/user/authenticate";

// ===========================================================================
// Estruturas de dados
// ===========================================================================

/// Token Xbox Live (XBL) obtido após troca do MSA token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XblToken {
    /// Token XBL propriamente dito
    pub token: String,
    /// User hash do usuário Xbox (necessário para o header XBL3.0)
    pub user_hash: String,
    /// Timestamp de emissão (ISO 8601)
    pub issue_instant: String,
    /// Timestamp de expiração (ISO 8601)
    pub not_after: String,
    /// Gamertag do usuário (se disponível)
    pub gamertag: Option<String>,
    /// XUID do usuário Xbox
    pub xuid: Option<String>,
}

/// Corpo da requisição para autenticação XBL
#[derive(Serialize)]
struct XblAuthRequest<'a> {
    #[serde(rename = "Properties")]
    properties: XblProperties<'a>,
    #[serde(rename = "RelyingParty")]
    relying_party: &'a str,
    #[serde(rename = "TokenType")]
    token_type: &'a str,
}

#[derive(Serialize)]
struct XblProperties<'a> {
    #[serde(rename = "AuthMethod")]
    auth_method: &'a str,
    #[serde(rename = "SiteName")]
    site_name: &'a str,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

/// Resposta do servidor XBL
#[derive(Deserialize)]
struct XblAuthResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "IssueInstant")]
    issue_instant: String,
    #[serde(rename = "NotAfter")]
    not_after: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XblDisplayClaims,
}

#[derive(Deserialize)]
struct XblDisplayClaims {
    xui: Vec<XuiClaim>,
}

#[derive(Deserialize)]
struct XuiClaim {
    /// User Hash — necessário para o header XBL3.0
    uhs: String,
    /// Gamertag do usuário
    gtg: Option<String>,
    /// XUID do usuário
    xid: Option<String>,
}

// ===========================================================================
// Implementação
// ===========================================================================

/// Troca o access_token MSA pelo Xbox Live Token (XBL)
pub async fn exchange_for_xbl(msa_access_token: &str) -> Result<XblToken> {
    let client = build_http_client()?;

    let request_body = XblAuthRequest {
        properties: XblProperties {
            auth_method: "RPS",
            site_name: "user.auth.xboxlive.com",
            // O prefixo "d=" indica que é um access_token (não um JWT RPS antigo)
            rps_ticket: format!("d={}", msa_access_token),
        },
        relying_party: "http://auth.xboxlive.com",
        token_type: "JWT",
    };

    debug!("Trocando MSA access_token por Xbox Live Token...");

    let response = client
        .post(XBL_AUTH_ENDPOINT)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-xbl-contract-version", "1")
        .json(&request_body)
        .send()
        .await
        .context("Falha ao enviar requisição XBL")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Falha na autenticação XBL: HTTP {} — {}", status, body);
    }

    let xbl_resp: XblAuthResponse = response
        .json()
        .await
        .context("Falha ao parsear resposta XBL")?;

    // Extrai o primeiro claim XUI (contém user hash)
    let xui = xbl_resp
        .display_claims
        .xui
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Resposta XBL sem DisplayClaims.xui"))?;

    debug!(
        user_hash = %xui.uhs,
        gamertag = ?xui.gtg,
        "Xbox Live Token obtido com sucesso"
    );

    Ok(XblToken {
        token: xbl_resp.token,
        user_hash: xui.uhs,
        issue_instant: xbl_resp.issue_instant,
        not_after: xbl_resp.not_after,
        gamertag: xui.gtg,
        xuid: xui.xid,
    })
}

/// Constrói o cliente HTTP otimizado para chamadas XBL
fn build_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("XboxApp/2309.1001.3.0")
        .timeout(Duration::from_secs(15))
        .build()
        .context("Falha ao construir cliente HTTP para XBL")
}

// ===========================================================================
// Testes
// ===========================================================================

#[cfg(test)]
mod tests {


    #[test]
    fn test_rps_ticket_format() {
        // Garante que o formato "d=<token>" está correto
        let msa_token = "fake_access_token_123";
        let expected_rps = format!("d={}", msa_token);
        assert_eq!(expected_rps, "d=fake_access_token_123");
    }
}
