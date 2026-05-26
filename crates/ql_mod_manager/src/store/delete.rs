use crate::{
    rate_limiter::lock,
    store::{DirStructure, ModError, ModId, ModIndex},
};
use ql_core::{Instance, IoError, err, info, json::VersionDetails, pt};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub async fn delete_mods(ids: Vec<ModId>, instance: Instance) -> Result<Vec<ModId>, ModError> {
    let _guard = lock().await;

    if ids.is_empty() {
        return Ok(ids);
    }

    info!("Deleting content:");
    let version_json = VersionDetails::load(&instance).await?;
    let mut index = ModIndex::load(&instance).await?;
    let dirs = DirStructure::new(instance.clone(), &version_json).await?;

    for id in &ids {
        pt!("Deleting: {id:?}");
        delete_mod(&mut index, id, &dirs).await?;
    }

    // Remove all orphaned mods (dependencies)

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
                err!("Dependent {id:?} does not exist");
            }
        }

        let mut orphaned_mods = HashSet::new();

        for (mod_id, mod_info) in &index.mods {
            if !mod_info.manually_installed && mod_info.dependents.is_empty() {
                pt!("Deleting dependency: {}", mod_info.name);
                orphaned_mods.insert(mod_id.clone());
            }
        }

        for orphan in orphaned_mods {
            has_been_removed = true;
            delete_mod(&mut index, &orphan, &dirs).await?;
        }

        if !has_been_removed {
            break;
        }
    }

    index.save(&instance).await?;
    info!("Finished deleting content");
    Ok(ids)
}

async fn delete_mod(index: &mut ModIndex, id: &ModId, dirs: &DirStructure) -> Result<(), ModError> {
    if let Some(mod_info) = index.mods.remove(id) {
        let Some(content_dir) = dirs.get(mod_info.project_type) else {
            debug_assert!(false, "modpack ended up in mod index");
            return Ok(());
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
    if let Err(error) = tokio::fs::remove_file(&path).await {
        if let std::io::ErrorKind::NotFound = error.kind() {
            err!("File does not exist, skipping: {path:?}");
        } else {
            let err = IoError::Io {
                error,
                path: path.clone(),
            };
            Err(err)?;
        }
    }
    Ok(())
}
