use std::{os::unix::fs::PermissionsExt, path::Path};

use tokio::fs;

use crate::{Shortcut, make_filename_safe};

pub async fn create(shortcut: &Shortcut, path: impl AsRef<Path>) -> std::io::Result<()> {
    let path = path.as_ref();
    create_inner(shortcut, path).await?;

    Ok(())
}

async fn create_inner(shortcut: &Shortcut, path: &Path) -> Result<(), std::io::Error> {
    let content = format!(
        r"[Desktop Entry]
Version=1.0
Type=Application
Name={name}
Comment={description}
Exec={exec}
{icon}
Terminal=false
Categories=Game;",
        name = shortcut.name,
        description = shortcut.description,
        exec = shortcut.exec,
        icon = shortcut
            .icon
            .as_deref()
            .map(|n| format!("Icon={n}"))
            .unwrap_or_default()
    );

    match fs::metadata(path).await {
        Ok(n) => {
            if n.is_dir() {
                write_file(
                    &path.join(format!(
                        "{}.desktop",
                        make_filename_safe(&shortcut.name, true)
                    )),
                    content,
                )
                .await?;
            }
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
