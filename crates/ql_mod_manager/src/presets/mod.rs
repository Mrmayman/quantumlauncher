//! Module to [`generate`] and [`load`] modpacks (collections of mods) in various formats.
//!
//! # Formats
//! - QMP (QuantumLauncher preset)
//! - Curseforge modpack (zip with manifest.json) (TODO: generate)
//! - Modrinth modpack (zip with modrinth.index.json) (TODO: generate)
//!
//! # QMP Format
//!
//! Includes
//! - Installed mods (both from store and from outside)
//! - Mod configuration
//!
//! Mod presets consist of a `.qmp` file (actually a zip), containing:
//! - `index.json` of `Preset` struct, contains info of mods downloaded from the store,
//!   only storing links instead of files to save space.
//! - `.jar` files in the root of the zip (at the top level),
//!   for any local, sideloaded mods from outside the mod store.
//!   - Note: mods installed through the mod store shouldn't be saved
//!     here, but rather their details should be entered in the `index.json`
//!   - Also, be careful regarding untrusted local mods, anyone can put malware there
//! - All configuration files in a `config/` folder. This will be extracted
//!   to the `.minecraft/config/` folder

use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

use owo_colors::OwoColorize;
use ql_core::{
    err, info,
    json::{InstanceConfigJson, VersionDetails},
    pt, InstanceSelection, IntoIoError, IntoJsonError, Loader, ModId, SelectedMod,
    LAUNCHER_VERSION_NAME,
};
use serde::{Deserialize, Serialize};
use zip::ZipWriter;

use crate::store::{install_modpack, CurseforgeNotAllowed, ModConfig, ModError, ModIndex};

#[must_use]
#[derive(Debug, Clone, Default)]
pub struct PresetOutput {
    pub local_files: Vec<String>,
    pub to_install: Vec<ModId>,
    pub curseforge_blocked: HashSet<CurseforgeNotAllowed>,
}

/// Generates a "Mod Preset" from the mods
/// installed in the `instance`.
///
/// This packages the contents of
/// `.minecraft/mods` and optionally `.minecraft/config`
/// into a `.qmp` file (a specialized ZIP file).
///
/// You have to manually provide which of the
/// instance's mods you want through the `selected_mods`
/// argument. You *can't* leave it empty, or nothing
/// will generate.
///
/// If `include_config` is true, the `config/` directory
/// will be included in the preset.
///
/// This returns a `Result` of `Vec<u8>`, containing
/// the bytes of the final `.qmp` file that you can save
/// anywhere you want.
pub async fn generate(
    instance: InstanceSelection,
    selected_mods: HashSet<SelectedMod>,
    include_config: bool,
) -> Result<Vec<u8>, ModError> {
    let dot_minecraft = instance.get_dot_minecraft_path();
    let mods_dir = dot_minecraft.join("mods");
    let config_dir = dot_minecraft.join("config");

    let minecraft_version = get_minecraft_version(&instance).await?;
    let instance_type = get_instance_type(&instance).await?;

    let index = ModIndex::load(&instance).await?;

    let mut entries_downloaded = HashMap::new();
    let mut entries_local: Vec<(String, Vec<u8>)> = Vec::new();

    for entry in selected_mods {
        match entry {
            SelectedMod::Downloaded { id, .. } => {
                add_downloaded_mod_to_entries(&mut entries_downloaded, &index, &id);
            }
            SelectedMod::Local { file_name } => {
                if is_already_covered(&index, &file_name) {
                    continue;
                }

                let entry = mods_dir.join(&file_name);
                let mod_bytes = tokio::fs::read(&entry).await.path(&entry)?;
                entries_local.push((file_name.clone(), mod_bytes));
            }
        }
    }

    let this = Preset {
        instance_type,
        launcher_version: LAUNCHER_VERSION_NAME.to_owned(),
        minecraft_version,
        entries_downloaded,
        entries_local: entries_local.iter().map(|(n, _)| n).cloned().collect(),
    };

    let file: Vec<u8> = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(file));

    for (name, bytes) in entries_local {
        zip.start_file(&name, zip::write::FileOptions::<()>::default())?;
        zip.write_all(&bytes)
            .map_err(|n| ModError::ZipIoError(n, name.clone()))?;
    }

    if include_config && config_dir.is_dir() {
        add_dir_to_zip_recursive(&config_dir, &mut zip, PathBuf::from("config")).await?;
    }

    zip.start_file("index.json", zip::write::FileOptions::<()>::default())?;
    let this_str = serde_json::to_string(&this).json_to()?;
    let this_str = this_str.as_bytes();
    zip.write_all(this_str)
        .map_err(|n| ModError::ZipIoError(n, "index.json".to_owned()))?;

    let file = zip.finish()?.get_ref().clone();
    info!("Built mod preset! Size: {} bytes", file.len());

    Ok(file)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Preset {
    launcher_version: String,
    minecraft_version: String,
    instance_type: Loader,
    #[serde(rename = "entries_modrinth")]
    entries_downloaded: HashMap<String, ModConfig>,
    entries_local: Vec<String>,
}

/// Installs a `.qmp` file as a "Mod Preset".
///
/// See the module documentation for what a preset is.
///
/// # Arguments
/// - `instance: InstanceSelection`:
///   The instance to which the preset will be installed.
/// - `zip: Vec<u8>`:
///   The `.qmp` file in binary form. Must be read from
///   disk earlier.
/// - `apply: bool`: Whether to actually install
///   the preset or **just preview it**
///
/// Returns a `Vec<String>` of mod id's to be installed
/// to "complete" the installation. You pass this to
/// [`crate::store::download_mods_bulk`]
///
/// # Errors
/// - The provided `zip` is not a valid `.zip` file.
/// - `index.json` in the zip file isn't valid JSON
/// - User lacks permission to access `QuantumLauncher/` folder
/// - instance directory is outside the launcher directory (escape attack)
/// ---
/// `details.json` and `config.json` (in instance dir):
/// - couldn't be loaded from disk
/// - couldn't be parsed into valid JSON
/// ---
/// - And many other things I probably forgot
pub async fn load(
    instance: InstanceSelection,
    file: Vec<u8>,
    apply: bool,
) -> Result<PresetOutput, ModError> {
    info!("Importing mod preset");

    let main_dir = instance.get_dot_minecraft_path();
    let mods_dir = main_dir.join("mods");

    let mut zip = zip::ZipArchive::new(Cursor::new(&file)).map_err(ModError::Zip)?;

    let version_json = VersionDetails::load(&instance).await?;
    let mut local_files = Vec::new();

    let index: Preset = {
        let Ok(mut index) = zip.by_name("index.json") else {
            // Else this ain't a QMP file!
            // Install as regular modpack
            return match install_modpack(file.clone(), instance.clone(), None)
                .await
                .map_err(Box::new)?
            {
                Some(curseforge_blocked) => Ok(PresetOutput {
                    local_files: Vec::new(),
                    to_install: Vec::new(),
                    curseforge_blocked,
                }),
                None => Err(ModError::NotValidPack),
            };
        };
        let buf = std::io::read_to_string(&mut index)
            .map_err(|n| ModError::ZipIoError(n, "index.json".to_owned()))?;
        serde_json::from_str(&buf).json(buf)?
    };

    let instance_type = get_instance_type(&instance).await?;
    // Only sideload mods if the version is the same
    let should_sideload =
        index.minecraft_version == version_json.get_id() && index.instance_type == instance_type;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(ModError::Zip)?;
        let name = file.name().to_owned();

        if name == "index.json" {
        } else if name.starts_with("config/") || name.starts_with("config\\") {
            if !apply {
                continue;
            }
            if !name.ends_with('/') && !name.ends_with('\\') {
                pt!("Config: {}", name.bright_black());
            }
            let path = main_dir.join(name.replace('\\', "/"));

            if file.is_dir() {
                tokio::fs::create_dir_all(&path).await.path(&path)?;
            } else {
                let parent = path.parent().unwrap();
                tokio::fs::create_dir_all(parent).await.path(parent)?;

                let mut buf = Vec::new();
                file.read_to_end(&mut buf)
                    .map_err(|n| ModError::ZipIoError(n, name.clone()))?;
                tokio::fs::write(&path, &buf).await.path(&path)?;
            }
        } else if name.contains('/') || name.contains('\\') {
            info!("Feature not implemented: {name}");
        } else {
            if !should_sideload {
                continue;
            }
            local_files.push(name.clone());
            if !apply {
                continue;
            }

            pt!("Local file: {name}");
            let path = mods_dir.join(&name);
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|n| ModError::ZipIoError(n, name))?;
            tokio::fs::write(&path, &buf).await.path(&path)?;
        }
    }

    let to_install = index
        .entries_downloaded
        .into_iter()
        .filter_map(|(k, n)| n.manually_installed.then_some(ModId::from_index_str(&k)))
        .collect();

    Ok(PresetOutput {
        local_files,
        to_install,
        curseforge_blocked: HashSet::new(),
    })
}

async fn get_instance_type(instance_name: &InstanceSelection) -> Result<Loader, ModError> {
    let config = InstanceConfigJson::read(instance_name).await?;
    Ok(config.mod_type)
}

fn add_downloaded_mod_to_entries(
    entries_modrinth: &mut HashMap<String, ModConfig>,
    index: &ModIndex,
    id: &ModId,
) {
    let id_str = id.get_index_str();
    let Some(config) = index.mods.get(&id_str) else {
        err!("Could not find id {id:?} ({id_str}) in index!");
        return;
    };

    entries_modrinth.insert(id_str, config.clone());

    for dep in &config.dependencies {
        add_downloaded_mod_to_entries(entries_modrinth, index, &ModId::from_index_str(dep));
    }
}

async fn get_minecraft_version(instance_name: &InstanceSelection) -> Result<String, ModError> {
    let version_json = VersionDetails::load(instance_name).await?;
    let minecraft_version = version_json.get_id().to_owned();
    Ok(minecraft_version)
}

async fn add_dir_to_zip_recursive(
    path: &Path,
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    accumulation: PathBuf,
) -> Result<(), ModError> {
    let mut dir = tokio::fs::read_dir(path).await.path(path)?;

    // # Explanation
    // For example, if the dir structure is:
    //
    // config
    // |- file1.txt
    // |- file2.txt
    // |- dir1
    // | |- file3.txt
    // | |- file4.txt
    //
    // Assume accumulation is "config" for example...

    while let Some(entry) = dir.next_entry().await.path(path)? {
        let path = entry.path();
        let accumulation = accumulation.join(path.file_name().unwrap());
        let acc_name = accumulation.to_string_lossy();

        if path.is_dir() {
            zip.add_directory(
                format!("{acc_name}/"),
                zip::write::FileOptions::<()>::default(),
            )
            .map_err(ModError::Zip)?;

            // ... accumulation = "config/dir1"
            // Then this call will have "config/dir1" as starting value.
            Box::pin(add_dir_to_zip_recursive(&path, zip, accumulation.clone())).await?;
        } else {
            // ... accumulation = "config/file1.txt"
            let bytes = tokio::fs::read(&path).await.path(path.clone())?;

            zip.start_file(&acc_name, zip::write::FileOptions::<()>::default())?;
            zip.write_all(&bytes)
                .map_err(|n| ModError::ZipIoError(n, acc_name.to_string()))?;
        }
    }

    Ok(())
}

fn is_already_covered(index: &ModIndex, mod_name: &String) -> bool {
    for config in index.mods.values() {
        if config.files.iter().any(|n| n.filename == *mod_name) {
            return true;
        }
    }
    false
}
