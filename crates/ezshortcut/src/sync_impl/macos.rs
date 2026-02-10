use std::path::PathBuf;

pub fn get_menu_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("Applications"))
}
