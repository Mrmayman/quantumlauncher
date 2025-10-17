use std::path::Path;

use ql_core::{
    find_forge_shim_file, json::InstanceConfigJson, InstanceSelection, IntoIoError,
    IntoStringError, LAUNCHER_DIR,
};

use crate::loaders::{self, change_instance_type};

use super::error::ForgeInstallError;

pub async fn uninstall(instance: InstanceSelection) -> Result<(), String> {
    match instance {
        InstanceSelection::Instance(instance) => uninstall_client(&instance).await,
        InstanceSelection::Server(instance) => uninstall_server(&instance).await.strerr(),
    }
}

pub async fn uninstall_client(instance: &str) -> Result<(), String> {
    let instance_dir = LAUNCHER_DIR.join("instances").join(instance);

    let forge_dir = instance_dir.join("forge");
    if forge_dir.is_dir() {
        tokio::fs::remove_dir_all(&forge_dir)
            .await
            .path(forge_dir)
            .strerr()?;
    }

    let mut config = InstanceConfigJson::read_from_dir(&instance_dir)
        .await
        .strerr()?;
    config.mod_type = if let Some(jar) = config
        .mod_type_info
        .as_ref()
        .and_then(|n| n.optifine_jar.as_deref())
    {
        let installer_path = instance_dir.join(".minecraft/mods").join(jar);
        if tokio::fs::try_exists(&installer_path)
            .await
            .path(&installer_path)
            .strerr()?
        {
            loaders::optifine::install(
                instance.to_owned(),
                installer_path.clone(),
                None,
                None,
                None,
            )
            .await
            .strerr()?;
            tokio::fs::remove_file(&installer_path)
                .await
                .path(&installer_path)
                .strerr()?;
            config.mod_type_info = None;
            "OptiFine"
        } else {
            "Vanilla"
        }
    } else {
        "Vanilla"
    }
    .to_owned();
    config.save_to_dir(&instance_dir).await.strerr()?;

    Ok(())
}

pub async fn uninstall_server(instance: &str) -> Result<(), ForgeInstallError> {
    let instance_dir = LAUNCHER_DIR.join("servers").join(instance);
    change_instance_type(&instance_dir, "Vanilla".to_owned(), None).await?;

    if let Some(forge_shim_file) = find_forge_shim_file(&instance_dir).await {
        tokio::fs::remove_file(&forge_shim_file)
            .await
            .path(forge_shim_file)?;
    }

    let libraries_dir = instance_dir.join("libraries");
    if libraries_dir.is_dir() {
        tokio::fs::remove_dir_all(&libraries_dir)
            .await
            .path(libraries_dir)?;
    }
    let forge_dir = instance_dir.join("forge");
    if forge_dir.is_dir() {
        tokio::fs::remove_dir_all(&forge_dir)
            .await
            .path(forge_dir)?;
    }

    delete_file(&instance_dir.join("run.sh")).await?;
    delete_file(&instance_dir.join("run.bat")).await?;
    delete_file(&instance_dir.join("user_jvm_args.txt")).await?;
    delete_file(&instance_dir.join("README.txt")).await?;

    Ok(())
}

async fn delete_file(file: &Path) -> Result<(), ForgeInstallError> {
    if file.is_file() {
        tokio::fs::remove_file(&file).await.path(file)?;
    }
    Ok(())
}
