//! # Módulo XSTS — Xbox Security Token Service
//!
//! Troca o **Xbox Live Token (XBL)** pelo **XSTS Token**, que é o token final
//! necessário para acessar as APIs do Xbox Cloud Gaming.
//!
//! ## Fluxo:
//! ```text
//! XBL Token
//!   │
//!   ▼
//! POST https://xsts.auth.xboxlive.com/xsts/authorize
//!   (RelyingParty = "http://xboxlive.com")  ← para xCloud
//!   │
//!   ▼
//! XSTS Token + UserHash
//!   │
//!   ▼
//! Header: "XBL3.0 x=<userhash>;<xsts_token>"
//! ```
//!
//! ## Erros de Autenticação Conhecidos:
//! | XErr Code | Significado |
//! |-----------|-------------|
//! | 2148916227 | Conta banida ou região não suportada |
//! | 2148916233 | Conta sem Xbox profile |
//! | 2148916235 | Xbox Live não disponível no país |
//! | 2148916236/7 | Conta adulta com restrições parentais |
//! | 2148916238 | Conta de menor sem permissão dos pais |

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

/// Endpoint do Xbox Security Token Service
const XSTS_AUTH_ENDPOINT: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";

/// Relying Party para xCloud — identifica o serviço que receberá o token
const XCLOUD_RELYING_PARTY: &str = "http://xboxlive.com";

// ===========================================================================
// Estruturas de dados
// ===========================================================================

/// Token XSTS — token final para autenticação no xCloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XstsToken {
    /// O token XSTS propriamente dito (longo JWT)
    pub token: String,
    /// User hash necessário para o header XBL3.0
    pub user_hash: String,
    /// Timestamp de expiração
    pub expires_at: DateTime<Utc>,
    /// Gamertag do usuário (se disponível nos claims)
    pub gamertag: Option<String>,
    /// XUID do usuário
    pub xuid: Option<String>,
}

impl XstsToken {
    /// Verifica se o token XSTS ainda é válido (com margem de 5 minutos)
    pub fn is_expired(&self) -> bool {
        let margin = chrono::Duration::minutes(5);
        self.expires_at <= Utc::now() + margin
    }
}

/// Corpo da requisição XSTS
#[derive(Serialize)]
struct XstsAuthRequest<'a> {
    #[serde(rename = "Properties")]
    properties: XstsProperties<'a>,
    #[serde(rename = "RelyingParty")]
    relying_party: &'a str,
    #[serde(rename = "TokenType")]
    token_type: &'a str,
}

#[derive(Serialize)]
struct XstsProperties<'a> {
    #[serde(rename = "SandboxId")]
    sandbox_id: &'a str,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<&'a str>,
}

/// Resposta do servidor XSTS (sucesso)
#[derive(Deserialize)]
struct XstsAuthResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "NotAfter")]
    not_after: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XstsDisplayClaims,
}

#[derive(Deserialize)]
struct XstsDisplayClaims {
    xui: Vec<XstsXuiClaim>,
}

#[derive(Deserialize)]
struct XstsXuiClaim {
    uhs: String,
    gtg: Option<String>,
    xid: Option<String>,
}

/// Resposta de erro do servidor XSTS
#[derive(Deserialize)]
struct XstsErrorResponse {
    #[serde(rename = "XErr")]
    x_err: Option<u64>,
    #[serde(rename = "Message")]
    message: Option<String>,
    #[serde(rename = "Redirect")]
    redirect: Option<String>,
}

// ===========================================================================
// Implementação
// ===========================================================================

/// Troca o Xbox Live Token (XBL) pelo XSTS Token para acesso ao xCloud
pub async fn exchange_for_xsts(xbl_token: &str) -> Result<XstsToken> {
    let client = build_http_client()?;

    let request_body = XstsAuthRequest {
        properties: XstsProperties {
            sandbox_id: "RETAIL",
            user_tokens: vec![xbl_token],
        },
        relying_party: XCLOUD_RELYING_PARTY,
        token_type: "JWT",
    };

    debug!("Trocando XBL Token por XSTS Token para xCloud...");

    let response = client
        .post(XSTS_AUTH_ENDPOINT)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-xbl-contract-version", "1")
        .json(&request_body)
        .send()
        .await
        .context("Falha ao enviar requisição XSTS")?;

    let status = response.status();

    if status.is_success() {
        let xsts_resp: XstsAuthResponse = response
            .json()
            .await
            .context("Falha ao parsear resposta XSTS")?;

        let xui = xsts_resp
            .display_claims
            .xui
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Resposta XSTS sem DisplayClaims.xui"))?;

        // Parseia o timestamp de expiração
        let expires_at = DateTime::parse_from_rfc3339(&xsts_resp.not_after)
            .context("Falha ao parsear NotAfter do XSTS")?
            .with_timezone(&Utc);

        debug!(
            user_hash = %xui.uhs,
            gamertag = ?xui.gtg,
            xuid = ?xui.xid,
            "XSTS Token obtido com sucesso!"
        );

        Ok(XstsToken {
            token: xsts_resp.token,
            user_hash: xui.uhs,
            expires_at,
            gamertag: xui.gtg,
            xuid: xui.xid,
        })
    } else if status == reqwest::StatusCode::UNAUTHORIZED {
        // Erro de autenticação — decodifica o código de erro XErr
        let error_resp: XstsErrorResponse = response.json().await.unwrap_or(XstsErrorResponse {
            x_err: None,
            message: None,
            redirect: None,
        });

        let error_msg = format_xsts_error(error_resp);
        bail!("Falha na autenticação XSTS: {}", error_msg);
    } else {
        let body = response.text().await.unwrap_or_default();
        bail!("Erro XSTS inesperado: HTTP {} — {}", status, body);
    }
}

/// Formata a mensagem de erro XSTS com explicação amigável
fn format_xsts_error(err: XstsErrorResponse) -> String {
    let explanation = match err.x_err {
        Some(2148916227) => "Sua conta foi banida ou a região não é suportada pelo Xbox Live.",
        Some(2148916233) => {
            "Sua conta Microsoft não tem um perfil Xbox. \
             Acesse xbox.com para criar um perfil gratuito."
        }
        Some(2148916235) => {
            "Xbox Live não está disponível no seu país/região. \
             Verifique: https://www.xbox.com/regions"
        }
        Some(2148916236) | Some(2148916237) => {
            "Esta conta de adulto possui restrições que impedem o acesso ao xCloud. \
             Verifique as configurações de conta em account.microsoft.com."
        }
        Some(2148916238) => {
            "Esta conta de menor de idade não tem permissão dos pais para acessar o xCloud. \
             O responsável deve conceder permissão em account.microsoft.com/family."
        }
        Some(code) => {
            return format!(
                "Código de erro desconhecido: {} (XErr={}). \
             Reporte em github.com/ynext/ynext-xcloud/issues",
                code, code
            )
        }
        None => "Erro de autenticação sem código específico.",
    };

    if let Some(redirect) = err.redirect {
        format!("{} Redirecionamento: {}", explanation, redirect)
    } else {
        explanation.to_string()
    }
}

/// Constrói o cliente HTTP para chamadas XSTS
fn build_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("XboxApp/2309.1001.3.0")
        .timeout(Duration::from_secs(15))
        .build()
        .context("Falha ao construir cliente HTTP para XSTS")
}

// ===========================================================================
// Testes
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_xsts_token_expiry() {
        let expired_token = XstsToken {
            token: "fake".to_string(),
            user_hash: "fake_hash".to_string(),
            expires_at: Utc::now() - chrono::Duration::hours(1),
            gamertag: None,
            xuid: None,
        };
        assert!(expired_token.is_expired(), "Token deveria estar expirado");

        let valid_token = XstsToken {
            token: "fake".to_string(),
            user_hash: "fake_hash".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            gamertag: None,
            xuid: None,
        };
        assert!(!valid_token.is_expired(), "Token deveria ser válido");
    }

    #[test]
    fn test_xsts_error_formatting() {
        let err_no_profile = XstsErrorResponse {
            x_err: Some(2148916233),
            message: None,
            redirect: Some("https://start.ui.xboxlive.com/AddJITAccount".to_string()),
        };
        let msg = format_xsts_error(err_no_profile);
        assert!(msg.contains("perfil Xbox"), "Deve mencionar perfil Xbox");
        assert!(msg.contains("xbox.com"), "Deve mencionar xbox.com");
    }
}
