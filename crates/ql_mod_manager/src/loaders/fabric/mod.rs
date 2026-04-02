use std::{
    path::{Path, PathBuf},
    sync::{Mutex, mpsc::Sender},
};

use ql_core::{
    GenericProgress, InstanceSelection, IntoIoError, IntoJsonError, LAUNCHER_DIR, Loader, do_jobs,
    download,
    file_utils::exists,
    info,
    json::{FabricJSON, V_1_12_2, VersionDetails, instance_config::ModTypeInfo},
    pt,
};
use version_compare::compare_versions;

use crate::loaders::fabric::version_list::get_latest_cursed_legacy_commit;

use super::change_instance_type;

mod error;
pub use error::FabricInstallError;
mod make_launch_jar;
mod uninstall;
pub use uninstall::uninstall;
mod version_compare;
mod version_list;

pub use version_list::{
    BackendType, FabricVersion, FabricVersionList, FabricVersionListItem, get_list_of_versions,
    get_list_of_versions_from_backend,
};

const CURSED_LEGACY_JSON: &str =
    include_str!("../../../../../assets/installers/cursed_legacy_fabric.json");

pub async fn install_server(
    loader_version: String,
    server_name: String,
    progress: Option<&Sender<GenericProgress>>,
    backend: BackendType,
) -> Result<(), FabricInstallError> {
    info!("Installing {backend} (version {loader_version}) for server");
    if let Some(progress) = &progress {
        _ = progress.send(GenericProgress::default());
    }

    let server_dir = LAUNCHER_DIR.join("servers").join(server_name);
    let libraries_dir = server_dir.join("libraries");
    tokio::fs::create_dir_all(&libraries_dir)
        .await
        .path(&libraries_dir)?;

    let version_json = VersionDetails::load_from_path(&server_dir).await?;
    let json: FabricJSON = {
        let json = if let BackendType::CursedLegacy = backend {
            CURSED_LEGACY_JSON.replace("INSERT_COMMIT", &get_latest_cursed_legacy_commit().await?)
        } else {
            get_fabric_json(&loader_version, backend, version_json.get_id(), "server").await?
        };
        let json_path = server_dir.join("fabric.json");
        tokio::fs::write(&json_path, &json).await.path(json_path)?;
        serde_json::from_str(&json).json(json)?
    };

    let number_of_libraries = json.libraries.len() + 1;
    let i = Mutex::new(0);

    let library_files: Vec<PathBuf> = do_jobs(json.libraries.iter().map(|library| {
        download_library(
            library,
            &libraries_dir,
            &version_json,
            &i,
            number_of_libraries,
            progress,
        )
    }))
    .await?
    .into_iter()
    .flatten()
    .collect();

    // TODO: Check if Legacy Fabric needs this
    let shade_libraries = (matches!(backend, BackendType::Fabric | BackendType::LegacyFabric)
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
        if backend.is_quilt() {
            Loader::Quilt
        } else {
            Loader::Fabric
        },
        Some(if let BackendType::Fabric | BackendType::Quilt = backend {
            ModTypeInfo::new_regular(loader_version)
        } else {
            ModTypeInfo::new_with_backend(loader_version, backend.to_string())
        }),
    )
    .await?;

    if let Some(progress) = &progress {
        _ = progress.send(GenericProgress::finished());
    }

    info!("Finished installing {backend}");

    Ok(())
}

async fn download_library(
    library: &ql_core::json::fabric::Library,
    libraries_dir: &Path,
    version_json: &VersionDetails,
    i: &Mutex<usize>,
    number_of_libraries: usize,
    progress: Option<&Sender<GenericProgress>>,
) -> Result<Option<PathBuf>, FabricInstallError> {
    if !library.is_allowed() || (library.is_lwjgl2() && version_json.is_before_or_eq(V_1_12_2)) {
        pt!("Skipping {}", library.name);
        return Ok(None);
    }

    let library_path = libraries_dir.join(library.get_path());

    let library_parent_dir = library_path
        .parent()
        .ok_or(FabricInstallError::PathBufParentError(library_path.clone()))?;
    tokio::fs::create_dir_all(&library_parent_dir)
        .await
        .path(library_parent_dir)?;

    let Some(url) = library.get_url() else {
        pt!("Skipping (no url): {}", library.name);
        return Ok(None);
    };
    download(&url).path(&library_path).await?;

    send_progress(i, library, progress, number_of_libraries);
    Ok::<_, FabricInstallError>(Some(library_path.clone()))
}

pub async fn install_client(
    loader_version: String,
    instance_name: String,
    progress: Option<&Sender<GenericProgress>>,
    backend: BackendType,
) -> Result<(), FabricInstallError> {
    let instance_dir = LAUNCHER_DIR.join("instances").join(instance_name);
    let libraries_dir = instance_dir.join("libraries");
    migrate_index_file(&instance_dir).await?;

    let version_json = VersionDetails::load_from_path(&instance_dir).await?;
    let game_version = version_json.get_id();

    let json: FabricJSON = {
        let json_path = instance_dir.join("fabric.json");
        let json = if let BackendType::CursedLegacy = backend {
            CURSED_LEGACY_JSON.replace("INSERT_COMMIT", &get_latest_cursed_legacy_commit().await?)
        } else {
            get_fabric_json(&loader_version, backend, game_version, "profile").await?
        };
        tokio::fs::write(&json_path, &json).await.path(json_path)?;
        serde_json::from_str(&json).json(json)?
    };

    info!("Started installing {backend}: {game_version}, {loader_version}");
    if let Some(progress) = &progress {
        _ = progress.send(GenericProgress::default());
    }

    let number_of_libraries = json.libraries.len();
    let i = Mutex::new(0);

    do_jobs(json.libraries.iter().map(|library| {
        download_library(
            library,
            &libraries_dir,
            &version_json,
            &i,
            number_of_libraries,
            progress,
        )
    }))
    .await?;

    change_instance_type(
        &instance_dir,
        if backend.is_quilt() {
            Loader::Quilt
        } else {
            Loader::Fabric
        },
        Some(if let BackendType::Fabric | BackendType::Quilt = backend {
            ModTypeInfo::new_regular(loader_version)
        } else {
            ModTypeInfo::new_with_backend(loader_version, backend.to_string())
        }),
    )
    .await?;

    if let Some(progress) = &progress {
        _ = progress.send(GenericProgress::default());
    }
    info!("Finished installing {backend}");

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

    let json = if let BackendType::OrnitheMCFabric | BackendType::OrnitheMCQuilt = backend {
        let fq = if backend.is_quilt() {
            "quilt"
        } else {
            "fabric"
        };
        let url1 = format!(
            "https://meta.ornithemc.net/v3/versions/{fq}-loader/{game_version}/{loader_version}/{implementation}/json"
        );
        let url2 = format!(
            "https://meta.ornithemc.net/v3/versions/{fq}-loader/{game_version}-{implementation_kind}/{loader_version}/{implementation}/json"
        );

        match download(&url1).string().await {
            Ok(n) => n,
            Err(err) => match download(&url2).string().await {
                Ok(n) => n,
                Err(_) => Err(err)?,
            },
        }
    } else {
        download(&format!(
            "{}/versions/loader/{game_version}/{loader_version}/{implementation}/json",
            backend.get_url()
        ))
        .string()
        .await?
    };

    Ok(json)
}

async fn migrate_index_file(instance_dir: &Path) -> Result<(), FabricInstallError> {
    let old_index_dir = instance_dir.join(".minecraft/mods/index.json");
    let new_index_dir = instance_dir.join(".minecraft/mod_index.json");
    if exists(&old_index_dir).await {
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
/// - `instance` - The instance (client/server) to install to
/// - `progress` - (Optional) A channel to send progress updates to.
/// - `backend` - Backend fabric implementation (Fabric/Quilt/Babric/OrnitheMC/...)
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
