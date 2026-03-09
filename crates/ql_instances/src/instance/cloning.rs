use ql_core::{InstanceSelection, LAUNCHER_DIR};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

async fn clone_selected(
    instance_name: &str,
    new_instance_name: String,
    selection: HashSet<String>,
) {
    let from = Path::new(LAUNCHER_DIR.as_path())
        .join("instances")
        .join(instance_name);
    let to = Path::new(LAUNCHER_DIR.as_path())
        .join("instances")
        .join(new_instance_name);

    for entry in WalkDir::new(&from) {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy();

        if selection.contains(&name.to_string()) {
            let relative_path = entry.path().strip_prefix(&from).unwrap();
            let target = to.join(relative_path);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&target).unwrap();
            } else {
                fs::create_dir_all(target.parent().unwrap()).unwrap();
                fs::copy(entry.path(), &target).unwrap();
            }
        }
    }
}

const PKG_ERR_PREFIX: &str = "while cloning instance:\n";
#[derive(Debug, Error)]
pub enum InstanceCloneError {
    #[error("{PKG_ERR_PREFIX}directory already exists: {0:?}")]
    DirectoryExists(PathBuf),
}

pub async fn clone_instance(
    instance: InstanceSelection,
    selection: HashSet<String>,
) -> Result<(), InstanceCloneError> {
    let current_instance_name = instance.get_name();
    let new_instance_name = format!("{current_instance_name} (copy)");

    let possible_dir = Path::new(LAUNCHER_DIR.as_path())
        .join("instances")
        .join(&new_instance_name);

    if possible_dir.is_dir() {
        return Err(InstanceCloneError::DirectoryExists(possible_dir));
    } else {
        clone_selected(current_instance_name, new_instance_name, selection).await;
    }

    Ok(())
}
