use std::fmt::{Display, Formatter};
use std::path::Path;

use ql_core::{
    file_utils, impl_3_errs_jri, info,
    json::{instance_config::ModTypeInfo, VersionDetails},
    pt, IntoIoError, IntoJsonError, IoError, JsonError, RequestError, LAUNCHER_DIR,
};
use serde::Deserialize;
use thiserror::Error;

use crate::loaders::change_instance_type;

/// Moves a directory from `old_path` to `new_path`.
/// If `new_path` exists, it will be deleted before the move.
async fn move_dir(old_path: &Path, new_path: &Path) -> Result<(), IoError> {
    if new_path.exists() {
        tokio::fs::remove_dir_all(new_path).await.path(new_path)?;
    }
    file_utils::copy_dir_recursive(old_path, new_path).await?;
    tokio::fs::remove_dir_all(old_path).await.path(old_path)?;
    Ok(())
}

pub enum PaperVer {
    Full(PaperVersion),
    Id(String),
    None,
}

impl From<Option<PaperVersion>> for PaperVer {
    fn from(v: Option<PaperVersion>) -> Self {
        match v {
            Some(v) => Self::Full(v),
            None => Self::None,
        }
    }
}

impl PaperVer {
    pub async fn get(&self, version: &str) -> Result<PaperVersion, PaperInstallerError> {
        if let PaperVer::Full(n) = self {
            return Ok(n.clone());
        };

        let list = get_list_of_versions(version.to_owned()).await?;
        Ok(match self {
            PaperVer::Full(_) => unreachable!(),
            PaperVer::Id(id) => list.into_iter().find(|n| n.id.to_string() == *id).ok_or(
                PaperInstallerError::NoMatchingVersionFound(version.to_owned()),
            )?,
            PaperVer::None => list
                .first()
                .ok_or(PaperInstallerError::NoMatchingVersionFound(
                    version.to_owned(),
                ))?
                .clone(),
        })
    }
}

pub async fn install(instance_name: String, version: PaperVer) -> Result<(), PaperInstallerError> {
    info!("Installing Paper");
    let server_dir = LAUNCHER_DIR.join("servers").join(&instance_name);
    let json = VersionDetails::load_from_path(&server_dir).await?;

    let version = version.get(json.get_id()).await?;

    pt!("Downloading jar");
    let jar_path = server_dir.join("paper_server.jar");
    file_utils::download_file_to_path(&version.downloads.server.url, true, &jar_path).await?;

    change_instance_type(
        &server_dir,
        "Paper".to_owned(),
        Some(ModTypeInfo {
            version: Some(version.id.to_string()),
            backend_implementation: None,
            optifine_jar: None,
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

pub async fn get_list_of_versions(
    version: String,
) -> Result<Vec<PaperVersion>, PaperInstallerError> {
    let url = format!("https://fill.papermc.io/v3/projects/paper/versions/{version}/builds");
    let json = file_utils::download_file_to_string(&url, false).await?;

    let not_found = json.contains("\"version_not_found\"");
    let json: Vec<PaperVersion> = match serde_json::from_str(&json).json(json) {
        Ok(n) => n,
        Err(e) => {
            let result = Err(if not_found {
                PaperInstallerError::NoMatchingVersionFound(version)
            } else {
                e.into()
            });
            return result;
        }
    };

    Ok(json)
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct PaperVersion {
    pub id: isize,
    pub downloads: PaperDownloads,
}

impl Display for PaperVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "version {}", self.id)
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct PaperDownloads {
    #[serde(rename = "server:default")]
    pub server: PaperDownloadsInner,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct PaperDownloadsInner {
    url: String,
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
