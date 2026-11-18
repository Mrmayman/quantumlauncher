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
    delete_mods_inner(ids, instance, true).await
}

/// Deletes only the requested entries while replacing them during an update.
///
/// Unlike a normal user deletion, updating a mod must not remove its
/// dependencies as orphaned content. The replacement downloader needs those
/// entries and their files to remain available while it rebuilds the index.
pub async fn delete_mods_for_update(
    ids: Vec<ModId>,
    instance: Instance,
) -> Result<Vec<ModId>, ModError> {
    delete_mods_inner(ids, instance, false).await
}

async fn delete_mods_inner(
    ids: Vec<ModId>,
    instance: Instance,
    remove_orphans: bool,
) -> Result<Vec<ModId>, ModError> {
    let _guard = lock().await;

    if ids.is_empty() {
        return Ok(ids);
    }

    info!("Deleting content:");
    let version_json = VersionDetails::load(&instance).await?;
    let mut index = ModIndex::load(&instance).await?;
    let dirs = DirStructure::new(instance.clone(), &version_json).await?;

    // Let's say we delete `DeletedMod`
    for id in &ids {
        pt!("Deleting: {id:?}");

        // `ParentMod` depends on `DeletedMod`,
        // so we need to remove `DeletedMod` from its dependencies
        let dependents = if let Some(mod_info) = index.mods.get(id) {
            mod_info.dependents.clone()
        } else {
            HashSet::new()
        };

        for dependent in &dependents {
            if let Some(dependent_info) = index.mods.get_mut(dependent) {
                dependent_info.dependencies.remove(id);
            }
        }

        delete_mod(&mut index, id, &dirs).await?;
    }

    if remove_orphans {
        let mut has_been_removed;
        loop {
            has_been_removed = false;
            let mut removed_dependents_map = HashMap::new();

            // `DeletedMod` depends on `ChildMod` but nothing else does
            // so `ChildMod` is useless now
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
    }

    index.save(&instance).await?;
    info!("Finished deleting content");
    Ok(ids)
}

/// Removes every indexed store entry with this project name before replacement.
/// This is needed because the same project can have been installed from either backend.
pub async fn delete_mod_named(name: &str, instance: Instance) -> Result<Vec<ModId>, ModError> {
    let index = ModIndex::load(&instance).await?;
    let ids = index
        .mods
        .iter()
        .filter(|(_, config)| config.name.as_ref() == name)
        .map(|(id, _)| id.clone())
        .collect();
    delete_mods(ids, instance).await
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
