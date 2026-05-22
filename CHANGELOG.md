# Changelog

Todos os recursos notáveis para este projeto serão documentados neste arquivo.

O formato é baseado no [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [Unreleased]

### Adicionado
- **Fase 2 (Sinalização WebRTC):** Esqueleto completo do módulo de sinalização com a API xCloud.
- Negociação SDP via `POST /v4/sessions/{id}/sdp`.
- Troca de candidatos ICE via `POST /v4/sessions/{id}/ice`.
- Estrutura baseada na exigência arquitetural de usar o GStreamer `webrtcbin` (Zero-Copy).
- **Fase 1 (Autenticação):** Módulo completo de autenticação Microsoft.
- Fluxo Device Code (OAuth 2.0) para Login de Contas Microsoft.
- Autenticação Xbox Live (XBL3.0 Token).
- Autenticação de Streaming Xbox (XSTS Token).
- Armazenamento seguro de tokens em arquivo `tokens.json` (`chmod 600`), funcionando nativamente em Linux/Windows.
- Configuração de CI/CD via GitHub Actions.
- Estrutura completa do CLI (comandos `auth login`, `auth status`, `auth logout`, `info`).
- Licenciamento duplo (MIT e Apache 2.0).
