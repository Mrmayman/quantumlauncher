use std::path::PathBuf;

pub fn get_menu_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("Microsoft/Windows/Start Menu/Programs"))
}
