use std::{
    path::Path,
    sync::{Arc, mpsc::Sender},
};

use ql_core::{GenericProgress, Instance, InstanceKind, IntoIoError, ListEntry};
use ql_mod_manager::store::CurseforgeNotAllowed;

use crate::{InstancePackageError, import::pipe_progress};

pub async fn import(
    zip_path: &Path,
    download_assets: bool,
    sender: Option<Arc<Sender<GenericProgress>>>,
) -> Result<Option<(Instance, CurseforgeNotAllowed)>, InstancePackageError> {
    let zip_bytes = tokio::fs::read(zip_path).await.path(zip_path)?;

    let peek_info = ql_mod_manager::presets::load(
        Instance::client(&instance_selection(zip_path)),
        &zip_bytes,
        false, // Just peek, don't install
    )
    .await?;

    let kind = if peek_info.is_server {
        InstanceKind::Server
    } else {
        InstanceKind::Client
    };

    // Create the instance
    let instance = Instance::new(&peek_info.instance_name, kind);
    let instance = create_instance(
        download_assets,
        &sender,
        ListEntry::with_kind(peek_info.game_version.clone(), "release"),
        &instance,
        zip_path,
    )
    .await?;

    // Install the loader
    ql_mod_manager::loaders::install_specified_loader(
        instance.clone(),
        peek_info.mod_type,
        sender.clone(),
        None,
    )
    .await
    .map_err(InstancePackageError::Loader)?;

    // Import the preset
    let out = ql_mod_manager::presets::load(
        instance.clone(),
        &zip_bytes,
        true, // Actually install this time
    )
    .await?;

    let not_allowed = ql_mod_manager::store::download_mods_bulk(
        out.to_install,
        instance.clone(),
        sender.as_deref(),
    )
    .await?;

    Ok(Some((instance, not_allowed)))
}

fn instance_selection(zip_path: &Path) -> String {
    zip_path
        .file_stem()
        .and_then(|n| n.to_str())
        .map(|n| n.to_owned())
        .unwrap_or(format!(
            "imported-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ))
}

async fn create_instance(
    download_assets: bool,
    sender: &Option<Arc<Sender<GenericProgress>>>,
    version: ListEntry,
    instance: &Instance,
    zip_path: &Path,
) -> Result<Instance, InstancePackageError> {
    let (d_send, d_recv) = std::sync::mpsc::channel();
    if let Some(sender) = sender.clone() {
        std::thread::spawn(move || {
            pipe_progress(d_recv, &sender);
        });
    }

    let name = instance.get_name().to_owned();

    let r = if instance.is_server() {
        ql_servers::create_server(name, version.clone(), Some(&d_send))
            .await
            .map_err(InstancePackageError::from)
    } else {
        ql_instances::create_instance(name, version.clone(), Some(d_send.clone()), download_assets)
            .await
            .map_err(InstancePackageError::from)
    };

    if r.as_ref().is_err_and(|n| n.already_exists()) {
        // Use different name
        let name = instance_selection(zip_path);
        match instance.kind {
            InstanceKind::Server => {
                ql_servers::create_server(name.clone(), version, Some(&d_send)).await?;
            }
            InstanceKind::Client => {
                ql_instances::create_instance(name.clone(), version, Some(d_send), download_assets)
                    .await?;
            }
        }
        Ok(Instance::new(&name, instance.kind))
    } else {
        Ok(instance.clone())
    }
}
