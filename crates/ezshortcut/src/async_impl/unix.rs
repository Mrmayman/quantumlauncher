use std::{os::unix::fs::PermissionsExt, path::Path};

use tokio::fs;

use crate::{make_filename_safe, Shortcut};

pub async fn create(shortcut: &Shortcut, path: impl AsRef<Path>) -> std::io::Result<()> {
    let path = path.as_ref();
    create_inner(shortcut, path).await?;

    Ok(())
}

async fn create_inner(shortcut: &Shortcut, path: &Path) -> Result<(), std::io::Error> {
    let desc = shortcut.description.trim();
    let content = format!(
        r"[Desktop Entry]
Version=1.0
Type=Application
Name={name}
{icon}{description}Exec={exec}
Terminal=false
Categories=Game;",
        name = shortcut.name,
        description = if desc.is_empty() {
            String::new()
        } else {
            format!("Comment={desc}\n")
        },
        exec = shortcut.exec,
        icon = shortcut
            .icon
            .as_deref()
            .map(|n| format!("Icon={n}\n"))
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
