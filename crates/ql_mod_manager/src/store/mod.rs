use std::{collections::HashSet, fmt::Display, path::PathBuf, sync::mpsc::Sender, time::Instant};

use chrono::DateTime;
use ql_core::{
    json::VersionDetails, GenericProgress, InstanceSelection, IntoIoError, Loader, ModId,
    StoreBackendType,
};

mod add_file;
mod curseforge;
mod delete;
mod error;
mod image;
mod local_json;
mod modpack;
mod modrinth;
mod recommended;
mod toggle;
mod update;

pub use add_file::add_files;
pub use curseforge::CurseforgeBackend;
pub use delete::delete_mods;
pub use error::{GameExpectation, ModError};
pub use image::{download_image, ImageResult};
pub use local_json::{ModConfig, ModFile, ModIndex};
pub use modpack::{install_modpack, PackError};
pub use modrinth::ModrinthBackend;
pub use recommended::{RecommendedMod, RECOMMENDED_MODS};
pub use toggle::{flip_filename, toggle_mods, toggle_mods_local};
pub use update::{apply_updates, check_for_updates};

pub const SOURCE_ID_MODRINTH: &str = "modrinth";
pub const SOURCE_ID_CURSEFORGE: &str = "curseforge";

#[allow(async_fn_in_trait)]
pub trait Backend {
    /// # Takes in
    /// - Query information,
    /// - Offset from the start (how far you scrolled down)
    /// - Query type (Mod/Resource Pack/Shader/...)
    ///
    /// Returns a search result containing a list of matching items
    async fn search(
        query: Query,
        offset: usize,
        query_type: QueryType,
    ) -> Result<SearchResult, ModError>;
    /// Gets the description of a mod based on its id.
    /// Returns the id and description `String`.
    ///
    /// This supports both Markdown and HTML.
    async fn get_description(id: &str) -> Result<(ModId, String), ModError>;
    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Loader,
    ) -> Result<(DateTime<chrono::FixedOffset>, String), ModError>;

    async fn download(
        id: &str,
        instance: &InstanceSelection,
        sender: Option<Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError>;

    async fn download_bulk(
        ids: &[String],
        instance: &InstanceSelection,
        ignore_incompatible: bool,
        set_manually_installed: bool,
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError>;
}

pub async fn get_description(id: ModId) -> Result<(ModId, String), ModError> {
    match &id {
        ModId::Modrinth(n) => ModrinthBackend::get_description(n).await,
        ModId::Curseforge(n) => CurseforgeBackend::get_description(n).await,
    }
}

pub async fn search(
    query: Query,
    offset: usize,
    backend: StoreBackendType,
    query_type: QueryType,
) -> Result<SearchResult, ModError> {
    match backend {
        StoreBackendType::Modrinth => ModrinthBackend::search(query, offset, query_type).await,
        StoreBackendType::Curseforge => CurseforgeBackend::search(query, offset, query_type).await,
    }
}

pub async fn download_mod(
    id: &ModId,
    instance: &InstanceSelection,
    sender: Option<Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
    match id {
        ModId::Modrinth(n) => ModrinthBackend::download(n, instance, sender).await,
        ModId::Curseforge(n) => CurseforgeBackend::download(n, instance, sender).await,
    }
}

pub async fn download_mods_bulk(
    ids: Vec<ModId>,
    instance: InstanceSelection,
    sender: Option<Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
    let (modrinth, other): (Vec<ModId>, Vec<ModId>) = ids.into_iter().partition(|n| match n {
        ModId::Modrinth(_) => true,
        ModId::Curseforge(_) => false,
    });

    let modrinth: Vec<String> = modrinth
        .into_iter()
        .map(|n| n.get_internal_id().to_owned())
        .collect();

    let curseforge: Vec<String> = other
        .into_iter()
        .map(|n| n.get_internal_id().to_owned())
        .collect();

    // if !other.is_empty() {
    //     err!("Unimplemented downloading for mods: {other:#?}");
    // }

    let not_allowed =
        ModrinthBackend::download_bulk(&modrinth, &instance, true, true, sender.as_ref()).await?;
    debug_assert!(not_allowed.is_empty());

    let not_allowed =
        CurseforgeBackend::download_bulk(&curseforge, &instance, true, true, sender.as_ref())
            .await?;

    Ok(not_allowed)
}

pub async fn get_latest_version_date(
    loader: Loader,
    mod_id: &ModId,
    version: &str,
) -> Result<(DateTime<chrono::FixedOffset>, String), ModError> {
    Ok(match mod_id {
        ModId::Modrinth(n) => ModrinthBackend::get_latest_version_date(n, version, loader).await?,
        ModId::Curseforge(n) => {
            CurseforgeBackend::get_latest_version_date(n, version, loader).await?
        }
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryType {
    Mods,
    ResourcePacks,
    Shaders,
    ModPacks,
    DataPacks,
    // TODO:
    // Plugins,
}

impl Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                QueryType::Mods => "Mods",
                QueryType::ResourcePacks => "Resource Packs",
                QueryType::Shaders => "Shaders",
                QueryType::ModPacks => "Modpacks",
                QueryType::DataPacks => "Data Packs",
            }
        )
    }
}

impl QueryType {
    // use this for the store since datapacks cant be installed globally,
    // only pre worlds, since you need to copy the datapack file into each world
    // once the launcher have support for installing datapacks properly delete this
    // and use ALL in the store too.
    pub const STORE_QUERIES: &'static [Self] = &[
        Self::Mods,
        Self::ResourcePacks,
        Self::Shaders,
        Self::ModPacks,
    ];

    pub const ALL: &'static [Self] = &[
        Self::DataPacks,
        Self::ResourcePacks,
        Self::ModPacks,
        Self::Mods,
        Self::Shaders,
    ];

    #[must_use]
    pub fn to_modrinth_str(&self) -> &'static str {
        match self {
            QueryType::Mods => "mod",
            QueryType::ResourcePacks => "resourcepack",
            QueryType::Shaders => "shader",
            QueryType::ModPacks => "modpack",
            QueryType::DataPacks => "datapack",
        }
    }

    #[must_use]
    pub fn from_modrinth_str(s: &str) -> Option<Self> {
        match s {
            "mod" => Some(QueryType::Mods),
            "resourcepack" => Some(QueryType::ResourcePacks),
            "shader" => Some(QueryType::Shaders),
            "modpack" => Some(QueryType::ModPacks),
            "datapack" => Some(QueryType::DataPacks),
            _ => None,
        }
    }

    #[must_use]
    pub fn to_curseforge_str(&self) -> &'static str {
        match self {
            QueryType::Mods => "mc-mods",
            QueryType::ResourcePacks => "texture-packs",
            QueryType::Shaders => "shaders",
            QueryType::ModPacks => "modpacks",
            QueryType::DataPacks => "data-packs",
        }
    }

    #[must_use]
    pub fn from_curseforge_str(s: &str) -> Option<Self> {
        match s {
            "mc-mods" => Some(QueryType::Mods),
            "texture-packs" => Some(QueryType::ResourcePacks),
            "shaders" => Some(QueryType::Shaders),
            "modpacks" => Some(QueryType::ModPacks),
            "data-packs" => Some(QueryType::DataPacks),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Query {
    pub name: String,
    pub version: String,
    pub loader: Loader,
    pub server_side: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub mods: Vec<SearchMod>,
    pub backend: StoreBackendType,
    pub start_time: Instant,
    pub offset: usize,
    pub reached_end: bool,
}

#[derive(Debug, Clone)]
pub struct SearchMod {
    pub title: String,
    pub description: String,
    pub downloads: usize,
    pub internal_name: String,
    pub project_type: String,
    pub id: String,
    pub icon_url: String,
}

impl SearchMod {
    #[must_use]
    pub fn get_id(&self, backend: StoreBackendType) -> ModId {
        ModId::from_pair(&self.id, backend)
    }
}

struct DirStructure {
    mods: PathBuf,
    resource_packs: PathBuf,
    shaders: PathBuf,
    data_packs: PathBuf,
}

impl DirStructure {
    pub async fn new(
        instance_name: &InstanceSelection,
        version_json: &VersionDetails,
    ) -> Result<Self, ModError> {
        // Minecraft 13w23b release date (1.6.1 snapshot)
        // Last version with Texture Packs instead of Resource Packs
        const V1_6_1: &str = "2013-06-08T00:32:01+00:00";

        let dot_minecraft_dir = instance_name.get_dot_minecraft_path();

        // this doesn't get loaded by default but there are datapack loader mods
        // that are used my modpacks that want to include datapacks.
        // for example https://modrinth.com/mod/dataloader
        let data_packs = dot_minecraft_dir.join("datapacks");
        tokio::fs::create_dir_all(&data_packs)
            .await
            .path(&data_packs)?;

        let resource_packs = if version_json.is_before_or_eq(V1_6_1) {
            "texturepacks"
        } else {
            "resourcepacks"
        };

        let resource_packs = dot_minecraft_dir.join(resource_packs);
        tokio::fs::create_dir_all(&resource_packs)
            .await
            .path(&resource_packs)?;

        let shaders = dot_minecraft_dir.join("shaderpacks");
        tokio::fs::create_dir_all(&shaders).await.path(&shaders)?;

        let mods = dot_minecraft_dir.join("mods");
        tokio::fs::create_dir_all(&mods).await.path(&mods)?;

        Ok(Self {
            mods,
            resource_packs,
            shaders,
            data_packs,
        })
    }

    pub fn get(&self, query_type: QueryType) -> Result<PathBuf, PackError> {
        Ok(match query_type {
            QueryType::DataPacks => self.data_packs.clone(),
            QueryType::ResourcePacks => self.resource_packs.clone(),
            QueryType::Mods => self.mods.clone(),
            QueryType::Shaders => self.shaders.clone(),
            QueryType::ModPacks => return Err(PackError::ModpackInModpack),
        })
    }
}

#[must_use]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CurseforgeNotAllowed {
    pub name: String,
    pub slug: String,
    pub filename: String,
    pub project_type: String,
    pub file_id: usize,
}
