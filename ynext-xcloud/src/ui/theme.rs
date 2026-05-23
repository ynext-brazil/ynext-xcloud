//! # Tema Visual — Estilo Xbox
//!
//! Paleta de cores e configurações globais de estilo que replicam
//! a identidade visual do Xbox Cloud Gaming (xcloud.xbox.com).
//!
//! ## Paleta
//!
//! | Token            | Hex       | Uso                          |
//! |------------------|-----------|------------------------------|
//! | `BG`             | `#0E0E0E` | Fundo da janela              |
//! | `SURFACE`        | `#1E1E1E` | Cards de jogos               |
//! | `SURFACE_HOVER`  | `#2A2A2A` | Card com mouse em cima       |
//! | `ACCENT`         | `#107C10` | Verde Xbox (bordas, botões)  |
//! | `ACCENT_HOVER`   | `#1A9C1A` | Verde mais claro ao hover    |
//! | `TEXT_PRIMARY`   | `#F2F2F2` | Títulos, texto principal     |
//! | `TEXT_SECONDARY` | `#8A8A8A` | Subtítulos, metadados        |
//! | `DANGER`         | `#C84B31` | Badge "Saindo em breve"      |

use egui::{Color32, FontId, Rounding, Style, Visuals};

// ---------------------------------------------------------------------------
// Cores
// ---------------------------------------------------------------------------

/// Fundo principal da janela
pub const BG: Color32 = Color32::from_rgb(14, 14, 14);

/// Superfície dos cards de jogo
pub const SURFACE: Color32 = Color32::from_rgb(30, 30, 30);

/// Superfície ao hover
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(42, 42, 42);

/// Verde Xbox — cor de destaque principal
pub const ACCENT: Color32 = Color32::from_rgb(16, 124, 16);

/// Verde Xbox mais claro (hover em botões/bordas)
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(26, 156, 26);

/// Texto principal (títulos, nomes de jogos)
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(242, 242, 242);

/// Texto secundário (subtítulos, metadados)
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(138, 138, 138);

/// Badge de aviso — usado em "Saindo em breve"
pub const DANGER: Color32 = Color32::from_rgb(200, 75, 49);

/// Borda sutil nos cards
pub const BORDER: Color32 = Color32::from_rgb(48, 48, 48);

/// Transparente
pub const TRANSPARENT: Color32 = Color32::TRANSPARENT;

// ---------------------------------------------------------------------------
// Dimensões
// ---------------------------------------------------------------------------

/// Largura do card de jogo
pub const CARD_WIDTH: f32 = 160.0;

/// Altura da cover art (proporção 3:4)
pub const CARD_COVER_HEIGHT: f32 = 213.0;

/// Altura total do card (cover + título)
pub const CARD_HEIGHT: f32 = CARD_COVER_HEIGHT + 36.0;

/// Gap entre cards na fileira horizontal
pub const CARD_GAP: f32 = 12.0;

/// Altura da top bar
pub const TOP_BAR_HEIGHT: f32 = 56.0;

/// Arredondamento dos cantos dos cards
pub const CARD_ROUNDING: Rounding = Rounding {
    nw: 8.0,
    ne: 8.0,
    sw: 8.0,
    se: 8.0,
};

/// Arredondamento dos botões
pub const BUTTON_ROUNDING: Rounding = Rounding {
    nw: 6.0,
    ne: 6.0,
    sw: 6.0,
    se: 6.0,
};

// ---------------------------------------------------------------------------
// Fontes
// ---------------------------------------------------------------------------

/// Fonte para títulos de seções
pub fn section_title_font() -> FontId {
    FontId::proportional(18.0)
}

/// Fonte para nome do jogo no card
pub fn card_title_font() -> FontId {
    FontId::proportional(12.5)
}

/// Fonte para texto de link "ver todos >"
pub fn link_font() -> FontId {
    FontId::proportional(13.0)
}

/// Fonte para badges (ex: "Saindo em breve")
pub fn badge_font() -> FontId {
    FontId::proportional(10.5)
}

// ---------------------------------------------------------------------------
// Estilo global
// ---------------------------------------------------------------------------

/// Aplica o tema Xbox ao contexto egui.
/// Deve ser chamado uma única vez na inicialização do `eframe::App`.
pub fn apply(ctx: &egui::Context) {
    let mut style = Style {
        visuals: Visuals::dark(),
        ..Default::default()
    };

    // Fundo global
    style.visuals.window_fill = BG;
    style.visuals.panel_fill = BG;
    style.visuals.faint_bg_color = SURFACE;
    style.visuals.extreme_bg_color = Color32::from_rgb(8, 8, 8);

    // Bordas e arredondamentos
    style.visuals.window_rounding = CARD_ROUNDING;
    style.visuals.menu_rounding = BUTTON_ROUNDING;

    // Widgets interativos — cor de fundo padrão
    style.visuals.widgets.inactive.weak_bg_fill = SURFACE;
    style.visuals.widgets.inactive.bg_fill = SURFACE;
    style.visuals.widgets.inactive.fg_stroke.color = TEXT_SECONDARY;

    // Hover
    style.visuals.widgets.hovered.weak_bg_fill = SURFACE_HOVER;
    style.visuals.widgets.hovered.bg_fill = SURFACE_HOVER;
    style.visuals.widgets.hovered.fg_stroke.color = TEXT_PRIMARY;
    style.visuals.widgets.hovered.bg_stroke.color = ACCENT;
    style.visuals.widgets.hovered.bg_stroke.width = 1.5;

    // Ativo (clicando)
    style.visuals.widgets.active.weak_bg_fill = ACCENT;
    style.visuals.widgets.active.bg_fill = ACCENT;
    style.visuals.widgets.active.fg_stroke.color = Color32::WHITE;

    // Seleção de texto
    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(16, 124, 16, 100);
    style.visuals.selection.stroke.color = ACCENT;

    // Hyperlinks
    style.visuals.hyperlink_color = ACCENT_HOVER;

    // Scrollbars sutis
    style.spacing.scroll.bar_width = 4.0;

    // Espaçamento interno dos widgets
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);

    ctx.set_style(style);
}
