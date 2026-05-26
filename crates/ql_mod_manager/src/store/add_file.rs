use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use owo_colors::OwoColorize;
use ql_core::{GenericProgress, Instance, IntoIoError, err, json::VersionDetails, pt};

use crate::{
    presets,
    store::{DirStructure, QueryType, download_mods_bulk},
};

use super::{
    CurseforgeNotAllowed,
    modpack::{self, PackError},
};

pub async fn add_files(
    instance: Instance,
    paths: Vec<PathBuf>,
    progress: Option<Sender<GenericProgress>>,
    project_type: QueryType,
) -> Result<HashSet<CurseforgeNotAllowed>, PackError> {
    let version_json = VersionDetails::load(&instance).await?;
    let mods_dir = instance.get_dot_minecraft_path().join("mods");
    let dirs = DirStructure::new(instance.clone(), &version_json).await?;

    let mut not_allowed = HashSet::new();

    send_progress(progress.as_ref(), &GenericProgress::default());

    let len = paths.len();
    for (i, path) in paths.into_iter().enumerate() {
        pt!("Adding file: {}", path.to_string_lossy().bright_black());
        let (Some(extension), Some(filename)) =
            (path.extension().and_then(OsStr::to_str), path.file_name())
        else {
            continue;
        };

        let extension = extension.to_lowercase();
        send_progress(
            progress.as_ref(),
            &GenericProgress {
                done: i,
                total: len,
                message: Some(format!("Installing {project_type}: ({}/{len})", i + 1)),
                has_finished: false,
            },
        );

        match extension.as_str() {
            "jar" => {
                tokio::fs::copy(&path, mods_dir.join(filename))
                    .await
                    .path(&path)?;
            }

            "zip" => {
                let Some(dir) = dirs.get(project_type) else {
                    modpack(&instance, progress.as_ref(), &mut not_allowed, &path).await?;
                    continue;
                };
                tokio::fs::copy(&path, dir.join(filename))
                    .await
                    .path(&path)?;
            }
            "mrpack" => {
                modpack(&instance, progress.as_ref(), &mut not_allowed, &path).await?;
            }
            "qmp" => {
                let file = tokio::fs::read(&path).await.path(&path)?;
                let out = presets::Preset::load(instance.clone(), file, true).await?;
                if !out.to_install.is_empty() {
                    download_mods_bulk(out.to_install, instance.clone(), progress.clone()).await?;
                }
            }

            extension => {
                err!("While adding mod files: Unrecognized extension: .{extension}");
            }
        }
    }

    send_progress(progress.as_ref(), &GenericProgress::finished());

    Ok(not_allowed)
}

async fn modpack(
    instance: &Instance,
    progress: Option<&Sender<GenericProgress>>,
    not_allowed: &mut HashSet<CurseforgeNotAllowed>,
    path: &Path,
) -> Result<(), PackError> {
    let file = tokio::fs::read(path).await.path(path)?;
    let filename = path.file_name().and_then(|n| n.to_str()).map(str::to_owned);

    if let Some(not_allowed_new) =
        modpack::install_modpack(file, filename, instance.clone(), progress).await?
    {
        not_allowed.extend(not_allowed_new);
    }

    Ok(())
}

fn send_progress(sender: Option<&Sender<GenericProgress>>, progress: &GenericProgress) {
    if let Some(sender) = sender {
        if sender.send(progress.clone()).is_ok() {
            return;
        }
    }
    if let Some(msg) = &progress.message {
        pt!("{msg}");
    }
}
