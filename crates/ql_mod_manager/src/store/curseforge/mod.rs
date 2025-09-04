use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicI32, mpsc::Sender},
    time::Instant,
};

use chrono::DateTime;
use download::ModDownloader;
use ql_core::{
    err, pt, GenericProgress, IntoJsonError, JsonDownloadError, ModId, RequestError, CLIENT,
};
use reqwest::header::HeaderValue;
use serde::Deserialize;

use crate::{rate_limiter::RATE_LIMITER, store::SearchMod};

use super::{Backend, CurseforgeNotAllowed, ModError, QueryType, SearchResult};
use categories::get_categories;
use ql_core::file_utils::check_for_success;

mod categories;
mod download;

const NOT_LOADED: i32 = -1;
pub static MC_ID: AtomicI32 = AtomicI32::new(NOT_LOADED);

#[derive(Deserialize, Clone, Debug)]
pub struct ModQuery {
    pub data: Mod,
}

impl ModQuery {
    pub async fn load(id: &str) -> Result<Self, JsonDownloadError> {
        let response = send_request(&format!("mods/{id}"), &HashMap::new()).await?;
        let response: ModQuery = serde_json::from_str(&response).json(response)?;
        Ok(response)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct Mod {
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub downloadCount: usize,
    pub logo: Option<Logo>,
    pub id: i32,
    pub latestFilesIndexes: Vec<CurseforgeFileIdx>,
    pub classId: i32,
    // latestFiles: Vec<CurseforgeFile>,
}

impl Mod {
    async fn get_file(
        &self,
        title: String,
        id: &str,
        version: String,
        loader: Option<&str>,
        query_type: QueryType,
    ) -> Result<(CurseforgeFileQuery, i32), ModError> {
        let Some(file) = (if let QueryType::Mods | QueryType::ModPacks = query_type {
            if let (Some(loader), true) = (
                loader,
                self.iter_files(version.clone())
                    .any(|n| n.modLoader.is_some()),
            ) {
                self.iter_files(version.clone())
                    .find(|n| {
                        if let Some(l) = n.modLoader.map(|n| n.to_string()) {
                            l == loader
                        } else {
                            false
                        }
                    })
                    .or_else(move || self.iter_files(version).next())
            } else {
                if loader.is_none() {
                    err!("You haven't installed a valid mod loader!");
                } else {
                    err!("Can't find a version of this mod compatible with your mod loader!");
                }
                pt!("Installing an arbitrary version anyway...");
                self.iter_files(version).next()
            }
        } else {
            self.iter_files(version).next().or_else(|| {
                err!("No exact compatible version found!\nPicking the closest one anyway");
                self.latestFilesIndexes.first()
            })
        }) else {
            return Err(ModError::NoCompatibleVersionFound(title));
        };

        let file_query = CurseforgeFileQuery::load(id, file.fileId).await?;

        Ok((file_query, file.fileId))
    }

    fn iter_files<'a>(&'a self, version: String) -> impl Iterator<Item = &'a CurseforgeFileIdx> {
        self.latestFilesIndexes
            .iter()
            .filter(move |n| n.gameVersion == version)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct CurseforgeFileIdx {
    // filename: String,
    gameVersion: String,
    fileId: i32,
    modLoader: Option<i32>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CurseforgeFileQuery {
    pub data: CurseforgeFile,
}

impl CurseforgeFileQuery {
    pub async fn load(mod_id: &str, file_id: i32) -> Result<Self, JsonDownloadError> {
        let response =
            send_request(&format!("mods/{mod_id}/files/{file_id}"), &HashMap::new()).await?;
        let response: Self = serde_json::from_str(&response).json(response)?;
        Ok(response)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct CurseforgeFile {
    pub fileName: String,
    pub downloadUrl: Option<String>,
    pub gameVersions: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub fileDate: String,
    pub displayName: String,
    pub fileLength: u64,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct Dependency {
    modId: usize,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Logo {
    url: String,
}

#[derive(Deserialize)]
struct CFSearchResult {
    data: Vec<Mod>,
}

impl CFSearchResult {
    async fn get_from_ids(ids: &[String]) -> Result<Self, ModError> {
        if ids.is_empty() {
            return Ok(Self { data: Vec::new() });
        }

        // Convert to JSON Array
        let ids: Vec<serde_json::Value> = ids
            .iter()
            .map(|s| s.parse::<u64>().map(serde_json::Value::from))
            .collect::<Result<_, _>>()?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(API_KEY).map_err(RequestError::from)?,
        );
        let response = CLIENT
            .post("https://api.curseforge.com/v1/mods")
            .headers(headers)
            .json(&serde_json::json!({"modIds" : ids}))
            .send()
            .await
            .map_err(RequestError::from)?;
        check_for_success(&response)?;
        let text = response.text().await.map_err(RequestError::from)?;
        Ok(serde_json::from_str(&text).json(text)?)
    }
}

pub struct CurseforgeBackend;

impl Backend for CurseforgeBackend {
    async fn search(
        query: super::Query,
        offset: usize,
        query_type: QueryType,
    ) -> Result<SearchResult, ModError> {
        const TOTAL_DOWNLOADS: &str = "6";

        let _lock = RATE_LIMITER.lock().await;
        let instant = Instant::now();

        let mut params = HashMap::from([
            ("gameId", get_mc_id().await?.to_string()),
            ("sortField", TOTAL_DOWNLOADS.to_owned()),
            ("sortOrder", "desc".to_owned()),
            ("index", offset.to_string()),
        ]);

        if let QueryType::Mods | QueryType::ModPacks = query_type {
            if let Some(loader) = query.loader {
                params.insert("modLoaderType", loader.to_curseforge().to_owned());
            }
            params.insert("gameVersion", query.version.clone());
        }

        let categories = get_categories().await?;
        let query_type_str = query_type.to_curseforge_str();
        if let Some(category) = categories.data.iter().find(|n| n.slug == query_type_str) {
            params.insert("classId", category.id.to_string());
        }

        if !query.name.is_empty() {
            params.insert("searchFilter", query.name.clone());
        }

        let response = send_request("mods/search", &params).await?;
        let response: CFSearchResult = serde_json::from_str(&response).json(response)?;

        Ok(SearchResult {
            mods: response
                .data
                .into_iter()
                .map(|n| SearchMod {
                    title: n.name,
                    description: n.summary,
                    downloads: n.downloadCount,
                    internal_name: n.slug,
                    id: n.id.to_string(),
                    project_type: query_type_str.to_owned(),
                    icon_url: n.logo.map(|n| n.url).unwrap_or_default(),
                })
                .collect(),
            start_time: instant,
            backend: ql_core::StoreBackendType::Curseforge,
            offset,
            // TODO: Check whether curseforge results have hit bottom
            reached_end: false,
        })
    }

    async fn get_description(id: &str) -> Result<(ModId, String), ModError> {
        #[derive(Deserialize)]
        struct Resp2 {
            data: String,
        }

        let map = HashMap::new();
        let description = send_request(&format!("mods/{id}/description"), &map).await?;
        let description: Resp2 = serde_json::from_str(&description).json(description)?;

        Ok((ModId::Curseforge(id.to_string()), description.data))
    }

    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Option<ql_core::Loader>,
    ) -> Result<(DateTime<chrono::FixedOffset>, String), ModError> {
        let response = ModQuery::load(id).await?;
        let loader = loader.map(|n| n.to_curseforge());

        let query_type = get_query_type(response.data.classId).await?;
        let (file_query, _) = response
            .data
            .get_file(
                response.data.name.clone(),
                id,
                version.to_owned(),
                loader,
                query_type,
            )
            .await?;

        let download_version_time = DateTime::parse_from_rfc3339(&file_query.data.fileDate)?;

        Ok((download_version_time, response.data.name))
    }

    async fn download(
        id: &str,
        instance: &ql_core::InstanceSelection,
        sender: Option<Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        let mut downloader = ModDownloader::new(instance.clone(), sender.as_ref()).await?;

        downloader.ensure_essential_mods().await?;

        downloader.download(id, None).await?;
        downloader.index.save(instance).await?;

        Ok(downloader.not_allowed)
    }

    async fn download_bulk(
        ids: &[String],
        instance: &ql_core::InstanceSelection,
        ignore_incompatible: bool,
        set_manually_installed: bool,
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        let mut downloader = ModDownloader::new(instance.clone(), sender).await?;
        downloader.ensure_essential_mods().await?;
        downloader.query_cache.extend(
            CFSearchResult::get_from_ids(ids)
                .await?
                .data
                .into_iter()
                .map(|n| (n.id.to_string(), n)),
        );

        let len = ids.len();
        for (i, id) in ids.iter().enumerate() {
            if let Some(sender) = &downloader.sender {
                _ = sender.send(GenericProgress {
                    done: i,
                    total: len,
                    message: None,
                    has_finished: false,
                });
            }

            let result = downloader.download(id, None).await;

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
        if let Some(sender) = &downloader.sender {
            _ = sender.send(GenericProgress::finished());
        }

        Ok(downloader.not_allowed)
    }
}

pub async fn send_request(
    api: &str,
    params: &HashMap<&str, String>,
) -> Result<String, RequestError> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static("application/json"),
    );
    headers.insert("x-api-key", HeaderValue::from_str(API_KEY)?);

    let url = format!("https://api.curseforge.com/v1/{api}");
    let response = CLIENT
        .get(&url)
        .headers(headers)
        .query(params)
        .send()
        .await?;

    check_for_success(&response)?;
    Ok(response.text().await?)
}

// Please don't steal :)
const API_KEY: &str = "$2a$10$2SyApFh1oojq/d6z8axjRO6I8yrWI8.m0BTJ20vXNTWfy2O0X5Zsa";

pub async fn get_mc_id() -> Result<i32, ModError> {
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Game>,
    }

    #[derive(Deserialize)]
    struct Game {
        id: i32,
        name: String,
    }

    let val = MC_ID.load(std::sync::atomic::Ordering::Acquire);

    if val == NOT_LOADED {
        let params = HashMap::new();

        let response = send_request("games", &params).await?;
        let response: Response = serde_json::from_str(&response).json(response)?;

        let Some(minecraft) = response
            .data
            .iter()
            .find(|n| n.name.eq_ignore_ascii_case("Minecraft"))
        else {
            return Err(ModError::NoMinecraftInCurseForge);
        };

        MC_ID.store(minecraft.id, std::sync::atomic::Ordering::Release);

        Ok(minecraft.id)
    } else {
        Ok(val)
    }
}

pub async fn get_query_type(class_id: i32) -> Result<QueryType, ModError> {
    let categories = get_categories().await?;
    Ok(
        if let Some(category) = categories.data.iter().find(|n| n.id == class_id) {
            QueryType::from_curseforge_str(&category.slug)
                .ok_or(ModError::UnknownProjectType(category.slug.clone()))?
        } else {
            QueryType::Mods
        },
    )
}
