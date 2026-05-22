#![allow(dead_code)]

//! # Ynext-Xcloud — Entrypoint Principal
//!
//! Cliente nativo open source para Xbox Cloud Gaming.
//! Construído com Rust para altíssimo desempenho e mínimo consumo de recursos.

mod auth;
mod signaling;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};

use crate::auth::token_store::TokenStore;

// ===========================================================================
// CLI — Interface de Linha de Comando
// ===========================================================================

/// Ynext-Xcloud: Cliente nativo Xbox Cloud Gaming
#[derive(Parser)]
#[command(
    name = "ynext-xcloud",
    version = env!("CARGO_PKG_VERSION"),
    about = "Cliente nativo open source para Xbox Cloud Gaming",
    long_about = "
╔══════════════════════════════════════════════════════════╗
║              🎮 YNEXT-XCLOUD v{}                       ║
║    Cliente Nativo Xbox Cloud Gaming — Ynext Automação    ║
╚══════════════════════════════════════════════════════════╝

Streaming de altíssimo desempenho com aceleração de hardware.
Zero dependência de navegador. Zero input lag.",
    author = "Ynext - Tecnologia e Automação"
)]
struct Cli {
    /// Nível de log (error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info", global = true)]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Gerenciar autenticação Microsoft/Xbox
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Iniciar uma sessão de streaming (em desenvolvimento)
    Stream {
        /// ID do jogo ou nome (ex: "Halo Infinite")
        #[arg(short, long)]
        game: Option<String>,
    },
    /// Exibir informações da conta autenticada
    Info,
}

#[derive(Subcommand)]
enum AuthAction {
    /// Fazer login na conta Microsoft/Xbox
    Login,
    /// Fazer logout e remover tokens salvos
    Logout,
    /// Verificar status da autenticação atual
    Status,
}

// ===========================================================================
// Main
// ===========================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Configura o sistema de logging estruturado
    setup_logging(&cli.log_level);

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "🎮 Ynext-Xcloud iniciando..."
    );

    // Executa o comando solicitado
    match cli.command {
        Commands::Auth { action } => handle_auth_command(action).await?,
        Commands::Stream { game } => handle_stream_command(game).await?,
        Commands::Info => handle_info_command().await?,
    }

    Ok(())
}

// ===========================================================================
// Handlers de comandos
// ===========================================================================

/// Handler para comandos de autenticação
async fn handle_auth_command(action: AuthAction) -> Result<()> {
    let mut store = TokenStore::new();

    match action {
        AuthAction::Login => {
            println!();
            println!("🔑 Iniciando autenticação no Xbox Cloud Gaming...");
            println!();

            match auth::authenticate(&mut store).await {
                Ok(auth_header) => {
                    println!();
                    println!("╔══════════════════════════════════════════════════════════╗");
                    println!("║          ✅ AUTENTICAÇÃO CONCLUÍDA COM SUCESSO!           ║");
                    println!("╠══════════════════════════════════════════════════════════╣");
                    println!("║  Seus tokens foram salvos com segurança no keyring       ║");
                    println!("║  do sistema operacional.                                  ║");
                    println!("║                                                          ║");
                    println!("║  Execute 'ynext-xcloud info' para ver sua conta.         ║");
                    println!("╚══════════════════════════════════════════════════════════╝");
                    println!();

                    // Em modo debug, exibe o header (apenas para desenvolvimento)
                    if std::env::var("XCLOUD_DEBUG_TOKENS").is_ok() {
                        println!(
                            "🔐 Auth Header (DEBUG): {}",
                            &auth_header[..50.min(auth_header.len())]
                        );
                    }
                }
                Err(e) => {
                    eprintln!();
                    eprintln!("❌ Falha na autenticação: {}", e);
                    eprintln!();
                    eprintln!("💡 Dicas:");
                    eprintln!("   • Verifique sua conexão com a internet");
                    eprintln!("   • Certifique-se que sua conta Microsoft tem Game Pass Ultimate");
                    eprintln!("   • Acesse https://xbox.com para verificar o status da conta");
                    std::process::exit(1);
                }
            }
        }

        AuthAction::Logout => {
            println!("🚪 Fazendo logout...");
            store.clear_all()?;
            println!("✅ Logout realizado. Todos os tokens foram removidos.");
        }

        AuthAction::Status => {
            if store.has_saved_tokens() {
                if let Some(xsts) = store.load_xsts_token()? {
                    if xsts.is_expired() {
                        println!("⚠️  Tokens expirados — Execute 'ynext-xcloud auth login' para renovar.");
                    } else {
                        println!("✅ Autenticado e tokens válidos");
                        if let Some(gamertag) = &xsts.gamertag {
                            println!("🎮 Gamertag: {}", gamertag);
                        }
                        if let Some(xuid) = &xsts.xuid {
                            println!("🆔 XUID: {}", xuid);
                        }
                        println!(
                            "⏰ Token expira em: {}",
                            xsts.expires_at.format("%d/%m/%Y %H:%M UTC")
                        );
                    }
                }
            } else {
                println!("❌ Não autenticado. Execute 'ynext-xcloud auth login'.");
            }
        }
    }

    Ok(())
}

/// Handler para comando de streaming (Fase 2 - Sinalização)
async fn handle_stream_command(game: Option<String>) -> Result<()> {
    let mut store = TokenStore::new();

    println!();
    println!("🎮 Iniciando Ynext-Xcloud Streaming...");

    if let Some(game_name) = game {
        println!("   Jogo selecionado: '{}'", game_name);
    }
    println!();

    // 1. Garantir que o usuário está autenticado e com token XSTS válido
    let auth_header = match auth::authenticate(&mut store).await {
        Ok(header) => header,
        Err(e) => {
            eprintln!("❌ Erro de autenticação: {}", e);
            eprintln!("💡 Execute 'ynext-xcloud auth login' para se autenticar.");
            std::process::exit(1);
        }
    };

    println!("✅ Autenticação confirmada (Token XBL3.0)");

    // 2. Mock do SDP Offer e ICE Candidates (serão substituídos pelo GStreamer na Fase 3)
    // Usamos um SDP H.264 básico para testes de handshake com a Microsoft.
    let mock_sdp_offer = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=video 9 UDP/TLS/RTP/SAVPF 96\r\na=rtpmap:96 H264/90000\r\n";
    let mock_local_ice = vec![]; // Sem candidatos ICE locais por enquanto

    // 3. Iniciar fluxo de sinalização WebRTC (SDP/ICE)
    match crate::signaling::establish_session(&auth_header, mock_sdp_offer, mock_local_ice).await {
        Ok(session) => {
            println!();
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║      🌐 SESSÃO WEBRTC ESTABELECIDA COM SUCESSO!          ║");
            println!("╠══════════════════════════════════════════════════════════╣");
            println!("║  Session ID: {:<43} ║", &session.session_id);
            println!(
                "║  Tamanho SDP Answer: {:<35} ║",
                format!("{} bytes", session.sdp_answer.len())
            );
            println!(
                "║  ICE Remotos Recebidos: {:<32} ║",
                session.ice_candidates.len()
            );
            println!("╚══════════════════════════════════════════════════════════╝");
            println!();
            println!("⚠️  Fase 3 (GStreamer Pipeline) não iniciada.");
            println!("   Streaming de vídeo será integrado no próximo passo.");
        }
        Err(e) => {
            eprintln!("❌ Falha na sinalização WebRTC com o xCloud: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handler para exibir informações da conta
async fn handle_info_command() -> Result<()> {
    let store = TokenStore::new();

    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              🎮 YNEXT-XCLOUD — INFORMAÇÕES               ║");
    println!("╠══════════════════════════════════════════════════════════╣");

    if let Some(xsts) = store.load_xsts_token()? {
        println!("║  Status: ✅ Autenticado                                  ║");
        if let Some(gamertag) = &xsts.gamertag {
            println!("║  Gamertag: {:<47}║", gamertag);
        }
        if let Some(xuid) = &xsts.xuid {
            println!("║  XUID: {:<51}║", xuid);
        }
        println!(
            "║  User Hash: {:<46}║",
            &xsts.user_hash[..20.min(xsts.user_hash.len())]
        );

        let status = if xsts.is_expired() {
            "⚠️  Expirado"
        } else {
            "✅ Válido"
        };
        println!("║  Token XSTS: {:<45}║", status);
        println!(
            "║  Expira em: {:<46}║",
            xsts.expires_at.format("%d/%m/%Y %H:%M UTC")
        );
    } else {
        println!("║  Status: ❌ Não autenticado                              ║");
        println!("║  Execute: ynext-xcloud auth login                        ║");
    }

    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  Versão: {:<49}║", env!("CARGO_PKG_VERSION"));
    println!("║  Sistema: {:<48}║", std::env::consts::OS);
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    Ok(())
}

// ===========================================================================
// Configuração de logging
// ===========================================================================

fn setup_logging(level: &str) {
    let log_level = match level.to_lowercase().as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .compact()
        .init();
}
