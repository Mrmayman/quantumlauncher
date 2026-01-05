//! This module provides functionality to read and manage Minecraft save files

use std::path::{Path, PathBuf};

use tokio::fs;

use crate::{do_jobs, info_no_log, IntoIoError, IoError, LAUNCHER_DIR};

/// Represents a Minecraft save/world
#[derive(Debug, Clone)]
pub struct Save {
    /// The name of the save (folder name)
    pub name: String,
    /// Path to the save directory
    pub path: PathBuf,
    /// Path to the world icon (icon.png), if it exists
    pub icon_path: Option<PathBuf>,
    /// Size of the save directory in bytes (if calculable)
    pub size_bytes: Option<u64>,
}

impl Save {
    /// Creates a new Save instance with basic information
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            icon_path: None,
            size_bytes: None,
        }
    }

    /// Checks if the save directory contains an icon.png file,
    /// adding it to field if found.
    async fn check_icon(&mut self) -> Result<(), IoError> {
        let icon_path = self.path.join("icon.png");
        if icon_path.exists() {
            self.icon_path = Some(icon_path);
        }
        Ok(())
    }

    /// Gets the size, calculating it only if not already cached
    async fn get_size(&mut self) -> Result<u64, IoError> {
        if let Some(size) = self.size_bytes {
            return Ok(size);
        }

        let size = calculate_directory_size(&self.path).await?;
        self.size_bytes = Some(size);
        Ok(size)
    }
}

/// Reads all save information from a Minecraft **client** instance.
///
/// Returns [`Save`]s containing metadata about each world.
pub async fn read_saves_info(instance: String) -> Result<Vec<Save>, IoError> {
    let saves_dir = LAUNCHER_DIR
        .join("instances")
        .join(&instance)
        .join(".minecraft/saves");
    if !saves_dir.exists() {
        fs::create_dir_all(&saves_dir).await.path(&saves_dir)?;
        return Ok(Vec::new());
    }

    let mut potential_saves = Vec::new();
    let mut entries = fs::read_dir(&saves_dir).await.path(&saves_dir)?;

    while let Some(entry) = entries.next_entry().await.path(&saves_dir)? {
        let entry_path = entry.path();

        if entry_path.is_dir() {
            let save_name = entry.file_name().to_string_lossy().to_string();
            potential_saves.push(Save::new(save_name, entry_path));
        }
    }

    let tasks = potential_saves.into_iter().map(|mut save| async move {
        save.check_icon().await?;
        save.get_size().await?;
        Ok(save)
    });

    let saves = do_jobs(tasks).await?;

    if !saves.is_empty() {
        let len = saves.len();
        info_no_log!(
            "Detected {len} save{} for instance {instance}",
            if len < 2 { "" } else { "s" }
        );
    }

    Ok(saves)
}

async fn calculate_directory_size(dir: &Path) -> Result<u64, IoError> {
    let mut total_size = 0;
    let mut entries = fs::read_dir(dir).await.path(dir)?;

    while let Some(entry) = entries.next_entry().await.path(dir)? {
        let entry_path = entry.path();
        let metadata = fs::metadata(&entry_path).await.path(&entry_path)?;

        if metadata.is_file() {
            total_size += metadata.len();
        } else if metadata.is_dir() {
            total_size += Box::pin(calculate_directory_size(&entry_path)).await?;
        }
    }

    Ok(total_size)
}
