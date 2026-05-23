//! # Pipeline de Áudio — Fase 5
//!
//! Monta a cadeia de decodificação de áudio Opus quando o `webrtcbin`
//! emite um novo pad de áudio via sinal `pad-added`.
//!
//! ## Pipeline
//!
//! ```text
//! webrtcbin (pad de áudio: application/x-rtp, encoding-name=OPUS)
//!     └─▶ rtpopusdepay     (Desempacota RTP → Opus raw)
//!               └─▶ opusdec        (Opus → PCM 48kHz estéreo)
//!                       └─▶ audioconvert   (Ajusta formato de sample)
//!                               └─▶ audioresample  (Reamostra para taxa nativa)
//!                                       └─▶ [pipewiresink|alsasink|wasapisink]
//! ```
//!
//! ## Regras
//!
//! - Codec de áudio: **Opus obrigatório**. Nenhum outro codec será aceito.
//! - O sink é selecionado em runtime pela função `select_audio_sink()`.
//! - Em CI / headless: `fakesink` é usado silenciosamente.

use anyhow::{Context, Result};
use gstreamer::prelude::*;
use tracing::info;

use crate::audio::select_audio_sink;

/// Conecta um pad de áudio do `webrtcbin` ao pipeline de decode Opus.
///
/// Esta função é chamada pelo sinal `pad-added` do `webrtcbin` quando
/// um pad de áudio (caps `audio/`) é detectado.
///
/// # Argumentos
///
/// * `pipeline` — O pipeline GStreamer principal
/// * `src_pad`  — O pad de áudio gerado pelo webrtcbin
pub fn connect_audio_pad(pipeline: &gstreamer::Pipeline, src_pad: &gstreamer::Pad) -> Result<()> {
    info!("🎵 Stream de áudio Opus detectado — montando cadeia de decode...");

    // Desempacotador RTP → Opus raw
    let depay = gstreamer::ElementFactory::make("rtpopusdepay")
        .build()
        .context("Falha ao criar 'rtpopusdepay'. Instale gstreamer1.0-plugins-good.")?;

    // Decoder Opus → PCM 48kHz estéreo
    let decode = gstreamer::ElementFactory::make("opusdec")
        .build()
        .context("Falha ao criar 'opusdec'. Instale gstreamer1.0-plugins-base.")?;

    // Conversão de formato de sample (garante compatibilidade com o sink)
    let convert = gstreamer::ElementFactory::make("audioconvert")
        .build()
        .context("Falha ao criar 'audioconvert'. Instale gstreamer1.0-plugins-base.")?;

    // Reamostragem para a taxa nativa do dispositivo de saída
    let resample = gstreamer::ElementFactory::make("audioresample")
        .build()
        .context("Falha ao criar 'audioresample'. Instale gstreamer1.0-plugins-base.")?;

    // Sink de áudio (PipeWire/ALSA/WASAPI/fake — seleção automática)
    let sink = select_audio_sink().context("Falha ao criar sink de áudio")?;

    // Adiciona todos os elementos ao pipeline
    pipeline
        .add_many([&depay, &decode, &convert, &resample, &sink])
        .context("Falha ao adicionar elementos de áudio ao pipeline")?;

    // Liga os elementos em sequência: depay → decode → convert → resample → sink
    gstreamer::Element::link_many([&depay, &decode, &convert, &resample, &sink])
        .context("Falha ao ligar cadeia de áudio Opus")?;

    // Sincroniza os novos elementos com o estado atual do pipeline
    for element in [&depay, &decode, &convert, &resample, &sink] {
        element
            .sync_state_with_parent()
            .context("Falha ao sincronizar estado do elemento de áudio")?;
    }

    // Conecta o pad do webrtcbin ao primeiro elemento da cadeia (depay)
    let depay_sink = depay
        .static_pad("sink")
        .context("rtpopusdepay sem pad 'sink'")?;

    src_pad
        .link(&depay_sink)
        .context("Falha ao linkar pad de áudio webrtcbin → rtpopusdepay")?;

    info!("✅ Cadeia de áudio Opus conectada: webrtcbin → rtpopusdepay → opusdec → audioconvert → audioresample → sink");

    Ok(())
}

// ===========================================================================
// Testes
// ===========================================================================

#[cfg(test)]
mod tests {


    #[test]
    fn test_select_audio_sink() {
        gstreamer::init().expect("GStreamer deve inicializar");
        // Em qualquer ambiente (CI ou produção), deve retornar pelo menos fakesink
        let result = crate::audio::select_audio_sink();
        assert!(
            result.is_ok(),
            "Deve existir pelo menos um sink de áudio disponível (fakesink): {:?}",
            result.err()
        );
    }

    #[test]
    #[ignore]
    fn test_opus_chain_elements_exist() {
        gstreamer::init().expect("GStreamer deve inicializar");

        let elements = ["rtpopusdepay", "opusdec", "audioconvert", "audioresample"];
        for element_name in &elements {
            let result = gstreamer::ElementFactory::make(element_name).build();
            assert!(
                result.is_ok(),
                "Elemento '{}' deve estar disponível (instale gstreamer1.0-plugins-base)",
                element_name
            );
        }
    }
}
