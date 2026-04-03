use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use ql_core::{
    InstanceSelection, IntoIoError, IntoJsonError, LAUNCHER_VERSION_NAME, err, file_utils::DirItem,
    info, json::VersionDetails,
};
use zip::ZipWriter;

use crate::{
    presets::{PresetJson, get_instance_type},
    store::{ModConfig, ModError, ModId, ModIndex, SelectedMod},
};

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
    dotmc_entries: Vec<DirItem>,
    include_config: bool,
) -> Result<Vec<u8>, ModError> {
    let dotmc_dir = instance.get_dot_minecraft_path();
    let mods_dir = dotmc_dir.join("mods");
    let config_dir = dotmc_dir.join("config");

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

    let json = PresetJson {
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
    zip.write_all(&serde_json::to_vec(&json).json_to()?)
        .map_err(|n| ModError::ZipIoError(n, "index.json".to_owned()))?;

    let file = zip.finish()?.get_ref().clone();
    info!("Built mod preset! Size: {} bytes", file.len());

    Ok(file)
}

async fn get_minecraft_version(instance_name: &InstanceSelection) -> Result<String, ModError> {
    let version_json = VersionDetails::load(instance_name).await?;
    let minecraft_version = version_json.get_id().to_owned();
    Ok(minecraft_version)
}

fn add_downloaded_mod_to_entries(
    entries_modrinth: &mut HashMap<ModId, ModConfig>,
    index: &ModIndex,
    id: &ModId,
) {
    let Some(config) = index.mods.get(id) else {
        err!("Could not find id {id:?} in index!");
        return;
    };

    entries_modrinth.insert(id.clone(), config.clone());

    for dep in &config.dependencies {
        add_downloaded_mod_to_entries(entries_modrinth, index, &dep);
    }
}

fn is_already_covered(index: &ModIndex, mod_name: &String) -> bool {
    for config in index.mods.values() {
        if config.files.iter().any(|n| n.filename == *mod_name) {
            return true;
        }
    }
    false
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
