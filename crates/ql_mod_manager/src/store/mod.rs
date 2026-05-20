use std::{
    collections::HashSet,
    sync::{Arc, mpsc::Sender},
};

use chrono::DateTime;
use ql_core::{GenericProgress, Instance, Loader, do_jobs, pt};

mod add_file;
mod curseforge;
mod delete;
mod error;
mod id;
pub mod image;
mod local_json;
mod modpack;
mod modrinth;
mod recommended;
mod toggle;
mod types;
mod update;

pub use add_file::add_files;
pub use curseforge::CurseforgeBackend;
pub use delete::delete_mods;
pub use error::{GameExpectation, ModError};
pub use id::ModId;
pub use local_json::{ModConfig, ModFile, ModIndex};
pub use modpack::{PackError, install_modpack};
pub use modrinth::ModrinthBackend;
pub use recommended::{RECOMMENDED_MODS, RecommendedMod};
pub use toggle::{flip_filename, toggle_mods, toggle_mods_local};
pub use types::{
    Category, CurseforgeNotAllowed, DirStructure, LocalMod, Query, QueryType, SearchMod,
    SearchResult, SelectedMod, StoreBackendType,
};
pub use update::{ChangelogFile, apply_updates, check_for_updates};

#[allow(async_fn_in_trait)]
pub trait Backend {
    /// # Takes in
    /// - Query information,
    /// - Offset from the start (how far you scrolled down)
    /// - Query type (Mod/Resource Pack/Shader/...)
    ///
    /// Returns a search result containing a list of matching items
    ///
    /// Note: Some `SearchResult` fields may be limited in info, such as:
    /// - Gallery image titles/descriptions/order
    /// - Project links
    ///
    /// For the full info use `get_info` or `get_info_bulk`
    async fn search(query: Query, offset: usize) -> Result<SearchResult, ModError>;
    /// Gets the description of a mod based on its id.
    /// Returns the id and description `String`.
    ///
    /// This may use Markdown, HTML, or a mix of both.
    async fn get_description(id: &str) -> Result<(ModId, String), ModError>;

    /// Gets the latest compatible mod version, based on provided Minecraft version and mod loader.
    ///
    /// Useful for update checking.
    ///
    /// Returns the release date and version name (eg: `v2.0.1`).
    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Loader,
    ) -> Result<(DateTime<chrono::FixedOffset>, String), ModError>;

    /// Downloads a single mod to the `instance`.
    ///
    /// Optionally takes in a `sender` to use if it's a modpack.
    async fn download(
        id: &str,
        instance: &Instance,
        sender: Option<Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError>;
    /// Downloads multiple mods to the `instance`.
    ///
    /// Uses efficient batched APIs and concurrent downloading when possible,
    /// so more efficient than [`Backend::download`] in a loop.
    async fn download_bulk(
        ids: &[Arc<str>],
        instance: &Instance,
        ignore_incompatible: bool,
        _set_manually_installed: bool,
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        // Fallback implementation
        let mut not_allowed = HashSet::new();
        for id in ids {
            // We don't do this concurrently as there's likely a lock on the index
            match Self::download(id, instance, sender.cloned()).await {
                Ok(n) => not_allowed.extend(n),
                Err(ModError::NoCompatibleVersionFound(name)) if ignore_incompatible => {
                    pt!("No compatible version found for mod {name} {id}, skipping...");
                }
                Err(err) => return Err(err),
            }
        }
        Ok(not_allowed)
    }

    /// Gets all the possible filter categories of content (Adventure, Redstone, QOL, etc).
    ///
    /// # Structure
    ///
    /// This is a tree structure, each [`Category`] can have subcategories.
    /// This function returns a list of root nodes.
    ///
    /// If you just want a basic list, feel free to just not have any child nodes.
    ///
    /// # Caching
    ///
    /// Usually this is cached, so fetching it multiple times is not expensive.
    /// (Note to implementors: **Please cache this with** `LazyLock`, `OnceCell` or similar structures!**)
    async fn get_categories(_: QueryType) -> Result<Vec<Category>, ModError> {
        Ok(Vec::new()) // Fallback
    }

    /// Gets metadata about a mod, such as its title, description, icon, download count, etc.
    async fn get_info(id: &str) -> Result<SearchMod, ModError>;
    /// Gets metadata about multiple mods in bulk, such as their title, description, icon, download count, etc.
    ///
    /// Uses efficient batched APIs and concurrent fetching when possible,
    /// so more efficient than [`Backend::get_info`] in a loop.
    async fn get_info_bulk(ids: &[Arc<str>]) -> Result<Vec<SearchMod>, ModError> {
        // Fallback implementation (concurrent)
        do_jobs(ids.iter().map(|n| Self::get_info(n))).await
    }

    /// Gets the direct download link for a mod file, based on its id.
    ///
    /// Useful for exporting to certain modpack formats (like modrinth).
    ///
    /// May return [`ModError::NoFilesFound`] if a Curseforge mod doesn't allow direct downloading.
    async fn get_download_link(
        instance: &Instance,
        id: &str,
        query_type: QueryType,
    ) -> Result<String, ModError>;
}

/// Gets the description of a mod based on its id.
/// Returns the id and description `String`.
///
/// This may use Markdown, HTML, or a mix of both.
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
) -> Result<SearchResult, ModError> {
    match backend {
        StoreBackendType::Modrinth => ModrinthBackend::search(query, offset).await,
        StoreBackendType::Curseforge => CurseforgeBackend::search(query, offset).await,
    }
}

/// Downloads a single mod to the `instance`.
///
/// Optionally takes in a `sender` to use if it's a modpack.
pub async fn download_mod(
    id: &ModId,
    instance: &Instance,
    sender: Option<Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
    match id {
        ModId::Modrinth(n) => ModrinthBackend::download(n, instance, sender).await,
        ModId::Curseforge(n) => CurseforgeBackend::download(n, instance, sender).await,
    }
}

/// Downloads multiple mods to the `instance`.
///
/// Uses efficient batched APIs and concurrent downloading when possible,
/// so more efficient than [`download_mod`] in a loop.
pub async fn download_mods_bulk(
    ids: Vec<ModId>,
    instance: Instance,
    sender: Option<Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
    let (modrinth, other): (Vec<ModId>, Vec<ModId>) = ids.into_iter().partition(|n| match n {
        ModId::Modrinth(_) => true,
        ModId::Curseforge(_) => false,
    });

    let modrinth: Vec<Arc<str>> = modrinth.into_iter().map(|n| n.get_internal_id()).collect();
    let curseforge: Vec<Arc<str>> = other.into_iter().map(|n| n.get_internal_id()).collect();

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

/// Gets the latest compatible mod version, based on provided Minecraft version and mod loader.
///
/// Returns the release date and version name (eg: `v2.0.1`).
///
/// Useful for checking for updates, or checking compatibility.
///
/// # Errors
///
/// - `NoCompatibleVersionFound` if mod doesn't support version
/// - Many other errors depending on backend
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

/// Gets categories of content (Adventure, Redstone, QOL, etc)
/// for a given query type (Mod/Resource Pack/Shader/...) from the backend.
pub async fn get_categories(
    query_type: QueryType,
    backend: StoreBackendType,
) -> Result<Vec<Category>, ModError> {
    match backend {
        StoreBackendType::Modrinth => ModrinthBackend::get_categories(query_type).await,
        StoreBackendType::Curseforge => CurseforgeBackend::get_categories(query_type).await,
    }
}

/// Gets metadata about a mod, such as its title, description, icon, download count, etc.
pub async fn get_info(id: &ModId) -> Result<SearchMod, ModError> {
    match id {
        ModId::Modrinth(n) => ModrinthBackend::get_info(n).await,
        ModId::Curseforge(n) => CurseforgeBackend::get_info(n).await,
    }
}

/// Gets metadata about multiple mods in bulk, such as their title, description, icon, download count, etc.
///
/// Uses efficient batched APIs and concurrent fetching when possible,
/// so more efficient than [`get_info`] in a loop.
pub async fn get_info_bulk(ids: Vec<ModId>) -> Result<Vec<SearchMod>, ModError> {
    let (modrinth, other): (Vec<ModId>, Vec<ModId>) = ids.into_iter().partition(|n| match n {
        ModId::Modrinth(_) => true,
        ModId::Curseforge(_) => false,
    });

    let modrinth: Vec<Arc<str>> = modrinth.into_iter().map(|n| n.get_internal_id()).collect();
    let curseforge: Vec<Arc<str>> = other.into_iter().map(|n| n.get_internal_id()).collect();

    let mut results = Vec::new();

    results.extend(ModrinthBackend::get_info_bulk(&modrinth).await?);
    results.extend(CurseforgeBackend::get_info_bulk(&curseforge).await?);

    Ok(results)
}

pub async fn get_download_link(
    instance: &Instance,
    id: &ModId,
    query_type: QueryType,
) -> Result<String, ModError> {
    match id {
        ModId::Modrinth(n) => ModrinthBackend::get_download_link(instance, n, query_type).await,
        ModId::Curseforge(n) => CurseforgeBackend::get_download_link(instance, n, query_type).await,
    }
}
