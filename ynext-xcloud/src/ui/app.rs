//! # App Principal do Launcher — XCloudApp
//!
//! Implementa `eframe::App`, o ponto central que orquestra
//! toda a renderização da UI frame a frame.
//!
//! ## Estados de tela
//!
//! - `Home`: tela principal com as 5 seções de jogos
//! - `SearchResults`: resultados filtrados pela query de busca
//! - `GameDetail`: tela de detalhes de um jogo selecionado

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::ui::widgets::CardStyle;

use crate::ui::catalog::{Game, SIGL_ALL, SIGL_LEAVING, SIGL_NEW, SIGL_POPULAR};
use crate::ui::widgets::CoverState;
use crate::ui::{theme, widgets};

// ---------------------------------------------------------------------------
// Telas (estado de navegação)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    SearchResults,
    GameDetail(String), // ID do jogo
    Category(String),
}

// ---------------------------------------------------------------------------
// Seções de jogos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Section {
    RecentlyPlayed,
    NewlyAdded,
    Popular,
    Leaving,
    AllGames,
}

impl Section {
    pub fn title(&self) -> &str {
        match self {
            Section::RecentlyPlayed => "▶  Continuar jogando",
            Section::NewlyAdded => "✨  Recém adicionados",
            Section::Popular => "🔥  Mais populares na nuvem",
            Section::Leaving => "⏳  Saindo em breve",
            Section::AllGames => "🎮  Todos os jogos",
        }
    }

    pub fn sigl_id(&self) -> Option<&str> {
        match self {
            Section::RecentlyPlayed => None, // requer XSTS token — tratado separado
            Section::NewlyAdded => Some(SIGL_NEW),
            Section::Popular => Some(SIGL_POPULAR),
            Section::Leaving => Some(SIGL_LEAVING),
            Section::AllGames => Some(SIGL_ALL),
        }
    }
}

// ---------------------------------------------------------------------------
// Estado compartilhado entre threads (carregamento assíncrono)
// ---------------------------------------------------------------------------

/// Estado de carregamento de uma seção
#[derive(Default)]
pub struct SectionData {
    pub games: Vec<Game>,
    pub loading: bool,
    pub error: Option<String>,
}

/// Mapa compartilhado entre a thread de UI e as tasks de carregamento
pub type SharedSections = Arc<Mutex<HashMap<String, SectionData>>>;

// ---------------------------------------------------------------------------
// App principal
// ---------------------------------------------------------------------------

pub struct XCloudApp {
    /// Estado atual de navegação
    pub screen: Screen,

    /// Query da caixa de busca
    search_query: String,

    /// Nome do usuário logado (do XSTS token)
    username: String,

    /// Dados das seções (carregados assincronamente)
    sections: SharedSections,

    /// Cache de capas: agora a chave é (game_id, style)
    pub covers: Arc<Mutex<HashMap<(String, CardStyle), CoverState>>>,

    /// Ordem de exibição das seções
    section_order: Vec<Section>,

    /// Jogos filtrados pela busca (atualizado a cada frame com query)
    search_results: Vec<Game>,

    /// Runtime tokio para disparar tarefas de background
    runtime: tokio::runtime::Handle,

    /// Flag de carregamento inicial já disparado
    initial_load_done: bool,

    /// Cliente HTTP (com User-Agent) para todas as requisições
    client: Arc<reqwest::Client>,

    /// Semáforo para limitar a concorrência no download das imagens
    image_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
}

impl XCloudApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        username: String,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        theme::apply(&cc.egui_ctx);
        let fonts = egui::FontDefinitions::default();
        cc.egui_ctx.set_fonts(fonts);

        let sections: SharedSections = Arc::new(Mutex::new(HashMap::new()));

        {
            let mut map = sections.lock().unwrap();
            for section in &[
                Section::NewlyAdded,
                Section::Popular,
                Section::Leaving,
                Section::AllGames,
            ] {
                map.insert(
                    section.title().to_string(),
                    SectionData {
                        loading: true,
                        ..Default::default()
                    },
                );
            }
        }

        let client = Arc::new(reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default());

        Self {
            screen: Screen::Home,
            search_query: String::new(),
            username,
            sections,
            covers: Arc::new(Mutex::new(HashMap::new())),
            section_order: vec![
                Section::RecentlyPlayed,
                Section::NewlyAdded,
                Section::Popular,
                Section::Leaving,
                Section::AllGames,
            ],
            search_results: Vec::new(),
            runtime,
            initial_load_done: false,
            client,
            image_semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(10)),
        }
    }

    fn start_catalog_load(&mut self, ctx: egui::Context) {
        let sections_clone = Arc::clone(&self.sections);
        let client = Arc::clone(&self.client);

        self.runtime.spawn(async move {
            use crate::ui::catalog::*;
            const SIGL_TOUCH: &str = "9c86f07a-f3e8-45ad-82a0-a1f759597059";

            let (new_res, pop_res, leave_res, all_res, touch_res) = tokio::join!(
                fetch_section(&client, SIGL_NEW, 30),
                fetch_section(&client, SIGL_POPULAR, 30),
                fetch_section(&client, SIGL_LEAVING, 20),
                fetch_section(&client, SIGL_ALL, 500),
                fetch_section(&client, SIGL_TOUCH, 30),
            );

            let mut leaving_ids = std::collections::HashSet::new();
            if let Ok(leave_games) = &leave_res {
                for g in leave_games { leaving_ids.insert(g.id.clone()); }
            }

            let apply_leaving = |games: &mut Vec<crate::ui::catalog::Game>| {
                for g in games.iter_mut() {
                    if leaving_ids.contains(&g.id) { g.is_leaving = true; }
                }
            };

            let mut map = sections_clone.lock().unwrap();

            if let Ok(mut novos) = new_res {
                apply_leaving(&mut novos);
                let entry = map.entry(Section::NewlyAdded.title().to_string()).or_default();
                entry.games = novos; entry.loading = false;
            }
            if let Ok(mut pop) = pop_res {
                apply_leaving(&mut pop);
                let entry = map.entry(Section::Popular.title().to_string()).or_default();
                entry.games = pop; entry.loading = false;
            }
            if let Ok(mut leave) = leave_res {
                apply_leaving(&mut leave);
                let entry = map.entry(Section::Leaving.title().to_string()).or_default();
                entry.games = leave; entry.loading = false;
            }
            if let Ok(mut all_games) = all_res {
                apply_leaving(&mut all_games);
                all_games.sort_by(|a, b| a.title.cmp(&b.title));
                let entry = map.entry(Section::AllGames.title().to_string()).or_default();
                entry.games = all_games; entry.loading = false;
            }
            if let Ok(mut touch) = touch_res {
                apply_leaving(&mut touch);
                let entry = map.entry("Jogar com toque".to_string()).or_default();
                entry.games = touch; entry.loading = false;
            }
            
            let entry_recent = map.entry(Section::RecentlyPlayed.title().to_string()).or_default();
            entry_recent.loading = false;

            ctx.request_repaint();
        });
    }

    fn update_search_results(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            return;
        }

        let query = self.search_query.to_lowercase();
        let map = self.sections.lock().unwrap();
        let mut seen_ids = std::collections::HashSet::new();

        self.search_results = map
            .values()
            .flat_map(|section| section.games.iter().cloned())
            .filter(|game| {
                game.title.to_lowercase().contains(&query) && seen_ids.insert(game.id.clone())
            })
            .collect();

        self.search_results.sort_by(|a, b| a.title.cmp(&b.title));
    }
    /// Dispara o download de uma capa de acordo com o estilo desejado
    fn spawn_cover_download(&self, game: &crate::ui::catalog::Game, ctx: &egui::Context, style: crate::ui::widgets::CardStyle) {
        let game_id = game.id.clone();
        let cache_key = (game_id.clone(), style);

        {
            let mut map = self.covers.lock().unwrap();
            if map.contains_key(&cache_key) {
                return;
            }
            map.insert(cache_key.clone(), CoverState::Loading);
        }

        let url = match style {
            crate::ui::widgets::CardStyle::Wide => game.hero_art_url.clone().or_else(|| game.cover_url.clone()),
            _ => game.box_art_url.clone().or_else(|| game.poster_url.clone()).or_else(|| game.cover_url.clone()),
        };

        if let Some(cover_url) = url {
            let covers_clone = Arc::clone(&self.covers);
            let ctx_clone = ctx.clone();
            let client = Arc::clone(&self.client);
            let semaphore = Arc::clone(&self.image_semaphore);

            self.runtime.spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                if let Ok(resp) = client.get(&cover_url).send().await {
                    if let Ok(bytes) = resp.bytes().await {
                        if let Ok(image) = image::load_from_memory(&bytes) {
                            let size = [image.width() as _, image.height() as _];
                            let image_buffer = image.to_rgba8();
                            let pixels = image_buffer.as_flat_samples();
                            
                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                size,
                                pixels.as_slice(),
                            );
                            
                            let texture = ctx_clone.load_texture(
                                format!("cover_{}_{:?}", game_id, style),
                                color_image,
                                egui::TextureOptions::LINEAR,
                            );
                            
                            covers_clone.lock().unwrap().insert(cache_key, CoverState::Ready(texture));
                            ctx_clone.request_repaint();
                            return;
                        }
                    }
                }
                covers_clone.lock().unwrap().insert(cache_key, CoverState::Failed);
                ctx_clone.request_repaint();
            });
        } else {
            self.covers.lock().unwrap().insert(cache_key, CoverState::Failed);
        }
    }
}

impl eframe::App for XCloudApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Dispara carregamento uma única vez no primeiro frame
        if !self.initial_load_done {
            self.start_catalog_load(ctx.clone());
            self.initial_load_done = true;
        }

        // Top Bar isolada para ocupar a largura toda
        egui::TopBottomPanel::top("top_bar_panel")
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(20, 20, 20)))
            .show(ctx, |ui| {
                let prev_query = self.search_query.clone();
                
                // Aplica margem interna apenas ao conteúdo da barra para alinhar com os cards
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(24.0, 0.0))
                    .show(ui, |ui| {
                        widgets::top_bar(ui, &mut self.search_query, &self.username);
                    });

                // Atualiza resultados de busca se a query mudou
                if self.search_query != prev_query {
                    if self.search_query.is_empty() {
                        self.screen = Screen::Home;
                    } else {
                        self.screen = Screen::SearchResults;
                        self.update_search_results();
                    }
                }
            });

        // Painel principal com fundo escuro sem margem para não cortar as sombras na lateral
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG))
            .show(ctx, |ui| {

                // --- Conteúdo com scroll vertical ---
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.add_space(16.0);

                        match &self.screen.clone() {
                            Screen::Home => self.render_home(ui),
                            Screen::SearchResults => self.render_search(ui),
                            Screen::GameDetail(id) => self.render_game_detail(ui, id.clone()),
                            Screen::Category(name) => self.render_category(ui, name.clone()),
                        }

                        ui.add_space(40.0);
                    });
            });
    }
}

// ---------------------------------------------------------------------------
// Renderização das telas
// ---------------------------------------------------------------------------

impl XCloudApp {
    fn render_home(&mut self, ui: &mut egui::Ui) {
        let sections_snap = {
            let map = self.sections.lock().unwrap();
            map.iter()
                .map(|(k, v)| (k.clone(), v.games.clone(), v.loading, v.error.clone()))
                .collect::<Vec<_>>()
        };

        let section_order = self.section_order.clone();
        for section in &section_order {
            // "Continuar jogando" ainda não tem dados do XSTS — exibe placeholder
            if *section == Section::RecentlyPlayed {
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new(section.title())
                            .font(theme::section_title_font())
                            .color(theme::TEXT_PRIMARY)
                            .strong(),
                    );
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new("Faça login e inicie uma sessão para ver seu histórico.")
                            .color(theme::TEXT_SECONDARY),
                    );
                });
                continue;
            }

            let section_title = section.title().to_string();

            if let Some((_, games, loading, error)) =
                sections_snap.iter().find(|(k, ..)| *k == section_title)
            {
                ui.add_space(20.0);
                
                // Título da seção com padding lateral
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new(section.title())
                            .font(theme::section_title_font())
                            .color(theme::TEXT_PRIMARY)
                            .strong(),
                    );
                });
                ui.add_space(10.0);

                if *loading {
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.spinner();
                        ui.label(
                            egui::RichText::new("Carregando jogos...").color(theme::TEXT_SECONDARY),
                        );
                    });
                } else if let Some(err) = error {
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.label(egui::RichText::new(format!("⚠️  {}", err)).color(theme::DANGER));
                    });
                } else {
                    let is_leaving = section_title == Section::Leaving.title();
                    let style = if is_leaving {
                        CardStyle::Wide
                    } else if section_title == Section::AllGames.title() {
                        CardStyle::Square
                    } else {
                        CardStyle::Tall
                    };

                    egui::ScrollArea::horizontal()
                        .id_salt(&section_title)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.spacing_mut().item_spacing.x = theme::CARD_GAP;
                                
                                let limit = 20;
                                for game in games.iter().take(limit) {
                                    self.spawn_cover_download(game, ui.ctx(), style);

                                    let cache_key = (game.id.clone(), style);
                                    let cover = {
                                        let map = self.covers.lock().unwrap();
                                        map.get(&cache_key).cloned().unwrap_or(CoverState::Loading)
                                    };

                                    if widgets::game_card(ui, &game.title, &cover, style, game.is_leaving, game.leaving_date.as_ref()) {
                                        self.screen = Screen::GameDetail(game.id.clone());
                                    }
                                }
                                
                                // Botão Ver Mais no final da lista
                                if games.len() > limit {
                                    let card_size = egui::Vec2::new(theme::CARD_WIDTH, theme::CARD_HEIGHT);
                                    let (rect, response) = ui.allocate_exact_size(card_size, egui::Sense::click());
                                    if ui.is_rect_visible(rect) {
                                        let painter = ui.painter();
                                        let bg_color = if response.hovered() { theme::SURFACE_HOVER } else { theme::SURFACE };
                                        painter.rect_filled(rect, theme::CARD_ROUNDING, bg_color);
                                        painter.text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "Ver todos\n➔",
                                            egui::FontId::proportional(20.0),
                                            if response.hovered() { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY },
                                        );
                                        if response.hovered() {
                                            painter.rect_stroke(rect, theme::CARD_ROUNDING, egui::Stroke::new(1.5, theme::ACCENT));
                                        }
                                        if response.clicked() {
                                            self.screen = Screen::Category(section_title.clone());
                                        }
                                    }
                                }

                                ui.add_space(24.0);
                            });
                        });
                }
            }
        }
    }

    fn render_search(&mut self, ui: &mut egui::Ui) {
        let results = self.search_results.clone();

        if results.is_empty() {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("🔍")
                        .font(egui::FontId::proportional(48.0))
                        .color(theme::TEXT_SECONDARY),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Nenhum jogo encontrado para \"{}\"",
                        self.search_query
                    ))
                    .color(theme::TEXT_SECONDARY),
                );
            });
            return;
        }

        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("Resultados da Busca")
                    .font(theme::section_title_font())
                    .color(theme::TEXT_PRIMARY),
            );
            ui.add_space(20.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(theme::CARD_GAP, theme::CARD_GAP);
                    for game in results.iter() {
                        let style = crate::ui::widgets::CardStyle::Tall;
                        self.spawn_cover_download(game, ui.ctx(), style);
                        let cache_key = (game.id.clone(), style);
                        let cover = {
                            let map = self.covers.lock().unwrap();
                            map.get(&cache_key).cloned().unwrap_or(CoverState::Loading)
                        };
                        
                        if widgets::game_card(ui, &game.title, &cover, style, game.is_leaving, game.leaving_date.as_ref()) {
                            self.screen = Screen::GameDetail(game.id.clone());
                        }
                    }
                });
            });
        });
    }

    fn render_category(&mut self, ui: &mut egui::Ui, category_name: String) {
        let mut category_games = vec![];
        {
            let sections = self.sections.lock().unwrap();
            if let Some(section_data) = sections.get(&category_name) {
                category_games = section_data.games.clone();
            }
        }

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            if ui.add(
                egui::Button::new(
                    egui::RichText::new("⬅ Voltar")
                        .font(egui::FontId::proportional(16.0))
                        .color(theme::TEXT_PRIMARY)
                )
                .fill(theme::SURFACE)
                .rounding(theme::CARD_ROUNDING)
            ).clicked() {
                self.screen = Screen::Home;
            }
        });
        
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new(format!("Todos os jogos: {}", category_name))
                    .font(theme::section_title_font())
                    .color(theme::TEXT_PRIMARY),
            );
            ui.add_space(20.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(theme::CARD_GAP, theme::CARD_GAP);
                    for game in category_games.iter() {
                        let style = crate::ui::widgets::CardStyle::Square;
                        self.spawn_cover_download(game, ui.ctx(), style);
                        let cache_key = (game.id.clone(), style);
                        let cover = {
                            let map = self.covers.lock().unwrap();
                            map.get(&cache_key).cloned().unwrap_or(CoverState::Loading)
                        };
                        
                        if widgets::game_card(ui, &game.title, &cover, style, game.is_leaving, game.leaving_date.as_ref()) {
                            self.screen = Screen::GameDetail(game.id.clone());
                        }
                    }
                });
            });
        });
    }

    fn render_game_detail(&mut self, ui: &mut egui::Ui, game_id: String) {
        // Encontrar o jogo
        let mut target_game = None;
        {
            let sections = self.sections.lock().unwrap();
            for (_, section_data) in sections.iter() {
                if let Some(g) = section_data.games.iter().find(|g| g.id == game_id) {
                    target_game = Some(g.clone());
                    break;
                }
            }
        }
        
        let game = match target_game {
            Some(g) => g,
            None => {
                ui.label(egui::RichText::new("Jogo não encontrado.").color(theme::DANGER));
                return;
            }
        };

        // Solicita a imagem se necessário
        let style = crate::ui::widgets::CardStyle::Tall;
        self.spawn_cover_download(&game, ui.ctx(), style);

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            
            // Botão voltar
            if ui.add(
                egui::Button::new(
                    egui::RichText::new("⬅ Voltar")
                        .font(egui::FontId::proportional(16.0))
                        .color(theme::TEXT_PRIMARY)
                )
                .fill(theme::SURFACE)
                .rounding(theme::CARD_ROUNDING)
            ).clicked() {
                self.screen = Screen::Home;
            }
        });

        ui.add_space(24.0);

        ui.horizontal(|ui| {
            ui.add_space(24.0);
            
            // Capa Esquerda
            let cache_key = (game.id.clone(), style);
            let cover_state = {
                let map = self.covers.lock().unwrap();
                map.get(&cache_key).cloned().unwrap_or(CoverState::Loading)
            };
            match cover_state {
                CoverState::Ready(texture) => {
                    ui.add(
                        egui::Image::new(&texture)
                            .max_height(400.0)
                            .rounding(theme::CARD_ROUNDING)
                    );
                }
                CoverState::Loading => {
                    let cover_size = egui::Vec2::new(300.0, 400.0);
                    let (cover_rect, _) = ui.allocate_exact_size(cover_size, egui::Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(cover_rect, theme::CARD_ROUNDING, theme::ACCENT);
                    painter.text(
                        cover_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Carregando capa...",
                        egui::FontId::proportional(16.0),
                        theme::TEXT_PRIMARY,
                    );
                }
                CoverState::Failed => {
                    let cover_size = egui::Vec2::new(300.0, 400.0);
                    let (cover_rect, _) = ui.allocate_exact_size(cover_size, egui::Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(cover_rect, theme::CARD_ROUNDING, theme::SURFACE);
                    painter.text(
                        cover_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Sem capa",
                        egui::FontId::proportional(16.0),
                        theme::TEXT_SECONDARY,
                    );
                }
            }

            ui.add_space(40.0);

            // Informações Direita
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(&game.title)
                        .font(egui::FontId::proportional(48.0))
                        .color(theme::TEXT_PRIMARY)
                        .strong(),
                );
                
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Nuvem • Otimizado para Xbox Series X|S")
                        .color(theme::TEXT_SECONDARY)
                        .font(egui::FontId::proportional(16.0))
                );

                ui.add_space(32.0);

                // Botão Jogar Gigante
                let play_btn = egui::Button::new(
                    egui::RichText::new("JOGAR")
                        .font(egui::FontId::proportional(24.0))
                        .color(egui::Color32::WHITE)
                        .strong()
                )
                .fill(theme::ACCENT)
                .rounding(theme::CARD_ROUNDING);

                if ui.add_sized([200.0, 60.0], play_btn).clicked() {
                    tracing::info!("🎮 Iniciando stream para: {} ({})", game.title, game.id);
                    // TODO: Iniciar streaming (Fase 2)
                }

                ui.add_space(40.0);

                // Estatísticas Mockadas
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("42")
                                .font(egui::FontId::proportional(32.0))
                                .color(theme::TEXT_PRIMARY)
                                .strong()
                        );
                        ui.label(
                            egui::RichText::new("Horas jogadas")
                                .color(theme::TEXT_SECONDARY)
                        );
                    });
                    
                    ui.add_space(48.0);
                    
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("3")
                                .font(egui::FontId::proportional(32.0))
                                .color(theme::TEXT_PRIMARY)
                                .strong()
                        );
                        ui.label(
                            egui::RichText::new("Amigos jogando")
                                .color(theme::TEXT_SECONDARY)
                        );
                    });
                });

                ui.add_space(40.0);
                
                // Descrição mockada
                ui.label(
                    egui::RichText::new("Jogue no modo multijogador com seus amigos e aproveite uma experiência sem igual pela nuvem do Xbox Cloud Gaming.")
                        .color(theme::TEXT_SECONDARY)
                        .font(egui::FontId::proportional(16.0))
                );
            });
        });
    }
}
