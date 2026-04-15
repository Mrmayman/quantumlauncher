//! A mod preset: bundle of mods and their configuration.
//!
//! Similar to a modpack, but stored in a more efficient and flexible
//! **QuantumLauncher-specific format**.
//!
//! ## Contents
//! - Installed mods (store + external)
//! - Mod configuration files
//!
//! ## Usage
//! - [`generate`] - create from instance
//! - [`load`] with `apply: true` - install preset
//! - [`load`] with `apply: false` - preview without installing
//!
//! ## Format
//! `.qmp` zip file containing:
//! - `index.json`: serialized [`Preset`] metadata
//! - Top-level `.jar` files for external mods
//! - `config/` directory (extracted to `.minecraft/config/`)

use std::{cmp::Ordering, collections::HashMap, io::ErrorKind};

use ql_core::{
    Instance, IntoIoError, IoError, Loader, file_utils::DirItem, json::InstanceConfigJson,
};
use serde::{Deserialize, Serialize};

use crate::store::{ModConfig, ModError, ModId};

mod generate;
pub use generate::generate;
mod load;
pub use load::{PresetOutput, load};

// TODO: (SERVER) Adapt both of these to also suit Minecraft servers,
// not just clients. This is super important!
const HARD_EXCEPTIONS: &[&str] = &[
    "versions",
    "usercache.json",
    "libraries",
    "resources",
    // Mods
    "mods",
    "mod_index.json",
];
/// Cached/unnecessary files that can be skipped to save space
pub const SOFT_EXCEPTIONS: &[&str] = &[
    "logs",
    "crash-reports",
    "downloads",
    "command_history.txt",
    "realms_persistence.json",
    "debug",
    ".cache",
    "launcher_profiles.json",
    "launcher_profiles_microsoft_store.json",
    // Fabric
    ".fabric",
    "data",
    // Common mods...
    "authlib-injector.log",
    "easy_npc",
    "CustomSkinLoader",
    ".bobby",
    "dynamic-data-pack-cache",
    "dynamic-resource-pack-cache",
    "usernamecache.json",
];

/// The main upfront choices the user will have to make.
///
/// Only for client instances, not servers.
///
/// Includes:
/// - Directory name
/// - Display name
/// - Enabled by default
pub const MAIN_CHOICES: &[(&str, &str, bool)] = &[
    ("saves", "Worlds", false),
    ("resourcepacks", "Resource Packs", true),
    ("texturepacks", "Texture Packs", true),
    ("shaderpacks", "Shaders", true),
];

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PresetJson {
    instance_name: Option<String>,
    is_server: Option<bool>,

    launcher_version: String,
    minecraft_version: String,
    instance_type: Loader,
    #[serde(rename = "entries_modrinth")]
    entries_downloaded: HashMap<ModId, ModConfig>,
    entries_local: Vec<String>,
}

async fn get_instance_type(instance_name: &Instance) -> Result<Loader, ModError> {
    let config = InstanceConfigJson::read(instance_name).await?;
    Ok(config.mod_type)
}

pub async fn get_mc_dir_contents(instance: &Instance) -> Result<Vec<DirItem>, IoError> {
    async fn get_contents_inner(
        dotmc_dir: std::path::PathBuf,
        contents: &mut Vec<DirItem>,
    ) -> Result<(), IoError> {
        let mut dir = tokio::fs::read_dir(&dotmc_dir).await.path(&dotmc_dir)?;

        while let Some(entry) = dir.next_entry().await.path(&dotmc_dir)? {
            if HARD_EXCEPTIONS.iter().any(|n| &entry.file_name() == n) {
                continue;
            }
            let is_file = entry.file_type().await.is_ok_and(|n| !n.is_dir());
            let name = entry.file_name().to_string_lossy().into_owned();
            contents.push(DirItem { name, is_file });
        }

        contents.sort_unstable_by(|a, b| {
            match (
                SOFT_EXCEPTIONS.contains(&a.name.as_str()),
                SOFT_EXCEPTIONS.contains(&b.name.as_str()),
            ) {
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                _ => match (a.is_file, b.is_file) {
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    _ => a.name.cmp(&b.name),
                },
            }
        });

        Ok(())
    }

    let dotmc_dir = instance.get_dot_minecraft_path();
    let mut contents = Vec::new();

    let res = get_contents_inner(dotmc_dir, &mut contents).await;
    if let Err(IoError::Io { error, .. }) = &res {
        if error.kind() != ErrorKind::NotFound {
            res?;
        }
    }

    Ok(contents)
}
