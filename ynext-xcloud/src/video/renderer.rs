//! # Renderer — Seleção de Sink de Vídeo por Plataforma
//!
//! Detecta em tempo de compilação e runtime qual sink de vídeo usar:
//!
//! - **Linux**: `glimagesink` (OpenGL — Zero-Copy de buffer de vídeo via EGL/DRM)
//! - **Windows**: `d3d11videosink` (DirectX 11 — integrado nativamente)
//! - **Fallback (dev)**: `autovideosink` (deixa o GStreamer escolher automaticamente)
//!
//! A futura integração com a UI `egui` (Fase 6) irá usar `GstVideoOverlay`
//! para injetar o handle de janela nativa no sink, embutindo o vídeo
//! diretamente dentro da interface gráfica.

use anyhow::{bail, Context, Result};
use tracing::{info, warn};

/// Cria e retorna o elemento de sink de vídeo adequado para a plataforma atual.
///
/// Esta função tenta os sinks na seguinte ordem de preferência:
/// 1. `glimagesink` (Linux — OpenGL Zero-Copy)
/// 2. `d3d11videosink` (Windows — DirectX 11)
/// 3. `waylandsink` (Linux com Wayland puro, sem GL)
/// 4. `autovideosink` (fallback automático do GStreamer)
pub fn select_video_sink() -> Result<gstreamer::Element> {
    // Tenta OpenGL sink (Linux — integra com VA-API via EGL DMA-BUF = verdadeiro Zero-Copy)
    if let Ok(el) = try_create_sink("glimagesink") {
        info!("🖥️  Sink de vídeo: glimagesink (OpenGL — Zero-Copy via EGL DMA-BUF)");
        return Ok(el);
    }

    // Tenta D3D11 sink (Windows — integração direta com decodificador D3D11)
    if let Ok(el) = try_create_sink("d3d11videosink") {
        info!("🖥️  Sink de vídeo: d3d11videosink (DirectX 11)");
        return Ok(el);
    }

    // Tenta Wayland sink (Linux sem suporte a GL, mas com Wayland)
    if let Ok(el) = try_create_sink("waylandsink") {
        info!("🖥️  Sink de vídeo: waylandsink (Wayland nativo)");
        return Ok(el);
    }

    // Tenta XVideo sink (X11 legado — menos eficiente mas amplamente suportado)
    if let Ok(el) = try_create_sink("xvimagesink") {
        warn!("⚠️  Sink de vídeo: xvimagesink (X11/XV — não é Zero-Copy, impacto de performance)");
        return Ok(el);
    }

    // Fallback automático (deixa o GStreamer escolher — desenvolvimento/CI)
    #[cfg(debug_assertions)]
    if let Ok(el) = try_create_sink("autovideosink") {
        warn!("⚠️  Sink de vídeo: autovideosink (fallback automático — apenas para debug)");
        return Ok(el);
    }

    bail!(
        "❌ Nenhum sink de vídeo compatível encontrado!\n\
         Linux:   sudo apt install gstreamer1.0-gl gstreamer1.0-x\n\
         Windows: Instale o GStreamer runtime (https://gstreamer.freedesktop.org/)"
    )
}

/// Tenta criar um elemento GStreamer pelo nome, retornando Err se não disponível.
fn try_create_sink(name: &str) -> Result<gstreamer::Element> {
    gstreamer::ElementFactory::make(name)
        .build()
        .with_context(|| format!("Elemento GStreamer '{}' não encontrado no sistema", name))
}

// ===========================================================================
// Configuração de janela (preparação para integração com egui na Fase 6)
// ===========================================================================

/// Configuração da janela de vídeo
#[derive(Debug, Clone)]
pub struct VideoWindowConfig {
    /// Largura da janela em pixels
    pub width: u32,
    /// Altura da janela em pixels
    pub height: u32,
    /// Modo tela cheia
    pub fullscreen: bool,
    /// Título da janela
    pub title: String,
}

impl Default for VideoWindowConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fullscreen: false,
            title: "Ynext Xcloud — Streaming".to_string(),
        }
    }
}

impl VideoWindowConfig {
    /// Cria uma configuração para 720p (hardware limitado)
    pub fn for_limited_hardware() -> Self {
        Self {
            width: 1280,
            height: 720,
            fullscreen: false,
            title: "Ynext Xcloud — Streaming (720p)".to_string(),
        }
    }

    /// Cria uma configuração para 1080p fullscreen (hardware capaz)
    pub fn fullscreen_1080p() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fullscreen: true,
            title: "Ynext Xcloud".to_string(),
        }
    }
}

// ===========================================================================
// Testes
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_video_sink() {
        gstreamer::init().unwrap();
        // Pelo menos um sink de vídeo deve estar disponível
        let result = select_video_sink();
        assert!(
            result.is_ok(),
            "Deve existir pelo menos um sink de vídeo disponível: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_window_config_defaults() {
        let config = VideoWindowConfig::default();
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert!(!config.fullscreen);
    }

    #[test]
    fn test_window_config_limited_hw() {
        let config = VideoWindowConfig::for_limited_hardware();
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
    }
}
