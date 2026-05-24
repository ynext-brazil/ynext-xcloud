//! # Widgets Reutilizáveis — Launcher Xbox
//!
//! Componentes UI que constroem o visual estilo Xbox:
//! - `GameCard`: card de jogo com cover art e hover animado
//! - `section_row`: linha horizontal de cards com título de seção
//! - `top_bar`: barra superior com logo, busca e perfil
//! - `search_bar`: campo de busca in-line

use egui::{Align, Align2, Color32, Image, Layout, Rect, RichText, Rounding, Sense, Ui, Vec2, pos2};

use crate::ui::theme;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CardStyle {
    Tall,
    Wide,
    Square,
}

impl CardStyle {
    pub fn cover_size(&self) -> Vec2 {
        match self {
            CardStyle::Tall => Vec2::new(160.0, 213.0),
            CardStyle::Wide => Vec2::new(284.0, 160.0),
            CardStyle::Square => Vec2::new(160.0, 160.0),
        }
    }
    
    pub fn card_size(&self) -> Vec2 {
        let mut size = self.cover_size();
        size.y += 36.0; // Espaço reservado para o título abaixo da capa
        size
    }
}

// ---------------------------------------------------------------------------
// GameCard
// ---------------------------------------------------------------------------

#[derive(Clone)]
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
pub fn game_card(ui: &mut Ui, title: &str, cover: &CoverState, style: CardStyle, is_leaving: bool, leaving_date: Option<&String>) -> bool {
    let card_size = style.card_size();
    let cover_size_base = style.cover_size();
    
    let (rect, response) = ui.allocate_exact_size(card_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let painter = ui.painter();

        let hover_t = ui.ctx().animate_bool_with_time(response.id.with("hover"), hovered, 0.15);

        // Animação de glow e crescimento
        let expand_amount = hover_t * 4.0;
        let draw_rect = rect.expand(expand_amount);

        // Sombra de foco (Glow Xbox Verde)
        if hover_t > 0.0 {
            painter.rect_stroke(
                draw_rect.expand(2.0),
                Rounding::same(8.0 + 2.0),
                egui::Stroke::new(2.0 * hover_t, theme::ACCENT.linear_multiply(hover_t * 0.8)),
            );
        }

        // Fundo do card
        let bg_color = if hovered {
            theme::SURFACE_HOVER
        } else {
            theme::SURFACE
        };
        painter.rect_filled(draw_rect, theme::CARD_ROUNDING, bg_color);

        // Borda verde fina permanente ao hover
        if hovered {
            painter.rect_stroke(
                draw_rect,
                theme::CARD_ROUNDING,
                egui::Stroke::new(1.5, theme::ACCENT),
            );
        }

        // Área da cover art (proporcional ao draw_rect expandido)
        let cover_height = cover_size_base.y + (expand_amount * 2.0 * (cover_size_base.y / card_size.y));
        let cover_rect = Rect::from_min_size(
            draw_rect.min,
            Vec2::new(draw_rect.width(), cover_height),
        );

        let cover_rounding = Rounding {
            nw: 8.0,
            ne: 8.0,
            sw: 0.0,
            se: 0.0,
        };

        match cover {
            CoverState::Ready(texture) => {
                let tex_size = texture.size_vec2();
                let target_size = cover_rect.size();
                
                // UV cropping (object-fit: cover) to prevent any stretching
                let image_aspect = tex_size.x / tex_size.y;
                let target_aspect = target_size.x / target_size.y;

                let uv = if image_aspect > target_aspect {
                    let crop_width = tex_size.y * target_aspect;
                    let crop_x = (tex_size.x - crop_width) / 2.0;
                    Rect::from_min_max(
                        pos2(crop_x / tex_size.x, 0.0),
                        pos2((crop_x + crop_width) / tex_size.x, 1.0)
                    )
                } else {
                    let crop_height = tex_size.x / target_aspect;
                    let crop_y = (tex_size.y - crop_height) / 2.0;
                    Rect::from_min_max(
                        pos2(0.0, crop_y / tex_size.y),
                        pos2(1.0, (crop_y + crop_height) / tex_size.y)
                    )
                };

                let image = Image::new(texture)
                    .uv(uv)
                    .fit_to_exact_size(cover_rect.size())
                    .rounding(cover_rounding);
                image.paint_at(ui, cover_rect);
            }
            CoverState::Loading => {
                // Placeholder: fundo verde Xbox com "..." animado
                painter.rect_filled(cover_rect, cover_rounding, theme::ACCENT);
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
                painter.rect_filled(cover_rect, cover_rounding, theme::SURFACE_HOVER);
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
            let badge_text = if let Some(date) = leaving_date {
                // Pegar apenas "YYYY-MM-DD" da string do xbox
                let short_date = date.split('T').next().unwrap_or(date);
                format!("Saindo ({})", short_date)
            } else {
                "Saindo em breve".to_string()
            };
            
            // Desenha a tarja no fundo do `cover_rect` (para crescer no hover também)
            let badge_rect = Rect::from_min_max(
                pos2(cover_rect.min.x, cover_rect.max.y - 20.0),
                pos2(cover_rect.max.x, cover_rect.max.y),
            );
            painter.rect_filled(
                badge_rect,
                Rounding { nw: 0.0, ne: 0.0, sw: cover_rounding.sw, se: cover_rounding.se },
                Color32::from_rgb(220, 53, 69).linear_multiply(0.9), // Vermelho intenso levemente translúcido
            );
            painter.text(
                badge_rect.center(),
                Align2::CENTER_CENTER,
                badge_text,
                egui::FontId::proportional(12.0),
                Color32::WHITE,
            );
        }

        // Título do jogo abaixo da cover
        let title_pos = egui::pos2(draw_rect.min.x + 8.0, cover_rect.max.y + 12.0);
        let title_display = if title.chars().count() > 18 {
            let truncated: String = title.chars().take(17).collect();
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
