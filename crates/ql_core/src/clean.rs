use std::{collections::HashSet, fs::Metadata, path::Path};

use fs::DirEntry;
use tokio::fs;

use crate::{
    file_utils::get_launcher_dir,
    info,
    json::{AssetIndex, VersionDetails},
    pt, IntoIoError, IntoJsonError, IoError, JsonFileError, LAUNCHER_DIR,
};

const SIZE_LIMIT_BYTES: u64 = 100 * 1024 * 1024; // 100 MB

/// Cleans the contents of a directory (by last accessed)
/// if it's larger than 100 MB.
///
/// # Arguments
/// - `dir_name`: Path relative to the root of the launcher dir.
///   For example, to clean `QuantumLauncher/downloads/cache/`
///   you pass in `downloads/cache`.
///
/// # Errors
/// If:
/// - Launcher dir couldn't be determined
/// - `dir_name` is pointing at a file
/// - User lacks permissions
pub async fn dir(dir_name: &str) -> Result<(), IoError> {
    let launcher_dir = get_launcher_dir()?;
    let dir = launcher_dir.join(dir_name);
    if dir == launcher_dir || dir_name.trim().is_empty() {
        return Ok(());
    }
    if !dir.exists() {
        fs::create_dir_all(&dir).await.path(dir)?;
        return Ok(());
    }

    let (total_size, mut files) = scan_files_to_delete(&dir).await?;

    if total_size <= SIZE_LIMIT_BYTES {
        return Ok(());
    }

    info!("Cleaning up {dir:?}");
    files.sort_unstable_by_key(|(_, metadata)| file_time(metadata));
    let cleaned_amount = delete_files(total_size, &files).await?;

    pt!(
        "Cleaned {:.1} MB",
        cleaned_amount as f64 / (1024.0 * 1024.0)
    );

    Ok(())
}

fn file_time(metadata: &Metadata) -> std::time::SystemTime {
    metadata
        .accessed()
        .or_else(|_| metadata.modified())
        .or_else(|_| metadata.created())
        .unwrap_or(std::time::SystemTime::now())
}

async fn scan_files_to_delete(dir: &Path) -> Result<(u64, Vec<(DirEntry, Metadata)>), IoError> {
    let mut total_size = 0;
    let mut files: Vec<(DirEntry, Metadata)> = Vec::new();
    let mut read_dir = fs::read_dir(dir).await.dir(dir)?;
    while let Some(entry) = read_dir.next_entry().await.dir(dir)? {
        let metadata = entry.metadata().await.path(entry.path())?;
        if metadata.is_file() {
            total_size += metadata.len();
            files.push((entry, metadata));
        }
    }
    Ok((total_size, files))
}

async fn delete_files(mut total_size: u64, files: &[(DirEntry, Metadata)]) -> Result<u64, IoError> {
    let mut cleaned_amount = 0;
    for (file, metadata) in files {
        let path = file.path();
        fs::remove_file(&path).await.path(path)?;
        let len = metadata.len();
        total_size -= len;
        cleaned_amount += len;

        if total_size <= SIZE_LIMIT_BYTES {
            break;
        }
    }
    Ok(cleaned_amount)
}

/// Cleans the assets directory by deleting unused files.
///
/// What this does:
/// - Traverses the JSONs of each instance
/// - Removes unused asset indexes (not referenced by any instance)
/// - Removes unused files (not referenced by asset indexes)
///
/// # Errors
/// - User lacks permissions
/// - File/directory/JSON structure is invalid
pub async fn assets_dir() -> Result<u64, JsonFileError> {
    let assets_dir = LAUNCHER_DIR.join("assets/dir");
    let indexes_dir = assets_dir.join("indexes");

    let indexes = get_used_indexes().await?;
    let hashes = get_used_hashes(&indexes_dir, &indexes).await?;

    let mut cleaned_size = 0;

    let objects_dir = assets_dir.join("objects");
    let mut objects = fs::read_dir(&objects_dir).await.path(&objects_dir)?;
    while let Some(next) = objects.next_entry().await.path(&objects_dir)? {
        let o_dir_path = next.path();
        let mut o_dir = fs::read_dir(&o_dir_path).await.path(&o_dir_path)?;

        let mut dir_is_empty = true;
        while let Some(object) = o_dir.next_entry().await.path(&o_dir_path)? {
            let name = object.file_name().to_string_lossy().to_string();
            if hashes.contains(&name) {
                dir_is_empty = false;
            } else {
                let path = object.path();
                let metadata = object.metadata().await.path(&path)?;
                cleaned_size += metadata.len();

                fs::remove_file(&path).await.path(path)?;
            }
        }

        if dir_is_empty {
            fs::remove_dir_all(&o_dir_path).await.path(&o_dir_path)?;
        }
    }

    Ok(cleaned_size)
}

async fn get_used_hashes(
    indexes_dir: &Path,
    index_files: &[String],
) -> Result<HashSet<String>, JsonFileError> {
    let mut jsons = Vec::new();
    if !fs::try_exists(indexes_dir).await.path(indexes_dir)? {
        fs::create_dir_all(indexes_dir).await.path(indexes_dir)?;
        return Ok(HashSet::new());
    }

    let mut indexes = fs::read_dir(indexes_dir).await.path(indexes_dir)?;
    while let Some(next) = indexes.next_entry().await.path(indexes_dir)? {
        let path = next.path();
        let name = next.file_name();
        if !index_files.iter().any(|n| **n == name) {
            fs::remove_file(&path).await.path(path)?;
            continue;
        }

        let json = fs::read_to_string(&path).await.path(path)?;
        let json: AssetIndex = serde_json::from_str(&json).json(json)?;
        jsons.push(json);
    }

    let hashes: HashSet<String> = jsons
        .into_iter()
        .flat_map(|n| n.objects.into_values().map(|n| n.hash))
        .collect();

    Ok(hashes)
}

async fn get_used_indexes() -> Result<Vec<String>, JsonFileError> {
    let instances_dir = LAUNCHER_DIR.join("instances");
    if !fs::try_exists(&instances_dir).await.path(&instances_dir)? {
        fs::create_dir_all(&instances_dir)
            .await
            .path(instances_dir)?;
        return Ok(Vec::new());
    }
    let mut instances = fs::read_dir(&instances_dir).await.path(&instances_dir)?;

    let mut used_files = Vec::new();

    while let Some(instance) = instances.next_entry().await.path(&instances_dir)? {
        let json_path = instance.path().join("details.json");
        if !fs::try_exists(&json_path).await.path(&json_path)? {
            continue;
        }
        let Ok(json) = fs::read_to_string(&json_path).await else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<VersionDetails>(&json) else {
            continue;
        };
        used_files.push(format!("{}.json", json.assetIndex.id));
    }

    Ok(used_files)
}
