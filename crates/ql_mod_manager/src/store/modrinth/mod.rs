use std::{
    collections::HashSet,
    sync::{Arc, mpsc::Sender},
    time::Instant,
};

use chrono::DateTime;
use download::version_sort;
use indexmap::IndexMap;
use info::ProjectInfo;
use ql_core::{GenericProgress, Instance, Loader, download, pt};
use serde::Deserialize;
use versions::ModVersion;

use crate::{
    rate_limiter::{RATE_LIMITER, lock},
    store::{Category, ModId, QueryType, SearchMod, StoreBackendType, types::GalleryItem},
};

use super::{Backend, CurseforgeNotAllowed, ModError, Query, SearchResult};

mod download;
mod info;
mod search;
mod versions;

pub struct ModrinthBackend;

impl Backend for ModrinthBackend {
    async fn search(query: Query, offset: usize) -> Result<SearchResult, ModError> {
        RATE_LIMITER.lock().await;
        let instant = Instant::now();

        let res = search::do_request(&query, offset).await?;
        let reached_end = res.hits.len() < res.limit;

        let res = SearchResult {
            mods: res
                .hits
                .into_iter()
                .map(|entry| SearchMod {
                    title: entry.title,
                    description: entry.description,
                    downloads: entry.downloads,
                    internal_name: entry.slug,
                    project_type: entry.project_type,
                    id: entry.project_id,
                    icon_url: entry.icon_url,
                    backend: StoreBackendType::Modrinth,
                    gallery: entry
                        .gallery
                        .into_iter()
                        .map(|url| GalleryItem {
                            url,
                            title: None,
                            description: None,
                        })
                        .collect(),
                    urls: Vec::new(),
                })
                .collect(),
            start_time: instant,
            backend: StoreBackendType::Modrinth,
            offset,
            reached_end,
        };

        Ok(res)
    }

    async fn get_description(id: &str) -> Result<(ModId, String), ModError> {
        let info = ProjectInfo::download(id).await?;
        Ok((ModId::Modrinth(info.id), info.body))
    }

    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Loader,
    ) -> Result<(DateTime<chrono::FixedOffset>, String), ModError> {
        let download_info = ModVersion::download(id).await?;
        let version = version.to_owned();

        let mut download_versions: Vec<ModVersion> = download_info
            .iter()
            .filter(|v| v.game_versions.contains(&version))
            .filter(|v| {
                loader.is_vanilla()
                    || v.loaders.first().is_none_or(|n| n == "minecraft") // ?
                    || v.loaders.contains(&loader.to_modrinth_str().to_owned())
            })
            .cloned()
            .collect();

        // Sort by date published
        download_versions.sort_by(version_sort);

        let download_version =
            download_versions
                .into_iter()
                .next_back()
                .ok_or(ModError::NoCompatibleVersionFound(
                    download_info
                        .first()
                        .map(|n| n.name.clone())
                        .unwrap_or_default(),
                ))?;

        let download_version_time = DateTime::parse_from_rfc3339(&download_version.date_published)?;

        Ok((download_version_time, download_version.version_number))
    }

    async fn download(
        id: &str,
        instance: &Instance,
        sender: Option<Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        let _guard = lock().await;

        let mut downloader = download::ModDownloader::new(instance, sender).await?;
        downloader.download(Arc::from(id), None, true).await?;

        downloader.index.save(instance).await?;

        pt!("Finished");

        Ok(HashSet::new())
    }

    async fn download_bulk(
        ids: &[Arc<str>],
        instance: &Instance,
        ignore_incompatible: bool,
        set_manually_installed: bool,
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        let _guard = lock().await;

        let mut downloader = download::ModDownloader::new(instance, None).await?;
        let bulk_info = ProjectInfo::download_bulk(ids).await?;

        downloader
            .info
            .extend(bulk_info.into_iter().map(|n| (n.id.clone(), n)));

        let len = ids.len();

        for (i, id) in ids.iter().enumerate() {
            if let Some(sender) = &sender {
                _ = sender.send(GenericProgress {
                    done: i,
                    total: len,
                    message: downloader
                        .info
                        .get(id)
                        .map(|n| format!("Downloading mod: {}", n.title)),
                    has_finished: false,
                });
            }

            let result = downloader.download(id.clone(), None, true).await;
            if let Err(ModError::NoCompatibleVersionFound(name)) = &result {
                if ignore_incompatible {
                    pt!("No compatible version found for mod {name} ({id}), skipping...");
                    continue;
                }
            }
            result?;

            if set_manually_installed {
                if let Some(config) = downloader.index.mods.get_mut(&ModId::Modrinth(id.clone())) {
                    config.manually_installed = true;
                }
            }
        }

        downloader.index.save(instance).await?;

        pt!("Finished");
        if let Some(sender) = &sender {
            _ = sender.send(GenericProgress::finished());
        }

        Ok(HashSet::new())
    }

    async fn get_categories(kind: super::QueryType) -> Result<Vec<Category>, ModError> {
        #[derive(Deserialize, Clone)]
        struct MCategory {
            name: String,
            project_type: String,
            header: String,
        }

        static CACHE: tokio::sync::OnceCell<Vec<MCategory>> = tokio::sync::OnceCell::const_new();

        let mcategories = CACHE
            .get_or_try_init(|| async {
                download("https://api.modrinth.com/v2/tag/category")
                    .json()
                    .await
            })
            .await?;
        let kind_str = kind.to_modrinth_str();

        let mut map: IndexMap<String, Vec<Category>> = IndexMap::new();
        for cat in mcategories.iter().filter(|n| n.project_type == kind_str) {
            let category = Category {
                name: slug_to_nice_name(&cat.name),
                slug: cat.name.clone(),
                children: Vec::new(),
                internal_id: None,
                is_usable: true,
            };
            match map.get_mut(&cat.header) {
                Some(n) => n.push(category),
                None => {
                    map.insert(cat.header.clone(), vec![category]);
                }
            }
        }

        Ok(if map.len() == 1 {
            map.into_iter().next().expect("len should be equal to 1").1
        } else {
            map.into_iter()
                .map(|(header, children)| Category {
                    name: slug_to_nice_name(&header),
                    slug: header,
                    children,
                    internal_id: None,
                    is_usable: false,
                })
                .collect()
        })
    }

    async fn get_info(id: &str) -> Result<SearchMod, ModError> {
        let mut info = ProjectInfo::download(id).await?;
        info.gallery.sort_by_key(|a| a.ordering);

        Ok(SearchMod {
            urls: info.build_urls(),
            title: info.title,
            description: info.description,
            downloads: info.downloads,
            internal_name: info.slug,
            project_type: info.project_type,
            id: info.id,
            icon_url: info.icon_url,
            backend: StoreBackendType::Modrinth,
            gallery: info.gallery.into_iter().map(GalleryItem::from).collect(),
        })
    }

    async fn get_info_bulk(ids: &[Arc<str>]) -> Result<Vec<SearchMod>, ModError> {
        let infos = ProjectInfo::download_bulk(ids).await?;
        Ok(infos
            .into_iter()
            .map(|mut info| {
                info.gallery.sort_by_key(|a| a.ordering);
                SearchMod {
                    urls: info.build_urls(),
                    title: info.title,
                    description: info.description,
                    downloads: info.downloads,
                    internal_name: info.slug,
                    project_type: info.project_type,
                    id: info.id,
                    icon_url: info.icon_url,
                    backend: StoreBackendType::Modrinth,
                    gallery: info.gallery.into_iter().map(GalleryItem::from).collect(),
                }
            })
            .collect())
    }

    async fn get_download_link(
        instance: &Instance,
        id: &str,
        query_type: QueryType,
    ) -> Result<String, ModError> {
        let downloader = download::ModDownloader::basic(instance).await?;
        downloader.get_download_link(id, query_type).await
    }
}

pub fn slug_to_nice_name(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
