//! # Catalog Client — Game Pass Catalog API
//!
//! Consome as APIs públicas e autenticadas do Xbox Game Pass para
//! buscar as listas de jogos exibidas no launcher.
//!
//! ## Endpoints usados
//!
//! - SIGL (listas curadas pela Microsoft): público, sem auth
//! - Display Catalog (detalhes + cover art): público
//! - Title History (continuar jogando): requer XSTS token (Fase 1)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Identificadores (SIGL) do Catálogo (Xbox Cloud Gaming)
// Extraídos da página oficial pt-BR/play
// ---------------------------------------------------------------------------

/// Jogos recém adicionados ao Game Pass / xCloud
pub const SIGL_NEW: &str = "06323672-b8c8-43cc-b0de-32d5a9834749";

/// Jogos mais populares na nuvem
pub const SIGL_POPULAR: &str = "6a589fa0-d493-472b-8e20-3813699d7056";

/// Jogos saindo do catálogo em breve
pub const SIGL_LEAVING: &str = "31ff2361-2772-4622-849b-f4f1abb4ad1b";

/// Todos os jogos disponíveis (Catálogo Completo incluindo Own-to-Play)
pub const SIGL_ALL: &str = "1bf84c2b-0643-4591-893f-d9edb703f692";

// ---------------------------------------------------------------------------
// Tipos de dados
// ---------------------------------------------------------------------------

/// Representa um jogo no catálogo do Game Pass
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    /// ID único do produto na Microsoft Store
    pub id: String,

    /// Título do jogo
    pub title: String,
    pub cover_url: Option<String>,
    pub box_art_url: Option<String>,
    pub hero_art_url: Option<String>,
    pub poster_url: Option<String>,
    pub is_leaving: bool,
    pub leaving_date: Option<String>,

    /// Plataformas disponíveis
    pub platforms: Vec<String>,

    /// Disponível via xCloud (cloud gaming)
    pub cloud_available: bool,

    /// Data de lançamento (para ordenar recém-adicionados)
    pub release_date: Option<String>,
}

/// Resposta da API SIGL com lista de IDs de jogos
#[derive(Debug, Deserialize)]
struct SiglResponse {
    #[serde(rename = "Items")]
    items: Vec<SiglItem>,
}

#[derive(Debug, Deserialize)]
struct SiglItem {
    #[serde(rename = "Id")]
    id: String,
}

/// Resposta do Display Catalog com detalhes dos jogos
#[derive(Debug, Deserialize)]
struct CatalogResponse {
    #[serde(rename = "Products")]
    products: Vec<CatalogProduct>,
}

#[derive(Debug, Deserialize)]
struct CatalogProduct {
    #[serde(rename = "ProductId")]
    product_id: String,

    #[serde(rename = "LocalizedProperties")]
    localized_properties: Vec<LocalizedProperty>,

    #[serde(rename = "Properties")]
    properties: Option<ProductProperties>,

    #[serde(rename = "MarketProperties")]
    market_properties: Option<Vec<MarketProperty>>,

    #[serde(rename = "DisplaySkuAvailabilities")]
    #[serde(default)]
    display_sku_availabilities: Vec<DisplaySkuAvailability>,
}

#[derive(Debug, Deserialize)]
struct DisplaySkuAvailability {
    #[serde(rename = "Availabilities")]
    #[serde(default)]
    availabilities: Vec<Availability>,
}

#[derive(Debug, Deserialize)]
struct Availability {
    #[serde(rename = "Conditions")]
    conditions: Conditions,
}

#[derive(Debug, Deserialize)]
struct Conditions {
    #[serde(rename = "EndDate")]
    end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MarketProperty {
    #[serde(rename = "OriginalReleaseDate")]
    original_release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalizedProperty {
    #[serde(rename = "ProductTitle")]
    product_title: String,

    #[serde(rename = "Images")]
    images: Vec<ProductImage>,
}

#[derive(Debug, Deserialize)]
struct ProductImage {
    #[serde(rename = "ImagePurpose")]
    image_purpose: String,

    #[serde(rename = "Uri")]
    uri: String,

    #[serde(rename = "Width")]
    width: Option<u32>,

    #[serde(rename = "Height")]
    height: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ProductProperties {
    #[serde(rename = "XboxConsoleGenCompatible")]
    xbox_compatible: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Função de busca de lista SIGL
// ---------------------------------------------------------------------------

/// Busca os IDs de uma lista curada da Microsoft (SIGL).
/// Retorna apenas os IDs — os detalhes são buscados em lote separado.
pub async fn fetch_sigl_ids(client: &reqwest::Client, sigl_id: &str) -> Result<Vec<String>> {
    let url = format!(
        "https://catalog.gamepass.com/sigls/v2?id={}&language=pt-BR&market=BR",
        sigl_id
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .context("Falha ao contactar o Game Pass Catalog")?;

    if !response.status().is_success() {
        anyhow::bail!("SIGL API retornou {}: {}", response.status(), sigl_id);
    }

    // A resposta é um array direto de objetos {Id: "..."}
    let items: Vec<HashMap<String, serde_json::Value>> = response
        .json()
        .await
        .context("Falha ao parsear resposta do SIGL")?;

    let ids: Vec<String> = items
        .into_iter()
        .filter_map(|item| {
            item.get("id")
                .or_else(|| item.get("Id"))
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .collect();

    Ok(ids)
}

// ---------------------------------------------------------------------------
// Busca de detalhes + cover art
// ---------------------------------------------------------------------------

/// Busca detalhes de até 20 jogos em lote pelo Display Catalog da Microsoft.
/// Retorna uma lista de `Game` com título e URL de cover art.
pub async fn fetch_game_details(client: &reqwest::Client, ids: &[String]) -> Result<Vec<Game>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }

    // A API aceita até ~20 IDs por requisição
    let ids_joined = ids.join(",");
    let url = format!(
        "https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds={}&market=BR&languages=pt-BR&MS-CV=DGU1mcuYo0WMMp.0",
        ids_joined
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .context("Falha ao contactar o Display Catalog")?;

    if !response.status().is_success() {
        anyhow::bail!("Display Catalog retornou {}", response.status());
    }

    let catalog: CatalogResponse = response
        .json()
        .await
        .context("Falha ao parsear Display Catalog")?;

    let games: Vec<Game> = catalog
        .products
        .into_iter()
        .filter_map(|product| {
            let localized = product.localized_properties.into_iter().next()?;
            let title = localized.product_title;
            if title.is_empty() {
                return None;
            }

            // Extração de capas específicas
            let get_image = |purpose: &str| -> Option<String> {
                localized.images.iter().find(|img| img.image_purpose == purpose).map(|img| {
                    let mut url = img.uri.clone();
                    if url.starts_with("//") {
                        url = format!("https:{}", url);
                    }
                    format!("{}?w=320&h=426&q=80", url)
                })
            };

            let box_art_url = get_image("BoxArt");
            let hero_art_url = get_image("TitledHeroArt");
            let poster_url = get_image("Poster");

            // Fallback para cover_url genérica (TitledHeroArt -> BoxArt -> Poster)
            let cover_url = hero_art_url.clone().or_else(|| box_art_url.clone()).or_else(|| poster_url.clone()).or_else(|| {
                localized.images.first().map(|img| {
                    let mut url = img.uri.clone();
                    if url.starts_with("//") {
                        url = format!("https:{}", url);
                    }
                    format!("{}?w=320&h=426&q=80", url)
                })
            });

            let release_date = product
                .market_properties
                .as_ref()
                .and_then(|props| props.first())
                .and_then(|p| p.original_release_date.clone());

            let mut leaving_date = None;
            if let Some(display_sku) = product.display_sku_availabilities.first() {
                if let Some(avail) = display_sku.availabilities.first() {
                    if let Some(end_date) = &avail.conditions.end_date {
                        if end_date != "9998-12-30T00:00:00.0000000Z" {
                            leaving_date = Some(end_date.clone());
                        }
                    }
                }
            }

            Some(Game {
                id: product.product_id,
                title,
                cover_url,
                box_art_url,
                hero_art_url,
                poster_url,
                platforms: vec!["Cloud".to_string()],
                cloud_available: true,
                release_date,
                is_leaving: false,
                leaving_date,
            })
        })
        .collect();

    Ok(games)
}

// ---------------------------------------------------------------------------
// Carrega uma seção completa (SIGL + detalhes) com limite de itens
// ---------------------------------------------------------------------------

/// Busca uma seção completa: IDs via SIGL + detalhes via Display Catalog.
/// Limita ao `max_items` primeiros resultados para não sobrecarregar a UI.
pub async fn fetch_section(
    client: &reqwest::Client,
    sigl_id: &str,
    max_items: usize,
) -> Result<Vec<Game>> {
    let ids = fetch_sigl_ids(client, sigl_id).await?;
    let ids_limited: Vec<String> = ids.into_iter().take(max_items).collect();

    // Busca em lotes de 20 (limite da API)
    let mut all_games = Vec::new();
    for chunk in ids_limited.chunks(20) {
        let games = fetch_game_details(client, chunk).await?;
        all_games.extend(games);
    }

    Ok(all_games)
}
