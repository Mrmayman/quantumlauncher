use ql_core::{
    GenericProgress, InstanceSelection, IntoIoError, IntoJsonError, ListEntry, Progress,
    file_utils::{self, exists},
    info,
    json::{InstanceConfigJson, VersionDetails},
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

use crate::InstanceInfo;

use super::InstancePackageError;

use ql_mod_manager::store::{CurseforgeNotAllowed, modpack};

pub const OUT_OF: usize = 4;

/// Imports a Minecraft instance from a `.zip` file exported by the launcher.
///
/// This function performs the following:
/// 1. Extracts the ZIP archive to a temporary directory.
/// 2. Reads the `quantum-config.json` from the extracted directory to get instance metadata.
/// 3. Creates a new instance using the extracted configuration.
/// 4. Copies the extracted files to the main instances directory.
///
/// Finally, it returns a bool indicating whether the file
/// was an actual packaged instance or not. You can use this
/// for fuzzy file detection, running this function and running
/// something else if it's `false`.
///
/// # Parameters
/// - `zip_path`: The path to the `.zip` archive to import. It must contain a `quantum-config.json` file inside the root of the zipped instance folder.
/// - `assets`: Whether to include additional assets during instance creation.
/// # Returns
/// A `Result` indicating success or containing an error if anything fails.
///
/// # Errors
/// - if ZIP file can't be opened or extracted
/// - if `quantum-config.json` or `details.json` are missing or malformed
/// - if instance creation (downloading) fails
/// - if user doesn't have permission to access launcher dir
pub async fn import_instance(
    zip_path: PathBuf,
    download_assets: bool,
    sender: Option<Sender<GenericProgress>>,
) -> Result<Option<(InstanceSelection, CurseforgeNotAllowed)>, InstancePackageError> {
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

    let try_ql = temp_dir.join("quantum-config.json");
    let try_qmp = temp_dir.join("index.json");
    let try_mmc = temp_dir.join("mmc-pack.json");

    if let Ok(instance_info) = fs::read_to_string(&try_ql).await {
        Ok(Some((
            import_quantumlauncher(
                download_assets,
                temp_dir,
                instance_info,
                sender.map(Arc::new),
            )
            .await?,
            CurseforgeNotAllowed::new(),
        )))
    } else if let Ok(mmc_pack) = fs::read_to_string(&try_mmc).await {
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
) -> Result<Option<(InstanceSelection, CurseforgeNotAllowed)>, InstancePackageError> {
    let zip_bytes = tokio::fs::read(zip_path).await.path(zip_path)?;

    let peek_info = ql_mod_manager::Preset::load(
        InstanceSelection::Instance(qmp_instance_selection(zip_path)),
        &zip_bytes,
        false, // Just peek, don't install
    )
    .await?;

    // Create the instance
    let instance = InstanceSelection::new(&peek_info.instance_name, peek_info.is_server);
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
    let out = ql_mod_manager::Preset::load(
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

#[deprecated = "TODO: Rewrite this"]
async fn import_quantumlauncher(
    download_assets: bool,
    temp_dir: &Path,
    instance_info: String,
    sender: Option<Arc<Sender<GenericProgress>>>,
) -> Result<InstanceSelection, InstancePackageError> {
    info!("Importing QuantumLauncher instance...");

    let instance_info: InstanceInfo = serde_json::from_str(&instance_info).json(instance_info)?;
    let version_json: VersionDetails = VersionDetails::load_from_path(temp_dir).await?;
    let config_json: InstanceConfigJson = {
        let path = temp_dir.join("config.json");
        let file = fs::read_to_string(&path).await.path(&path)?;
        serde_json::from_str(&file).json(file)?
    };

    let instance = InstanceSelection::new(&instance_info.instance_name, instance_info.is_server);

    pt!("Name: {} ", instance_info.instance_name);
    pt!("Version : {}", version_json.get_id());
    pt!("Exceptions : {:?} ", instance_info.exceptions);
    let version = ListEntry::with_kind(version_json.id.clone(), &version_json.r#type);

    let instance =
        create_instance_qmp(download_assets, &sender, version, &instance, temp_dir).await?;
    let instance_path = instance.get_instance_path();

    ql_mod_manager::loaders::install_specified_loader(
        instance.clone(),
        config_json.mod_type,
        sender.clone(),
        None,
    )
    .await
    .map_err(InstancePackageError::Loader)?;

    pt!("Copying packaged files");
    if let Some(sender) = &sender {
        _ = sender.send(GenericProgress {
            done: 2,
            total: OUT_OF,
            message: Some("Copying files...".to_owned()),
            has_finished: false,
        });
    }
    file_utils::copy_dir_recursive(temp_dir, &instance_path).await?;
    info!("Finished importing QuantumLauncher instance");
    Ok(instance)
}

async fn create_instance_qmp(
    download_assets: bool,
    sender: &Option<Arc<Sender<GenericProgress>>>,
    version: ListEntry,
    instance: &InstanceSelection,
    zip_path: &Path,
) -> Result<InstanceSelection, InstancePackageError> {
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
        if instance.is_server() {
            ql_servers::create_server(name.clone(), version, Some(&d_send)).await?;
        } else {
            ql_instances::create_instance(name.clone(), version, Some(d_send), download_assets)
                .await?;
        }
        Ok(InstanceSelection::new(&name, instance.is_server()))
    } else {
        Ok(instance.clone())
    }
}

async fn import_modpack(
    download_assets: bool,
    peek_info: ql_mod_manager::store::modpack::PeekInfo,
    zip_path: &Path,
    sender: Option<Arc<Sender<GenericProgress>>>,
) -> Result<(InstanceSelection, CurseforgeNotAllowed), InstancePackageError> {
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

    let instance = InstanceSelection::new(&instance_name, false);

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
