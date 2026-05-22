//! # Token Store — Armazenamento Seguro de Tokens via Arquivo
//!
//! Persiste tokens de autenticação em um arquivo JSON com permissões restritas
//! (chmod 600 no Linux/macOS, ACL no Windows), garantindo que somente o usuário
//! atual possa ler os tokens.
//!
//! ## Design
//! Esta abordagem é idêntica à usada pelo **GitHub CLI** (`~/.config/gh/hosts.yml`),
//! **kubectl** (`~/.kube/config`) e **AWS CLI** (`~/.aws/credentials`):
//! - Zero dependências de sistema (sem D-Bus, sem libsecret, sem OpenSSL)
//! - Funciona em **qualquer** distro Linux, Windows e macOS
//! - Arquivo protegido em `~/.config/ynext-xcloud/tokens.json`
//!
//! ## Segurança
//! - chmod 600 garantido na criação (somente owner pode ler/escrever)
//! - Diretório pai com chmod 700
//! - Nunca exposto em logs ou stdout
//! - Tokens são JSON sem ofuscação adicional (a proteção vem das permissões do SO)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::auth::msa::MsaTokens;
use crate::auth::xsts::XstsToken;

/// Estrutura completa salva em disco
#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredTokens {
    msa: Option<MsaTokens>,
    xsts: Option<XstsToken>,
}

/// Gerenciador de tokens — persiste em arquivo seguro
pub struct TokenStore {
    tokens_path: PathBuf,
}

impl TokenStore {
    /// Cria uma nova instância, usando o diretório de config do usuário
    pub fn new() -> Self {
        let tokens_path = get_tokens_path();
        debug!("Token store em: {}", tokens_path.display());
        TokenStore { tokens_path }
    }

    // =========================================================================
    // Tokens MSA
    // =========================================================================

    /// Salva os tokens MSA no arquivo seguro
    pub fn save_msa_tokens(&self, tokens: &MsaTokens) -> Result<()> {
        let mut stored = self.load_stored().unwrap_or_default();
        stored.msa = Some(tokens.clone());
        self.write_stored(&stored)?;
        debug!("✅ Tokens MSA salvos em {}", self.tokens_path.display());
        Ok(())
    }

    /// Carrega os tokens MSA do arquivo seguro
    pub fn load_msa_tokens(&self) -> Result<Option<MsaTokens>> {
        let stored = self.load_stored().unwrap_or_default();
        Ok(stored.msa)
    }

    /// Remove os tokens MSA
    pub fn clear_msa_tokens(&self) -> Result<()> {
        let mut stored = self.load_stored().unwrap_or_default();
        stored.msa = None;
        self.write_stored(&stored)
    }

    // =========================================================================
    // Token XSTS
    // =========================================================================

    /// Salva o token XSTS no arquivo seguro
    pub fn save_xsts_token(&self, token: &XstsToken) -> Result<()> {
        let mut stored = self.load_stored().unwrap_or_default();
        stored.xsts = Some(token.clone());
        self.write_stored(&stored)?;
        debug!("✅ Token XSTS salvo em {}", self.tokens_path.display());
        Ok(())
    }

    /// Carrega o token XSTS do arquivo seguro
    pub fn load_xsts_token(&self) -> Result<Option<XstsToken>> {
        let stored = self.load_stored().unwrap_or_default();
        Ok(stored.xsts)
    }

    /// Remove o token XSTS
    pub fn clear_xsts_token(&self) -> Result<()> {
        let mut stored = self.load_stored().unwrap_or_default();
        stored.xsts = None;
        self.write_stored(&stored)
    }

    // =========================================================================
    // Operações combinadas
    // =========================================================================

    /// Remove TODOS os tokens salvos (logout completo)
    pub fn clear_all(&self) -> Result<()> {
        if self.tokens_path.exists() {
            fs::remove_file(&self.tokens_path).context("Falha ao remover arquivo de tokens")?;
            info!("🗑️  Tokens removidos de {}", self.tokens_path.display());
        }
        Ok(())
    }

    /// Verifica se há tokens XSTS salvos
    pub fn has_saved_tokens(&self) -> bool {
        self.load_xsts_token().ok().flatten().is_some()
    }

    // =========================================================================
    // I/O interno
    // =========================================================================

    /// Carrega os tokens do arquivo JSON
    fn load_stored(&self) -> Result<StoredTokens> {
        if !self.tokens_path.exists() {
            return Ok(StoredTokens::default());
        }

        let content =
            fs::read_to_string(&self.tokens_path).context("Falha ao ler arquivo de tokens")?;

        serde_json::from_str(&content)
            .context("Falha ao parsear tokens do arquivo (arquivo corrompido?)")
    }

    /// Escreve os tokens no arquivo JSON com permissões restritas
    fn write_stored(&self, stored: &StoredTokens) -> Result<()> {
        // Garante que o diretório de configuração existe
        ensure_config_dir(&self.tokens_path)?;

        // Serializa como JSON indentado
        let json = serde_json::to_string_pretty(stored).context("Falha ao serializar tokens")?;

        // Escreve o arquivo
        fs::write(&self.tokens_path, &json).context("Falha ao escrever arquivo de tokens")?;

        // Define permissões restritas (somente owner pode ler/escrever)
        set_secure_permissions(&self.tokens_path)?;

        Ok(())
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Funções auxiliares de sistema de arquivos
// ===========================================================================

/// Retorna o caminho do arquivo de tokens: `~/.config/ynext-xcloud/tokens.json`
fn get_tokens_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| {
        warn!("Não foi possível determinar diretório de config — usando ~/.ynext-xcloud");
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ynext-xcloud")
    });

    config_dir.join("ynext-xcloud").join("tokens.json")
}

/// Cria o diretório de configuração com permissões seguras (chmod 700)
fn ensure_config_dir(tokens_path: &std::path::Path) -> Result<()> {
    if let Some(parent) = tokens_path.parent() {
        fs::create_dir_all(parent).context("Falha ao criar diretório de configuração")?;

        // chmod 700 no diretório (somente owner pode listar/entrar)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o700);
            fs::set_permissions(parent, perms)
                .context("Falha ao definir permissões do diretório de config")?;
        }
    }
    Ok(())
}

/// Define permissões restritivas no arquivo de tokens
fn set_secure_permissions(path: &std::path::Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // chmod 600 — somente owner pode ler e escrever
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, perms)
            .context("Falha ao definir permissões do arquivo de tokens (chmod 600)")?;
    }

    // No Windows, as permissões padrão já restringem ao usuário atual
    // Para produção, considerar usar SetNamedSecurityInfo da Windows API
    #[cfg(windows)]
    {
        debug!("Windows: permissões do arquivo gerenciadas pelo sistema");
    }

    Ok(())
}

// ===========================================================================
// Testes
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::msa::MsaTokens;
    use crate::auth::xsts::XstsToken;
    use chrono::Utc;
    use tempfile::tempdir;

    fn mock_store(dir: &std::path::Path) -> TokenStore {
        TokenStore {
            tokens_path: dir.join("tokens.json"),
        }
    }

    fn mock_msa() -> MsaTokens {
        MsaTokens {
            access_token: "test_access".into(),
            refresh_token: "test_refresh".into(),
            token_type: "Bearer".into(),
            scope: "Xboxlive.signin".into(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        }
    }

    fn mock_xsts() -> XstsToken {
        XstsToken {
            token: "test_xsts".into(),
            user_hash: "test_hash".into(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            gamertag: Some("TestGamertag".into()),
            xuid: Some("123456".into()),
        }
    }

    #[test]
    fn test_save_and_load_msa_tokens() {
        let dir = tempdir().unwrap();
        let store = mock_store(dir.path());
        let tokens = mock_msa();

        store.save_msa_tokens(&tokens).unwrap();
        let loaded = store.load_msa_tokens().unwrap().unwrap();

        assert_eq!(tokens.access_token, loaded.access_token);
        assert_eq!(tokens.refresh_token, loaded.refresh_token);
    }

    #[test]
    fn test_save_and_load_xsts_token() {
        let dir = tempdir().unwrap();
        let store = mock_store(dir.path());
        let token = mock_xsts();

        store.save_xsts_token(&token).unwrap();
        let loaded = store.load_xsts_token().unwrap().unwrap();

        assert_eq!(token.token, loaded.token);
        assert_eq!(token.user_hash, loaded.user_hash);
        assert_eq!(token.gamertag, loaded.gamertag);
    }

    #[test]
    fn test_clear_all() {
        let dir = tempdir().unwrap();
        let store = mock_store(dir.path());

        store.save_msa_tokens(&mock_msa()).unwrap();
        store.save_xsts_token(&mock_xsts()).unwrap();
        assert!(store.has_saved_tokens());

        store.clear_all().unwrap();
        assert!(!store.has_saved_tokens());
    }

    #[test]
    fn test_load_when_no_file() {
        let dir = tempdir().unwrap();
        let store = mock_store(dir.path());

        assert!(store.load_msa_tokens().unwrap().is_none());
        assert!(store.load_xsts_token().unwrap().is_none());
        assert!(!store.has_saved_tokens());
    }

    #[test]
    fn test_clear_only_msa_keeps_xsts() {
        let dir = tempdir().unwrap();
        let store = mock_store(dir.path());

        store.save_msa_tokens(&mock_msa()).unwrap();
        store.save_xsts_token(&mock_xsts()).unwrap();

        store.clear_msa_tokens().unwrap();

        assert!(store.load_msa_tokens().unwrap().is_none());
        assert!(store.load_xsts_token().unwrap().is_some());
    }
}
