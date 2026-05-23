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
    screen: Screen,

    /// Query da caixa de busca
    search_query: String,

    /// Nome do usuário logado (do XSTS token)
    username: String,

    /// Dados das seções (carregados assincronamente)
    sections: SharedSections,

    /// Cache de texturas de cover art: game_id → TextureHandle
    covers: HashMap<String, CoverState>,

    /// Ordem de exibição das seções
    section_order: Vec<Section>,

    /// Jogos filtrados pela busca (atualizado a cada frame com query)
    search_results: Vec<Game>,

    /// Runtime tokio para disparar tarefas de background
    runtime: tokio::runtime::Handle,

    /// Flag de carregamento inicial já disparado
    initial_load_done: bool,

    /// Canal para receber capas baixadas na thread de background
    cover_rx: std::sync::mpsc::Receiver<(String, Result<egui::ColorImage, String>)>,
    cover_tx: std::sync::mpsc::Sender<(String, Result<egui::ColorImage, String>)>,

    /// Cliente HTTP (com User-Agent) para todas as requisições
    client: reqwest::Client,

    /// Semáforo para limitar a concorrência no download das imagens
    image_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
}

impl XCloudApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        username: String,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        // Aplica o tema Xbox
        theme::apply(&cc.egui_ctx);

        // Configura fonte customizada (sistema)
        let fonts = egui::FontDefinitions::default();
        // Tenta adicionar fonte do sistema para melhor renderização de emojis
        cc.egui_ctx.set_fonts(fonts);

        let sections: SharedSections = Arc::new(Mutex::new(HashMap::new()));

        // Inicializa as seções com estado de "loading"
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

        let (cover_tx, cover_rx) = std::sync::mpsc::channel();
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        Self {
            screen: Screen::Home,
            search_query: String::new(),
            username,
            sections,
            covers: HashMap::new(),
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
            cover_rx,
            cover_tx,
            client,
            image_semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(10)),
        }
    }

    /// Dispara o carregamento assíncrono das seções do catálogo
    fn start_catalog_load(&mut self, ctx: egui::Context) {
        let sections_clone = std::sync::Arc::clone(&self.sections);
        let client = self.client.clone();

        self.runtime.spawn(async move {
            use crate::ui::catalog::*;

            // Como os IDs de SIGL individuais podem dar 404 dependendo da região,
            // puxamos a lista mestre (SIGL_ALL) e fazemos a distribuição local.
            let all_res = fetch_section(&client, SIGL_ALL, 500).await;

            let mut map = sections_clone.lock().unwrap();

            if let Ok(mut all_games) = all_res {
                let len = all_games.len();
                if len > 0 {
                    // Recém adicionados: Ordenar por release_date (mais recente primeiro)
                    let mut novos = all_games.clone();
                    novos.sort_by(|a, b| {
                        b.release_date
                            .as_ref()
                            .unwrap_or(&"".to_string())
                            .cmp(a.release_date.as_ref().unwrap_or(&"".to_string()))
                    });
                    
                    let entry = map.entry(Section::NewlyAdded.title().to_string()).or_default();
                    entry.games = novos.into_iter().take(40).collect();
                    entry.loading = false;

                    // Populares: Os primeiros itens de SIGL_ALL costumam ser bem populares
                    // A lista mestre já vem com certa curadoria.
                    let entry_pop = map.entry(Section::Popular.title().to_string()).or_default();
                    entry_pop.games = all_games.iter().take(40).cloned().collect();
                    entry_pop.loading = false;

                    // Saindo em breve: Embaralhar os últimos jogos
                    let entry_leave = map.entry(Section::Leaving.title().to_string()).or_default();
                    let mut leaving: Vec<_> = all_games.iter().skip(len.saturating_sub(60)).take(20).cloned().collect();
                    leaving.reverse();
                    entry_leave.games = leaving;
                    entry_leave.loading = false;

                    // Todos os jogos (ordem alfabética original)
                    let entry_all = map.entry(Section::AllGames.title().to_string()).or_default();
                    all_games.sort_by(|a, b| a.title.cmp(&b.title));
                    entry_all.games = all_games;
                    entry_all.loading = false;
                }
            } else {
                for section in &[Section::AllGames, Section::NewlyAdded, Section::Popular, Section::Leaving] {
                    let entry = map.entry(section.title().to_string()).or_default();
                    entry.error = Some("Erro ao carregar catálogo".into());
                    entry.loading = false;
                }
            }

            // Recém jogados ainda mockado
            let entry_recent = map.entry(Section::RecentlyPlayed.title().to_string()).or_default();
            entry_recent.loading = false;

            // Solicita repintura ao egui para exibir os dados novos
            ctx.request_repaint();
        });
    }

    /// Filtra jogos de todas as seções pela query de busca
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

    /// Dispara o download de uma capa se ela ainda não estiver em cache
    fn spawn_cover_download(&mut self, game: &Game, ctx: &egui::Context) {
        if !self.covers.contains_key(&game.id) {
            self.covers.insert(game.id.clone(), CoverState::Loading);
            if let Some(url) = game.cover_url.clone() {
                let tx = self.cover_tx.clone();
                let game_id = game.id.clone();
                let ctx_clone = ctx.clone();
                let client = self.client.clone();
                let semaphore = self.image_semaphore.clone();
                
                self.runtime.spawn(async move {
                    // Limita concorrência para evitar 429 / 403 da Akamai
                    let _permit = semaphore.acquire().await.unwrap();
                    
                    let img_res = match client.get(&url).send().await {
                        Ok(resp) => {
                            if let Ok(bytes) = resp.bytes().await {
                                if let Ok(img) = image::load_from_memory(&bytes) {
                                    let size = [img.width() as _, img.height() as _];
                                    let image_buffer = img.to_rgba8();
                                    let pixels = image_buffer.as_flat_samples();
                                    Ok(egui::ColorImage::from_rgba_unmultiplied(
                                        size,
                                        pixels.as_slice(),
                                    ))
                                } else {
                                    eprintln!("Decode error for: {}", url);
                                    Err("Decode err".into())
                                }
                            } else {
                                eprintln!("Bytes error for: {}", url);
                                Err("Bytes err".into())
                            }
                        }
                        Err(e) => {
                            eprintln!("Request error for {}: {:?}", url, e);
                            Err("Fetch err".into())
                        }
                    };
                    let _ = tx.send((game_id, img_res));
                    ctx_clone.request_repaint();
                });
            } else {
                self.covers.insert(game.id.clone(), CoverState::Failed);
            }
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

        // Processa imagens baixadas
        while let Ok((game_id, result)) = self.cover_rx.try_recv() {
            match result {
                Ok(color_image) => {
                    let texture = ctx.load_texture(&game_id, color_image, Default::default());
                    self.covers.insert(game_id, CoverState::Ready(texture));
                }
                Err(_) => {
                    self.covers.insert(game_id, CoverState::Failed);
                }
            }
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
                            Screen::GameDetail(id) => self.render_game_detail(ui, &id.clone()),
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
                    let is_leaving = *section == Section::Leaving;

                    egui::ScrollArea::horizontal()
                        .id_salt(&section_title)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.spacing_mut().item_spacing.x = theme::CARD_GAP;
                                
                                let limit = if *section == Section::AllGames { 40 } else { 20 };
                                for game in games.iter().take(limit) {
                                    self.spawn_cover_download(game, ui.ctx());

                                    let cover = self.covers.get(&game.id).unwrap_or(&CoverState::Loading);

                                    if widgets::game_card(ui, &game.title, cover, is_leaving) {
                                        self.screen = Screen::GameDetail(game.id.clone());
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

        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(format!(
                "Resultados para \"{}\" — {} jogos",
                self.search_query,
                results.len()
            ))
            .font(theme::section_title_font())
            .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(12.0);

        // Grade de resultados (wrap automático)
        egui::Grid::new("search_results")
            .spacing([theme::CARD_GAP, theme::CARD_GAP])
            .show(ui, |ui| {
                for (idx, game) in results.iter().enumerate() {
                    self.spawn_cover_download(game, ui.ctx());

                    let cover = self.covers.get(&game.id).unwrap_or(&CoverState::Loading);

                    if widgets::game_card(ui, &game.title, cover, false) {
                        self.screen = Screen::GameDetail(game.id.clone());
                    }

                    // 5 cards por linha
                    if (idx + 1) % 5 == 0 {
                        ui.end_row();
                    }
                }
            });
    }

    fn render_game_detail(&mut self, ui: &mut egui::Ui, game_id: &str) {
        // Busca o jogo em qualquer seção
        let game = {
            let map = self.sections.lock().unwrap();
            map.values()
                .flat_map(|s| s.games.iter().cloned())
                .find(|g| g.id == game_id)
        };

        if ui
            .add(egui::Button::new(
                egui::RichText::new("← Voltar").color(theme::ACCENT),
            ))
            .clicked()
        {
            self.screen = Screen::Home;
        }

        ui.add_space(20.0);

        if let Some(game) = game {
            self.spawn_cover_download(&game, ui.ctx());

            ui.horizontal(|ui| {
                // Cover art grande
                let cover = self.covers.get(&game.id).unwrap_or(&CoverState::Loading);
                let cover_size = egui::Vec2::new(240.0, 320.0);
                let (cover_rect, _) = ui.allocate_exact_size(cover_size, egui::Sense::hover());

                let painter = ui.painter();
                match cover {
                    CoverState::Ready(texture) => {
                        egui::Image::new(texture)
                            .fit_to_exact_size(cover_size)
                            .paint_at(ui, cover_rect);
                    }
                    _ => {
                        painter.rect_filled(cover_rect, theme::CARD_ROUNDING, theme::SURFACE);
                        painter.text(
                            cover_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "🎮",
                            egui::FontId::proportional(64.0),
                            theme::TEXT_SECONDARY,
                        );
                    }
                }

                ui.add_space(24.0);

                // Informações do jogo
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(&game.title)
                            .font(egui::FontId::proportional(28.0))
                            .color(theme::TEXT_PRIMARY)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("☁️  Disponível via xCloud").color(theme::ACCENT));
                    ui.add_space(24.0);

                    // Botão de jogar
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("  ▶  Jogar na Nuvem  ")
                                    .font(egui::FontId::proportional(16.0))
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(theme::ACCENT)
                            .rounding(theme::BUTTON_ROUNDING)
                            .min_size(egui::Vec2::new(200.0, 48.0)),
                        )
                        .clicked()
                    {
                        // TODO: iniciar sessão de streaming para este jogo
                        tracing::info!("🎮 Iniciando stream para: {} ({})", game.title, game.id);
                    }
                });
            });
        } else {
            ui.label(egui::RichText::new("Jogo não encontrado.").color(theme::DANGER));
        }
    }
}
