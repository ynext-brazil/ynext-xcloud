//! # Módulo de Áudio — Fase 5
//!
//! Seleção do sink de áudio correto conforme a plataforma e os elementos
//! disponíveis em runtime no sistema.
//!
//! ## Prioridade de Sinks (Linux)
//!
//! 1. `pipewiresink`   — PipeWire (Ubuntu 22.04+, Fedora 35+)
//! 2. `pulsesink`      — PulseAudio (sistemas legados)
//! 3. `alsasink`       — ALSA direto (fallback mínimo)
//! 4. `fakesink`       — CI / headless (silencioso, não quebra o build)
//!
//! ## Prioridade de Sinks (Windows)
//!
//! 1. `wasapisink`     — Windows Audio Session API (baixa latência)
//! 2. `directsoundsink`— DirectSound (fallback legacy)
//! 3. `fakesink`       — CI headless

pub mod pipeline;

use anyhow::{Context, Result};
use tracing::{info, warn};

/// Seleciona o sink de áudio mais adequado para a plataforma atual.
///
/// A seleção segue uma ordem de prioridade que privilegia
/// servidores de som modernos de baixa latência.
pub fn select_audio_sink() -> Result<gstreamer::Element> {
    // Sinks na ordem de preferência para cada plataforma
    #[cfg(target_os = "linux")]
    let candidates = &[
        ("pipewiresink", "PipeWire (baixa latência)"),
        ("pulsesink", "PulseAudio"),
        ("alsasink", "ALSA direto"),
        ("fakesink", "Fake (CI/headless)"),
    ];

    #[cfg(target_os = "windows")]
    let candidates = &[
        ("wasapisink", "WASAPI (baixa latência)"),
        ("directsoundsink", "DirectSound"),
        ("fakesink", "Fake (CI/headless)"),
    ];

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let candidates = &[("fakesink", "Fake (plataforma sem suporte de áudio nativo)")];

    for (element_name, description) in candidates.iter() {
        if let Ok(sink) = gstreamer::ElementFactory::make(element_name).build() {
            info!(
                "🔊 Sink de áudio selecionado: {} ({})",
                element_name, description
            );
            return Ok(sink);
        }
    }

    // Fallback universal — não deve chegar aqui pois fakesink sempre existe
    warn!("⚠️  Nenhum sink de áudio encontrado, usando autoaudiosink como emergência");
    gstreamer::ElementFactory::make("autoaudiosink")
        .build()
        .context("Nenhum sink de áudio disponível no sistema")
}
