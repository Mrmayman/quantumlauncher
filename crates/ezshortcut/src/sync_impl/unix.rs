use std::path::PathBuf;

pub fn get_menu_path() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        Some(PathBuf::from(dir).join("applications"))
    } else {
        dirs::home_dir().map(|h| h.join(".local/share/applications"))
    }
}
