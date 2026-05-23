#![allow(dead_code)]

//! # Ynext-Xcloud — Entrypoint Principal
//!
//! Cliente nativo open source para Xbox Cloud Gaming.
//! Construído com Rust para altíssimo desempenho e mínimo consumo de recursos.

#[cfg(feature = "streaming")]
mod audio;
mod auth;
mod input;
mod signaling;
#[cfg(feature = "ui")]
mod ui;
#[cfg(feature = "streaming")]
mod video;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};

use crate::auth::token_store::TokenStore;

#[cfg(feature = "streaming")]
use gstreamer::prelude::*;

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
    /// Abrir o launcher gráfico (requer feature `ui`)
    Launch,
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
        Commands::Launch => handle_launch_command().await?,
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

/// Handler para comando de streaming (Fase 3 — Pipeline GStreamer)
async fn handle_stream_command(game: Option<String>) -> Result<()> {
    let mut store = TokenStore::new();

    println!();
    println!("🎮 Iniciando Ynext-Xcloud Streaming...");

    if let Some(ref game_name) = game {
        println!("   Jogo selecionado: '{}'", game_name);
    }
    println!();

    // 1. Verificar autenticação e obter token XBL3.0
    #[allow(unused_variables)]
    let auth_header = match auth::authenticate(&mut store).await {
        Ok(header) => header,
        Err(e) => {
            eprintln!("❌ Erro de autenticação: {}", e);
            eprintln!("💡 Execute 'ynext-xcloud auth login' para se autenticar.");
            std::process::exit(1);
        }
    };

    println!("✅ Autenticação confirmada (Token XBL3.0)");

    // 2-7. Pipeline GStreamer (apenas disponível com --features streaming)
    #[cfg(feature = "streaming")]
    {
        println!("⏳ Iniciando pipeline GStreamer e conectando ao xCloud...");
        println!();

        // Inicializa o GStreamer para gerar o SDP Offer real via webrtcbin
        gstreamer::init().map_err(|e| anyhow::anyhow!("Falha ao inicializar GStreamer: {}", e))?;

        // Cria o elemento webrtcbin para gerar o SDP Offer real
        let webrtcbin = gstreamer::ElementFactory::make("webrtcbin")
            .name("sdp_generator")
            .property_from_str("bundle-policy", "max-bundle")
            .build()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Falha ao criar webrtcbin. Instale gstreamer1.0-plugins-bad: {}",
                    e
                )
            })?;

        // Canal one-shot para capturar o SDP Offer gerado pelo webrtcbin
        let (sdp_tx, sdp_rx) = tokio::sync::oneshot::channel::<String>();
        let sdp_tx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(sdp_tx)));

        let sdp_tx_clone = sdp_tx.clone();
        webrtcbin.connect("on-negotiation-needed", false, move |args| {
            let webrtc = &args[0].get::<gstreamer::Element>().unwrap();
            let promise = gstreamer::Promise::with_change_func({
                let sdp_tx = sdp_tx_clone.clone();
                let webrtc = webrtc.clone();
                move |reply| {
                    if let Ok(Some(s)) = reply {
                        if let Ok(offer) =
                            s.get::<gstreamer_webrtc::WebRTCSessionDescription>("offer")
                        {
                            let sdp_text = offer.sdp().as_text().unwrap_or_default();
                            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                                let sdp_tx = sdp_tx.clone();
                                rt.spawn(async move {
                                    let mut guard = sdp_tx.lock().await;
                                    if let Some(tx) = guard.take() {
                                        let _ = tx.send(sdp_text);
                                    }
                                });
                            }
                            webrtc.emit_by_name::<()>(
                                "set-local-description",
                                &[&offer, &None::<gstreamer::Promise>],
                            );
                        }
                    }
                }
            });
            webrtc.emit_by_name::<()>("create-offer", &[&None::<gstreamer::Structure>, &promise]);
            None
        });

        let tmp_pipeline = gstreamer::Pipeline::new();
        tmp_pipeline.add(&webrtcbin).ok();

        // 4.1. Solicita a criação do DataChannel "input" para que conste no SDP Offer
        webrtcbin.emit_by_name::<Option<gstreamer::Object>>(
            "create-data-channel",
            &[&"input", &None::<gstreamer::Structure>],
        );

        tmp_pipeline.set_state(gstreamer::State::Playing).ok();

        let sdp_offer = match tokio::time::timeout(std::time::Duration::from_secs(10), sdp_rx).await
        {
            Ok(Ok(sdp)) => {
                println!("✅ SDP Offer gerado pelo webrtcbin ({} bytes)", sdp.len());
                sdp
            }
            _ => {
                tracing::warn!("⚠️  Timeout ao gerar SDP — usando SDP mínimo de fallback");
                "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\n\
                 m=video 9 UDP/TLS/RTP/SAVPF 96\r\na=rtpmap:96 H264/90000\r\n"
                    .to_string()
            }
        };

        tmp_pipeline.set_state(gstreamer::State::Null).ok();

        let session =
            match crate::signaling::establish_session(&auth_header, &sdp_offer, vec![]).await {
                Ok(s) => {
                    println!();
                    println!("╔══════════════════════════════════════════════════════════╗");
                    println!("║      🌐 SESSÃO WEBRTC ESTABELECIDA COM SUCESSO!          ║");
                    println!("╠══════════════════════════════════════════════════════════╣");
                    println!("║  Session ID: {:<43} ║", &s.session_id);
                    println!(
                        "║  SDP Answer: {:<43} ║",
                        format!("{} bytes", s.sdp_answer.len())
                    );
                    println!("║  ICE Remotos: {:<42} ║", s.ice_candidates.len());
                    println!("╚══════════════════════════════════════════════════════════╝");
                    println!();
                    s
                }
                Err(e) => {
                    eprintln!("❌ Falha na sinalização WebRTC: {}", e);
                    std::process::exit(1);
                }
            };

        println!("▶️  Iniciando pipeline de vídeo H.264 com aceleração de hardware...");

        // 7.1. Cria canais de comunicação de Input (Fase 4)
        let (input_tx, input_rx) = tokio::sync::mpsc::channel::<crate::input::InputReport>(16);

        // 7.2. Inicializa e inicia o InputManager
        let (input_shutdown_tx, input_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        match crate::input::InputManager::new(input_tx) {
            Ok(input_manager) => {
                input_manager.start(input_shutdown_rx);
                println!("🎮 Módulo de Input inicializado (Aguardando gamepad...)");
            }
            Err(e) => {
                tracing::warn!("⚠️ Não foi possível inicializar Input: {}", e);
            }
        }

        match video::start_pipeline(session, input_rx).await {
            Ok(handle) => {
                println!("✅ Pipeline em execução! Pressione Ctrl+C para encerrar.");
                tokio::signal::ctrl_c()
                    .await
                    .map_err(|e| anyhow::anyhow!("Falha ao registrar Ctrl+C: {}", e))?;
                println!();
                println!("🛑 Encerrando streaming...");
                let _ = handle.shutdown_tx.send(());
                let _ = input_shutdown_tx.send(()); // Desliga o gilrs também
            }
            Err(e) => {
                eprintln!("❌ Falha ao iniciar pipeline GStreamer: {}", e);
                eprintln!("💡 sudo apt install gstreamer1.0-plugins-bad gstreamer1.0-vaapi");
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(feature = "streaming"))]
    {
        println!("⚠️  Streaming não disponível nesta build.");
        println!("   Compile com: cargo build --features streaming");
        println!("   Requer: libgstreamer1.0-dev libgstreamer-plugins-bad1.0-dev");
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

// ===========================================================================
// Launcher gráfico (Fase 6)
// ===========================================================================

/// Handler do subcomando `launch` — abre o launcher gráfico egui
async fn handle_launch_command() -> Result<()> {
    #[cfg(feature = "ui")]
    {
        use crate::auth::token_store::TokenStore;

        // Tenta obter o nome do usuário do token salvo
        let store = TokenStore::new();
        let username = store
            .load_xsts_token()
            .ok()
            .flatten()
            .map(|t| t.gamertag.unwrap_or_else(|| "Xbox User".to_string()))
            .unwrap_or_else(|| "Xbox User".to_string());

        info!("🖥️  Iniciando launcher gráfico para: {}", username);

        // O egui precisa rodar na thread principal — obtemos o handle do runtime
        let runtime = tokio::runtime::Handle::current();

        // `run_launcher` é bloqueante (loop de eventos da janela)
        crate::ui::run_launcher(username, runtime)?;
    }

    #[cfg(not(feature = "ui"))]
    {
        eprintln!("⚠️  O launcher gráfico não está compilado neste binário.");
        eprintln!("   Recompile com: cargo build --features ui");
    }

    Ok(())
}
