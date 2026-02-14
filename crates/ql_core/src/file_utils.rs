use std::{
    collections::HashSet,
    ffi::OsStr,
    io::{Cursor, Write},
    path::{Path, PathBuf, MAIN_SEPARATOR},
    sync::LazyLock,
};

use std::sync::mpsc::Sender;

use futures::StreamExt;
use reqwest::header::InvalidHeaderValue;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio_util::io::StreamReader;
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::{
    error::{DownloadFileError, IoError},
    retry, IntoIoError, IntoJsonError, JsonDownloadError, CLIENT, WEBSITE,
};

/// The path to the QuantumLauncher root folder.
///
/// This uses the current dir or executable location (portable mode)
/// if a `qldir.txt` is found, otherwise it uses the system data dir:
/// - `~/.local/share` on Linux
/// - `~/AppData/Roaming` on Windows
/// - `~/Library/Application Support` on macOS
///
/// Use [`get_launcher_dir`] for a non-panicking solution.
///
/// # Panics
/// - if data dir is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the launcher directory could not be created (permissions issue)
#[allow(clippy::doc_markdown)]
pub static LAUNCHER_DIR: LazyLock<PathBuf> = LazyLock::new(|| get_launcher_dir().unwrap());

/// Returns the path to the QuantumLauncher root folder.
///
/// This uses the current dir or executable location (portable mode)
/// if a `qldir.txt` is found, otherwise it uses the system data dir:
/// - `~/.local/share` on Linux
/// - `~/AppData/Roaming` on Windows
/// - `~/Library/Application Support` on macOS
///
/// # Errors
/// - if data dir is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the launcher directory could not be created (permissions issue)
#[allow(clippy::doc_markdown)]
pub fn get_launcher_dir() -> Result<PathBuf, IoError> {
    let launcher_directory = if let Ok(n) = std::env::var("QL_DIR").or(std::env::var("QLDIR")) {
        canonicalize_s(&n)
    } else if let Some(n) = check_qlportable_file() {
        canonicalize_s(&n.path)
    } else {
        dirs::data_dir()
            .ok_or(IoError::LauncherDirNotFound)?
            .join("QuantumLauncher")
    };

    std::fs::create_dir_all(&launcher_directory).path(&launcher_directory)?;
    Ok(launcher_directory)
}

struct QlDirInfo {
    path: PathBuf,
}

fn line_and_body(input: &str) -> (String, String) {
    let mut lines = input.trim().lines();

    // Get the first line (if any)
    if let Some(first) = lines.next() {
        let rest = lines.collect::<Vec<_>>().join("\n");
        return (first.trim().to_owned(), rest);
    }

    (String::default(), String::default())
}

fn check_qlportable_file() -> Option<QlDirInfo> {
    const PORTABLE_FILENAME: &str = "qldir.txt";

    let places = [
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_owned)),
        std::env::current_dir().ok(),
        dirs::data_dir().map(|d| d.join("QuantumLauncher")),
        dirs::config_dir().map(|d| d.join("QuantumLauncher")),
    ];

    for (i, place) in places
        .into_iter()
        .enumerate()
        .filter_map(|(i, n)| n.map(|n| (i, n)))
    {
        let qldir_path = place.join(PORTABLE_FILENAME);
        let Ok(contents) = std::fs::read_to_string(&qldir_path) else {
            continue;
        };
        let (path, qldir_options) = line_and_body(&contents);

        let flags: HashSet<_> = qldir_options
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect();

        unsafe {
            if flags.contains("i_vulkan") {
                std::env::set_var("WGPU_BACKEND", "vulkan");
            } else if flags.contains("i_opengl") {
                std::env::set_var("WGPU_BACKEND", "opengl");
            } else if flags.contains("i_directx") {
                std::env::set_var("WGPU_BACKEND", "dx12");
            } else if flags.contains("i_metal") {
                std::env::set_var("WGPU_BACKEND", "metal");
            }
        }

        let mut join_dir = !flags.contains("top");

        let path = if let (Some(stripped), Some(home)) = (path.strip_prefix("~"), dirs::home_dir())
        {
            home.join(stripped)
        } else if path == "." {
            join_dir = false;
            place
        } else if path.is_empty() && i < 2 {
            place
        } else {
            PathBuf::from(&path)
        };

        return Some(if join_dir {
            QlDirInfo {
                path: path.join("QuantumLauncher"),
            }
        } else {
            QlDirInfo { path }
        });
    }

    None
}

/// Returns whether the user is new to QuantumLauncher,
/// i.e. whether they have never used the launcher before.
///
/// It checks whether the launcher directory does not exist.
#[must_use]
#[allow(clippy::doc_markdown)]
pub fn is_new_user() -> bool {
    let Some(data_directory) = dirs::data_dir() else {
        return false;
    };
    let launcher_directory = data_directory.join("QuantumLauncher");
    !launcher_directory.exists()
}

/// Downloads a file from the given URL into a `String`.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_string(url: &str, user_agent: bool) -> Result<String, RequestError> {
    async fn inner(url: &str, user_agent: bool) -> Result<String, RequestError> {
        let mut get = CLIENT.get(url);
        if user_agent {
            get = get.header(
                "User-Agent",
                format!("Mrmayman/quantumlauncher ({WEBSITE})"),
            );
        }
        let response = get.send().await?;
        check_for_success(&response)?;
        Ok(response.text().await?)
    }

    retry(|| async { inner(url, user_agent).await }).await
}

/// Downloads a file from the given URL into a JSON.
///
/// More specifically, it tries to parse the contents
/// into anything implementing `serde::Deserialize`
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_json<T: DeserializeOwned>(
    url: &str,
    user_agent: bool,
) -> Result<T, JsonDownloadError> {
    async fn inner<T: DeserializeOwned>(
        url: &str,
        user_agent: bool,
    ) -> Result<T, JsonDownloadError> {
        let text = download_file_to_string(url, user_agent).await?;
        Ok(serde_json::from_str(&text).json(text)?)
    }

    retry(|| async { inner(url, user_agent).await }).await
}

/// Downloads a file from the given URL into a `Vec<u8>`.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_bytes(url: &str, user_agent: bool) -> Result<Vec<u8>, RequestError> {
    async fn inner(url: &str, user_agent: bool) -> Result<Vec<u8>, RequestError> {
        let mut get = CLIENT.get(url);
        if user_agent {
            get = get.header("User-Agent", "quantumlauncher");
        }
        let response = get.send().await?;
        check_for_success(&response)?;
        Ok(response.bytes().await?.to_vec())
    }

    retry(|| async { inner(url, user_agent).await }).await
}

/// Downloads a file from the given URL and saves it to a path.
///
/// This uses `tokio` streams internally allowing for highly
/// efficient downloading.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
/// - `path`: the `&Path` to save the files to
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_path(
    url: &str,
    user_agent: bool,
    path: impl AsRef<Path>,
) -> Result<(), DownloadFileError> {
    async fn inner(url: &str, user_agent: bool, path: &Path) -> Result<(), DownloadFileError> {
        let mut get = CLIENT.get(url);
        if user_agent {
            get = get.header("User-Agent", "quantumlauncher");
        }
        let response = get.send().await?;
        check_for_success(&response)?;

        let stream = response
            .bytes_stream()
            .map(|n| n.map_err(std::io::Error::other));
        let mut stream = StreamReader::new(stream);

        if let Some(parent) = path.parent() {
            if !parent.is_dir() {
                tokio::fs::create_dir_all(&parent).await.path(parent)?;
            }
        }

        let mut file = tokio::fs::File::create(&path).await.path(path)?;
        tokio::io::copy(&mut stream, &mut file).await.path(path)?;
        Ok(())
    }

    retry(|| async { inner(url, user_agent, path.as_ref()).await }).await
}

/// Downloads a file from the given URL and saves it to a path,
/// while optionally sending progress updates.
///
/// Progress is reported as bytes downloaded vs total content length when known.
/// When the response doesn't provide a `Content-Length`, the progress bar will
/// remain indeterminate (0/1) and only the message will update.
///
/// # Errors
/// Same as [`download_file_to_path`].
pub async fn download_file_to_path_with_progress(
    url: &str,
    user_agent: bool,
    path: &Path,
    progress_sender: Option<&Sender<crate::GenericProgress>>,
    progress_label: Option<&str>,
) -> Result<(), DownloadFileError> {
    async fn inner(
        url: &str,
        user_agent: bool,
        path: &Path,
        progress_sender: Option<&Sender<crate::GenericProgress>>,
        progress_label: Option<&str>,
    ) -> Result<(), DownloadFileError> {
        let mut get = CLIENT.get(url);
        if user_agent {
            get = get.header("User-Agent", "quantumlauncher");
        }
        let response = get.send().await?;
        check_for_success(&response)?;

        let total_len = response.content_length();

        if let Some(parent) = path.parent() {
            if !parent.is_dir() {
                tokio::fs::create_dir_all(&parent).await.path(parent)?;
            }
        }

        let mut file = tokio::fs::File::create(&path).await.path(path)?;

        let mut downloaded: u64 = 0;
        let mut stream = response
            .bytes_stream()
            .map(|n| n.map_err(std::io::Error::other));

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| IoError::Io {
                error: e.to_string(),
                path: path.to_owned(),
            })?;
            file.write_all(&chunk).await.path(path)?;
            downloaded = downloaded.saturating_add(chunk.len() as u64);

            if let Some(sender) = progress_sender {
                let (done, total) = match total_len {
                    Some(total) if total > 0 => {
                        let done = downloaded.min(total);
                        (done as usize, total as usize)
                    }
                    _ => (0, 1),
                };

                let message = progress_label.map(|label| {
                    if let Some(total) = total_len {
                        format!("{label} ({downloaded}/{total} bytes)")
                    } else {
                        format!("{label} ({downloaded} bytes)")
                    }
                });

                let _ = sender.send(crate::GenericProgress {
                    done,
                    total,
                    message,
                    has_finished: false,
                });
            }
        }

        file.flush().await.path(path)?;
        Ok(())
    }

    retry(|| async { inner(url, user_agent, path, progress_sender, progress_label).await }).await
}

/// Downloads a file from the given URL into a `Vec<u8>`,
/// with a custom user agent.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_bytes_with_agent(
    url: &str,
    user_agent: &str,
) -> Result<Vec<u8>, RequestError> {
    async fn inner(url: &str, user_agent: &str) -> Result<Vec<u8>, RequestError> {
        let response = CLIENT
            .get(url)
            .header("User-Agent", user_agent)
            .send()
            .await?;
        check_for_success(&response)?;
        Ok(response.bytes().await?.to_vec())
    }

    retry(|| async { inner(url, user_agent).await }).await
}

/// # Errors
/// If the HTTP response status is not a success code.
pub fn check_for_success(response: &Response) -> Result<(), RequestError> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(RequestError::DownloadError {
            code: response.status(),
            url: response.url().clone(),
        })
    }
}

const NETWORK_ERROR_MSG: &str = r"
- Check your internet connection
- Check if you are behind a firewall/proxy
- Try doing the action again

";

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("Download Error (code {code}){NETWORK_ERROR_MSG}Url: {url}")]
    DownloadError {
        code: reqwest::StatusCode,
        url: reqwest::Url,
    },
    #[error("Network Request Error{NETWORK_ERROR_MSG}{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Download Error (invalid header value){NETWORK_ERROR_MSG}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}

impl RequestError {
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            RequestError::DownloadError { code, url } => {
                format!("Download Error (code {code})\nUrl: {url}")
            }
            RequestError::ReqwestError(error) => format!("Network Request Error:\n{error}"),
            RequestError::InvalidHeaderValue(_) => {
                "Download Error: invalid header value".to_owned()
            }
        }
    }
}

/// Sets the executable bit on a file.
///
/// This makes a file executable on Unix systems,
/// i.e. it can be run as a program.
///
/// # Errors
/// Returns an error if:
/// - the file does not exist
/// - the user doesn't have permission to read the file metadata
/// - the user doesn't have permission to change the file metadata
#[cfg(target_family = "unix")]
pub async fn set_executable(path: &Path) -> Result<(), IoError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = tokio::fs::metadata(path).await.path(path)?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x
    tokio::fs::set_permissions(path, perms).await.path(path)
}

#[cfg(unix)]
use std::os::unix::fs::symlink;

use reqwest::Response;
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};

/// Creates a symbolic link (i.e. the file at `dest` "points" to `src`,
/// accessing `dest` will actually access `src`)
///
/// # Errors
/// (depending on platform):
/// - If `dest` already exists
/// - If `src` doesn't exist
/// - If user doesn't have permission for `src`
/// - If the path is invalid (part of path is not a directory for example)
/// - Other niche stuff (Read only filesystem, Running out of disk space)
pub fn create_symlink(src: &Path, dest: &Path) -> Result<(), IoError> {
    #[cfg(unix)]
    {
        symlink(src, dest).path(src)
    }

    #[cfg(windows)]
    {
        if src.is_dir() {
            symlink_dir(src, dest).path(src)
        } else {
            symlink_file(src, dest).path(src)
        }
    }
}

/// Recursively copies the contents of
/// the `src` dir to the `dst` dir.
///
/// File structure:
/// ```txt
/// src/
///     a.txt
///     b.txt
///     c/
///         d.txt
/// ```
/// To
/// ```txt
/// dst/
///     a.txt
///     b.txt
///     c/
///         d.txt
/// ```
///
/// # Errors
/// - `src` doesn't exist
/// - `dst` already has a dir with the same name as a file
/// - User doesn't have permissions for `src`/`dst` access
pub async fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), IoError> {
    copy_dir_recursive_ext(src, dst, &[]).await?;
    Ok(())
}

/// Recursively copies the contents of
/// the `src` dir to the `dst` dir.
///
/// File structure:
/// ```txt
/// src/
///     a.txt
///     b.txt
///     c/
///         d.txt
/// ```
/// To
/// ```txt
/// dst/
///     a.txt
///     b.txt
///     c/
///         d.txt
/// ```
///
/// This function has a few extra
/// features compared to the non-ext one:
///
/// - Allows specifying exceptions for not copying.
/// - More coming in the future.
///
/// # Errors
/// - `src` doesn't exist
/// - `dst` already has a dir with the same name as a file
/// - User doesn't have permissions for `src`/`dst` access
pub async fn copy_dir_recursive_ext(
    src: &Path,
    dst: &Path,
    exceptions: &[PathBuf],
) -> Result<(), IoError> {
    if src.is_file() {
        tokio::fs::copy(src, dst).await.path(src)?;
        return Ok(());
    }
    if !dst.exists() {
        tokio::fs::create_dir_all(dst).await.path(dst)?;
    }

    let mut dir = tokio::fs::read_dir(src).await.path(src)?;
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if exceptions.iter().any(|n| *n == path || path.starts_with(n)) {
            continue;
        }

        Box::pin(copy_dir_recursive_ext(&path, &dest_path, exceptions)).await?;
    }

    Ok(())
}

/// Reads all the entries from a directory into a `Vec<String>`.
/// This includes both files and folders.
///
/// # Errors
/// - `dir` doesn't exist
/// - User doesn't have access to `dir`
///
/// Additionally, this skips any file/folder names
/// that has broken encoding (not UTF-8 or ASCII).
pub async fn read_filenames_from_dir<P: AsRef<Path>>(dir: P) -> Result<Vec<DirItem>, IoError> {
    let dir: &Path = dir.as_ref();
    if !dir.exists() {
        tokio::fs::create_dir_all(dir).await.path(dir)?;
        return Ok(Vec::new());
    }

    let mut entries = tokio::fs::read_dir(dir).await.dir(dir)?;
    let mut filenames = Vec::new();

    while let Some(entry) = entries.next_entry().await.map_err(|n| IoError::ReadDir {
        error: n.to_string(),
        parent: dir.to_owned(),
    })? {
        if let Some(name) = entry.file_name().to_str() {
            filenames.push(DirItem {
                name: name.to_owned(),
                is_file: entry.path().is_file(),
            });
        }
    }

    Ok(filenames)
}

#[derive(Debug, Clone)]
pub struct DirItem {
    pub name: String,
    pub is_file: bool,
}

/// Finds the first in the specified directory
/// that matches the criteria specified by the
/// input function.
///
/// It reads the directory's entries, passing
/// the path and name to the input function.
/// If `true` is returned then that entry's path
/// will be returned, else it continues searching.
///
/// The order in which it searches is platform and filesystem
/// dependent, so essentially **non-deterministic**.
pub async fn find_item_in_dir<F: FnMut(&Path, &str) -> bool>(
    parent_dir: &Path,
    mut f: F,
) -> Result<Option<PathBuf>, IoError> {
    let mut entries = tokio::fs::read_dir(parent_dir).await.path(parent_dir)?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if let Some(file_name) = path.file_name().and_then(OsStr::to_str) {
            if f(&path, file_name) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

/// Extract a ZIP archive to a directory
///
/// If `strip_toplevel` is true, this removes the common root directory
/// (matches old `zip-extract` behavior).
pub async fn extract_zip_archive<
    R: std::io::Read + std::io::Seek + Send + 'static,
    P: AsRef<Path>,
>(
    reader: R,
    extract_to: P,
    strip_toplevel: bool,
) -> Result<(), zip::result::ZipError> {
    let mut archive = ZipArchive::new(reader)?;
    let extract_to = canonicalize_a(extract_to).await;

    if strip_toplevel {
        tokio::task::spawn_blocking(move || {
            archive.extract_unwrapped_root_dir(extract_to, zip::read::root_dir_common_filter)
        })
        .await
        .unwrap()?;
    } else {
        tokio::task::spawn_blocking(move || archive.extract(extract_to))
            .await
            .unwrap()?;
    }

    Ok(())
}

pub async fn zip_directory_to_bytes<P: AsRef<Path>>(dir: P) -> std::io::Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buffer);

    let file_options = FileOptions::<()>::default().unix_permissions(0o755);
    let dir_options = FileOptions::<()>::default()
        .unix_permissions(0o755)
        .compression_method(zip::CompressionMethod::Stored);

    let dir = dir.as_ref();
    let base_path = dir;

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        let relative_path = path
            .strip_prefix(base_path)
            .map_err(std::io::Error::other)?;
        let mut name_in_zip = relative_path.to_string_lossy().to_string();
        // .replace('\\', "/");

        if path.is_dir() {
            // Add directory entries with trailing slash (required for Java jar loading)
            if !name_in_zip.is_empty() {
                if !name_in_zip.ends_with(MAIN_SEPARATOR) {
                    name_in_zip.push(MAIN_SEPARATOR);
                }
                zip.start_file(name_in_zip, dir_options)?;
            }
        } else {
            // Add file
            zip.start_file(name_in_zip, file_options)?;
            let bytes = tokio::fs::read(path)
                .await
                .path(path)
                .map_err(std::io::Error::other)?;
            zip.write_all(&bytes)?;
        }
    }

    zip.finish()?;
    Ok(buffer.into_inner())
}

/// Used for moving the launcher dir from `.config` to `.local`.
/// Gets the old location of the launcher dir using the same methods as before the
/// migration so if the user have overwritten it using `$XGD_CONFIG_DIR` we don't lose track of it.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[must_use]
pub fn migration_legacy_launcher_dir() -> Option<PathBuf> {
    if check_qlportable_file().is_some() {
        return None;
    }
    Some(dirs::config_dir()?.join("QuantumLauncher"))
}

/// Same as [`get_launcher_dir`] but doesn't create the folder if not found.
#[must_use]
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn migration_launcher_dir() -> Option<PathBuf> {
    if check_qlportable_file().is_some() {
        return None;
    }
    Some(dirs::data_dir()?.join("QuantumLauncher"))
}

// ========
// This is one thing I find lacking in rust.
// See https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/
// for more info.

pub async fn canonicalize_a(p: impl AsRef<Path>) -> PathBuf {
    let p = p.as_ref();
    #[allow(unused_mut)]
    if let Ok(mut n) = tokio::fs::canonicalize(p).await {
        #[cfg(target_os = "windows")]
        {
            let s = n.to_string_lossy();
            if let Some(s) = s.strip_prefix("\\\\?\\") {
                n = PathBuf::from(s);
            }
        }
        n
    } else {
        p.to_owned()
    }
}

pub fn canonicalize_s(p: impl AsRef<Path>) -> PathBuf {
    let p = p.as_ref();
    #[allow(unused_mut)]
    if let Ok(mut n) = std::fs::canonicalize(p) {
        #[cfg(target_os = "windows")]
        {
            let s = n.to_string_lossy();
            if let Some(s) = s.strip_prefix("\\\\?\\") {
                n = PathBuf::from(s);
            }
        }
        n
    } else {
        p.to_owned()
    }
}

// ========

pub async fn exists(p: impl AsRef<Path>) -> bool {
    tokio::fs::try_exists(p).await.is_ok_and(|n| n)
}
