use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, atomic::AtomicI32, mpsc::Sender},
    time::Instant,
};

use chrono::DateTime;
use download::ModDownloader;
use ql_core::{
    CLIENT, GenericProgress, IntoJsonError, JsonDownloadError, Loader, RequestError, err, pt,
};
use reqwest::header::HeaderValue;
use serde::Deserialize;

use crate::{
    rate_limiter::{RATE_LIMITER, lock},
    store::{
        Category, ModId, SearchMod, StoreBackendType,
        curseforge::categories::CfCategory,
        types::{GalleryItem, UrlKind},
    },
};

use super::{Backend, CurseforgeNotAllowed, ModError, QueryType, SearchResult};
use categories::get_categories;
use ql_core::request::check_for_success;

mod categories;
mod download;

const NOT_LOADED: i32 = -1;
pub static MC_ID: AtomicI32 = AtomicI32::new(NOT_LOADED);

#[derive(Deserialize, Clone, Debug)]
pub struct ModQuery {
    pub data: Mod,
}

impl ModQuery {
    pub async fn load<T: std::fmt::Display>(id: T) -> Result<Self, JsonDownloadError> {
        let response = send_request(&format!("mods/{id}"), &HashMap::new()).await?;
        let response: ModQuery = serde_json::from_str(&response).json(response)?;
        Ok(response)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Mod {
    pub name: Arc<str>,
    pub slug: String,
    pub summary: String,
    #[serde(rename = "downloadCount")]
    download_count: usize,
    pub logo: Option<Logo>,
    pub id: i32,
    #[serde(rename = "latestFilesIndexes")]
    latest_files_indexes: Vec<CurseforgeFileIdx>,
    #[serde(rename = "classId")]
    pub class_id: i32,
    screenshots: Vec<CfScreenshot>,
    links: CfLinks,
    // latestFiles: Vec<CurseforgeFile>,
}

impl Mod {
    async fn get_file<T: std::fmt::Display>(
        &self,
        title: Arc<str>,
        id: T,
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
                self.latest_files_indexes.first()
            })
        }) else {
            return Err(ModError::NoCompatibleVersionFound(title));
        };

        let file_query = CurseforgeFileQuery::load(id, file.fileId).await?;

        Ok((file_query, file.fileId))
    }

    fn iter_files(&self, version: String) -> impl Iterator<Item = &CurseforgeFileIdx> {
        self.latest_files_indexes
            .iter()
            .filter(move |n| n.gameVersion == version)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct CfScreenshot {
    title: String,
    description: String,
    url: String,
}

impl From<CfScreenshot> for GalleryItem {
    fn from(value: CfScreenshot) -> Self {
        Self {
            url: value.url,
            title: Some(value.title),
            description: Some(value.description),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CfLinks {
    website: Option<String>,
    wiki: Option<String>,
    issues: Option<String>,
    source: Option<String>,
}

impl CfLinks {
    fn build_urls(&self) -> Vec<(UrlKind, String)> {
        let mut urls = Vec::new();
        if let Some(website_url) = &self.website {
            if !website_url.is_empty() {
                urls.push((UrlKind::Website, website_url.clone()));
            }
        }
        if let Some(wiki_url) = &self.wiki {
            if !wiki_url.is_empty() {
                urls.push((UrlKind::Wiki, wiki_url.clone()));
            }
        }
        if let Some(issues_url) = &self.issues {
            if !issues_url.is_empty() {
                urls.push((UrlKind::Issues, issues_url.clone()));
            }
        }
        if let Some(source_url) = &self.source {
            if !source_url.is_empty() {
                urls.push((UrlKind::Source, source_url.clone()));
            }
        }
        urls
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
    pub async fn load<T: std::fmt::Display>(
        mod_id: T,
        file_id: i32,
    ) -> Result<Self, JsonDownloadError> {
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
    dependencies: Vec<Dependency>,
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
    pub url: String,
}

#[derive(Deserialize)]
pub struct CFSearchResult {
    pub data: Vec<Mod>,
}

impl CFSearchResult {
    pub async fn get_from_ids(ids: &[Arc<str>]) -> Result<Self, ModError> {
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
    async fn search(query: super::Query, offset: usize) -> Result<SearchResult, ModError> {
        const TOTAL_DOWNLOADS: &str = "6";

        RATE_LIMITER.lock().await;
        let instant = Instant::now();

        let mut params = HashMap::from([
            ("gameId", get_mc_id().await?.to_string()),
            ("sortField", TOTAL_DOWNLOADS.to_owned()),
            ("sortOrder", "desc".to_owned()),
            ("index", offset.to_string()),
        ]);

        if let QueryType::Mods | QueryType::ModPacks = query.kind {
            if !query.loader.is_vanilla() {
                params.insert("modLoaderType", query.loader.to_curseforge_num().to_owned());
            }
            params.insert("gameVersion", query.version.clone());
        }

        let categories = get_categories().await?;
        let query_type_str = query.kind.to_curseforge_str();
        if query.kind == QueryType::DataPacks {
            // Curseforge returns categories that have the same slug but have different ids
            // /data-packs                  id: 6945 (the right one)
            // /texture-packs/data-packs    id: 5139 (actually texture packs)
            params.insert("classId", 6945.to_string());
        } else if let Some(category) = categories.data.iter().find(|n| n.slug == query_type_str) {
            params.insert("classId", category.id.to_string());
        }

        if !query.name.is_empty() {
            params.insert("searchFilter", query.name.clone());
        }
        if !query.categories.is_empty() {
            let categories: Vec<i32> = query
                .categories
                .iter()
                .take(10) // Curseforge only allows up to 10 category ids
                .filter_map(|n| n.internal_id)
                .collect();
            params.insert("categoryIds", serde_json::to_string(&categories).json_to()?);
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
                    downloads: n.download_count,
                    internal_name: n.slug,
                    id: Arc::from(n.id.to_string()),
                    project_type: query_type_str.to_owned(),
                    icon_url: n.logo.map(|n| n.url),
                    backend: StoreBackendType::Curseforge,
                    gallery: n.screenshots.into_iter().map(GalleryItem::from).collect(),
                    urls: n.links.build_urls(),
                })
                .collect(),
            start_time: instant,
            backend: StoreBackendType::Curseforge,
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

        Ok((
            ModId::Curseforge(Arc::from(id.to_string())),
            description.data,
        ))
    }

    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Loader,
    ) -> Result<(DateTime<chrono::FixedOffset>, String), ModError> {
        let response = ModQuery::load(id).await?;
        let loader = loader.not_vanilla().map(|n| n.to_curseforge_num());

        let query_type = get_query_type(response.data.class_id).await?;
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

        Ok((download_version_time, file_query.data.displayName))
    }

    async fn download(
        id: &str,
        instance: &ql_core::Instance,
        sender: Option<Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        let _guard = lock().await;
        let mut downloader = ModDownloader::new(instance.clone(), sender.as_ref()).await?;

        downloader.ensure_essential_mods().await?;

        downloader.download(id, None).await?;
        downloader.index.save(instance).await?;

        Ok(downloader.not_allowed)
    }

    async fn download_bulk(
        ids: &[Arc<str>],
        instance: &ql_core::Instance,
        ignore_incompatible: bool,
        set_manually_installed: bool,
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        let _guard = lock().await;
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
                if let Some(config) = downloader
                    .index
                    .mods
                    .get_mut(&ModId::Curseforge(id.clone()))
                {
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

    async fn get_categories(kind: QueryType) -> Result<Vec<Category>, ModError> {
        let categories = get_categories().await?;

        // TODO:
        // - mc-addons, customization: addition to existing mods
        // - bukkit-plugins
        // - worlds

        let kind_str = kind.to_curseforge_str();

        let Some(project_type_id) = categories
            .data
            .iter()
            .filter(|c| c.parent_category_id.is_none())
            .filter(|c| c.slug == kind_str)
            .map(|c| c.id)
            .next()
        else {
            return Err(ModError::CfCategoryNotFound(kind));
        };

        let root_ids: Vec<i32> = categories
            .data
            .iter()
            .filter(|c| c.parent_category_id == Some(project_type_id))
            .map(|c| c.id)
            .collect();

        Ok(root_ids
            .into_iter()
            .filter_map(|id| build_node(id, &categories.data))
            .collect())
    }

    async fn get_info(id: &str) -> Result<SearchMod, ModError> {
        let query = ModQuery::load(id).await?;
        Ok(SearchMod {
            title: query.data.name,
            description: query.data.summary,
            downloads: query.data.download_count,
            internal_name: query.data.slug,
            id: Arc::from(query.data.id.to_string()),
            project_type: get_query_type(query.data.class_id)
                .await
                .unwrap_or(QueryType::Mods)
                .to_curseforge_str()
                .to_owned(),
            icon_url: query.data.logo.map(|n| n.url),
            backend: StoreBackendType::Curseforge,
            gallery: query
                .data
                .screenshots
                .into_iter()
                .map(GalleryItem::from)
                .collect(),
            urls: query.data.links.build_urls(),
        })
    }

    async fn get_info_bulk(ids: &[Arc<str>]) -> Result<Vec<SearchMod>, ModError> {
        let queries = CFSearchResult::get_from_ids(ids).await?;
        let mut out = Vec::with_capacity(queries.data.len());
        for query in queries.data {
            out.push(SearchMod {
                title: query.name,
                description: query.summary,
                downloads: query.download_count,
                internal_name: query.slug,
                id: Arc::from(query.id.to_string()),
                project_type: get_query_type(query.class_id)
                    .await
                    .unwrap_or(QueryType::Mods)
                    .to_curseforge_str()
                    .to_owned(),
                icon_url: query.logo.map(|n| n.url),
                backend: StoreBackendType::Curseforge,
                gallery: query
                    .screenshots
                    .into_iter()
                    .map(GalleryItem::from)
                    .collect(),
                urls: query.links.build_urls(),
            });
        }

        Ok(out)
    }

    async fn get_download_link(
        instance: &ql_core::Instance,
        id: &str,
        query_type: QueryType,
    ) -> Result<String, ModError> {
        let mut downloader = ModDownloader::basic(instance.clone()).await?;
        let url = downloader.get_download_link(id, query_type).await?;
        Ok(url)
    }
}

fn build_node(id: i32, list: &[CfCategory]) -> Option<Category> {
    let cf = list.iter().find(|n| n.id == id)?;

    let children = list
        .iter()
        .filter(|c| c.parent_category_id == Some(cf.id))
        .filter_map(|c| build_node(c.id, list))
        .collect();

    Some(Category {
        name: cf.name.clone(),
        slug: cf.slug.clone(),
        children,
        internal_id: Some(cf.id),
        is_usable: true,
    })
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
