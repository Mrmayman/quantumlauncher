use std::{
    path::Path,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
};

use crate::loaders::paper::PaperVer;
use forge::ForgeInstallProgress;
use ql_core::{
    json::{instance_config::ModTypeInfo, InstanceConfigJson},
    GenericProgress, InstanceSelection, IntoStringError, JsonFileError, Loader, Progress,
};

pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod optifine;
pub mod paper;

pub(crate) const FORGE_INSTALLER_CLIENT: &[u8] =
    include_bytes!("../../../../assets/installers/forge/ForgeInstaller.class");
pub(crate) const FORGE_INSTALLER_SERVER: &[u8] =
    include_bytes!("../../../../assets/installers/forge/ForgeInstallerServer.class");

async fn change_instance_type(
    instance_dir: &Path,
    loader: Loader,
    extras: Option<ModTypeInfo>,
) -> Result<(), JsonFileError> {
    let mut config = InstanceConfigJson::read_from_dir(instance_dir).await?;
    config.mod_type = loader;
    config.mod_type_info = extras;
    config.save_to_dir(instance_dir).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum LoaderInstallResult {
    Ok,
    NeedsOptifine,
    Unsupported,
}

pub async fn install_specified_loader(
    instance: InstanceSelection,
    loader: Loader,
    progress: Option<Arc<Sender<GenericProgress>>>,
    specified_version: Option<String>,
) -> Result<LoaderInstallResult, String> {
    match loader {
        Loader::Vanilla => {}
        Loader::Fabric => {
            // TODO: Add legacy fabric support
            fabric::install(
                specified_version,
                instance,
                progress.as_deref(),
                fabric::BackendType::Fabric,
            )
            .await
            .strerr()?;
        }
        Loader::Quilt => {
            fabric::install(
                specified_version,
                instance,
                progress.as_deref(),
                fabric::BackendType::Quilt,
            )
            .await
            .strerr()?;
        }

        Loader::Forge => {
            let (send, recv) = std::sync::mpsc::channel();
            if let Some(progress) = progress {
                std::thread::spawn(move || {
                    pipe_progress(recv, &progress);
                });
            }

            // TODO: Java install progress
            forge::install(specified_version, instance, Some(send), None)
                .await
                .strerr()?;
        }
        Loader::Neoforge => {
            let (send, recv) = std::sync::mpsc::channel();
            if let Some(progress) = progress {
                std::thread::spawn(move || {
                    pipe_progress(recv, &progress);
                });
            }

            neoforge::install(specified_version, instance, Some(send), None)
                .await
                .strerr()?;
        }

        Loader::Paper => {
            if !instance.is_server() {
                return Ok(LoaderInstallResult::Unsupported);
            }
            paper::install(
                instance.get_name().to_owned(),
                if let Some(s) = specified_version {
                    PaperVer::Id(s)
                } else {
                    PaperVer::None
                },
            )
            .await
            .strerr()?;
        }

        Loader::OptiFine => {
            return Ok(if instance.is_server() {
                LoaderInstallResult::Unsupported
            } else {
                LoaderInstallResult::NeedsOptifine
            })
        }

        Loader::Liteloader | Loader::Modloader | Loader::Rift => {
            return Ok(LoaderInstallResult::Unsupported)
        }
    }
    Ok(LoaderInstallResult::Ok)
}

fn pipe_progress(rec: Receiver<ForgeInstallProgress>, snd: &Sender<GenericProgress>) {
    for item in rec {
        _ = snd.send(item.into_generic());
    }
}

pub async fn uninstall_loader(instance: InstanceSelection) -> Result<(), String> {
    let loader = InstanceConfigJson::read(&instance).await.strerr()?.mod_type;

    match loader {
        Loader::Fabric | Loader::Quilt => fabric::uninstall(instance).await.strerr(),
        Loader::Forge | Loader::Neoforge => forge::uninstall(instance).await.strerr(),
        Loader::OptiFine => optifine::uninstall(instance.get_name().to_owned(), true)
            .await
            .strerr(),
        Loader::Paper => paper::uninstall(instance.get_name().to_owned())
            .await
            .strerr(),
        // Not yet supported
        Loader::Liteloader | Loader::Modloader | Loader::Rift | Loader::Vanilla => Ok(()),
    }
}
