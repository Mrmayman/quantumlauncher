use std::{
    collections::HashSet,
    io::{Cursor, Read},
    sync::mpsc::Sender,
};

use ql_core::{
    GenericProgress, Instance, IntoIoError, IntoJsonError, err, info,
    json::{InstanceConfigJson, VersionDetails},
    pt,
};

mod curseforge;
mod error;
mod modrinth;

pub use error::PackError;

use crate::{Preset, store::download_mods_bulk};

use super::CurseforgeNotAllowed;

/// Installs a Curseforge or Modrinth modpack.
///
/// Unlike [`crate::Preset`] (`.qmp`) which are QuantumLauncher-only,
/// these are standard modpack files.
///
/// Returns:
/// - `Ok(Some(HashSet))` for Curseforge mods that must be downloaded manually (from browser).
/// - `Ok(None)` if the format is unsupported.
/// - `Err(_)` on failure.
pub async fn install_modpack(
    file: Vec<u8>,
    filename: Option<String>,
    instance: Instance,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<Option<HashSet<CurseforgeNotAllowed>>, PackError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(file.as_slice()))?;

    info!("Installing modpack");
    let json = VersionDetails::load(&instance).await?;

    // If user accidentally added regular file
    if zip.by_name("pack.mcmeta").is_ok() {
        if zip.file_names().any(|n| n.starts_with("data/")) {
            write_regular_file(&file, filename, &instance, "datapacks").await?;
        } else {
            // Resource Pack/Canvas Shader
            let dir = if json.is_legacy_texturepacks() {
                "texturepacks"
            } else {
                "resourcepacks"
            };
            write_regular_file(&file, filename, &instance, dir).await?;
        }
        return Ok(Some(HashSet::new()));
    } else if zip.file_names().any(|n| n.starts_with("shaders/")) {
        // Shader pack
        write_regular_file(&file, filename, &instance, "shaderpacks").await?;
        return Ok(Some(HashSet::new()));
    }

    let index_json_modrinth: Option<modrinth::PackIndex> =
        read_json_from_zip(&mut zip, "modrinth.index.json")?;
    let index_json_curseforge: Option<curseforge::PackIndex> =
        read_json_from_zip(&mut zip, "manifest.json")?;

    if index_json_modrinth.is_none() && index_json_curseforge.is_none() {
        if zip.by_name("index.json").is_ok() {
            // Then it's a QMP preset?

            // Recursion: Won't happen as this function is only called by [`Preset::load`]
            // if there's no `index.json`
            let out = Box::pin(Preset::load(instance.clone(), file, true)).await?;

            return Box::pin(download_mods_bulk(
                out.to_install,
                instance,
                sender.cloned(),
            ))
            .await
            .map(|n| if n.is_empty() { None } else { Some(n) })
            .map_err(PackError::Mod);
        }
        return Err(PackError::NoBackendFound);
    }

    let overrides = index_json_curseforge
        .as_ref()
        .map_or("overrides".to_owned(), |n| n.overrides.clone());

    let mc_dir = instance.get_dot_minecraft_path();
    let config = InstanceConfigJson::read(&instance).await?;

    let mut is_valid = false;

    if let Some(index) = index_json_modrinth {
        is_valid = true;
        modrinth::install(&instance, &mc_dir, &config, &json, &index, sender).await?;
    }
    let not_allowed = if let Some(index) = index_json_curseforge {
        is_valid = true;
        curseforge::install(&instance, &config, &json, &index, sender).await?
    } else {
        HashSet::new()
    };

    if !is_valid {
        return Ok(None);
    }

    let len = zip.len();
    for i in 0..len {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_owned();

        if name == "modrinth.index.json" || name == "manifest.json" || name == "modlist.html" {
            continue;
        }

        if let Some(sender) = sender {
            _ = sender.send(GenericProgress {
                done: i,
                total: len,
                message: Some(format!(
                    "Modpack: Creating overrides: {name} ({i}/{len})",
                    i = i + 1
                )),
                has_finished: false,
            });
        }

        if let Some(name) = name
            .strip_prefix(&format!("{overrides}/"))
            .or(name.strip_prefix(&format!("{overrides}\\")))
        {
            let path = mc_dir.join(name);
            let parent = if file.is_dir() {
                &path
            } else {
                let Some(parent) = path.parent() else {
                    continue;
                };
                parent
            };
            tokio::fs::create_dir_all(parent).await.path(parent)?;

            if file.is_file() {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)
                    .map_err(|n| PackError::ZipIoError(n, name.to_owned()))?;

                tokio::fs::write(&path, &buf).await.path(&path)?;
            }
        } else {
            err!("Unrecognised file: {name}");
        }
    }

    pt!("Done!");

    Ok(Some(not_allowed))
}

async fn write_regular_file(
    file: &[u8],
    name: Option<String>,
    instance: &Instance,
    dir_name: &str,
) -> Result<(), PackError> {
    let dir = instance.get_dot_minecraft_path().join(dir_name);
    tokio::fs::create_dir_all(&dir).await.path(&dir)?;

    let name = name.unwrap_or_else(|| {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|n| n.as_secs())
            .unwrap_or(0);
        format!("{dir_name}_{time}")
    });
    let Some(safe_name) = std::path::Path::new(&name).file_name() else {
        err!("Tried writing outside dir: {name}");
        return Ok(());
    };

    let path = dir.join(safe_name);
    tokio::fs::write(&path, file).await.path(&path)?;
    Ok(())
}

fn read_json_from_zip<T: serde::de::DeserializeOwned>(
    zip: &mut zip::ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<T>, PackError> {
    Ok(if let Ok(mut index_file) = zip.by_name(name) {
        let buf = std::io::read_to_string(&mut index_file)
            .map_err(|n| PackError::ZipIoError(n, name.to_owned()))?;

        Some(serde_json::from_str(&buf).json(buf)?)
    } else {
        None
    })
}
