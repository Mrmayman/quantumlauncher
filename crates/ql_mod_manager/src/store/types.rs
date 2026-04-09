use std::{fmt::Display, time::Instant};

use ql_core::Loader;
use serde::{Deserialize, Serialize};

use crate::store::ModId;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackendType {
    #[serde(rename = "modrinth")]
    Modrinth,
    #[serde(rename = "curseforge")]
    Curseforge,
}

impl StoreBackendType {
    /// Human readable description
    ///
    /// Zero-allocations, unlike `std::fmt::Display`
    #[must_use]
    pub const fn desc(self) -> &'static str {
        match self {
            StoreBackendType::Modrinth => "Modrinth",
            StoreBackendType::Curseforge => "Curseforge",
        }
    }

    #[must_use]
    pub fn can_pick_any_or_all(self) -> bool {
        matches!(self, StoreBackendType::Modrinth)
    }

    #[must_use]
    pub fn can_filter_open_source(self) -> bool {
        matches!(self, StoreBackendType::Modrinth)
    }

    #[must_use]
    pub fn can_sort_ascending(self) -> bool {
        matches!(self, StoreBackendType::Curseforge)
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
    /// Use this for the store since datapacks can't be installed globally,
    /// only per worlds, since you need to copy the datapack file into each world.
    ///
    /// Once the launcher has support for installing datapacks properly,
    /// delete this and use ALL in the store too.
    pub const STORE_QUERIES: &'static [Self] = &[
        Self::Mods,
        Self::ModPacks,
        Self::ResourcePacks,
        Self::Shaders,
    ];

    pub const ALL: &'static [Self] = &[
        Self::Mods,
        Self::ModPacks,
        Self::DataPacks,
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

    pub sort_by: SearchSortBy,
    /// Whether to sort in ascending order instead of descending.
    ///
    /// Only supported on Curseforge, see [`StoreBackendType::can_sort_ascending`]
    pub sort_ascending: bool,

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

    pub slug: String,
    pub project_type: String, // used for building page URL
    pub internal_id: String,
    pub backend: StoreBackendType,

    pub icon_url: Option<String>,
    pub gallery: Vec<GalleryItem>,
    pub urls: Vec<(UrlKind, String)>,
}

impl SearchMod {
    #[must_use]
    pub fn get_id(&self) -> ModId {
        ModId::from_pair(&self.internal_id, self.backend)
    }

    #[must_use]
    pub fn get_page_url(&self) -> String {
        format!(
            "{base}{ty}/{slug}",
            base = match self.backend {
                StoreBackendType::Modrinth => "https://modrinth.com/",
                StoreBackendType::Curseforge => "https://www.curseforge.com/minecraft/",
            },
            ty = self.project_type,
            slug = self.slug
        )
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSortBy {
    TotalDownloads,
    LastUpdated,
    ReleasedDate,

    // Modrinth-only
    Relevance,
    Follows,
    // Curseforge-only
    Featured,
    Popularity,
    Name,
    Author,
    Category,
    GameVersion,
    EarlyAccess,
    FeaturedReleased,
    Rating,
}

impl SearchSortBy {
    pub const fn default_option(backend: StoreBackendType) -> Self {
        match backend {
            StoreBackendType::Modrinth => Self::Relevance,
            StoreBackendType::Curseforge => Self::TotalDownloads,
        }
    }

    pub const fn default_choices(backend: StoreBackendType) -> &'static [Self] {
        match backend {
            StoreBackendType::Modrinth => &[
                Self::TotalDownloads,
                Self::LastUpdated,
                Self::ReleasedDate,
                Self::Relevance,
                Self::Follows,
            ],
            StoreBackendType::Curseforge => &[
                Self::TotalDownloads,
                Self::LastUpdated,
                Self::ReleasedDate,
                Self::Featured,
                Self::Popularity,
                Self::Name,
                Self::Author,
                Self::Category,
                Self::GameVersion,
                Self::EarlyAccess,
                Self::FeaturedReleased,
                Self::Rating,
            ],
        }
    }

    pub const fn get_curseforge_id(self) -> Option<usize> {
        Some(match self {
            Self::Featured => 1,
            Self::Popularity => 2,
            Self::LastUpdated => 3,
            Self::Name => 4,
            Self::Author => 5,
            Self::TotalDownloads => 6,
            Self::Category => 7,
            Self::GameVersion => 8,
            Self::EarlyAccess => 9,
            Self::FeaturedReleased => 10,
            Self::ReleasedDate => 11,
            Self::Rating => 12,
            Self::Relevance => return None,
            Self::Follows => return None,
        })
    }

    pub const fn get_modrinth_id(self) -> Option<&'static str> {
        match self {
            Self::Relevance => Some("relevance"),
            Self::Follows => Some("follows"),
            Self::LastUpdated => Some("updated"),
            Self::ReleasedDate => Some("newest"),
            Self::TotalDownloads => Some("downloads"),
            _ => None,
        }
    }
}

impl Display for SearchSortBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TotalDownloads => "Total Downloads",
            Self::LastUpdated => "Last Updated",
            Self::ReleasedDate => "Release Date",
            Self::Relevance => "Relevance",
            Self::Follows => "Follows",
            Self::Featured => "Featured",
            Self::Popularity => "Popularity",
            Self::Name => "Name",
            Self::Author => "Author",
            Self::Category => "Category",
            Self::GameVersion => "Game Version",
            Self::EarlyAccess => "Early Access",
            Self::FeaturedReleased => "Featured Released",
            Self::Rating => "Rating",
        })
    }
}
