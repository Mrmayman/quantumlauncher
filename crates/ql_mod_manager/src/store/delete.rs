use crate::{
    rate_limiter::lock,
    store::{ModError, ModIndex},
};
use ql_core::{err, info, pt, InstanceSelection, IoError, ModId};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub async fn delete_mods(
    ids: Vec<ModId>,
    instance: InstanceSelection,
) -> Result<Vec<ModId>, ModError> {
    let _guard = lock().await;

    if ids.is_empty() {
        return Ok(ids);
    }

    info!("Deleting content:");
    let mut index = ModIndex::load(&instance).await?;

    let dot_mc_dir = instance.get_dot_minecraft_path();
    let mods_dir = dot_mc_dir.join("mods");
    let resourcepacks_dir = dot_mc_dir.join("resourcepacks");
    let texturepacks_dir = dot_mc_dir.join("texturepacks");
    let shaderpacks_dir = dot_mc_dir.join("shaderpacks");
    let datapacks_dir = dot_mc_dir.join("datapacks");

    for id in &ids {
        pt!("Deleting: {id:?}");
        delete_mod(
            &mut index,
            id,
            &mods_dir,
            &resourcepacks_dir,
            &texturepacks_dir,
            &shaderpacks_dir,
            &datapacks_dir,
        )
        .await?;
    }

    let mut has_been_removed;
    // let mut iteration = 0;
    loop {
        // iteration += 1;
        // pt!("Iteration {iteration}");

        has_been_removed = false;
        let mut removed_dependents_map = HashMap::new();

        for (mod_id, mod_info) in &index.mods {
            if !mod_info.manually_installed {
                let mut removed_dependents = HashSet::new();
                for dependent in &mod_info.dependents {
                    if !index.mods.contains_key(dependent) {
                        removed_dependents.insert(dependent.clone());
                    }
                }
                removed_dependents_map.insert(mod_id.clone(), removed_dependents);
            }
        }

        for (id, removed_dependents) in removed_dependents_map {
            if let Some(mod_info) = index.mods.get_mut(&id) {
                for dependent in removed_dependents {
                    has_been_removed = true;
                    mod_info.dependents.remove(&dependent);
                }
            } else {
                err!("Dependent {id} does not exist");
            }
        }

        let mut orphaned_mods = HashSet::new();

        for (mod_id, mod_info) in &index.mods {
            if !mod_info.manually_installed && mod_info.dependents.is_empty() {
                pt!("Deleting dependency: {}", mod_info.name);
                orphaned_mods.insert(ModId::from_index_str(mod_id));
            }
        }

        for orphan in orphaned_mods {
            has_been_removed = true;
            delete_mod(
                &mut index,
                &orphan,
                &mods_dir,
                &resourcepacks_dir,
                &texturepacks_dir,
                &shaderpacks_dir,
                &datapacks_dir,
            )
            .await?;
        }

        if !has_been_removed {
            break;
        }
    }

    index.save(&instance).await?;
    info!("Finished deleting content");
    Ok(ids)
}

async fn delete_mod(
    index: &mut ModIndex,
    id: &ModId,
    mods_dir: &Path,
    resourcepacks_dir: &Path,
    texturepacks_dir: &Path,
    shaderpacks_dir: &Path,
    datapacks_dir: &Path,
) -> Result<(), ModError> {
    if let Some(mod_info) = index.mods.remove(&id.get_index_str()) {
        // Determine the correct directory based on project type
        let content_dir: &Path = match mod_info.project_type.as_str() {
            "resourcepack" => {
                if resourcepacks_dir.exists() {
                    resourcepacks_dir
                } else {
                    texturepacks_dir
                }
            }
            "shader" => shaderpacks_dir,
            "datapack" => datapacks_dir,
            _ => mods_dir, // "mod" or unknown defaults to mods
        };

        for file in &mod_info.files {
            if mod_info.enabled {
                delete_file(content_dir, &file.filename).await?;
            } else {
                delete_file(content_dir, &format!("{}.disabled", file.filename)).await?;
            }
        }
    } else {
        err!("Deleted content does not exist");
    }
    Ok(())
}

async fn delete_file(mods_dir: &Path, file: &str) -> Result<(), ModError> {
    let path = mods_dir.join(file);
    if let Err(err) = tokio::fs::remove_file(&path).await {
        if let std::io::ErrorKind::NotFound = err.kind() {
            err!("File does not exist, skipping: {path:?}");
        } else {
            let err = IoError::Io {
                error: err.to_string(),
                path: path.clone(),
            };
            Err(err)?;
        }
    }
    Ok(())
}
