use ql_core::{json::instance_config::ModTypeInfo, InstanceSelection, IntoIoError, Loader};

use crate::loaders::{change_instance_type, forge::ForgeInstaller};

use super::{error::ForgeInstallError, ForgeProgress};

pub async fn install_server(
    forge_version: Option<String>, // example: "11.15.1.2318" for 1.8.9
    instance_name: String,
    mut progress: Option<sipper::Sender<ForgeProgress>>,
) -> Result<(), ForgeInstallError> {
    if let Some(progress) = &mut progress {
        progress.send(ForgeProgress::P1Start).await;
    }

    let mut installer = ForgeInstaller::new(
        forge_version,
        progress,
        InstanceSelection::Server(instance_name),
    )
    .await?;

    let (_, installer_name, installer_path) = installer.download_forge_installer().await?;

    installer.run_installer(&installer_name).await?;

    tokio::fs::remove_file(&installer_path)
        .await
        .path(installer_path)?;

    installer.delete("ClientInstaller.java").await?;
    installer.delete("ClientInstaller.class").await?;
    installer.delete("ForgeInstaller.java").await?;
    installer.delete("ForgeInstaller.class").await?;

    installer.delete("README.txt").await?;
    installer.delete("run.bat").await?;
    installer.delete("run.sh").await?;
    installer.delete("user_jvm_args.txt").await?;

    change_instance_type(
        &installer.instance_dir,
        Loader::Forge,
        Some(ModTypeInfo {
            version: Some(installer.version.clone()),
            backend_implementation: None,
            optifine_jar: None,
        }),
    )
    .await?;

    Ok(())
}
