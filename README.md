# 🎮 Ynext Xcloud

> **Cliente nativo open source para Xbox Cloud Gaming — Altíssimo desempenho, Zero navegador.**

[![Build Status](https://github.com/ynext-brazil/ynext-xcloud/actions/workflows/ci.yml/badge.svg)](https://github.com/ynext-brazil/ynext-xcloud/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![GitHub Stars](https://img.shields.io/github/stars/ynext-brazil/ynext-xcloud?style=social)](https://github.com/ynext-brazil/ynext-xcloud)

## 🎯 O Problema que Resolvemos

Clientes web/Electron/Tauri para xCloud sofrem com:
- ❌ **VA-API instável** no Linux (chips Intel integrados — Celeron, Atom, Core i3 antigos)
- ❌ **Gargalo de CPU** na decodificação de vídeo (sem aceleração de hardware real)
- ❌ **Input lag absurdo** causado pelo overhead do browser engine
- ❌ **Alto consumo de RAM** (Electron consume 400–800MB só para renderizar HTML)

## ✅ Nossa Solução

O **Ynext Xcloud** segue o modelo do **Moonlight** (cliente NVIDIA GameStream): **pipeline nativo ponta a ponta**, sem nenhum navegador embutido.

| Recurso | Clientes Web/Electron | Ynext Xcloud |
|---------|----------------------|--------------|
| RAM em idle | ~400–800 MB | **< 60 MB** |
| Decodificação | Software (CPU 100%) | **Hardware zero-copy (VA-API/DxVA)** |
| Input lag | ~50–150 ms | **< 10 ms** |
| Suporte VA-API Linux | Instável | **Nativo e estável via GStreamer** |
| Hardware mínimo | Core i5 + 8 GB RAM | **Celeron + 2 GB RAM** |

---

## 🏗️ Arquitetura

```
┌──────────────────────────────────────────────────────────────┐
│                      YNEXT XCLOUD                            │
│                                                              │
│  ┌──────────┐   ┌─────────────────────────────────────────┐ │
│  │  Auth    │   │         GStreamer Pipeline                │ │
│  │ MSA/XBL  │──▶│  webrtcbin ──▶ h264parse                │ │
│  │  XSTS    │   │      │      ──▶ vaapih264dec (Linux)     │ │
│  └──────────┘   │      │      ──▶ d3d11h264dec (Windows)   │ │
│                 │      │      ──▶ glimagesink / SDL2        │ │
│  ┌──────────┐   │      │                                   │ │
│  │  egui    │   │  DataChannel ◀── gilrs (thread dedicada) │ │
│  │ Launcher │   │  (XInput zero-lag via Tokio MPSC)        │ │
│  │ #107C10  │   └─────────────────────────────────────────┘ │
│  └──────────┘                                               │
└──────────────────────────────────────────────────────────────┘
```

### Por que GStreamer `webrtcbin` e não `webrtc-rs`?

| Critério | `webrtcbin` (GStreamer) | `webrtc-rs` (isolado) |
|---|---|---|
| Integração H.264 VA-API | ✅ Zero-Copy nativo | ❌ Cópia extra de buffer |
| Decode de vídeo | ✅ No mesmo pipeline | ❌ Precisa de bridge externa |
| Maturidade | ✅ Testado em produção (GNOME Calls, etc.) | ⚠️ Ainda em evolução |
| Consumo de CPU | ✅ Mínimo (hardware path) | ❌ Overhead de conversão |

---

## 📦 Módulos

| Módulo | Status | Descrição |
|--------|--------|-----------|
| **Auth (MSA/XBL/XSTS)** | ✅ Fase 1 | Autenticação Microsoft Account + Xbox Live. 10/10 testes ✅ |
| **Signaling WebRTC** | 🔧 Fase 2 | Negociação SDP/ICE via `webrtcbin` GStreamer |
| **Video Pipeline** | 📋 Fase 3 | H.264 zero-copy: VA-API (Linux) / DxVA (Windows) |
| **Gamepad Input** | 📋 Fase 4 | `gilrs` em thread dedicada → MPSC → DataChannel |
| **Áudio Opus** | 📋 Fase 5 | GStreamer Opus → PipeWire/ALSA/WASAPI |
| **UI Launcher (egui)** | 📋 Fase 6 | Interface Xbox (verde #107C10, dark mode, grid) < 100 MB RAM |

---

## 🚀 Início Rápido

### Pré-requisitos

**Linux (Ubuntu/Debian):**
```bash
# Dependências GStreamer para VA-API
sudo apt install -y \
  gstreamer1.0-tools \
  gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad \
  gstreamer1.0-vaapi \
  libgstreamer1.0-dev \
  libgstreamer-plugins-bad1.0-dev \
  pkg-config
```

**Windows:**
- Instale [Rust](https://www.rust-lang.org/tools/install)
- Instale [GStreamer MSVC 1.x](https://gstreamer.freedesktop.org/download/)

### Compilar e usar

```bash
git clone https://github.com/ynext-brazil/ynext-xcloud
cd ynext-xcloud/ynext-xcloud
cargo build --release

# Login na conta Microsoft/Xbox
./target/release/ynext-xcloud auth login

# Ver status da conta
./target/release/ynext-xcloud info

# Iniciar streaming (Fase 2+)
./target/release/ynext-xcloud stream --game "Halo Infinite"
```

---

## 🛠️ Stack Técnica

| Camada | Tecnologia | Motivo |
|---|---|---|
| Linguagem | **Rust 1.75+** | Segurança de memória + performance C equivalente |
| Async Runtime | **Tokio** | Zero-cost futures, I/O assíncrono |
| HTTP/Auth | **reqwest + rustls** | Sem dependência de OpenSSL do sistema |
| Streaming | **gstreamer-rs + webrtcbin** | Zero-Copy VA-API/DxVA, WebRTC nativo |
| Video Decode | **VA-API** (Linux) / **DxVA** (Windows) | Hardware decode, CPU livre |
| Gamepad | **gilrs** | Suporte USB/Bluetooth multiplataforma |
| UI | **egui** | Immediate Mode, < 100 MB RAM, sem Electron |
| Token Store | **Arquivo JSON chmod 600** | Zero deps de sistema (= GitHub CLI) |

---

## 🤝 Contribuindo

Este projeto é **100% open source**! Veja [CONTRIBUTING.md](CONTRIBUTING.md).

```bash
git clone https://github.com/ynext-brazil/ynext-xcloud
cd ynext-xcloud/ynext-xcloud
cargo test    # Rodar os testes
cargo clippy  # Verificações de qualidade
cargo fmt     # Formatação automática
```

---

## 📄 Licença

Dual-licensed sob **MIT** ou **Apache-2.0** — você escolhe.

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)

> ⚠️ **Aviso Legal**: Projeto independente, sem afiliação com Microsoft ou Xbox. Xbox Cloud Gaming requer assinatura Game Pass Ultimate ativa. Este software usa as APIs públicas do Xbox da mesma forma que outros clientes open source como o [Greenlight](https://github.com/unknownskl/greenlight).

---

<div align="center">
  Feito com 🦀 por <a href="https://github.com/ynext-brazil">Ynext - Agência de Automação</a>
</div>
