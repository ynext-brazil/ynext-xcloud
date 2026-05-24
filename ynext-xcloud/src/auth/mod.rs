//! # Ynext Xcloud — Módulo de Autenticação Microsoft
//!
//! Este módulo implementa o fluxo completo de autenticação para o Xbox Cloud Gaming:
//!
//! ```text
//! Usuário
//!   │
//!   ▼
//! [1] Microsoft Account OAuth 2.0 — Device Code Flow
//!     POST https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode
//!   │
//!   ▼
//! [2] Polling para obter access_token + refresh_token (Microsoft Account)
//!     POST https://login.microsoftonline.com/consumers/oauth2/v2.0/token
//!   │
//!   ▼
//! [3] Trocar por Xbox Live Token (XBL)
//!     POST https://user.auth.xboxlive.com/user/authenticate
//!   │
//!   ▼
//! [4] Trocar XBL por XSTS Token (Xbox Security Token Service)
//!     POST https://xsts.auth.xboxlive.com/xsts/authorize
//!   │
//!   ▼
//! [5] Header final: "XBL3.0 x=<userhash>;<xsts_token>"
//! ```

pub mod msa;
pub mod token_store;
pub mod xbl;
pub mod xsts;

use anyhow::Result;
use tracing::info;

use crate::auth::token_store::TokenStore;

/// Orquestra o fluxo completo de autenticação.
/// Retorna o cabeçalho de autorização pronto para uso nas APIs do xCloud.
pub async fn authenticate(store: &mut TokenStore) -> Result<String> {
    // 1. Tenta carregar tokens existentes e válidos
    if let Some(auth_header) = try_load_saved_tokens(store).await? {
        info!("✅ Tokens válidos carregados do keyring — sem necessidade de login");
        return Ok(auth_header);
    }

    // 2. Fluxo MSA (Microsoft Account) — Device Code
    info!("🔑 Iniciando fluxo de autenticação Microsoft...");
    let msa_tokens = msa::device_code_flow().await?;
    store.save_msa_tokens(&msa_tokens)?;

    // 3. Troca MSA access_token por Xbox Live Token
    info!("🎮 Trocando por Xbox Live Token (XBL)...");
    let xbl_token = xbl::exchange_for_xbl(&msa_tokens.access_token).await?;

    // 4. Troca XBL por XSTS Token
    info!("🔐 Obtendo XSTS Token para xCloud...");
    let xsts_token = xsts::exchange_for_xsts(&xbl_token.token).await?;
    store.save_xsts_token(&xsts_token)?;

    // 5. Monta o cabeçalho de autorização final
    let auth_header = format!("XBL3.0 x={};{}", xsts_token.user_hash, xsts_token.token);

    info!("✅ Autenticação completa! xCloud pronto.");
    Ok(auth_header)
}

/// Tenta carregar e validar tokens previamente salvos.
/// Se o access_token expirou mas o refresh_token ainda é válido, renova automaticamente.
async fn try_load_saved_tokens(store: &mut TokenStore) -> Result<Option<String>> {
    // Verifica se temos tokens XSTS válidos (não expirados)
    if let Some(xsts) = store.load_xsts_token()? {
        if !xsts.is_expired() {
            let auth_header = format!("XBL3.0 x={};{}", xsts.user_hash, xsts.token);
            return Ok(Some(auth_header));
        }
    }

    // XSTS expirou — tenta renovar via refresh_token MSA
    if let Some(msa_tokens) = store.load_msa_tokens()? {
        if msa_tokens.has_valid_refresh_token() {
            info!("🔄 Renovando tokens via refresh_token...");
            let renewed = msa::refresh_tokens(&msa_tokens.refresh_token).await?;
            store.save_msa_tokens(&renewed)?;

            let xbl_token = xbl::exchange_for_xbl(&renewed.access_token).await?;
            let xsts_token = xsts::exchange_for_xsts(&xbl_token.token).await?;
            store.save_xsts_token(&xsts_token)?;

            let auth_header = format!("XBL3.0 x={};{}", xsts_token.user_hash, xsts_token.token);
            return Ok(Some(auth_header));
        }
    }

    Ok(None)
}
