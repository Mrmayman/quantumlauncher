use ql_core::{
    GenericProgress, Instance, InstanceKind, IntoIoError, ListEntry, Progress,
    file_utils::{self, exists},
    info,
    json::InstanceConfigJson,
    pt,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{Receiver, Sender},
    },
};
use tokio::fs;

use super::InstancePackageError;

use ql_mod_manager::store::{CurseforgeNotAllowed, modpack};

pub const OUT_OF: usize = 4;

/// Imports a Minecraft instance from various package formats.
///
/// Supports:
/// - QMP
/// - MultiMC/PrismLauncher
/// - Curseforge
/// - Modrinth
///
/// # Parameters
/// - `zip_path`: The path to the archive to import
/// - `download_assets`: Whether to download music/sound
///   (recommended to enable, but disabling this saves space)
/// - `sender`: An optional `Sender` for progress updates
///
/// # Returns
/// - `Ok(Some((instance, curseforge_not_allowed)))`:
///   The imported instance and any applicable Curseforge not allowed warning
/// - `Ok(None)`: Unsupported or invalid package format
/// - `Err(InstancePackageError)`: An error occurred during the import process
///
/// # Errors
/// - if ZIP file can't be opened or extracted
/// - if instance creation, loader installation or mod downloading fails
/// - if user doesn't have permission to access launcher dir
pub async fn import_instance(
    zip_path: PathBuf,
    download_assets: bool,
    sender: Option<Sender<GenericProgress>>,
) -> Result<Option<(Instance, CurseforgeNotAllowed)>, InstancePackageError> {
    let temp_dir_obj = tempfile::TempDir::new().map_err(InstancePackageError::TempDir)?;
    let temp_dir = temp_dir_obj.path();

    pt!("Extracting zip to {temp_dir:?}");
    let zip_file = std::fs::File::open(&zip_path).path(&zip_path)?;
    if let Some(sender) = &sender {
        _ = sender.send(GenericProgress {
            done: 0,
            total: OUT_OF,
            message: Some("Extracting Archive...".to_owned()),
            has_finished: false,
        });
    }
    file_utils::extract_zip_archive(std::io::BufReader::new(zip_file), temp_dir, true).await?;

    let try_qmp = temp_dir.join("index.json");
    let try_mmc = temp_dir.join("mmc-pack.json");

    if let Ok(mmc_pack) = fs::read_to_string(&try_mmc).await {
        Ok(Some((
            crate::multimc::import(download_assets, temp_dir, &mmc_pack, sender.map(Arc::new))
                .await?,
            CurseforgeNotAllowed::new(),
        )))
    } else if exists(&try_qmp).await {
        import_qmp(&zip_path, download_assets, sender.map(Arc::new)).await
    } else {
        // Try modpack fallback
        let zip_bytes = tokio::fs::read(&zip_path).await.path(&zip_path)?;
        Ok(if let Some(peek_info) = modpack::peek(&zip_bytes)? {
            Some(import_modpack(download_assets, peek_info, &zip_path, sender.map(Arc::new)).await?)
        } else {
            None
        })
    }
}

async fn import_qmp(
    zip_path: &PathBuf,
    download_assets: bool,
    sender: Option<Arc<Sender<GenericProgress>>>,
) -> Result<Option<(Instance, CurseforgeNotAllowed)>, InstancePackageError> {
    let zip_bytes = tokio::fs::read(zip_path).await.path(zip_path)?;

    let peek_info = ql_mod_manager::presets::load(
        Instance::client(&qmp_instance_selection(zip_path)),
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
    let instance = create_instance_qmp(
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

fn qmp_instance_selection(zip_path: &Path) -> String {
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

async fn create_instance_qmp(
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
        let name = qmp_instance_selection(zip_path);
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

async fn import_modpack(
    download_assets: bool,
    peek_info: ql_mod_manager::store::modpack::PeekInfo,
    zip_path: &Path,
    sender: Option<Arc<Sender<GenericProgress>>>,
) -> Result<(Instance, CurseforgeNotAllowed), InstancePackageError> {
    info!("Importing modpack as instance...");

    // Generate instance name from modpack metadata first, then fallback to zip filename
    let instance_name = if !peek_info.name.is_empty() {
        peek_info.name.clone()
    } else {
        zip_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("imported-modpack")
            .to_string()
    };

    let instance = Instance::client(&instance_name);

    pt!("Name: {} ", instance_name);
    pt!("Version: {}", peek_info.game_version);
    pt!("Loader: {:?}", peek_info.loader);

    let version = ListEntry::with_kind(peek_info.game_version.clone(), "release");

    let (d_send, d_recv) = std::sync::mpsc::channel();
    if let Some(sender) = sender.clone() {
        std::thread::spawn(move || {
            pipe_progress(d_recv, &sender);
        });
    }

    // Create the instance
    ql_instances::create_instance(instance_name, version, Some(d_send), download_assets).await?;

    // Install the loader
    ql_mod_manager::loaders::install_specified_loader(
        instance.clone(),
        peek_info.loader,
        sender.clone(),
        None,
    )
    .await
    .map_err(InstancePackageError::Loader)?;

    // Install the modpack
    pt!("Installing modpack");
    let zip_bytes = tokio::fs::read(&zip_path).await.path(zip_path)?;
    let (_, not_allowed) = modpack::install(
        &zip_bytes,
        instance.clone(),
        sender.as_ref().map(|s| s.as_ref()),
    )
    .await?;

    if let Some(ram) = peek_info.recommended_ram_mb {
        let mut config = InstanceConfigJson::read(&instance).await?;
        config.ram_in_mb = ram;
        config.save(&instance).await?;
    }

    info!("Finished importing modpack instance");
    Ok((instance, not_allowed))
}

pub fn pipe_progress<T: Progress>(rec: Receiver<T>, snd: &Sender<GenericProgress>) {
    for item in rec {
        _ = snd.send(item.into_generic());
    }
}
