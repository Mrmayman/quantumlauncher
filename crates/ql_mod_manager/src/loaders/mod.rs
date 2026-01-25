use sipper::Sender;
use std::path::Path;

use crate::loaders::paper::PaperVer;
use ql_core::{
    json::{instance_config::ModTypeInfo, InstanceConfigJson},
    pipe_progress, GenericProgress, InstanceSelection, IntoStringError, JsonFileError, Loader,
};

pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod optifine;
pub mod paper;

pub(crate) const FORGE_INSTALLER_CLIENT: &[u8] =
    include_bytes!("../../../../assets/installers/ForgeInstaller.class");
pub(crate) const FORGE_INSTALLER_SERVER: &[u8] =
    include_bytes!("../../../../assets/installers/ForgeInstallerServer.class");

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
    progress: Option<Sender<GenericProgress>>,
    specified_version: Option<String>,
) -> Result<LoaderInstallResult, String> {
    match loader {
        Loader::Vanilla => {}
        Loader::Fabric => {
            // TODO: Add legacy fabric support
            fabric::install(
                specified_version,
                instance,
                progress,
                fabric::BackendType::Fabric,
            )
            .await
            .strerr()?;
        }
        Loader::Quilt => {
            fabric::install(
                specified_version,
                instance,
                progress,
                fabric::BackendType::Quilt,
            )
            .await
            .strerr()?;
        }

        Loader::Forge => {
            // TODO: Java install progress
            pipe_progress(progress, |sender| {
                forge::install(specified_version, instance, sender)
            })
            .await
            .strerr()?;
        }
        Loader::Neoforge => {
            pipe_progress(progress, |sender| {
                neoforge::install(specified_version, instance, sender)
            })
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
