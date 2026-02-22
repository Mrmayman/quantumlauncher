use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use tokio::fs;

use crate::Shortcut;

/// Fetches path to the `.desktop` files' folder (Applications Menu)
#[must_use]
pub fn get_menu_path() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        Some(PathBuf::from(dir).join("applications"))
    } else {
        dirs::home_dir().map(|h| h.join(".local/share/applications"))
    }
}

fn refresh_applications() {
    tokio::task::spawn(async {
        _ = tokio::process::Command::new("update-desktop-database")
            .output()
            .await;
        _ = tokio::process::Command::new("kbuildsycoca5").output().await;
    });
}

pub async fn create_in_applications(shortcut: &Shortcut) -> std::io::Result<()> {
    let path = dirs::data_dir()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Couldn't access data dir (.local/share)",
        ))?
        .join("applications");
    fs::create_dir_all(&path).await?;
    create(shortcut, path).await?;
    refresh_applications();
    Ok(())
}

pub async fn create(shortcut: &Shortcut, path: impl AsRef<Path>) -> std::io::Result<()> {
    let path = path.as_ref();
    create_inner(shortcut, path).await?;

    Ok(())
}

async fn create_inner(shortcut: &Shortcut, path: &Path) -> Result<(), std::io::Error> {
    let desc = shortcut.description.trim();
    let args: String = shortcut.get_formatted_args();
    let content = format!(
        r"[Desktop Entry]
Version=1.0
Type=Application
Name={name}
{icon}{description}Exec={exec:?} {args}
Terminal=false
Categories=Game;",
        name = shortcut.name,
        description = if desc.is_empty() {
            String::new()
        } else {
            format!("Comment={desc}\n")
        },
        exec = shortcut.exec,
        icon = (!shortcut.icon.is_empty())
            .then(|| format!("Icon={}\n", shortcut.icon))
            .unwrap_or_default()
    );

    match fs::metadata(path).await {
        Ok(n) if n.is_dir() => {
            write_file(&path.join(shortcut.get_filename()), content).await?;
        }
        _ => write_file(path, content).await?,
    };
    Ok(())
}

async fn write_file(path: &Path, content: String) -> std::io::Result<()> {
    fs::write(path, content).await?;
    let mut perms = fs::metadata(path).await?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).await?;
    Ok(())
}
