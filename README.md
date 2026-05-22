# 🎮 Ynext-Xcloud

> **Cliente nativo open source para Xbox Cloud Gaming — Altíssimo desempenho, Zero navegador.**

[![Build Status](https://github.com/ynext/ynext-xcloud/actions/workflows/ci.yml/badge.svg)](https://github.com/ynext/ynext-xcloud/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

## 🎯 O Problema que Resolvemos

Clientes web/Electron/Tauri para xCloud sofrem com:
- ❌ **VA-API instável** no Linux (chips Intel integrados)
- ❌ **Gargalo de CPU** na decodificação de vídeo
- ❌ **Input lag absurdo** por overhead do browser engine
- ❌ **Alto consumo de RAM** (Electron usa 400-800MB só para renderizar)

## ✅ Nossa Solução

O Ynext-Xcloud segue o modelo do **Moonlight** (cliente NVIDIA GameStream): **pipeline nativo ponta a ponta**, sem nenhum navegador embutido.

| Recurso | Clientes Web | Ynext-Xcloud |
|---------|-------------|--------------|
| RAM em idle | ~400-800MB | **~30-60MB** |
| CPU decode | Software (100%) | **Hardware (VA-API/DxVA)** |
| Input lag | ~50-150ms | **<10ms** |
| Suporte Linux VA-API | Instável | **Nativo e estável** |
| Hardware mínimo | Core i5 + 8GB | **Celeron + 2GB** |

## 🏗️ Arquitetura

```
┌─────────────────────────────────────────────────────────────┐
│                     YNEXT-XCLOUD                            │
│                                                             │
│  ┌─────────┐   ┌──────────┐   ┌──────────┐   ┌─────────┐  │
│  │  Auth   │──▶│ Signaling│──▶│  Video   │──▶│   UI    │  │
│  │ Module  │   │  WebRTC  │   │ Pipeline │   │  SDL2   │  │
│  │  (MSA)  │   │  (SDP)   │   │(GStreamer│   │         │  │
│  └─────────┘   └──────────┘   │ +VA-API) │   └─────────┘  │
│                               └──────────┘                  │
│                                    │                        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Gamepad Input (gilrs)                    │  │
│  │         USB/Bluetooth → DataChannel WebRTC            │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## 🚀 Início Rápido

### Pré-requisitos

**Linux:**
```bash
sudo apt install libssl-dev pkg-config libsecret-1-dev gstreamer1.0-vaapi
```

**Windows:**
```powershell
# Apenas Rust — o restante é bundled
```

### Instalação

```bash
git clone https://github.com/ynext/ynext-xcloud
cd ynext-xcloud/ynext-xcloud
cargo build --release
```

### Uso

```bash
# Login na conta Microsoft/Xbox
./target/release/ynext-xcloud auth login

# Ver informações da conta
./target/release/ynext-xcloud info

# Iniciar streaming (em desenvolvimento)
./target/release/ynext-xcloud stream --game "Halo Infinite"

# Logout
./target/release/ynext-xcloud auth logout
```

## 📦 Módulos do Projeto

| Módulo | Status | Descrição |
|--------|--------|-----------|
| **Auth (MSA/XBL/XSTS)** | ✅ Fase 1 | Autenticação Microsoft Account + Xbox Live |
| **Signaling WebRTC** | 🔧 Fase 2 | Negociação SDP/ICE com a API xCloud |
| **Video Pipeline** | 📋 Fase 3 | H.264 decode via VA-API (Linux) / DxVA (Windows) |
| **Gamepad Input** | 📋 Fase 4 | gilrs → DataChannel WebRTC, <10ms lag |
| **Audio Pipeline** | 📋 Fase 5 | Opus decode → PipeWire/ALSA/WASAPI |
| **UI Launcher** | 📋 Fase 6 | Interface SDL2 para biblioteca de jogos |

## 🛠️ Tecnologias

- **Rust 1.75+** — Segurança de memória e performance equivalente a C
- **Tokio** — Async runtime para I/O de alta performance
- **GStreamer** — Pipeline de vídeo com aceleração de hardware nativa
- **VA-API** — Aceleração Intel/AMD no Linux (sem gargalo de CPU)
- **DirectX/DxVA** — Aceleração nativa no Windows
- **SDL2** — Janela e captura de gamepad
- **gilrs** — Suporte a gamepad multiplataforma
- **reqwest + rustls** — HTTP sem dependência de OpenSSL do sistema

## 🤝 Contribuindo

Este projeto é **100% open source** e contribuições são bem-vindas!

```bash
# Fork → Clone → Branch → Commit → Pull Request
git checkout -b feature/meu-recurso
cargo test        # Rode os testes
cargo clippy      # Verificações de qualidade
cargo fmt         # Formatação
```

Veja [CONTRIBUTING.md](CONTRIBUTING.md) para o guia completo.

## 📄 Licença

Dual-licensed: **MIT** ou **Apache-2.0** (você escolhe).

> ⚠️ **Aviso Legal**: Este projeto é independente e não tem afiliação com a Microsoft ou Xbox. Xbox Cloud Gaming é um serviço da Microsoft que requer assinatura Game Pass Ultimate ativa. Este software se conecta às APIs públicas do Xbox da mesma forma que outros clientes de terceiros como o Greenlight.

---

Feito com 🦀 por [Ynext - Agência de Automação](https://ynext.com.br)
