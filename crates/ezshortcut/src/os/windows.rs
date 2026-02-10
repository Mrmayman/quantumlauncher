use crate::Shortcut;
use std::path::{Path, PathBuf};

pub fn get_menu_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("Microsoft/Windows/Start Menu/Programs"))
}

pub async fn create(shortcut: &Shortcut, path: impl AsRef<Path>) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Shortcuts aren't supported on this platform",
    ))
}
