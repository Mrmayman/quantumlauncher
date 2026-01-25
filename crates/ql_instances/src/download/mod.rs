use sipper::Sender;

use ql_core::{
    info, json::VersionDetails, DownloadProgress, InstanceSelection, IntoIoError, IntoStringError,
    ListEntry, LAUNCHER_DIR, LAUNCHER_VERSION_NAME,
};

mod downloader;
mod libraries;

pub use downloader::DownloadError;
pub(crate) use downloader::GameDownloader;

/// Creates a Minecraft instance.
///
/// # Arguments
/// - `instance_name` : Name of the instance (for example: "my cool instance")
/// - `version` : Version of the game to download (for example: "1.21.1", "1.12.2", "b1.7.3", etc.)
/// - `progress_sender` : If you want, you can create an `mpsc::channel()` of [`DownloadProgress`],
///   provide the receiver and keep polling the sender for progress updates. *If not needed, leave as `None`*
/// - `download_assets` : Whether to download the assets. Default: true. Disable this if you want to speed
///   up the download or reduce file size. *Disabling this will make the game completely silent;
///   No sounds or music will play*
///
/// # Returns
/// The instance name that you passed in.
///
/// # Errors
/// Check the [`DownloadError`] documentation (if there is, lol).
/// This is crap code and you must have standards. (WTF: )
pub async fn create_instance(
    instance_name: String,
    version: ListEntry,
    progress_sender: Option<Sender<DownloadProgress>>,
    download_assets: bool,
) -> Result<String, DownloadError> {
    info!(
        "Started creating instance: {} (kind: {})",
        version.name, version.kind
    );

    // An empty asset directory
    if !download_assets {
        let assets_dir = LAUNCHER_DIR.join("assets/null");
        tokio::fs::create_dir_all(&assets_dir)
            .await
            .path(assets_dir)?;
    }

    let mut game_downloader =
        GameDownloader::new(&instance_name, &version, progress_sender).await?;

    tokio::try_join!(
        game_downloader.download_logging_config(),
        game_downloader.download_jar()
    )?;
    game_downloader.download_libraries().await?;
    game_downloader.library_extras().await?;

    if download_assets {
        game_downloader.download_assets().await?;
    }

    game_downloader
        .version_json
        .save_to_dir(&game_downloader.instance_dir)
        .await?;
    game_downloader.create_profiles_json().await?;
    game_downloader.create_config_json().await?;

    let version_file_path = LAUNCHER_DIR
        .join("instances")
        .join(&instance_name)
        .join("launcher_version.txt");
    tokio::fs::write(&version_file_path, LAUNCHER_VERSION_NAME)
        .await
        .path(version_file_path)?;

    let mods_dir = LAUNCHER_DIR
        .join("instances")
        .join(&instance_name)
        .join(".minecraft/mods");
    tokio::fs::create_dir_all(&mods_dir).await.path(mods_dir)?;

    info!("Finished creating instance: {instance_name}");

    Ok(instance_name)
}

pub async fn repeat_stage(
    instance: InstanceSelection,
    stage: DownloadProgress,
    sender: Option<Sender<DownloadProgress>>,
) -> Result<(), String> {
    debug_assert!(!instance.is_server());

    info!("Redownloading part of instance ({stage})");
    let instance_dir = instance.get_instance_path();
    let mut downloader = GameDownloader::with_existing_instance(
        VersionDetails::load(&instance).await.strerr()?,
        instance_dir.clone(),
        sender,
    );

    match stage {
        DownloadProgress::DownloadingLibraries { .. } => {
            let libraries_dir = instance_dir.join("libraries");
            tokio::fs::remove_dir_all(&libraries_dir)
                .await
                .path(&libraries_dir)
                .strerr()?;

            downloader.download_libraries().await.strerr()?;
        }
        DownloadProgress::DownloadingAssets { .. } => {
            downloader.download_assets().await.strerr()?;
        }
        DownloadProgress::DownloadingJar => {
            downloader.download_jar().await.strerr()?;
        }
        DownloadProgress::DownloadingJsonManifest | DownloadProgress::DownloadingVersionJson => {
            unimplemented!()
        }
    }
    info!("Finished redownloading");

    Ok(())
}
