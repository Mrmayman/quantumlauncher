use sipper::Sender;
use std::{collections::HashSet, ffi::OsStr, path::PathBuf};

use ql_core::{GenericProgress, Instance, IntoIoError, err, pt};

use crate::{presets, store::download_mods_bulk};

use super::{
    CurseforgeNotAllowed,
    modpack::{self, PackError},
};

pub async fn add_files(
    instance: Instance,
    paths: Vec<PathBuf>,
    mut progress: Option<Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, PackError> {
    let mods_dir = instance.get_dot_minecraft_path().join("mods");

    let mut not_allowed = HashSet::new();

    send_progress(progress.as_mut(), &GenericProgress::default()).await;

    let len = paths.len();
    for (i, path) in paths.into_iter().enumerate() {
        pt!("Adding file: {path:?}");
        let (Some(extension), Some(filename)) =
            (path.extension().and_then(OsStr::to_str), path.file_name())
        else {
            continue;
        };

        let extension = extension.to_lowercase();

        let file_type = match extension.as_str() {
            "jar" => "mod",
            "zip" | "mrpack" => "modpack",
            "qmp" => "QuantumLauncher mod preset",
            _ => "Unknown file (ERROR)",
        };
        send_progress(
            progress.as_mut(),
            &GenericProgress {
                done: i,
                total: len,
                message: Some(format!("Installing {file_type}: ({}/{len})", i + 1)),
                has_finished: false,
            },
        )
        .await;

        match extension.as_str() {
            "jar" => {
                tokio::fs::copy(&path, mods_dir.join(filename))
                    .await
                    .path(&path)?;
            }
            "zip" | "mrpack" => {
                let file = tokio::fs::read(&path).await.path(&path)?;
                if let Some(not_allowed_new) =
                    modpack::install_modpack(file, instance.clone(), progress.as_mut()).await?
                {
                    not_allowed.extend(not_allowed_new);
                }
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

    send_progress(progress.as_mut(), &GenericProgress::finished()).await;

    Ok(not_allowed)
}

async fn send_progress(sender: Option<&mut Sender<GenericProgress>>, progress: &GenericProgress) {
    if let Some(sender) = sender {
        sender.send(progress.clone()).await;
        return;
    }
    if let Some(msg) = &progress.message {
        pt!("{msg}");
    }
}
