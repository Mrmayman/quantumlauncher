use std::{collections::HashSet, sync::Arc};

use chrono::DateTime;
use ql_core::file_utils;
use serde::Deserialize;

use crate::{rate_limiter::RATE_LIMITER, store::local_json::ModFile};

use super::ModError;

#[derive(Deserialize, Debug, Clone)]
pub struct ModVersion {
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub id: Arc<str>,
    // pub project_id: String,
    // pub author_id: String,
    // pub featured: bool,
    pub name: Arc<str>,
    pub version_number: String,
    // pub changelog: Option<String>,
    // pub changelog_url: Option<String>,
    pub date_published: DateTime<chrono::FixedOffset>,
    // pub downloads: usize,
    // pub version_type: String,
    // pub status: String,
    // pub requested_status: Option<String>,
    pub files: Vec<ModFile>,
    pub dependencies: Vec<Dependency>,
}

impl ModVersion {
    pub async fn download_all(project_id: &str) -> Result<Vec<Self>, ModError> {
        let mut versions = Self::download_page(project_id, 0).await?;
        let mut seen_ids: HashSet<Arc<str>> = versions.iter().map(|v| v.id.clone()).collect();
        let mut offset = versions.len();

        loop {
            let page = Self::download_page(project_id, offset).await?;
            if page.is_empty() {
                break;
            }

            let page_len = page.len();
            let previous_len = versions.len();
            versions.extend(page.into_iter().filter(|v| seen_ids.insert(v.id.clone())));
            offset += page_len;

            if versions.len() == previous_len {
                break;
            }
        }

        Ok(versions)
    }

    pub async fn download_page(project_id: &str, offset: usize) -> Result<Vec<Self>, ModError> {
        RATE_LIMITER.lock().await;
        let url = format!(
            "https://api.modrinth.com/v2/project/{project_id}/version?include_changelog=false&limit=100&offset={offset}"
        );
        Ok(file_utils::download_file_to_json(&url, true).await?)
    }

    pub async fn download_by_id(version_id: &str) -> Result<Self, ModError> {
        RATE_LIMITER.lock().await;
        let url = format!("https://api.modrinth.com/v2/version/{version_id}");
        Ok(file_utils::download_file_to_json(&url, true).await?)
    }

    // pub async fn is_compatible(
    //     project_id: &str,
    //     minecraft_version: &String,
    //     instance_loader: &Loader,
    // ) -> Result<bool, ModError> {
    //     let versions = Self::download(project_id).await?;
    //     Ok(versions.iter().any(|n| {
    //         n.game_versions.contains(minecraft_version)
    //             && n.loaders
    //                 .contains(&instance_loader.to_modrinth_str().to_owned())
    //     }))
    // }
}

#[derive(Deserialize, Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Dependency {
    // pub version_id: Option<serde_json::Value>,
    pub project_id: Option<Arc<str>>,
    // pub file_name: Option<serde_json::Value>,
    pub dependency_type: String,
}
