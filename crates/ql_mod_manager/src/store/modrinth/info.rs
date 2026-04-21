use ql_core::{err, file_utils};
use serde::Deserialize;
use std::fmt::Write;

use crate::{rate_limiter::RATE_LIMITER, store::types::UrlKind};

use super::ModError;

#[derive(Deserialize, Debug, Clone)]
pub struct ProjectInfo {
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub loaders: Vec<String>,
    pub id: String,
    pub body: String,
    pub project_type: String,
    pub slug: String,
    // pub categories: Vec<String>,
    // pub client_side: String,
    // pub server_side: String,
    // pub status: String,
    // pub requested_status: Option<String>,
    // pub additional_categories: Vec<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
    pub wiki_url: Option<String>,
    pub discord_url: Option<String>,
    pub donation_urls: Vec<DonationLink>,
    pub downloads: usize,
    // pub color: Option<usize>,
    // pub thread_id: Option<String>,
    // pub monetization_status: Option<String>,
    // pub team: String,
    // pub published: String,
    // pub updated: String,
    // pub approved: Option<String>,
    // pub followers: usize,
    // pub license: License,
    // pub versions: Vec<String>,
    // pub game_versions: Vec<String>,
    pub gallery: Vec<MGallery>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MGallery {
    pub url: String,
    // pub featured: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    // pub created: String,
    pub ordering: i64,
}

impl From<MGallery> for crate::store::types::GalleryItem {
    fn from(value: MGallery) -> Self {
        Self {
            url: value.url,
            title: value.title,
            description: value.description,
        }
    }
}

impl ProjectInfo {
    pub async fn download(id: &str) -> Result<Self, ModError> {
        RATE_LIMITER.lock().await;
        let url = format!("https://api.modrinth.com/v2/project/{id}");
        let file: Self = match file_utils::download_file_to_json(&url, true).await {
            Ok(file) => file,
            Err(err) => {
                err!("Could not parse mod project json from url: {url}");
                return Err(err.into());
            }
        };
        Ok(file)
    }

    pub async fn download_bulk(ids: &[String]) -> Result<Vec<Self>, ModError> {
        RATE_LIMITER.lock().await;
        let mut url = "https://api.modrinth.com/v2/projects?ids=[".to_owned();
        let len = ids.len();
        for (i, id) in ids.iter().enumerate() {
            _ = write!(url, "{id:?}");
            if i + 1 < len {
                url.push_str(", ");
            }
        }
        url.push(']');

        Ok(file_utils::download_file_to_json(&url, false).await?)
    }

    pub fn build_urls(&self) -> Vec<(UrlKind, String)> {
        let mut urls = Vec::new();
        if let Some(issues) = &self.issues_url {
            urls.push((UrlKind::Issues, issues.clone()));
        }
        if let Some(source) = &self.source_url {
            urls.push((UrlKind::Source, source.clone()));
        }
        if let Some(wiki) = &self.wiki_url {
            urls.push((UrlKind::Wiki, wiki.clone()));
        }
        if let Some(discord) = &self.discord_url {
            urls.push((UrlKind::Discord, discord.clone()));
        }
        for donation in &self.donation_urls {
            urls.push((
                UrlKind::Donation(donation.platform.clone()),
                donation.url.clone(),
            ));
        }
        urls
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct DonationLink {
    // pub id: String,
    pub platform: String,
    pub url: String,
}

/*#[derive(Deserialize, Debug, Clone)]
pub struct License {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}*/
