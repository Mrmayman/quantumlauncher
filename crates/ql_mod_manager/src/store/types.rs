use std::{fmt::Display, time::Instant};

use ql_core::Loader;
use serde::{Deserialize, Serialize};

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
    Downloaded { name: String, id: ModId },
    Local { file_name: String },
}

impl SelectedMod {
    #[must_use]
    pub fn from_pair(name: String, id: Option<ModId>) -> Self {
        match id {
            Some(id) => Self::Downloaded { name, id },
            None => Self::Local { file_name: name },
        }
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
    pub title: String,
    pub description: String,
    pub downloads: usize,
    pub internal_name: String,
    pub project_type: String,
    pub id: String,
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
