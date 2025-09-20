use std::{
    path::Path,
    sync::{mpsc::Sender, Mutex},
};

use ql_core::{
    do_jobs, file_utils, info,
    json::{instance_config::ModTypeInfo, FabricJSON, VersionDetails},
    pt, GenericProgress, InstanceSelection, IntoIoError, IntoJsonError, RequestError, LAUNCHER_DIR,
};
use version_compare::compare_versions;

use crate::loaders::fabric::version_list::get_latest_cursed_legacy_commit;

use super::change_instance_type;

mod error;
pub use error::FabricInstallError;
mod make_launch_jar;
mod uninstall;
pub use uninstall::{uninstall, uninstall_client, uninstall_server};
mod version_compare;
mod version_list;

pub use version_list::{
    get_list_of_versions, BackendType, FabricVersion, FabricVersionListItem, VersionList,
};

const CURSED_LEGACY_JSON: &str =
    &include_str!("../../../../../assets/installers/cursed_legacy_fabric.json");

async fn download_file_to_string(url: &str, backend: BackendType) -> Result<String, RequestError> {
    file_utils::download_file_to_string(
        &format!(
            "{}{}{url}",
            backend.get_url(),
            if url.starts_with('/') { "" } else { "/" },
        ),
        false,
    )
    .await
}

pub async fn install_server(
    loader_version: String,
    server_name: String,
    progress: Option<&Sender<GenericProgress>>,
    backend: BackendType,
) -> Result<(), FabricInstallError> {
    let loader_name = if backend.is_quilt() {
        "Quilt"
    } else {
        "Fabric"
    };
    info!("Installing {loader_name} (version {loader_version}) for server");

    if let Some(progress) = &progress {
        _ = progress.send(GenericProgress::default());
    }

    let server_dir = LAUNCHER_DIR.join("servers").join(server_name);

    let libraries_dir = server_dir.join("libraries");
    tokio::fs::create_dir_all(&libraries_dir)
        .await
        .path(&libraries_dir)?;

    let json = VersionDetails::load_from_path(&server_dir).await?;
    let game_version = json.get_id();

    let json = if let BackendType::CursedLegacy = backend {
        CURSED_LEGACY_JSON.replace("INSERT_COMMIT", &get_latest_cursed_legacy_commit().await?)
    } else {
        get_fabric_json(&loader_version, backend, game_version, "server").await?
    };

    let json_path = server_dir.join("fabric.json");
    tokio::fs::write(&json_path, &json).await.path(json_path)?;

    let json: FabricJSON = serde_json::from_str(&json).json(json)?;

    let number_of_libraries = json.libraries.len() + 1;
    let i = Mutex::new(0);

    let library_files = do_jobs(json.libraries.iter().map(|library| async {
        let library_path = libraries_dir.join(library.get_path());

        let library_parent_dir = library_path
            .parent()
            .ok_or(FabricInstallError::PathBufParentError(library_path.clone()))?;
        tokio::fs::create_dir_all(&library_parent_dir)
            .await
            .path(library_parent_dir)?;

        file_utils::download_file_to_path(&library.get_url(), false, &library_path).await?;

        send_progress(&i, library, progress, number_of_libraries);
        Ok::<_, FabricInstallError>(library_path.clone())
    }))
    .await?;

    // TODO: Check if Legacy Fabric needs this
    let shade_libraries = (matches!(backend, BackendType::Fabric)
        && compare_versions(&loader_version, "0.12.5").is_le())
        | matches!(backend, BackendType::CursedLegacy);
    let launch_jar = server_dir.join("fabric-server-launch.jar");

    info!("Making launch jar");
    make_launch_jar::make_launch_jar(
        &launch_jar,
        &server_dir,
        json.mainClassServer.as_deref().unwrap_or(&json.mainClass),
        &library_files,
        shade_libraries,
    )
    .await?;

    change_instance_type(
        &server_dir,
        loader_name.to_owned(),
        Some(ModTypeInfo {
            version: loader_version,
            backend_implementation: if let BackendType::Fabric | BackendType::Quilt = backend {
                None
            } else {
                Some(backend.to_string())
            },
        }),
    )
    .await?;

    if let Some(progress) = &progress {
        _ = progress.send(GenericProgress::finished());
    }

    info!("Finished installing {loader_name}");

    Ok(())
}

pub async fn install_client(
    loader_version: String,
    instance_name: String,
    progress: Option<&Sender<GenericProgress>>,
    backend: BackendType,
) -> Result<(), FabricInstallError> {
    let loader_name = if backend.is_quilt() {
        "Quilt"
    } else {
        "Fabric"
    };

    let instance_dir = LAUNCHER_DIR.join("instances").join(instance_name);

    migrate_index_file(&instance_dir).await?;

    let lock_path = instance_dir.join("fabric.lock");
    tokio::fs::write(
        &lock_path,
        "If you see this, fabric/quilt was not installed correctly.",
    )
    .await
    .path(&lock_path)?;

    let libraries_dir = instance_dir.join("libraries");

    let version_json = VersionDetails::load_from_path(&instance_dir).await?;
    let game_version = version_json.get_id();

    let json_path = instance_dir.join("fabric.json");
    let json = if let BackendType::CursedLegacy = backend {
        CURSED_LEGACY_JSON.replace("INSERT_COMMIT", &get_latest_cursed_legacy_commit().await?)
    } else {
        get_fabric_json(&loader_version, backend, game_version, "profile").await?
    };
    tokio::fs::write(&json_path, &json).await.path(json_path)?;

    let json: FabricJSON = serde_json::from_str(&json).json(json)?;

    info!("Started installing {loader_name}: {game_version}, {loader_version}");

    if let Some(progress) = &progress {
        _ = progress.send(GenericProgress::default());
    }

    let number_of_libraries = json.libraries.len();
    let i = Mutex::new(0);

    do_jobs(json.libraries.iter().map(|library| async {
        let path = libraries_dir.join(library.get_path());
        let url = library.get_url();

        let parent_dir = path
            .parent()
            .ok_or(FabricInstallError::PathBufParentError(path.clone()))?;
        tokio::fs::create_dir_all(parent_dir)
            .await
            .path(parent_dir)?;
        file_utils::download_file_to_path(&url, false, &path).await?;

        send_progress(&i, library, progress, number_of_libraries);
        Ok::<_, FabricInstallError>(())
    }))
    .await?;

    change_instance_type(
        &instance_dir,
        loader_name.to_owned(),
        Some(ModTypeInfo {
            version: loader_version,
            backend_implementation: if let BackendType::Fabric | BackendType::Quilt = backend {
                None
            } else {
                Some(backend.to_string())
            },
        }),
    )
    .await?;

    if let Some(progress) = &progress {
        _ = progress.send(GenericProgress::default());
    }

    tokio::fs::remove_file(&lock_path).await.path(lock_path)?;

    info!("Finished installing {loader_name}",);

    Ok(())
}

async fn get_fabric_json(
    loader_version: &str,
    backend: BackendType,
    game_version: &str,
    implementation: &str,
) -> Result<String, FabricInstallError> {
    let implementation_kind = if implementation == "server" {
        "server"
    } else {
        "client"
    };

    Ok(if let BackendType::OrnitheMC = backend {
        let url1 = format!("https://meta.ornithemc.net/v3/versions/fabric-loader/{game_version}/{loader_version}/{implementation}/json");
        let url2 = format!("https://meta.ornithemc.net/v3/versions/fabric-loader/{game_version}-{implementation_kind}/{loader_version}/{implementation}/json");
        match file_utils::download_file_to_string(&url1, false).await {
            Ok(n) => n,
            Err(err) => match file_utils::download_file_to_string(&url2, false).await {
                Ok(n) => n,
                Err(_) => Err(err)?,
            },
        }
    } else {
        download_file_to_string(
            &format!("/versions/loader/{game_version}/{loader_version}/{implementation}/json"),
            backend,
        )
        .await?
    })
}

async fn migrate_index_file(instance_dir: &Path) -> Result<(), FabricInstallError> {
    let old_index_dir = instance_dir.join(".minecraft/mods/index.json");
    let new_index_dir = instance_dir.join(".minecraft/mod_index.json");
    if old_index_dir.exists() {
        let index = tokio::fs::read_to_string(&old_index_dir)
            .await
            .path(&old_index_dir)?;

        tokio::fs::remove_file(&old_index_dir)
            .await
            .path(old_index_dir)?;
        tokio::fs::write(&new_index_dir, &index)
            .await
            .path(new_index_dir)?;
    }
    Ok(())
}

fn send_progress(
    i: &Mutex<usize>,
    library: &ql_core::json::fabric::Library,
    progress: Option<&Sender<GenericProgress>>,
    number_of_libraries: usize,
) {
    let mut i = i.lock().unwrap();
    *i += 1;
    let i = *i;
    let message = format!(
        "Downloaded library ({} / {number_of_libraries}) {}",
        i + 1,
        library.name
    );
    pt!("{message}");
    if let Some(progress) = progress {
        _ = progress.send(GenericProgress {
            done: i + 1,
            total: number_of_libraries,
            message: Some(message),
            has_finished: false,
        });
    }
}

/// Installs Fabric or Quilt to the given instance.
///
/// # Arguments
/// - `loader_version` - (Optional) The version of the loader to install.
///   Will pick the latest compatible one if not specified.
/// - `instance_name` - The name of the instance to install to.
///   `InstanceSelection::Instance(n)` for a client instance,
///   `InstanceSelection::Server(n)` for a server instance.
/// - `progress` - A channel to send progress updates to.
/// - `is_quilt` - Whether to install Quilt instead of Fabric.
///   As much as people want you to think, Quilt is almost
///   identical to Fabric. So it's just a matter of changing the URL.
///
/// Returns the `is_quilt` bool (so that the launcher can remember
/// whether quilt or fabric was installed)
pub async fn install(
    loader_version: Option<String>,
    instance: InstanceSelection,
    progress: Option<&Sender<GenericProgress>>,
    mut backend: BackendType,
) -> Result<(), FabricInstallError> {
    let loader_version = if let Some(n) = loader_version {
        n
    } else {
        let (list, new_backend) = get_list_of_versions(instance.clone(), backend.is_quilt())
            .await?
            .just_get_one();
        backend = new_backend;
        list.first()
            .ok_or(FabricInstallError::NoVersionFound)?
            .loader
            .version
            .clone()
    };
    match instance {
        InstanceSelection::Instance(n) => {
            install_client(loader_version, n, progress, backend).await
        }
        InstanceSelection::Server(n) => install_server(loader_version, n, progress, backend).await,
    }
}
