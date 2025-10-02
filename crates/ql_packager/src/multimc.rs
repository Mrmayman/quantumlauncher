use chrono::DateTime;
use std::{
    path::Path,
    sync::{mpsc::Sender, Arc, Mutex},
};

use crate::{import::pipe_progress, import::OUT_OF, InstancePackageError};
use ql_core::{
    do_jobs, err, file_utils, info,
    json::{
        FabricJSON, InstanceConfigJson, Manifest, VersionDetails, V_1_12_2,
        V_OFFICIAL_FABRIC_SUPPORT,
    },
    pt, GenericProgress, InstanceSelection, IntoIoError, IntoJsonError, ListEntry, Loader,
};
use ql_mod_manager::loaders::fabric::{self, get_list_of_versions_from_backend};
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmcPack {
    pub components: Vec<MmcPackComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct MmcPackComponent {
    pub cachedName: String,
    pub cachedVersion: String,
}

#[derive(Debug, Clone)]
pub struct InstanceRecipe {
    is_lwjgl3: bool,
    mc_version: String,
    loader: Option<Loader>,
    loader_version: Option<String>,
}

impl InstanceRecipe {
    async fn setup_lwjgl3(&mut self) -> Result<(), InstancePackageError> {
        async fn adjust_for_lwjgl3(mc_version: &str) -> Result<bool, InstancePackageError> {
            let manifest = Manifest::download().await?;
            if let Some(version) = manifest.find_name(mc_version) {
                if let (Ok(look), Ok(expect)) = (
                    DateTime::parse_from_rfc3339(&version.releaseTime),
                    DateTime::parse_from_rfc3339(V_1_12_2),
                ) {
                    if look <= expect {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }

        if self.is_lwjgl3 && adjust_for_lwjgl3(&self.mc_version).await? {
            self.mc_version.push_str("-lwjgl3");
        }
        Ok(())
    }
}

pub async fn import(
    download_assets: bool,
    temp_dir: &Path,
    mmc_pack: &str,
    sender: Option<Arc<Sender<GenericProgress>>>,
) -> Result<InstanceSelection, InstancePackageError> {
    info!("Importing MultiMC instance...");
    let mmc_pack: MmcPack = serde_json::from_str(mmc_pack).json(mmc_pack.to_owned())?;

    let ini = {
        let ini_path = temp_dir.join("instance.cfg");
        let ini = fs::read_to_string(&ini_path).await.path(ini_path)?;
        ini::Ini::load_from_str(&filter_bytearray(&ini))?
    };

    let instance_name = ini
        .get_from(Some("General"), "name")
        .or(ini.get_from(None::<String>, "name"))
        .ok_or_else(|| {
            InstancePackageError::IniFieldMissing("General".to_owned(), "name".to_owned())
        })?
        .to_owned();
    let instance_selection = InstanceSelection::new(&instance_name, false);

    let mut instance_recipe = InstanceRecipe {
        is_lwjgl3: false,
        mc_version: "(MultiMC) Couldn't find minecraft version".to_owned(),
        loader: None,
        loader_version: None,
    };

    for component in &mmc_pack.components {
        match component.cachedName.as_str() {
            "Minecraft" => {
                instance_recipe.mc_version = component.cachedVersion.clone();
            }

            "Forge" => {
                instance_recipe.loader = Some(Loader::Forge);
                instance_recipe.loader_version = Some(component.cachedVersion.clone());
            }
            "NeoForge" => {
                instance_recipe.loader = Some(Loader::Forge);
                instance_recipe.loader_version = Some(component.cachedVersion.clone());
            }
            "Fabric Loader" => {
                instance_recipe.loader = Some(Loader::Fabric);
                instance_recipe.loader_version = Some(component.cachedVersion.clone());
            }
            "Quilt Loader" => {
                instance_recipe.loader = Some(Loader::Forge);
                instance_recipe.loader_version = Some(component.cachedVersion.clone());
            }

            "LWJGL 3" => instance_recipe.is_lwjgl3 = true,

            "LWJGL 2" | "Intermediary Mappings" => {}
            name => err!("Unknown MultiMC Component: {name}"),
        }
    }

    instance_recipe.setup_lwjgl3().await?;
    mmc_minecraft(
        download_assets,
        sender.clone(),
        &instance_name,
        instance_recipe.mc_version.clone(),
    )
    .await?;

    if let Some(loader) = instance_recipe.loader {
        match loader {
            n @ (Loader::Fabric | Loader::Quilt) => {
                install_fabric(
                    sender.as_deref(),
                    &instance_selection,
                    instance_recipe.loader_version.clone(),
                    matches!(n, Loader::Quilt),
                )
                .await?;
            }
            n @ (Loader::Forge | Loader::Neoforge) => {
                mmc_forge(
                    sender.clone(),
                    &instance_selection,
                    instance_recipe.loader_version.clone(),
                    matches!(n, Loader::Neoforge),
                )
                .await?;
            }
            loader => {
                err!("Unimplemented MultiMC Component: {loader:?}")
            }
        }
    }

    copy_files(temp_dir, sender, &instance_selection).await?;

    let mut config = InstanceConfigJson::read(&instance_selection).await?;
    if let Some(jvmargs) = ini.get_from(Some("General"), "JvmArgs") {
        let mut java_args = config.java_args.clone().unwrap_or_default();
        java_args.extend(jvmargs.split_whitespace().map(str::to_owned));
        config.java_args = Some(java_args);
    }
    config.save(&instance_selection).await?;
    info!("Finished importing MultiMC instance");
    Ok(instance_selection)
}

async fn install_fabric(
    sender: Option<&Sender<GenericProgress>>,
    instance_selection: &InstanceSelection,
    version: Option<String>,
    is_quilt: bool,
) -> Result<(), InstancePackageError> {
    let backend = if is_quilt {
        fabric::BackendType::Quilt
    } else {
        fabric::BackendType::Fabric
    };

    let version_json = VersionDetails::load(instance_selection).await?;
    if !version_json.is_before_or_eq(V_OFFICIAL_FABRIC_SUPPORT) {
        ql_mod_manager::loaders::fabric::install(
            version,
            instance_selection.clone(),
            sender,
            backend,
        )
        .await?;
        return Ok(());
    }

    // Hack for versions below 1.14
    let url = format!(
        "https://{}/versions/loader/1.14.4/{}/profile/json",
        if is_quilt {
            "meta.quiltmc.org/v3"
        } else {
            "meta.fabricmc.net/v2"
        },
        if let Some(version) = version.clone() {
            version
        } else {
            get_list_of_versions_from_backend("1.14.4", backend, false)
                .await?
                .first()
                .map(|n| n.loader.version.clone())
                .unwrap_or_else(|| " No versions found! ".to_owned())
        }
    );
    let fabric_json_text = file_utils::download_file_to_string(&url, false).await?;
    let fabric_json: FabricJSON =
        serde_json::from_str(&fabric_json_text).json(fabric_json_text.clone())?;

    let instance_path = instance_selection.get_instance_path();
    let libraries_dir = instance_path.join("libraries");

    info!("Custom fabric implementation, installing libraries:");
    let i = Mutex::new(0);
    let len = fabric_json.libraries.len();
    do_jobs(fabric_json.libraries.iter().map(|library| async {
        if library.name.starts_with("net.fabricmc:intermediary") {
            return Ok::<_, InstancePackageError>(());
        }
        let path_str = library.get_path();
        let Some(url) = library.get_url() else {
            return Ok::<_, InstancePackageError>(());
        };
        let path = libraries_dir.join(&path_str);

        let parent_dir = path
            .parent()
            .ok_or(InstancePackageError::PathBufParent(path.clone()))?;
        tokio::fs::create_dir_all(parent_dir)
            .await
            .path(parent_dir)?;
        file_utils::download_file_to_path(&url, false, &path).await?;

        {
            let mut i = i.lock().unwrap();
            *i += 1;
            pt!(
                "({i}/{len}) {}\n    Path: {path_str}\n    Url: {url}",
                library.name
            );
            if let Some(sender) = sender {
                _ = sender.send(GenericProgress {
                    done: *i,
                    total: len,
                    message: Some(format!("Installing fabric: library {}", library.name)),
                    has_finished: false,
                });
            }
        }

        Ok(())
    }))
    .await?;

    let mut config = InstanceConfigJson::read(instance_selection).await?;
    config.main_class_override = Some(fabric_json.mainClass.clone());
    config.mod_type = "Fabric".to_owned();
    config.save(instance_selection).await?;

    let fabric_json_path = instance_path.join("fabric.json");
    tokio::fs::write(&fabric_json_path, &fabric_json_text)
        .await
        .path(&fabric_json_path)?;
    Ok(())
}

async fn copy_files(
    temp_dir: &Path,
    sender: Option<Arc<Sender<GenericProgress>>>,
    instance_selection: &InstanceSelection,
) -> Result<(), InstancePackageError> {
    let src = temp_dir.join("minecraft");
    if src.is_dir() {
        let dst = instance_selection.get_dot_minecraft_path();
        if let Some(sender) = sender.as_deref() {
            _ = sender.send(GenericProgress {
                done: 2,
                total: OUT_OF,
                message: Some("Copying files...".to_owned()),
                has_finished: false,
            });
        }
        file_utils::copy_dir_recursive(&src, &dst).await?;
    }

    copy_folder_over(temp_dir, instance_selection, "jarmods").await?;
    copy_folder_over(temp_dir, instance_selection, "patches").await?;

    Ok(())
}

async fn copy_folder_over(
    temp_dir: &Path,
    instance_selection: &InstanceSelection,
    path: &'static str,
) -> Result<(), InstancePackageError> {
    let src = temp_dir.join(path);
    if src.is_dir() {
        let dst = instance_selection.get_instance_path().join(path);
        file_utils::copy_dir_recursive(&src, &dst).await?;
    }
    Ok(())
}

async fn mmc_minecraft(
    download_assets: bool,
    sender: Option<Arc<Sender<GenericProgress>>>,
    instance_name: &str,
    version: String,
) -> Result<(), InstancePackageError> {
    let version = ListEntry {
        name: version,
        is_classic_server: false,
    };
    let (d_send, d_recv) = std::sync::mpsc::channel();
    if let Some(sender) = sender.clone() {
        std::thread::spawn(move || {
            pipe_progress(d_recv, &sender);
        });
    }
    ql_instances::create_instance(
        instance_name.to_owned(),
        version,
        Some(d_send),
        download_assets,
    )
    .await?;
    Ok(())
}

async fn mmc_forge(
    sender: Option<Arc<Sender<GenericProgress>>>,
    instance_selection: &InstanceSelection,
    version: Option<String>,
    is_neoforge: bool,
) -> Result<(), InstancePackageError> {
    let (f_send, f_recv) = std::sync::mpsc::channel();
    if let Some(sender) = sender.clone() {
        std::thread::spawn(move || {
            pipe_progress(f_recv, &sender);
        });
    }
    if is_neoforge {
        ql_mod_manager::loaders::neoforge::install(
            version,
            instance_selection.clone(),
            Some(f_send),
            None, // TODO: Java install progress
        )
        .await?;
    } else {
        ql_mod_manager::loaders::forge::install(
            version,
            instance_selection.clone(),
            Some(f_send),
            None, // TODO: Java install progress
        )
        .await?;
    }
    Ok(())
}

fn filter_bytearray(input: &str) -> String {
    // PrismLauncher puts some weird ByteArray
    // field in the INI config file, that our cute little ini parser
    // doesn't understand. So we have to filter it out.
    input
        .lines()
        .filter(|n| !n.starts_with("mods_Page\\Columns"))
        .collect::<Vec<_>>()
        .join("\n")
}
