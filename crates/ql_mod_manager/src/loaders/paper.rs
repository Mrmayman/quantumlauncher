use std::{collections::HashMap, path::Path};

use ql_core::{
    file_utils, impl_3_errs_jri, info,
    json::{instance_config::ModTypeInfo, VersionDetails},
    pt, IntoIoError, IoError, JsonError, RequestError, LAUNCHER_DIR,
};
use serde::Deserialize;
use thiserror::Error;

use crate::loaders::change_instance_type;

#[derive(Deserialize)]
pub struct PaperVersions {
    // latest: String,
    versions: HashMap<String, String>,
}

// TODO: Migrate to
// `https://fill.papermc.io/v3/projects/paper/versions/{MC_VERSION}/builds`
const PAPER_VERSIONS_URL: &str = "https://qing762.is-a.dev/api/papermc";

/// Moves a directory from `old_path` to `new_path`.
/// If `new_path` exists, it will be deleted before the move.
///
/// # Arguments
///
/// * `old_path` - The path to the directory you want to move.
/// * `new_path` - The destination path.
///
/// # Errors
///
/// Returns an `IoError` if any operation fails.
async fn move_dir(old_path: &Path, new_path: &Path) -> Result<(), IoError> {
    if new_path.exists() {
        tokio::fs::remove_dir_all(new_path).await.path(new_path)?;
    }
    file_utils::copy_dir_recursive(old_path, new_path).await?;
    tokio::fs::remove_dir_all(old_path).await.path(old_path)?;
    Ok(())
}

pub async fn uninstall(instance_name: String) -> Result<(), PaperInstallerError> {
    let server_dir = LAUNCHER_DIR.join("servers").join(instance_name);

    let jar_path = server_dir.join("paper_server.jar");
    tokio::fs::remove_file(&jar_path).await.path(jar_path)?;

    // Paper stores Nether and End dimension worlds
    // in a separate directory, so we migrate it back.

    move_dir(
        &server_dir.join("world_nether/DIM-1"),
        &server_dir.join("world/DIM-1"),
    )
    .await?;
    move_dir(
        &server_dir.join("world_the_end/DIM1"),
        &server_dir.join("world/DIM1"),
    )
    .await?;

    let path = server_dir.join("world_nether");
    tokio::fs::remove_dir_all(&path).await.path(path)?;
    let path = server_dir.join("world_the_end");
    tokio::fs::remove_dir_all(&path).await.path(path)?;

    change_instance_type(&server_dir, "Vanilla".to_owned(), None).await?;

    Ok(())
}

pub async fn install(instance_name: String) -> Result<(), PaperInstallerError> {
    info!("Installing Paper");
    pt!("Getting version list");
    let paper_version: PaperVersions =
        file_utils::download_file_to_json(PAPER_VERSIONS_URL, false).await?;

    let server_dir = LAUNCHER_DIR.join("servers").join(&instance_name);

    let json = VersionDetails::load(&ql_core::InstanceSelection::Server(instance_name)).await?;

    let url = paper_version.versions.get(json.get_id()).ok_or(
        PaperInstallerError::NoMatchingVersionFound(json.get_id().to_owned()),
    )?;
    let version = extract_build_number(url);

    pt!("Downloading jar");
    let jar_path = server_dir.join("paper_server.jar");
    file_utils::download_file_to_path(url, true, &jar_path).await?;

    change_instance_type(
        &server_dir,
        "Paper".to_owned(),
        version.map(|n| ModTypeInfo {
            version: n,
            backend_implementation: None,
        }),
    )
    .await?;

    pt!("Done");
    Ok(())
}

const PAPER_INSTALL_ERR_PREFIX: &str = "while installing Paper for Minecraft server:\n";

#[derive(Debug, Error)]
pub enum PaperInstallerError {
    #[error("{PAPER_INSTALL_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("{PAPER_INSTALL_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{PAPER_INSTALL_ERR_PREFIX}json error: {0}")]
    Json(#[from] JsonError),
    #[error("{PAPER_INSTALL_ERR_PREFIX}no matching paper version found for {0}")]
    NoMatchingVersionFound(String),
}

impl_3_errs_jri!(PaperInstallerError, Json, Request, Io);

/// Gets the Paper version (build number) from the url.
///
/// Eg: This function would return `"32"` given the url
/// `https://api.papermc.io/v2/projects/paper/versions/1.21.7/builds/32/downloads/paper-1.21.7-32.jar`
fn extract_build_number(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"/builds/(\d+)").unwrap();
    if let Some(captures) = re.captures(url) {
        captures.get(1).map(|m| m.as_str().to_string())
    } else {
        None
    }
}
