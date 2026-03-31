use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

use owo_colors::OwoColorize;
use ql_core::{
    InstanceSelection, IntoIoError, IntoJsonError, LAUNCHER_VERSION_NAME, Loader, ModId,
    SelectedMod, err, info,
    json::{InstanceConfigJson, VersionDetails},
    pt,
};
use serde::{Deserialize, Serialize};
use zip::ZipWriter;

use crate::store::{ModConfig, ModError, ModIndex};

#[must_use]
#[derive(Debug, Clone, Default)]
pub struct PresetOutput {
    pub instance_name: String,
    pub is_server: bool,

    pub local_files: Vec<String>,
    pub to_install: Vec<ModId>,
    pub is_regular_modpack: bool,

    pub game_version: String,
    pub mod_type: Loader,
}

/// A mod preset: bundle of mods and their configuration.
///
/// Similar to a modpack, but stored in a more efficient and flexible
/// **QuantumLauncher-specific format**.
///
/// ## Contents
/// - Installed mods (store + external)
/// - Mod configuration files
///
/// ## Usage
/// - [`Preset::generate`] - create from instance
/// - [`Preset::load`] with `apply: true` - install preset
/// - [`Preset::load`] with `apply: false` - preview without installing
///
/// ## Format
/// `.qmp` zip file containing:
/// - `index.json`: serialized [`Preset`] metadata
/// - Top-level `.jar` files for external mods
/// - `config/` directory (extracted to `.minecraft/config/`)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Preset {
    pub instance_name: Option<String>,
    pub is_server: Option<bool>,

    pub launcher_version: String,
    pub minecraft_version: String,
    pub instance_type: Loader,
    #[serde(rename = "entries_modrinth")]
    pub entries_downloaded: HashMap<String, ModConfig>,
    pub entries_local: Vec<String>,
}

impl Preset {
    /// Generates a `.qmp` preset from instance mods.
    ///
    /// Packages `.minecraft/mods` and optionally `.minecraft/config` into a preset.
    ///
    /// # Arguments
    /// - `instance`: target instance
    /// - `selected_mods`: mods to include (if empty, no mods will be included!)
    /// - `include_config`: whether to include `config/` directory
    ///
    /// Returns bytes of the generated `.qmp` file.
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

        let this = Self {
            instance_name: Some(instance.get_name().to_owned()),
            is_server: Some(instance.is_server()),
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

    /// Installs or previews a `.qmp` preset file.
    ///
    /// # Arguments
    /// - `instance`: target instance
    /// - `file`: `.qmp` file bytes
    /// - `apply`: whether to install or just preview
    ///
    /// Returns mod IDs for installation with [`crate::store::download_mods_bulk`].
    ///
    /// # Errors
    /// - Invalid zip file or JSON
    /// - Permission or path access issues
    /// - Instance configuration errors
    pub async fn load(
        instance: InstanceSelection,
        file: &[u8],
        apply: bool,
    ) -> Result<PresetOutput, ModError> {
        info!("Importing mod preset");

        let main_dir = instance.get_dot_minecraft_path();
        let mods_dir = main_dir.join("mods");

        let mut zip = zip::ZipArchive::new(Cursor::new(&file)).map_err(ModError::Zip)?;

        let version_json = VersionDetails::load(&instance).await.ok();
        let instance_type = get_instance_type(&instance).await.ok();

        let mut local_files = Vec::new();

        let index: Self = {
            let Ok(mut index) = zip.by_name("index.json") else {
                // Else this ain't a QMP file!
                // Install as regular modpack
                return Ok(PresetOutput {
                    instance_name: instance.get_name().to_owned(),
                    is_server: instance.is_server(),
                    local_files: Vec::new(),
                    to_install: Vec::new(),
                    is_regular_modpack: true,
                    game_version: version_json
                        .map(|n| n.get_id().to_owned())
                        .unwrap_or_default(),
                    mod_type: instance_type.unwrap_or(Loader::Vanilla),
                });
            };
            let buf = std::io::read_to_string(&mut index)
                .map_err(|n| ModError::ZipIoError(n, "index.json".to_owned()))?;
            serde_json::from_str(&buf).json(buf)?
        };

        // Only sideload mods if the version is the same
        let should_sideload = version_json.is_some_and(|n| n.get_id() == index.minecraft_version)
            && instance_type.is_some_and(|n| n == index.instance_type);

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
            instance_name: index
                .instance_name
                .unwrap_or_else(|| instance.get_name().to_owned()),
            is_server: index.is_server.unwrap_or(instance.is_server()),
            local_files,
            to_install,
            is_regular_modpack: false,
            game_version: index.minecraft_version,
            mod_type: index.instance_type,
        })
    }
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
