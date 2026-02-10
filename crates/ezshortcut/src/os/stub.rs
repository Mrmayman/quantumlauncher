use crate::Shortcut;
use std::path::{Path, PathBuf};

pub fn get_menu_path() -> Option<PathBuf> {
    None
}

pub async fn create(shortcut: &Shortcut, path: impl AsRef<Path>) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Shortcuts aren't supported on this platform",
    ))
}
