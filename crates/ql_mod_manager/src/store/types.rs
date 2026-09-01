use std::{
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use ql_core::{Instance, IntoIoError, IoError, Loader, json::VersionDetails};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::store::ModId;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackendType {
    #[serde(rename = "curseforge")]
    Curseforge,
    #[serde(rename = "modrinth")]
    #[serde(other)] // Backwards compat (ugly)
    Modrinth,
}

impl StoreBackendType {
    #[must_use]
    pub fn can_pick_any_or_all(self) -> bool {
        matches!(self, StoreBackendType::Modrinth)
    }

    #[must_use]
    pub fn can_filter_open_source(self) -> bool {
        matches!(self, StoreBackendType::Modrinth)
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum SelectedMod {
    Downloaded { name: Arc<str>, id: ModId },
    Local(LocalMod),
}

impl SelectedMod {
    #[must_use]
    pub fn new(name: Arc<str>, id: Option<ModId>, project_type: QueryType) -> Self {
        match id {
            Some(id) => Self::Downloaded { name, id },
            None => Self::Local(LocalMod(name, project_type)),
        }
    }

    #[must_use]
    pub fn local(&self) -> Option<&LocalMod> {
        if let SelectedMod::Local(l) = self {
            Some(l)
        } else {
            None
        }
    }
}

#[must_use]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CurseforgeNotAllowed {
    pub name: Arc<str>,
    pub slug: String,
    pub filename: String,
    pub project_type: QueryType,
    pub file_id: usize,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default, Hash, PartialOrd, Ord,
)]
pub enum QueryType {
    #[default]
    Mods,
    Shaders,
    ModPacks,
    DataPacks,
    #[serde(other)] // Something that isn't as strictly tracked as mods
    ResourcePacks,
    // TODO:
    // Plugins,
}

impl Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            QueryType::Mods => "Mods",
            QueryType::ResourcePacks => "Resource Packs",
            QueryType::Shaders => "Shaders",
            QueryType::ModPacks => "Modpacks",
            QueryType::DataPacks => "Data Packs",
        })
    }
}

impl QueryType {
    pub const ALL: &'static [Self] = &[
        Self::Mods,
        Self::ModPacks,
        // /// Datapacks can't be installed globally (only per worlds),
        // /// since you need to copy the datapack file into each world.
        // ///
        // /// Once the launcher has support for installing datapacks properly,
        // /// uncomment this.
        // Self::DataPacks,
        Self::ResourcePacks,
        Self::Shaders,
    ];

    /// The query types that can be stored locally in the mod index.
    pub const INDEX_SUPPORTED: &'static [Self] = &[
        Self::Mods,
        // Self::DataPacks,
        Self::ResourcePacks,
        Self::Shaders,
    ];

    #[must_use]
    pub const fn to_modrinth_str(self) -> &'static str {
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
    pub const fn to_curseforge_str(self) -> &'static str {
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

    #[must_use]
    pub const fn is_toggleable(self) -> bool {
        matches!(self, QueryType::Mods)
    }

    #[must_use]
    pub const fn get_extensions(self) -> &'static [&'static str] {
        match self {
            QueryType::Mods => &["jar"],
            QueryType::Shaders | QueryType::ResourcePacks | QueryType::DataPacks => &["zip"],
            QueryType::ModPacks => &["zip", "mrpack", "qmp"],
        }
    }

    pub fn get_dirname(&self, is_legacy_texturepacks: bool) -> &str {
        match self {
            QueryType::Mods => "mods",
            QueryType::Shaders => "shaderpacks",
            QueryType::ModPacks => "modpacks",
            QueryType::DataPacks => "datapacks",
            QueryType::ResourcePacks => {
                if is_legacy_texturepacks {
                    "texturepacks"
                } else {
                    "resourcepacks"
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub slug: String,
    pub children: Vec<Category>,
    pub internal_id: Option<i32>,
    /// If `true`, can be toggled and serves a purpose.
    ///
    /// Else purely for organization (use its [`Self::children`] instead)
    pub is_usable: bool,
}

impl Category {
    #[must_use]
    pub fn search_for_slug(&self, slug: &str) -> Option<&Self> {
        if self.slug == slug {
            return Some(self);
        }

        for child in &self.children {
            if let Some(found) = child.search_for_slug(slug) {
                return Some(found);
            }
        }

        None
    }
}

#[derive(Clone, Debug)]
pub struct Query {
    pub name: String,
    pub version: String,
    pub loader: Loader,

    pub server_side: bool,
    pub kind: QueryType,
    /// Used if supported (modrinth supports it, curseforge doesn't).
    /// Use [`StoreBackendType::can_filter_open_source`] for checking this.
    pub open_source: bool,
    pub categories: Vec<Category>,
    /// Whether to search mods with *all* of the categories,
    /// or just any of them.
    ///
    /// Used if supported (modrinth supports it, curseforge doesn't).
    /// Use [`StoreBackendType::can_pick_any_or_all`] for checking this.
    pub categories_use_all: bool,
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
    pub title: Arc<str>,
    pub description: String,
    pub downloads: usize,
    pub internal_name: String,
    pub project_type: String,
    pub id: Arc<str>,
    pub icon_url: Option<String>,
    pub backend: StoreBackendType,

    pub gallery: Vec<GalleryItem>,
    pub urls: Vec<(UrlKind, String)>,
}

impl SearchMod {
    #[must_use]
    pub fn get_id(&self) -> ModId {
        ModId::from_pair(&self.id, self.backend)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct GalleryItem {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum UrlKind {
    Issues,
    Source,
    Wiki,

    // Curseforge-only
    Website,
    // Modrinth-only
    Discord,
    Donation(String), // Service name
}

impl Display for UrlKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            UrlKind::Issues => "Issues",
            UrlKind::Source => "Source",
            UrlKind::Wiki => "Wiki",
            UrlKind::Website => "Website",
            UrlKind::Discord => "Discord",
            UrlKind::Donation(n) => return f.write_fmt(format_args!("Donation ({n})")),
        })
    }
}

pub struct DirStructure {
    pub instance: Instance,
    mods: PathBuf,
    resource_packs: PathBuf,
    shaders: PathBuf,
    data_packs: PathBuf,
    pub is_legacy: bool,
}

impl DirStructure {
    pub async fn new(instance: Instance, version_json: &VersionDetails) -> Result<Self, IoError> {
        let mc_dir = instance.get_dot_minecraft_path();

        // this doesn't get loaded by default but there are datapack loader mods
        // that are used my modpacks that want to include datapacks.
        // for example https://modrinth.com/mod/dataloader
        let data_packs = mc_dir.join("datapacks");
        fs::create_dir_all(&data_packs).await.path(&data_packs)?;

        let is_legacy = version_json.is_legacy_texturepacks();

        let resource_packs = if is_legacy {
            "texturepacks"
        } else {
            "resourcepacks"
        };

        let resource_packs = mc_dir.join(resource_packs);
        fs::create_dir_all(&resource_packs)
            .await
            .path(&resource_packs)?;

        let shaders = mc_dir.join("shaderpacks");
        fs::create_dir_all(&shaders).await.path(&shaders)?;

        let mods = mc_dir.join("mods");
        fs::create_dir_all(&mods).await.path(&mods)?;

        Ok(Self {
            instance,
            mods,
            resource_packs,
            shaders,
            data_packs,
            is_legacy,
        })
    }

    #[must_use]
    pub fn get(&self, query_type: QueryType) -> Option<&Path> {
        Some(match query_type {
            QueryType::DataPacks => &self.data_packs,
            QueryType::ResourcePacks => &self.resource_packs,
            QueryType::Mods => &self.mods,
            QueryType::Shaders => &self.shaders,
            // Note: A lot of code relies on the assumption
            // that this returns None only for modpacks,
            // so be careful
            QueryType::ModPacks => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Hash, PartialOrd, Eq)]
pub struct LocalMod(pub Arc<str>, pub QueryType);
