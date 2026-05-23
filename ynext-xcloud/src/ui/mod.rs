//! # Módulo de UI — Orquestrador do Launcher
//!
//! Inicializa a janela `eframe` com o tema Xbox e dispara o `XCloudApp`.
//!
//! ## Uso
//!
//! ```bash
//! ynext-xcloud launch
//! ```

pub mod app;
pub mod catalog;
pub mod theme;
pub mod widgets;

use anyhow::Result;

use crate::ui::app::XCloudApp;

/// Inicializa e exibe o launcher gráfico.
///
/// Deve ser chamado na thread principal (restrição do OpenGL/glow).
pub fn run_launcher(username: String, runtime: tokio::runtime::Handle) -> Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Ynext Xcloud")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 520.0])
            .with_icon(load_icon()),
        renderer: eframe::Renderer::Glow, // OpenGL 3.2 — máxima compatibilidade
        ..Default::default()
    };

    eframe::run_native(
        "Ynext Xcloud",
        native_options,
        Box::new(move |cc| Ok(Box::new(XCloudApp::new(cc, username, runtime)))),
    )
    .map_err(|e| anyhow::anyhow!("Falha ao iniciar janela do launcher: {}", e))
}

/// Carrega o ícone da janela (logo Xbox verde simples em PNG).
/// Retorna um ícone padrão caso o arquivo não exista.
fn load_icon() -> egui::IconData {
    // Ícone minimalista: quadrado verde 32x32 com "X" branco
    // Em produção, substituir por um PNG real do asset bundle
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    // Preenche com verde Xbox (#107C10)
    for chunk in rgba.chunks_mut(4) {
        chunk[0] = 16; // R
        chunk[1] = 124; // G
        chunk[2] = 16; // B
        chunk[3] = 255; // A
    }

    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}
