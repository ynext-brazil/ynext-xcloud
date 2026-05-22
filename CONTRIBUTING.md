# Guia de Contribuição — Ynext-Xcloud

Obrigado pelo interesse em contribuir com o Ynext-Xcloud! 🎮

## 🛠️ Setup de Desenvolvimento

### Pré-requisitos

**Linux (Ubuntu/Debian):**
```bash
# Rust (se não tiver instalado)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Dependências do sistema
sudo apt install -y \
  libssl-dev \
  pkg-config \
  libsecret-1-dev \
  libdbus-1-dev \
  gstreamer1.0-tools \
  gstreamer1.0-vaapi \
  gstreamer1.0-plugins-bad
```

**Windows:**
- Instale [Rust](https://www.rust-lang.org/tools/install)
- As demais dependências são bundled

### Clonar e compilar

```bash
git clone https://github.com/ynext/ynext-xcloud
cd ynext-xcloud/ynext-xcloud
cargo build            # Debug (mais rápido)
cargo build --release  # Release (otimizado)
```

## 🧪 Rodando os Testes

```bash
cargo test                    # Todos os testes
cargo test auth               # Apenas testes do módulo auth
cargo test -- --nocapture     # Com saída de println
```

## 🔍 Qualidade de Código

Antes de abrir um PR, certifique-se que:

```bash
cargo fmt             # Formatação automática
cargo clippy          # Lints e boas práticas
cargo test            # Todos os testes passam
```

## 📁 Estrutura do Projeto

```
src/
├── auth/         # Módulo 1: Autenticação Microsoft/Xbox
├── signaling/    # Módulo 2: WebRTC SDP/ICE
├── video/        # Módulo 3: Pipeline GStreamer + VA-API
├── input/        # Módulo 4: Gamepad
├── audio/        # Módulo 5: Áudio Opus
└── ui/           # Módulo 6: Interface SDL2
```

## 🐛 Reportando Bugs

Use o template de issues no GitHub com:
- Versão do Ynext-Xcloud
- Sistema operacional e versão
- Modelo do hardware (CPU, GPU)
- Passos para reproduzir
- Logs com `--log-level debug`

## 📄 Licença

Ao contribuir, você concorda que suas contribuições serão licenciadas sob **MIT OR Apache-2.0**.
