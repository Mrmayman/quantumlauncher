use std::{collections::HashSet, time::Instant};

use chrono::DateTime;
use download::version_sort;
use info::ProjectInfo;
use ql_core::{pt, GenericProgress, InstanceSelection, Loader, ModId};
use sipper::Sender;
use versions::ModVersion;

use crate::{
    rate_limiter::{lock, RATE_LIMITER},
    store::{SearchMod, StoreBackendType},
};

use super::{Backend, CurseforgeNotAllowed, ModError, Query, QueryType, SearchResult};

mod download;
mod info;
mod search;
mod versions;

pub struct ModrinthBackend;

impl Backend for ModrinthBackend {
    async fn search(
        query: Query,
        offset: usize,
        query_type: QueryType,
    ) -> Result<SearchResult, ModError> {
        RATE_LIMITER.lock().await;
        let instant = Instant::now();

        let res = search::do_request(&query, offset, query_type).await?;
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

        Ok((download_version_time, download_version.name))
    }

    async fn download(
        id: &str,
        instance: &InstanceSelection,
        sender: Option<Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        let _guard = lock().await;

        let mut downloader = download::ModDownloader::new(instance, sender).await?;
        downloader.download(id, None, true).await?;

        downloader.index.save(instance).await?;

        pt!("Finished");

        Ok(HashSet::new())
    }

    async fn download_bulk(
        ids: &[String],
        instance: &InstanceSelection,
        ignore_incompatible: bool,
        set_manually_installed: bool,
        sender: Option<&mut Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        let _guard = lock().await;

        let mut downloader = download::ModDownloader::new(instance, None).await?;
        let bulk_info = ProjectInfo::download_bulk(ids).await?;

        downloader
            .info
            .extend(bulk_info.into_iter().map(|n| (n.id.clone(), n)));

        let len = ids.len();

        for (i, id) in ids.iter().enumerate() {
            if let Some(mut sender) = sender.as_ref().map(|n| (*n).clone()) {
                sender
                    .send(GenericProgress {
                        done: i,
                        total: len,
                        message: downloader
                            .info
                            .get(id)
                            .map(|n| format!("Downloading mod: {}", n.title)),
                        has_finished: false,
                    })
                    .await;
            }

            let result = downloader.download(id, None, true).await;
            if let Err(ModError::NoCompatibleVersionFound(name)) = &result {
                if ignore_incompatible {
                    pt!("No compatible version found for mod {name} ({id}), skipping...");
                    continue;
                }
            }
            result?;

            if set_manually_installed {
                if let Some(config) = downloader.index.mods.get_mut(id) {
                    config.manually_installed = true;
                }
            }
        }

        downloader.index.save(instance).await?;

        pt!("Finished");
        if let Some(sender) = sender {
            sender.send(GenericProgress::finished()).await;
        }

        Ok(HashSet::new())
    }
}
