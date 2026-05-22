pub mod protocol;

use anyhow::Result;
use gilrs::{Event, Gilrs};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info};

pub use protocol::InputReport;

/// Orquestrador do sistema de input (Fase 4)
pub struct InputManager {
    gilrs: Gilrs,
    input_tx: mpsc::Sender<InputReport>,
}

impl InputManager {
    /// Inicializa o gerenciador de gamepads
    pub fn new(input_tx: mpsc::Sender<InputReport>) -> Result<Self> {
        let gilrs = Gilrs::new()
            .map_err(|e| anyhow::anyhow!("Falha ao inicializar gilrs (gamepad): {}", e))?;

        Ok(Self { gilrs, input_tx })
    }

    /// Inicia o loop de polling de input em uma thread separada (Zero-Lag)
    pub fn start(mut self, mut shutdown_rx: tokio::sync::oneshot::Receiver<()>) {
        tokio::task::spawn_blocking(move || {
            info!("🎮 Loop de captura de Input (Gamepad) iniciado.");

            // Frequência de polling: ~1000Hz (1ms) para simular zero-lag
            let poll_interval = Duration::from_millis(1);

            loop {
                // Verifica se deve desligar
                if shutdown_rx.try_recv().is_ok() {
                    info!("🛑 Loop de Input encerrado.");
                    break;
                }

                // Processa todos os eventos pendentes no gilrs
                while let Some(Event {
                    id,
                    event: _,
                    time: _,
                }) = self.gilrs.next_event()
                {
                    // Ignoramos o detalhe do evento e apenas enviamos o state atual inteiro
                    // do controle para o pipeline GStreamer.
                    if let Some(gamepad) = self.gilrs.connected_gamepad(id) {
                        let index: usize = id.into();
                        let report = InputReport::from_gamepad(&gamepad, index as u8);

                        // Envia para o canal MPSC (non-blocking)
                        if let Err(e) = self.input_tx.try_send(report) {
                            error!("Falha ao enviar InputReport: {}", e);
                        }
                    }
                }

                // Pequena pausa para não travar a CPU em 100%
                std::thread::sleep(poll_interval);
            }
        });
    }
}
