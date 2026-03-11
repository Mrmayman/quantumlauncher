use ql_core::{file_utils, InstanceSelection, IoError};
use std::collections::HashSet;
use std::path::PathBuf;
use thiserror::Error;

const PKG_ERR_PREFIX: &str = "while cloning instance:\n";
#[derive(Debug, Error)]
pub enum InstanceCloneError {
    #[error("{PKG_ERR_PREFIX}failed to execute recursive directory clone wrapper: {0:?}")]
    Io(IoError),
    #[error("{PKG_ERR_PREFIX}directory already exists: {0:?}")]
    DirectoryExists(PathBuf),
}

impl From<IoError> for InstanceCloneError {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

pub async fn clone_instance(
    instance: InstanceSelection,
    exceptions: HashSet<String>,
) -> Result<(), InstanceCloneError> {
    let current_instance_name = instance.get_name();
    let new_instance_name = format!("{current_instance_name} (copy)");

    let current_instance = instance.get_instance_path();
    let new_instance = current_instance.parent().unwrap().join(&new_instance_name);

    if new_instance.is_dir() {
        return Err(InstanceCloneError::DirectoryExists(new_instance));
    }

    let exceptions: Vec<PathBuf> = exceptions
        .iter()
        .map(|n| current_instance.join(n))
        .collect();

    file_utils::copy_dir_recursive_ext(&current_instance, &new_instance, &exceptions).await?;

    Ok(())
}
