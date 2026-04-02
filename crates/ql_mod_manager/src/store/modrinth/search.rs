use std::collections::BTreeMap;

use ql_core::IntoJsonError;
use serde::Deserialize;

use crate::store::{ModError, Query, QueryType};

pub async fn do_request(query: &Query, offset: usize) -> Result<Search, ModError> {
    const SEARCH_URL: &str = "https://api.modrinth.com/v2/search";

    let mut params = BTreeMap::from([
        ("index", "relevance".to_owned()),
        ("limit", "100".to_owned()),
        ("offset", offset.to_string()),
    ]);
    if !query.name.is_empty() {
        params.insert("query", query.name.clone());
    }

    let mut filters = vec![
        vec![format!("project_type:{}", query.kind.to_modrinth_str())],
        vec![format!("versions:{}", query.version)],
    ];

    if let QueryType::Mods | QueryType::ModPacks = query.kind {
        if !query.loader.is_vanilla() {
            filters.push(vec![format!(
                "categories:'{}'",
                query.loader.to_modrinth_str()
            )]);
        }
    }
    if query.open_source {
        filters.push(vec!["open_source:true".to_owned()]);
    }
    if !query.categories.is_empty() {
        let iter = query
            .categories
            .iter()
            .map(|c| format!("categories:{}", c.slug));
        if query.categories_use_all {
            // Each element has their own vec![]
            filters.extend(iter.map(|n| vec![n]));
        } else {
            // All in single vec![] inside filters
            filters.push(iter.collect());
        }
    }

    let filters = serde_json::to_string(&filters).json_to()?;
    params.insert("facets", filters);

    let text = ql_core::CLIENT
        .get(SEARCH_URL)
        .query(&params)
        .send()
        .await?
        .text()
        .await?;

    let json: Search = match serde_json::from_str(&text) {
        Ok(json) => json,
        Err(e) => {
            #[derive(Deserialize)]
            struct Error {
                error: String,
                description: String,
            }

            if let Ok(error) = serde_json::from_str::<Error>(&text) {
                return Err(ModError::ApiError {
                    error_id: error.error,
                    description: error.description,
                });
            }

            return Err(e).json(text).map_err(ModError::Json);
        }
    };

    Ok(json)
}

#[derive(Deserialize, Debug, Clone)]
pub struct Search {
    pub hits: Vec<Entry>,
    // pub offset: usize,
    pub limit: usize,
    // pub total_hits: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Entry {
    pub title: String,
    pub project_id: String,
    pub icon_url: Option<String>,
    pub description: String,
    pub downloads: usize,
    pub slug: String,
    pub project_type: String,
    // pub author: String,
    // pub categories: Vec<String>,
    // pub display_categories: Vec<String>,
    // pub versions: Vec<String>,
    // pub follows: usize,
    // pub date_created: String,
    // pub date_modified: String,
    // pub latest_version: String,
    // pub license: String,
    // pub client_side: String,
    // pub server_side: String,
    // pub featured_gallery: Option<String>,
    // pub color: Option<usize>,
    // pub thread_id: Option<String>,
    // pub monetization_status: Option<String>,
    #[serde(default)]
    pub gallery: Vec<String>, // URLs
}
