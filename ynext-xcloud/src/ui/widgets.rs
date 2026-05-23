//! # Widgets Reutilizáveis — Launcher Xbox
//!
//! Componentes UI que constroem o visual estilo Xbox:
//! - `GameCard`: card de jogo com cover art e hover animado
//! - `section_row`: linha horizontal de cards com título de seção
//! - `top_bar`: barra superior com logo, busca e perfil
//! - `search_bar`: campo de busca in-line

use egui::{Align, Color32, Image, Layout, Rect, RichText, Rounding, Sense, Ui, Vec2};

use crate::ui::theme;

// ---------------------------------------------------------------------------
// GameCard
// ---------------------------------------------------------------------------

/// Estado de carregamento da cover art
pub enum CoverState {
    /// Ainda baixando — exibe placeholder verde
    Loading,
    /// Imagem carregada e pronta para exibir
    Ready(egui::TextureHandle),
    /// Falhou em baixar — exibe placeholder cinza com ícone
    Failed,
}

/// Renderiza um card de jogo (cover art + título + hover animado).
///
/// Retorna `true` se o usuário clicou no card.
pub fn game_card(ui: &mut Ui, title: &str, cover: &CoverState, is_leaving: bool) -> bool {
    let card_size = Vec2::new(theme::CARD_WIDTH, theme::CARD_HEIGHT);

    let (rect, response) = ui.allocate_exact_size(card_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let painter = ui.painter();

        // Fundo do card — muda sutilmente ao hover
        let bg_color = if hovered {
            theme::SURFACE_HOVER
        } else {
            theme::SURFACE
        };
        painter.rect_filled(rect, theme::CARD_ROUNDING, bg_color);

        // Borda verde ao hover
        if hovered {
            painter.rect_stroke(
                rect,
                theme::CARD_ROUNDING,
                egui::Stroke::new(1.5, theme::ACCENT),
            );
        }

        // Área da cover art
        let cover_rect = Rect::from_min_size(
            rect.min,
            Vec2::new(theme::CARD_WIDTH, theme::CARD_COVER_HEIGHT),
        );

        match cover {
            CoverState::Ready(texture) => {
                // Exibe a imagem carregada
                let image = Image::new(texture).fit_to_exact_size(cover_rect.size());
                image.paint_at(ui, cover_rect);
            }
            CoverState::Loading => {
                // Placeholder: fundo verde Xbox com "..." animado
                painter.rect_filled(
                    cover_rect,
                    Rounding {
                        nw: 8.0,
                        ne: 8.0,
                        sw: 0.0,
                        se: 0.0,
                    },
                    theme::ACCENT,
                );
                painter.text(
                    cover_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "⏳",
                    egui::FontId::proportional(28.0),
                    Color32::WHITE,
                );
            }
            CoverState::Failed => {
                // Placeholder: fundo cinza escuro com ícone de jogo
                painter.rect_filled(
                    cover_rect,
                    Rounding {
                        nw: 8.0,
                        ne: 8.0,
                        sw: 0.0,
                        se: 0.0,
                    },
                    theme::SURFACE_HOVER,
                );
                painter.text(
                    cover_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "🎮",
                    egui::FontId::proportional(32.0),
                    theme::TEXT_SECONDARY,
                );
            }
        }

        // Badge "Saindo em breve" (canto superior direito)
        if is_leaving {
            let badge_text = "Saindo";
            let badge_rect = Rect::from_min_size(
                egui::pos2(cover_rect.max.x - 56.0, cover_rect.min.y + 6.0),
                Vec2::new(50.0, 18.0),
            );
            painter.rect_filled(badge_rect, Rounding::same(4.0), theme::DANGER);
            painter.text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                badge_text,
                theme::badge_font(),
                Color32::WHITE,
            );
        }

        // Título do jogo abaixo da cover
        let title_pos = egui::pos2(rect.min.x + 6.0, cover_rect.max.y + 6.0);
        let title_display = if title.chars().count() > 20 {
            let truncated: String = title.chars().take(19).collect();
            format!("{}…", truncated)
        } else {
            title.to_string()
        };
        painter.text(
            title_pos,
            egui::Align2::LEFT_TOP,
            title_display,
            theme::card_title_font(),
            if hovered {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            },
        );
    }

    response.clicked()
}

// ---------------------------------------------------------------------------
// Linha de seção horizontal (título + scroll horizontal de cards)
// ---------------------------------------------------------------------------

/// Renderiza uma seção com título e uma fileira horizontal de cards roláveis.
///
/// Retorna o índice do card clicado (se algum foi).
pub fn section_row<'a>(
    ui: &mut Ui,
    title: &str,
    games: impl Iterator<Item = (&'a str, &'a CoverState, bool)>,
    is_leaving_section: bool,
) -> Option<usize> {
    let mut clicked = None;

    ui.add_space(20.0);

    // Cabeçalho da seção
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .font(theme::section_title_font())
                .color(theme::TEXT_PRIMARY)
                .strong(),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Label::new(
                        RichText::new("ver todos >")
                            .font(theme::link_font())
                            .color(theme::ACCENT),
                    )
                    .sense(Sense::click()),
                )
                .clicked()
            {
                // TODO: navegar para lista completa
            }
        });
    });

    ui.add_space(10.0);

    // Scroll horizontal dos cards
    egui::ScrollArea::horizontal()
        .id_salt(title)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme::CARD_GAP;
                for (idx, (game_title, cover, _)) in games.enumerate() {
                    let is_leaving = is_leaving_section;
                    if game_card(ui, game_title, cover, is_leaving) {
                        clicked = Some(idx);
                    }
                }
            });
        });

    clicked
}

// ---------------------------------------------------------------------------
// Barra superior
// ---------------------------------------------------------------------------

/// Renderiza a top bar com logo Xbox, campo de busca e perfil do usuário.
pub fn top_bar(ui: &mut Ui, search_query: &mut String, username: &str) {
    let bar_height = theme::TOP_BAR_HEIGHT;

    ui.add_space(0.0);
    let (bar_rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), bar_height), Sense::hover());

    let painter = ui.painter();

    // Fundo da top bar — levemente diferente do bg principal
    painter.rect_filled(bar_rect, Rounding::ZERO, Color32::from_rgb(20, 20, 20));

    // Linha inferior sutil (separador)
    painter.hline(
        bar_rect.x_range(),
        bar_rect.max.y,
        egui::Stroke::new(1.0, theme::BORDER),
    );

    // Renderiza o conteúdo dentro da área da bar
    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(bar_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    child_ui.add_space(16.0);

    // Logo Xbox ▪ com texto
    child_ui.label(
        RichText::new("⬛ YNEXT XCLOUD")
            .font(egui::FontId::proportional(17.0))
            .color(theme::TEXT_PRIMARY)
            .strong(),
    );

    child_ui.add_space(20.0);

    // Campo de busca
    let search = egui::TextEdit::singleline(search_query)
        .hint_text("🔍  Pesquisar jogos...")
        .desired_width(280.0)
        .frame(true);
    child_ui.add(search);

    // Empurra perfil para a direita
    child_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.add_space(16.0);
        ui.label(
            RichText::new(format!("👤 {}", username))
                .font(egui::FontId::proportional(13.0))
                .color(theme::TEXT_SECONDARY),
        );
    });
}
