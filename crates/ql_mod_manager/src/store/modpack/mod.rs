use std::{
    io::{Cursor, Read},
    sync::mpsc::Sender,
};

use ql_core::{
    GenericProgress, InstanceSelection, IntoIoError, IntoJsonError, Loader, err, info,
    json::{InstanceConfigJson, VersionDetails},
    pt,
};

mod curseforge;
mod error;
mod modrinth;

pub use error::PackError;

use crate::{presets, store::download_mods_bulk};

use super::CurseforgeNotAllowed;

#[derive(Debug, Clone)]
pub struct PeekInfo {
    pub name: String,
    pub game_version: String,
    pub loader: Loader,
    pub recommended_ram_mb: Option<usize>,
}

/// Installs a modpack file (Curseforge or Modrinth) to the instance.
///
/// Not to be confused with [`crate::Preset`] (QuantumLauncher-only `.qmp` presets).
///
/// # Arguments
/// - `file`: Modpack file bytes.
/// - `instance`: Target instance.
/// - `sender`: Optional progress notifier.
///
/// # Returns
/// - `Ok(bool, CurseForgeNotAllowed)`:
///     1) Whether the modpack was recognized and is valid.
///     2) Mods blocked by Curseforge (must download manually).
/// - `Err`: Installation error.
pub async fn install(
    file: &[u8],
    instance: InstanceSelection,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<(bool, CurseforgeNotAllowed), PackError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(file))?;

    info!("Installing modpack");

    let index_json_modrinth: Option<modrinth::PackIndex> =
        read_json_from_zip(&mut zip, "modrinth.index.json")?;
    let index_json_curseforge: Option<curseforge::PackIndex> =
        read_json_from_zip(&mut zip, "manifest.json")?;

    if index_json_modrinth.is_none() && index_json_curseforge.is_none() {
        if zip.by_name("index.json").is_ok() {
            // Then it's a QMP preset?

            // Recursion: Won't happen as this function is only called by [`Preset::load`]
            // if there's no `index.json`
            let out = Box::pin(presets::load(instance.clone(), file, true)).await?;

            return Box::pin(download_mods_bulk(out.to_install, instance, sender))
                .await
                .map(|n| (true, n))
                .map_err(|n| n.into());
        }
        return Err(PackError::NoBackendFound);
    }

    let overrides = index_json_curseforge
        .as_ref()
        .map_or("overrides".to_owned(), |n| n.overrides.clone());

    let mc_dir = instance.get_dot_minecraft_path();
    let config = InstanceConfigJson::read(&instance).await?;
    let json = VersionDetails::load(&instance).await?;

    let mut is_valid = false;

    if let Some(index) = index_json_modrinth {
        is_valid = true;
        modrinth::install(&instance, &mc_dir, &config, &json, &index, sender).await?;
    }
    let not_allowed = if let Some(index) = index_json_curseforge {
        is_valid = true;
        curseforge::install(&instance, &config, &json, &index, sender).await?
    } else {
        CurseforgeNotAllowed::new()
    };

    if !is_valid {
        return Ok((false, CurseforgeNotAllowed::new()));
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

    Ok((true, not_allowed))
}

/// Extracts metadata (name, version, loader) from a modpack file.
///
/// # Arguments
/// - `file`: Modpack file bytes.
///
/// # Returns
/// - `Ok(Some(PeekInfo))`: Modpack metadata.
/// - `Ok(None)`: Not a recognized modpack.
/// - `Err`: Parse or read error.
pub fn peek(file: &[u8]) -> Result<Option<PeekInfo>, PackError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(file))?;

    let index_json_modrinth: Option<modrinth::PackIndex> =
        read_json_from_zip(&mut zip, "modrinth.index.json")?;
    let index_json_curseforge: Option<curseforge::PackIndex> =
        read_json_from_zip(&mut zip, "manifest.json")?;

    if let Some(index) = index_json_modrinth {
        // Handle Modrinth modpack
        let Some(game_version) = index.dependencies.get("minecraft").cloned() else {
            return Ok(None);
        };

        let loader = index
            .dependencies
            .keys()
            .filter(|k| *k != "minecraft")
            .filter_map(|k| match k.as_str() {
                "forge" => Some(Loader::Forge),
                "neoforge" => Some(Loader::Neoforge),
                "fabric-loader" => Some(Loader::Fabric),
                "quilt-loader" => Some(Loader::Quilt),
                _ => None,
            })
            .next()
            .unwrap_or(Loader::Vanilla);

        Ok(Some(PeekInfo {
            name: index.name,
            game_version,
            loader,
            recommended_ram_mb: None,
        }))
    } else if let Some(index) = index_json_curseforge {
        // Handle Curseforge modpack
        let game_version = index.minecraft.version;

        let loader = index
            .minecraft
            .modLoaders
            .first()
            .and_then(|l| {
                let loader_id = l.id.split('-').next().unwrap_or(&l.id);
                match loader_id {
                    "forge" => Some(Loader::Forge),
                    "neoforge" => Some(Loader::Neoforge),
                    "fabric" => Some(Loader::Fabric),
                    "quilt" => Some(Loader::Quilt),
                    _ => None,
                }
            })
            .ok_or(PackError::NoLoadersSpecified)?;

        Ok(Some(PeekInfo {
            name: index.name,
            game_version,
            loader,
            recommended_ram_mb: index.minecraft.recommendedRam,
        }))
    } else {
        Ok(None)
    }
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
