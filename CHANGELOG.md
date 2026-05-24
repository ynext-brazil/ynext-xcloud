# Changelog

Todos os recursos notáveis para este projeto serão documentados neste arquivo.

O formato é baseado no [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [Unreleased]

### Adicionado
- **Fase 4 (Interface Gráfica Nativa em egui):** Reescrita profunda do layout para replicar a UI original do Xbox Cloud Gaming.
  - Implementação do motor dinâmico `CardStyle` suportando as proporções `Tall` (Pôster), `Wide` (Banner/Hero) e `Square` (Quadrada).
  - Algoritmo matemático de Recorte UV (semelhante ao `object-fit: cover`) para encaixe de texturas sem distorção.
  - Grade responsiva infinita ("Todos os jogos") com suporte a wrap automático e rolagem fluida.
  - Gerenciador de download de capas asíncrono via `reqwest` com sistema de cache thread-safe (`Mutex<HashMap>`) para evitar Memory Leaks ou Rate Limits.
  - Mapeamento avançado do catálogo `SIGL_ALL` suportando renderização de +2.800 títulos simultâneos, preparando suporte ao "Transmita o Seu".
- **Fase 3 (Pipeline de Vídeo H.264 Zero-Copy):** Integração completa do pipeline GStreamer.
  - `src/video/mod.rs`: Orquestrador do pipeline — inicializa GStreamer e gerencia o ciclo de vida.
  - `src/video/pipeline.rs`: Montagem do pipeline `webrtcbin → rtph264depay → h264parse → vaapih264dec/d3d11h264dec → glimagesink`.
  - `src/video/renderer.rs`: Seleção de sink de vídeo por plataforma com detecção automática em runtime.
  - `src/video/channels.rs`: Canais Tokio MPSC para comunicação zero-cópia entre Fase 2 e Fase 3.
  - Suporte a aceleração de hardware: VA-API (Linux Intel/AMD), D3D11 (Windows), NVDEC (NVIDIA).
  - Fallback para software decoder `avdec_h264` apenas em modo debug.
- **Fase 2 (Sinalização WebRTC):** SDP Offer agora gerado pelo `webrtcbin` real (removido o mock estático).
  - `main.rs` atualizado: `webrtcbin` gera SDP Offer real via sinal `on-negotiation-needed`.
  - Integração completa Fase 2 → Fase 3 via canais MPSC.
- **CI (GitHub Actions):** Atualizado para instalar pacotes GStreamer `-dev` no runner Ubuntu.
- **Fase 2 (Sinalização WebRTC):** Esqueleto completo do módulo de sinalização com a API xCloud.
- **Fase 1 (Autenticação):** Módulo completo de autenticação Microsoft.

### Alterado
- `Cargo.toml`: Adicionadas dependências `gstreamer`, `gstreamer-webrtc`, `gstreamer-sdp`, `gstreamer-video`.
- `Cargo.toml`: Feature flag `gl-sink` para habilitar `gstreamer-gl` em Linux/macOS.
- `.github/workflows/ci.yml`: Substitui `libsecret-1-dev` e `libdbus-1-dev` pelas libs GStreamer.
- Nome da empresa atualizado de "Agência de Automação" para "Tecnologia e Automação" em todos os arquivos.
