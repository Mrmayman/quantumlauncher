use tokio::fs;
use windows::{
    core::{Interface, HSTRING},
    Win32::{
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IPersistFile,
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
        },
        UI::Shell::{
            FOLDERID_Programs, IShellLinkW, SHGetKnownFolderPath, ShellLink, KNOWN_FOLDER_FLAG,
        },
    },
};

use crate::Shortcut;
use std::path::Path;

pub fn get_menu_path() -> Option<std::path::PathBuf> {
    unsafe {
        let path_ptr =
            SHGetKnownFolderPath(&FOLDERID_Programs, KNOWN_FOLDER_FLAG::default(), None).ok()?;

        let path = path_ptr.to_string().ok()?;
        CoTaskMemFree(Some(path_ptr.0 as _));
        Some(std::path::PathBuf::from(path).join("QuantumLauncher"))
    }
}

pub async fn create(shortcut: &Shortcut, path: impl AsRef<Path>) -> std::io::Result<()> {
    create_inner(shortcut, path.as_ref()).await?;
    Ok(())
}

async fn create_inner(shortcut: &Shortcut, path: &Path) -> std::io::Result<()> {
    let path = match fs::metadata(path).await {
        Ok(n) if n.is_dir() => path.join(shortcut.get_filename()),
        _ => path.to_owned(),
    };

    let args = shortcut
        .exec_args
        .iter()
        .map(|a| quote_windows_arg(a))
        .collect::<Vec<_>>()
        .join(" ");

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(ioerr)?;

        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        shell_link.SetPath(&HSTRING::from(&shortcut.exec))?;
        if !args.trim().is_empty() {
            shell_link.SetArguments(&HSTRING::from(args))?;
        }
        if !shortcut.description.trim().is_empty() {
            shell_link.SetDescription(&HSTRING::from(&shortcut.description))?;
        }

        let persist: IPersistFile = shell_link.cast()?;
        persist.Save(&HSTRING::from(path.as_path()), true)?;

        CoUninitialize();
    }

    Ok(())
}

fn ioerr(err: impl std::error::Error + Send + Sync + 'static) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, Box::new(err))
}

pub async fn create_in_applications(shortcut: &Shortcut) -> std::io::Result<()> {
    let start_menu = tokio::task::spawn_blocking(|| get_menu_path())
        .await?
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Start menu path not found")
        })?;
    fs::create_dir_all(&start_menu).await?;
    create_inner(shortcut, &start_menu).await
}

fn quote_windows_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    let needs_quotes = arg.chars().any(|c| c == ' ' || c == '\t' || c == '"');

    if !needs_quotes {
        return arg.to_string();
    }

    let mut result = String::from("\"");
    let mut backslashes = 0;

    for ch in arg.chars() {
        match ch {
            '\\' => {
                backslashes += 1;
            }
            '"' => {
                result.push_str(&"\\".repeat(backslashes * 2 + 1));
                result.push('"');
                backslashes = 0;
            }
            _ => {
                if backslashes > 0 {
                    result.push_str(&"\\".repeat(backslashes));
                    backslashes = 0;
                }
                result.push(ch);
            }
        }
    }

    if backslashes > 0 {
        result.push_str(&"\\".repeat(backslashes * 2));
    }

    result.push('"');
    result
}
