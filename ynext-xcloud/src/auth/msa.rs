//! # Módulo MSA — Microsoft Account Authentication
//!
//! Implementa o **OAuth 2.0 Device Code Flow** da Microsoft.
//!
//! ## Por que Device Code Flow?
//! - Aplicações nativas sem redirecionamento de URL não podem usar Authorization Code Flow
//! - O usuário autoriza em `microsoft.com/devicelogin` usando qualquer browser
//! - Nosso cliente faz polling até receber os tokens
//!
//! ## Referência:
//! https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-device-auth-grant

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

// ===========================================================================
// Constantes da API Microsoft
// ===========================================================================

/// Client ID da aplicação Xbox (pública, bem conhecida — usada pelo Xbox App oficial)
/// Esta é a mesma utilizada por outros clientes open source como o Greenlight
const XBOX_APP_CLIENT_ID: &str = "000000004C12AE6F";

/// Escopo necessário para acessar Xbox Live e xCloud
/// `xboxlive.signin` + `offline_access` (refresh tokens)
const MSA_SCOPE: &str = "Xboxlive.signin Xboxlive.offline_access offline_access";

/// Endpoint para Device Code Request
const DEVICE_CODE_ENDPOINT: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";

/// Endpoint para Token Exchange (polling)
const TOKEN_ENDPOINT: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

/// Intervalo padrão de polling (segundos) — será sobrescrito pelo server_interval
const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;

// ===========================================================================
// Estruturas de dados
// ===========================================================================

/// Resposta do endpoint de Device Code
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    /// Código que o usuário digita em microsoft.com/devicelogin
    pub user_code: String,
    /// Código que usamos internamente para fazer polling
    pub device_code: String,
    /// URL onde o usuário autoriza (sempre microsoft.com/devicelogin)
    pub verification_uri: String,
    /// Tempo de vida do device_code em segundos
    pub expires_in: u64,
    /// Intervalo mínimo entre tentativas de polling (segundos)
    pub interval: u64,
    /// Mensagem amigável para o usuário (formatada pela Microsoft)
    pub message: String,
}

/// Tokens MSA retornados após autenticação bem-sucedida
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsaTokens {
    /// Token de acesso (curta duração — ~1 hora)
    pub access_token: String,
    /// Token de renovação (longa duração — pode ser 90 dias)
    pub refresh_token: String,
    /// Tipo do token (sempre "Bearer")
    pub token_type: String,
    /// Escopos concedidos
    pub scope: String,
    /// Tempo de expiração (UTC)
    pub expires_at: DateTime<Utc>,
}

impl MsaTokens {
    /// Verifica se o access_token ainda é válido (com margem de 5 minutos)
    pub fn is_access_token_valid(&self) -> bool {
        self.expires_at > Utc::now() + Duration::minutes(5)
    }

    /// Verifica se temos um refresh_token para renovação
    /// (refresh_tokens não expiram facilmente, mas guardamos por segurança)
    pub fn has_valid_refresh_token(&self) -> bool {
        !self.refresh_token.is_empty()
    }
}

/// Resposta raw do endpoint de token (antes de processar)
#[derive(Debug, Deserialize)]
struct RawTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    token_type: Option<String>,
    scope: Option<String>,
    expires_in: Option<u64>,
    // Campos de erro
    error: Option<String>,
    error_description: Option<String>,
}

// ===========================================================================
// Implementação do Device Code Flow
// ===========================================================================

/// Executa o fluxo completo de Device Code:
/// 1. Solicita device_code e user_code
/// 2. Exibe instrução ao usuário
/// 3. Faz polling até obter os tokens
pub async fn device_code_flow() -> Result<MsaTokens> {
    let client = build_http_client()?;

    // Passo 1: Solicitar device_code
    let device_code_resp = request_device_code(&client).await?;

    // Passo 2: Exibir instruções claras para o usuário
    display_auth_instructions(&device_code_resp);

    // Passo 3: Polling até obter tokens ou timeout
    let tokens = poll_for_tokens(&client, &device_code_resp).await?;

    Ok(tokens)
}

/// Renova tokens usando o refresh_token existente
pub async fn refresh_tokens(refresh_token: &str) -> Result<MsaTokens> {
    let client = build_http_client()?;

    let params = [
        ("client_id", XBOX_APP_CLIENT_ID),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("scope", MSA_SCOPE),
    ];

    debug!("Enviando refresh_token para renovação...");

    let response = client
        .post(TOKEN_ENDPOINT)
        .form(&params)
        .send()
        .await
        .context("Falha ao enviar requisição de renovação de token")?;

    let raw: RawTokenResponse = response
        .json()
        .await
        .context("Falha ao parsear resposta de renovação")?;

    parse_token_response(raw)
}

// ===========================================================================
// Funções auxiliares privadas
// ===========================================================================

/// Constrói o cliente HTTP com configurações otimizadas
fn build_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("XboxApp/2309.1001.3.0 Mozilla/5.0")
        .timeout(StdDuration::from_secs(30))
        .build()
        .context("Falha ao construir cliente HTTP")
}

/// Solicita o device_code ao servidor Microsoft
async fn request_device_code(client: &reqwest::Client) -> Result<DeviceCodeResponse> {
    let params = [
        ("client_id", XBOX_APP_CLIENT_ID),
        ("scope", MSA_SCOPE),
    ];

    debug!("Solicitando device_code ao servidor Microsoft...");

    let response = client
        .post(DEVICE_CODE_ENDPOINT)
        .form(&params)
        .send()
        .await
        .context("Falha ao solicitar device_code")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Falha na requisição de device_code: HTTP {} — {}", status, body);
    }

    let device_code_resp: DeviceCodeResponse = response
        .json()
        .await
        .context("Falha ao parsear resposta do device_code")?;

    debug!(
        user_code = %device_code_resp.user_code,
        expires_in = device_code_resp.expires_in,
        "Device code obtido com sucesso"
    );

    Ok(device_code_resp)
}

/// Exibe as instruções de autenticação para o usuário de forma clara
fn display_auth_instructions(resp: &DeviceCodeResponse) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║          🎮 YNEXT-XCLOUD — AUTENTICAÇÃO MICROSOFT            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  1. Abra o navegador e acesse:                              ║");
    println!("║     👉  {}                ║", resp.verification_uri);
    println!("║                                                              ║");
    println!("║  2. Digite o código abaixo quando solicitado:               ║");
    println!("║                                                              ║");
    println!("║              ┌─────────────────┐                            ║");
    println!("║              │   {}        │                            ║", resp.user_code);
    println!("║              └─────────────────┘                            ║");
    println!("║                                                              ║");
    println!("║  ⏳ Aguardando autorização... (expira em {}s)         ║", resp.expires_in);
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}

/// Faz polling no endpoint de token até obter resposta ou timeout
async fn poll_for_tokens(
    client: &reqwest::Client,
    device_code_resp: &DeviceCodeResponse,
) -> Result<MsaTokens> {
    let poll_interval = StdDuration::from_secs(
        device_code_resp.interval.max(DEFAULT_POLL_INTERVAL_SECS)
    );
    let timeout_at = std::time::Instant::now()
        + StdDuration::from_secs(device_code_resp.expires_in);

    let mut attempt = 0u32;

    loop {
        if std::time::Instant::now() > timeout_at {
            bail!("⏰ Tempo de autenticação expirado. Execute novamente para tentar de novo.");
        }

        sleep(poll_interval).await;
        attempt += 1;

        debug!(attempt, "Verificando autorização...");

        let params = [
            ("client_id", XBOX_APP_CLIENT_ID),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code_resp.device_code.as_str()),
        ];

        let response = client
            .post(TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .context("Falha no polling de tokens")?;

        let raw: RawTokenResponse = response
            .json()
            .await
            .context("Falha ao parsear resposta de polling")?;

        match raw.error.as_deref() {
            None => {
                // Sem campo de erro — verificamos se temos tokens
                if raw.access_token.is_some() {
                    info!("🎉 Autorização concedida!");
                    return parse_token_response(raw);
                }
            }
            Some("authorization_pending") => {
                // Usuário ainda não autorizou — continua polling (normal)
                debug!("Aguardando autorização do usuário...");
            }
            Some("slow_down") => {
                // Server pediu para esperar mais
                warn!("Servidor pediu redução de velocidade — aumentando intervalo");
                sleep(StdDuration::from_secs(5)).await;
            }
            Some("authorization_declined") => {
                bail!("❌ Autorização negada pelo usuário.");
            }
            Some("expired_token") => {
                bail!("⏰ Device code expirou. Execute novamente para obter um novo código.");
            }
            Some(other_error) => {
                let desc = raw.error_description.as_deref().unwrap_or("sem descrição");
                bail!("❌ Erro de autenticação: {} — {}", other_error, desc);
            }
        }

        // Exibe progresso a cada 5 tentativas
        if attempt % 5 == 0 {
            let remaining = timeout_at
                .saturating_duration_since(std::time::Instant::now())
                .as_secs();
            info!("⏳ Aguardando autorização... ({}s restantes)", remaining);
        }
    }
}

/// Converte a resposta raw de token em `MsaTokens`
fn parse_token_response(raw: RawTokenResponse) -> Result<MsaTokens> {
    let access_token = raw
        .access_token
        .ok_or_else(|| anyhow!("Resposta sem access_token"))?;

    let refresh_token = raw
        .refresh_token
        .ok_or_else(|| anyhow!("Resposta sem refresh_token"))?;

    let expires_in = raw
        .expires_in
        .ok_or_else(|| anyhow!("Resposta sem expires_in"))?;

    let expires_at = Utc::now() + Duration::seconds(expires_in as i64);

    Ok(MsaTokens {
        access_token,
        refresh_token,
        token_type: raw.token_type.unwrap_or_else(|| "Bearer".to_string()),
        scope: raw.scope.unwrap_or_else(|| MSA_SCOPE.to_string()),
        expires_at,
    })
}

// ===========================================================================
// Testes
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_msa_tokens_validity() {
        let tokens = MsaTokens {
            access_token: "test_access".to_string(),
            refresh_token: "test_refresh".to_string(),
            token_type: "Bearer".to_string(),
            scope: MSA_SCOPE.to_string(),
            expires_at: Utc::now() + Duration::hours(1),
        };

        assert!(tokens.is_access_token_valid(), "Token deveria ser válido");
        assert!(tokens.has_valid_refresh_token(), "Deveria ter refresh_token");
    }

    #[test]
    fn test_expired_msa_tokens() {
        let tokens = MsaTokens {
            access_token: "test_access".to_string(),
            refresh_token: "test_refresh".to_string(),
            token_type: "Bearer".to_string(),
            scope: MSA_SCOPE.to_string(),
            expires_at: Utc::now() - Duration::hours(1), // Expirado há 1 hora
        };

        assert!(!tokens.is_access_token_valid(), "Token deveria estar expirado");
        assert!(tokens.has_valid_refresh_token(), "Ainda deveria ter refresh_token");
    }
}
