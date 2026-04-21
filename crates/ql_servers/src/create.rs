use std::sync::mpsc::Sender;

use ql_core::{
    DownloadProgress, IntoIoError, IntoJsonError, IntoStringError, LAUNCHER_DIR, ListEntry,
    download,
    file_utils::{self, exists},
    info,
    json::{InstanceConfigJson, Manifest, VersionDetails, instance_config::VersionInfo},
    pt, sanitize_instance_name,
};

use crate::ServerError;

/// Creates a minecraft server with the given name and version.
///
/// # Arguments
/// - `name` - The name of the server.
/// - `version` - The version of the server.
/// - `sender` - A sender to send progress updates to
///   (optional).
///
/// # Errors
///
/// TLDR; there's a lot of errors. I only wrote this because
/// clippy was bothering me (WTF: )
///
/// If:
/// - server already exists
/// - EULA and `config.json` file couldn't be saved
/// ## Server Jar...
/// - ...couldn't be downloaded from
///   Mojang/Omniarchive (internet/server issue)
/// - ...couldn't be saved to a file
/// - classic server zip file couldn't be extracted
/// - classic server zip file doesn't have a `minecraft-server.jar`
/// ## Manifest...
/// - ...couldn't be downloaded
/// - ...couldn't be parsed into JSON
/// - ...doesn't have server version
/// ## Version JSON...
/// - ...couldn't be downloaded
/// - ...couldn't be parsed into JSON
/// - ...couldn't be saved to `details.json`
/// - ...doesn't have `downloads` field
pub async fn create_server(
    name: String,
    version: ListEntry,
    sender: Option<&Sender<DownloadProgress>>,
) -> Result<String, ServerError> {
    let name = sanitize_instance_name(name);
    if name.is_empty() {
        return Err(ServerError::InvalidName);
    }

    info!(
        "Started creating server: {name} (version: {}, kind: {})",
        version.name, version.kind
    );
    progress_manifest(sender);
    let manifest = Manifest::download().await?;

    let server_dir = get_server_dir(&name).await?;
    let server_jar_path = server_dir.join("server.jar");

    let mut is_classic_server = false;

    let version_manifest = manifest
        .find_name(&version.name)
        .ok_or(ServerError::VersionNotFoundInManifest(version.name.clone()))?;
    progress_json(sender);

    let version_json: VersionDetails =
        file_utils::download_file_to_json(&version_manifest.url, false).await?;
    let Some(server) = &version_json.downloads.server else {
        return Err(ServerError::NoServerDownload);
    };

    progress_server_jar(sender);
    if version.name.starts_with("c0.") {
        is_classic_server = true;

        let archive = file_utils::download_file_to_bytes(&server.url, true).await?;
        file_utils::extract_zip_archive(std::io::Cursor::new(archive), &server_dir, true).await?;

        let old_path = server_dir.join("minecraft-server.jar");
        tokio::fs::rename(&old_path, &server_jar_path)
            .await
            .path(old_path)?;
    } else {
        download(&server.url).path(&server_jar_path).await?;
    }

    version_json.save_to_dir(&server_dir).await?;
    write_eula(&server_dir).await?;
    write_config(is_classic_server, &server_dir, &version_json).await?;

    let mods_dir = server_dir.join("mods");
    tokio::fs::create_dir(&mods_dir).await.path(mods_dir)?;

    pt!("Finished");

    Ok(name)
}

async fn write_config(
    is_classic_server: bool,
    server_dir: &std::path::Path,
    version_json: &VersionDetails,
) -> Result<(), ServerError> {
    let server_config = InstanceConfigJson::new(
        ql_core::InstanceKind::Server,
        is_classic_server,
        VersionInfo::new(&version_json.id),
    );
    let server_config_path = server_dir.join("config.json");
    tokio::fs::write(
        &server_config_path,
        serde_json::to_string(&server_config).json_to()?,
    )
    .await
    .path(server_config_path)?;
    Ok(())
}

async fn get_server_dir(name: &str) -> Result<std::path::PathBuf, ServerError> {
    let server_dir = LAUNCHER_DIR.join("servers").join(name);
    if exists(&server_dir).await {
        return Err(ServerError::ServerAlreadyExists);
    }
    tokio::fs::create_dir_all(&server_dir)
        .await
        .path(&server_dir)?;
    Ok(server_dir)
}

fn progress_manifest(sender: Option<&Sender<DownloadProgress>>) {
    pt!("Downloading Manifest");
    if let Some(sender) = sender {
        sender
            .send(DownloadProgress::DownloadingJsonManifest)
            .unwrap();
    }
}

async fn write_eula(server_dir: &std::path::Path) -> Result<(), ServerError> {
    let eula_path = server_dir.join("eula.txt");
    tokio::fs::write(&eula_path, "eula=true\n")
        .await
        .path(eula_path)?;
    Ok(())
}

fn progress_server_jar(sender: Option<&Sender<DownloadProgress>>) {
    pt!("Downloading server jar");
    if let Some(sender) = sender {
        sender.send(DownloadProgress::DownloadingJar).unwrap();
    }
}

fn progress_json(sender: Option<&Sender<DownloadProgress>>) {
    pt!("Downloading version JSON");
    if let Some(sender) = sender {
        sender
            .send(DownloadProgress::DownloadingVersionJson)
            .unwrap();
    }
}

/// Deletes a server with the given name.
///
/// # Errors
/// - If the server does not exist.
/// - If the server directory couldn't be deleted.
/// - If the launcher directory couldn't be found or created.
pub fn delete_server(name: &str) -> Result<(), String> {
    let server_dir = LAUNCHER_DIR.join("servers").join(name);
    std::fs::remove_dir_all(&server_dir)
        .path(server_dir)
        .strerr()?;

    Ok(())
}
