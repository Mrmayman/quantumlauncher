use std::sync::mpsc::Sender;

use omniarchive_api::{ListEntry, MinecraftVersionCategory};
use ql_core::{
    file_utils, info,
    json::{InstanceConfigJson, Manifest, OmniarchiveEntry, VersionDetails},
    pt, GenericProgress, IntoIoError, IntoJsonError, IntoStringError, LAUNCHER_DIR,
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
///   mojang/omniarchive (internet/server issue)
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
    sender: Option<Sender<GenericProgress>>,
) -> Result<String, ServerError> {
    info!("Creating server");
    pt!("Downloading Manifest");
    progress_manifest(sender.as_ref());
    let manifest = Manifest::download().await?;

    let server_dir = get_server_dir(&name).await?;
    let server_jar_path = server_dir.join("server.jar");

    let mut is_classic_server = false;

    let version_json = match &version {
        ListEntry::Normal(version) => {
            let (jar, json) = download_from_mojang(&manifest, version, sender.as_ref()).await?;
            tokio::fs::write(&server_jar_path, jar)
                .await
                .path(server_jar_path)?;
            json
        }
        ListEntry::Omniarchive {
            category,
            url,
            nice_name,
            ..
        } => {
            let (jar, json) =
                download_from_omniarchive(category, &manifest, nice_name, sender.as_ref(), url)
                    .await?;
            tokio::fs::write(&server_jar_path, jar)
                .await
                .path(server_jar_path)?;
            json
        }
        ListEntry::OmniarchiveClassicZipServer { name, url } => {
            is_classic_server = true;

            let version_json = download_omniarchive_version(
                &MinecraftVersionCategory::Classic,
                &manifest,
                name,
                sender.as_ref(),
            )
            .await?;

            progress_server_jar(sender.as_ref());
            let archive = file_utils::download_file_to_bytes(url, true).await?;
            zip_extract::extract(std::io::Cursor::new(archive), &server_dir, true)?;

            let old_path = server_dir.join("minecraft-server.jar");
            tokio::fs::rename(&old_path, &server_jar_path)
                .await
                .path(old_path)?;

            version_json
        }
    };

    write_json(&server_dir, version_json).await?;
    write_eula(&server_dir).await?;
    write_config(version, is_classic_server, &server_dir).await?;

    let mods_dir = server_dir.join("mods");
    tokio::fs::create_dir(&mods_dir).await.path(mods_dir)?;

    pt!("Finished");

    Ok(name)
}

async fn write_config(
    version: ListEntry,
    is_classic_server: bool,
    server_dir: &std::path::Path,
) -> Result<(), ServerError> {
    let server_config = InstanceConfigJson {
        mod_type: "Vanilla".to_owned(),
        java_override: None,
        ram_in_mb: 2048,
        enable_logger: Some(true),
        java_args: None,
        game_args: None,
        omniarchive: get_omniarchive(version),
        is_classic_server: is_classic_server.then_some(true),

        // Doesn't affect servers:

        // I could add GC tuning to servers too, but I can't find
        // a way to measure performance on a server. Besides this setting
        // makes performance worse on clients so I guess it's same for servers?
        do_gc_tuning: None,
        // This won't do anything on servers. Who wants to lose their *only way*
        // to control the server instantly after starting it?
        close_on_start: None,
    };
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
    if server_dir.exists() {
        return Err(ServerError::ServerAlreadyExists);
    }
    tokio::fs::create_dir_all(&server_dir)
        .await
        .path(&server_dir)?;
    Ok(server_dir)
}

fn progress_manifest(sender: Option<&Sender<GenericProgress>>) {
    if let Some(sender) = sender {
        sender
            .send(GenericProgress {
                done: 0,
                total: 3,
                message: Some("Downloading Manifest".to_owned()),
                has_finished: false,
            })
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

async fn write_json(
    server_dir: &std::path::Path,
    version_json: VersionDetails,
) -> Result<(), ServerError> {
    let version_json_path = server_dir.join("details.json");
    tokio::fs::write(
        &version_json_path,
        serde_json::to_string(&version_json).json_to()?,
    )
    .await
    .path(version_json_path)?;
    Ok(())
}

fn get_omniarchive(version: ListEntry) -> Option<OmniarchiveEntry> {
    if let ListEntry::Omniarchive {
        category,
        name,
        url,
        nice_name,
    } = version
    {
        Some(OmniarchiveEntry {
            name,
            url,
            category: category.to_string(),
            nice_name: Some(nice_name),
        })
    } else {
        None
    }
}

async fn download_from_omniarchive(
    category: &MinecraftVersionCategory,
    manifest: &Manifest,
    name: &str,
    sender: Option<&Sender<GenericProgress>>,
    url: &str,
) -> Result<(Vec<u8>, VersionDetails), ServerError> {
    let version_json = download_omniarchive_version(category, manifest, name, sender).await?;
    info!("Downloading server jar");
    progress_server_jar(sender);
    let server_jar = file_utils::download_file_to_bytes(url, false).await?;
    Ok((server_jar, version_json))
}

fn progress_server_jar(sender: Option<&Sender<GenericProgress>>) {
    if let Some(sender) = sender {
        sender
            .send(GenericProgress {
                done: 2,
                total: 3,
                message: Some("Downloading Server Jar".to_owned()),
                has_finished: false,
            })
            .unwrap();
    }
}

async fn download_from_mojang(
    manifest: &Manifest,
    version: &str,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<(Vec<u8>, VersionDetails), ServerError> {
    let version = manifest
        .find_name(version)
        .ok_or(ServerError::VersionNotFoundInManifest(version.to_owned()))?;
    pt!("Downloading version JSON");
    progress_json(sender);
    let version_json: VersionDetails =
        file_utils::download_file_to_json(&version.url, false).await?;
    let Some(server) = &version_json.downloads.server else {
        return Err(ServerError::NoServerDownload);
    };

    pt!("Downloading server jar");
    progress_server_jar(sender);
    let server_jar = file_utils::download_file_to_bytes(&server.url, false).await?;
    Ok((server_jar, version_json))
}

async fn download_omniarchive_version(
    category: &MinecraftVersionCategory,
    manifest: &Manifest,
    name: &str,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<VersionDetails, ServerError> {
    let version = match category {
        MinecraftVersionCategory::PreClassic => manifest.find_fuzzy(name, "rd-"),
        MinecraftVersionCategory::Classic => manifest.find_fuzzy(name, "c0."),
        MinecraftVersionCategory::Alpha => manifest.find_fuzzy(name, "a1."),
        MinecraftVersionCategory::Beta => manifest.find_fuzzy(name, "b1."),
        MinecraftVersionCategory::Indev => manifest.find_name("c0.30_01c"),
        MinecraftVersionCategory::Infdev => manifest.find_name("inf-20100618"),
    }
    .ok_or(ServerError::VersionNotFoundInManifest(name.to_owned()))?;
    info!("Downloading version JSON");
    progress_json(sender);
    let version_json: VersionDetails =
        file_utils::download_file_to_json(&version.url, false).await?;
    Ok(version_json)
}

fn progress_json(sender: Option<&Sender<GenericProgress>>) {
    if let Some(sender) = sender {
        sender
            .send(GenericProgress {
                done: 1,
                total: 3,
                message: Some("Downloading Version JSON".to_owned()),
                has_finished: false,
            })
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
