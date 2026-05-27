use std::{
    collections::HashSet,
    ffi::OsStr,
    io::{Cursor, Write},
    path::{MAIN_SEPARATOR, Path, PathBuf},
    sync::{LazyLock, Mutex},
};

use flate2::read::GzDecoder;
use reqwest::header::InvalidHeaderValue;
use serde::de::DeserializeOwned;
use thiserror::Error;
use walkdir::WalkDir;
use zip::{ZipArchive, ZipWriter, write::FileOptions};

use crate::{IntoIoError, JsonDownloadError, download, error::IoError};

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

static PORTABLE_WARNING: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

fn set_portable_warning(message: impl Into<String>) -> bool {
    if let Ok(mut warning) = PORTABLE_WARNING.lock() {
        if warning.is_none() {
            *warning = Some(message.into());
            return true;
        }
    }
    false
}

#[must_use]
pub fn take_portable_warning() -> Option<String> {
    PORTABLE_WARNING
        .lock()
        .ok()
        .and_then(|mut warning| warning.take())
}

/// The path to the QuantumLauncher cache folder.
#[allow(clippy::doc_markdown)]
pub static LAUNCHER_CACHE_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| get_launcher_cache_dir().unwrap());

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
    let mut allow_fallback = false;
    let launcher_directory = if let Ok(n) = std::env::var("QL_DIR").or(std::env::var("QLDIR")) {
        canonicalize_s(&n)
    } else if let Some(n) = check_qlportable_file() {
        allow_fallback = true;
        canonicalize_s(&n.path)
    } else {
        dirs::data_dir()
            .ok_or(IoError::LauncherDirNotFound)?
            .join("QuantumLauncher")
    };

    if let Err(err) = std::fs::create_dir_all(&launcher_directory).path(&launcher_directory) {
        if allow_fallback {
            if let IoError::Io { error, .. } = &err {
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ReadOnlyFilesystem
                ) {
                    if let Some(fallback_dir) = dirs::data_dir().map(|d| d.join("QuantumLauncher"))
                    {
                        if std::fs::create_dir_all(&fallback_dir)
                            .path(&fallback_dir)
                            .is_ok()
                        {
                            let warning = "Portable data path is read-only. Portable mode was disabled and the system data directory is being used instead.";
                            if set_portable_warning(warning) {
                                eprintln!("{warning}");
                            }
                            return Ok(fallback_dir);
                        }
                    }
                }
            }
        }

        return Err(err);
    }

    Ok(launcher_directory)
}

/// Returns the path to the cache directory for downloadables for QuantumLauncher.
///
/// This uses `dirs::cache_dir()` as the highest-priority choice:
/// - `$XDG_CACHE_HOME/QuantumLauncher` or `$HOME/.cache/QuantumLauncher` on Linux
/// - `$HOME/Library/Caches/QuantumLauncher` on macOS
/// - `{FOLDERID_LocalAppData}\QuantumLauncher` on Windows
///
/// If unavailable, this resorts to:
/// - `$LAUNCHER_DIR/downloads/cache`
///
/// # Errors
/// - if data dir is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the cache directory could not be created (permissions issue)
pub fn get_launcher_cache_dir() -> Result<PathBuf, IoError> {
    let cache_dir = dirs::cache_dir()
        .map(|n| n.join("QuantumLauncher"))
        .filter(|n| *n != *LAUNCHER_DIR) // Don't want to accidentally put cache in our root dir
        .unwrap_or_else(|| LAUNCHER_DIR.join("downloads/cache"));

    std::fs::create_dir_all(&cache_dir).path(&cache_dir)?;
    Ok(cache_dir)
}

#[cfg(unix)]
fn expand_tilde(path: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    if path == "~" {
        return Some(home);
    }
    if let Some(stripped) = path.strip_prefix("~/") {
        return Some(home.join(stripped));
    }
    None
}

#[cfg(not(unix))]
fn expand_tilde(_path: &str) -> Option<PathBuf> {
    None
}

/// Deletes the `running` sentinel file if it exists.
pub fn cleanup_running_file() {
    if let Ok(dir) = get_launcher_dir() {
        _ = std::fs::remove_file(dir.join("running"));
    }
}

pub(crate) struct QlDirInfo {
    pub path: PathBuf,
    #[allow(dead_code)]
    pub flags: HashSet<String>,
}

/// The filename of the portable mode marker file.
pub const PORTABLE_FILENAME: &str = "qldir.txt";

#[derive(Debug, Clone, PartialEq)]
pub struct QlDirStatus {
    pub path: Option<PathBuf>,
    pub flags: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PortableModeKind {
    /// `qldir.txt` is beside the executable. Contained path and flags are from the file.
    Portable(QlDirStatus),
    /// `qldir.txt` is in the system data/config directory. Contained path and flags are from the file.
    SystemRedirect(QlDirStatus),
}

/// Returns the current portable-mode state, or `None` if it is not active.
///
/// Reads `qldir.txt` from beside the executable (if it exists).
/// Returns the current portable-mode state for the executable directory.
#[must_use]
pub fn get_portable_status() -> Option<QlDirStatus> {
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_owned))
    {
        let qldir_path = exe_dir.join(PORTABLE_FILENAME);
        if let Ok(contents) = std::fs::read_to_string(&qldir_path) {
            let (path_str, flags_str) = line_and_body(&contents);
            let path = if path_str.is_empty() {
                None
            } else {
                Some(PathBuf::from(path_str))
            };
            let flags = flags_str
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|n| !n.is_empty())
                .collect();
            return Some(QlDirStatus { path, flags });
        }
    }
    None
}

/// Returns the current portable-mode state for the system data directory.
#[must_use]
pub fn get_system_redirect_status() -> Option<QlDirStatus> {
    if let Some(data_dir) = dirs::data_dir().map(|d| d.join("QuantumLauncher")) {
        let qldir_path = data_dir.join(PORTABLE_FILENAME);
        if let Ok(contents) = std::fs::read_to_string(&qldir_path) {
            let (path_str, flags_str) = line_and_body(&contents);
            let path = if path_str.is_empty() {
                None
            } else {
                Some(PathBuf::from(path_str))
            };
            let flags = flags_str
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|n| !n.is_empty())
                .collect();
            return Some(QlDirStatus { path, flags });
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq)]
pub struct FullPortableStatus {
    pub portable: Option<QlDirStatus>,
    pub system_redirect: Option<QlDirStatus>,
}

/// Returns the current portable-mode state, checking both locations.
#[must_use]
pub fn portable_mode_status() -> FullPortableStatus {
    FullPortableStatus {
        portable: get_portable_status(),
        system_redirect: get_system_redirect_status(),
    }
}

/// Creates a `qldir.txt` file next to the executable to enable portable mode.
///
/// If `path` is empty or `.`, the launcher stores data in a
/// `QuantumLauncher/` folder next to the executable.
///
/// # Errors
/// - The executable path cannot be determined
/// - The file cannot be written (permissions, disk full, etc.)
pub fn create_portable_file(path: String, flags: HashSet<String>) -> Result<(), IoError> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| IoError::Io {
            error: e,
            path: PathBuf::from("<current_exe>"),
        })?
        .parent()
        .map(Path::to_owned)
        .ok_or_else(|| IoError::Io {
            error: std::io::Error::other("Could not determine executable directory"),
            path: PathBuf::from("<exe parent>"),
        })?;
    ensure_qldir_target_dir(&exe_dir, &path, &flags, false)?;
    let qldir_path = exe_dir.join(PORTABLE_FILENAME);
    let mut contents = path;
    if !flags.is_empty() {
        contents.push('\n');
        contents.push_str(&flags.into_iter().collect::<Vec<_>>().join(","));
    }
    std::fs::write(&qldir_path, contents).path(&qldir_path)
}

/// Creates a `qldir.txt` file in the system data directory to enable system-level redirection.
///
/// # Errors
/// - The system data directory cannot be determined
/// - The file cannot be written (permissions, disk full, etc.)
pub fn create_system_redirect_file(path: String, flags: HashSet<String>) -> Result<(), IoError> {
    let data_dir = dirs::data_dir()
        .map(|d| d.join("QuantumLauncher"))
        .ok_or_else(|| IoError::Io {
            error: std::io::Error::other("Could not determine system data directory"),
            path: PathBuf::from("<system data dir>"),
        })?;

    // Ensure the directory exists
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).path(&data_dir)?;
    }

    ensure_qldir_target_dir(&data_dir, &path, &flags, true)?;
    let qldir_path = data_dir.join(PORTABLE_FILENAME);
    let mut contents = path;
    if !flags.is_empty() {
        contents.push('\n');
        contents.push_str(&flags.into_iter().collect::<Vec<_>>().join(","));
    }
    std::fs::write(&qldir_path, contents).path(&qldir_path)
}

fn ensure_qldir_target_dir(
    place: &Path,
    path: &str,
    flags: &HashSet<String>,
    relative_from_parent: bool,
) -> Result<(), IoError> {
    let mut join_dir = !flags.contains("top");
    let target = if let Some(expanded) = expand_tilde(path) {
        expanded
    } else if path == "." {
        join_dir = false;
        place.to_owned()
    } else if path.is_empty() && !relative_from_parent {
        place.to_owned()
    } else {
        let path_buf = PathBuf::from(path);
        if path_buf.is_relative() {
            let base_dir = if relative_from_parent {
                place.parent().unwrap_or(place)
            } else {
                place
            };
            base_dir.join(path_buf)
        } else {
            path_buf
        }
    };

    let target = if join_dir {
        target.join("QuantumLauncher")
    } else {
        target
    };

    std::fs::create_dir_all(&target).path(&target)
}

/// Deletes the `qldir.txt` file from beside the executable.
pub fn delete_portable_file() -> Result<(), IoError> {
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_owned))
    {
        let qldir_path = exe_dir.join(PORTABLE_FILENAME);
        if qldir_path.exists() {
            return std::fs::remove_file(&qldir_path).path(&qldir_path);
        }
    }

    Err(IoError::Io {
        error: std::io::Error::new(std::io::ErrorKind::NotFound, "Portable qldir.txt not found"),
        path: PathBuf::from("qldir.txt"),
    })
}

/// Deletes the `qldir.txt` file from the system data directory.
pub fn delete_system_redirect_file() -> Result<(), IoError> {
    if let Some(data_dir) = dirs::data_dir().map(|d| d.join("QuantumLauncher")) {
        let qldir_path = data_dir.join(PORTABLE_FILENAME);
        if qldir_path.exists() {
            return std::fs::remove_file(&qldir_path).path(&qldir_path);
        }
    }

    Err(IoError::Io {
        error: std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "System redirect qldir.txt not found",
        ),
        path: PathBuf::from("qldir.txt"),
    })
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
            .filter(|n| !n.is_empty())
            .collect();

        let mut join_dir = !flags.contains("top");

        let path = if let Some(expanded) = expand_tilde(&path) {
            expanded
        } else if path == "." {
            join_dir = false;
            place
        } else if path.is_empty() && i < 2 {
            place
        } else {
            let path_buf = PathBuf::from(&path);
            if path_buf.is_relative() {
                let base_dir = if i >= 2 {
                    place.parent().unwrap_or(&place)
                } else {
                    &place
                };
                base_dir.join(path_buf)
            } else {
                path_buf
            }
        };

        return Some(if join_dir {
            QlDirInfo {
                path: path.join("QuantumLauncher"),
                flags,
            }
        } else {
            QlDirInfo { path, flags }
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
    let mut r = download(url);
    if user_agent {
        r = r.user_agent_ql();
    }
    r.string().await
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
    let mut r = download(url);
    if user_agent {
        r = r.user_agent_ql();
    }
    r.json().await
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
    let mut r = download(url);
    if user_agent {
        r = r.user_agent_ql();
    }
    r.bytes().await
}

const NETWORK_ERROR_MSG: &str = r"
- Check your internet connection
- Check if you are behind a firewall/proxy
- Try doing the action again

";

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("Download Error (middleware): {0}")]
    MiddlewareError(#[from] reqwest_middleware::Error),
    #[error("Download Error (code {code}){NETWORK_ERROR_MSG}Url: {url}")]
    DownloadError {
        code: reqwest::StatusCode,
        url: reqwest::Url,
    },
    #[error("Network Request Error{NETWORK_ERROR_MSG}{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Download Error (invalid header value){NETWORK_ERROR_MSG}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error("{0}")]
    Message(String),
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
            RequestError::MiddlewareError(error) => format!("Download error (middleware): {error}"),
            RequestError::Message(msg) => msg.clone(),
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
    #[cfg(windows)]
    use std::os::windows::fs as osfs;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dest).path(src)
    }
    #[cfg(windows)]
    {
        if src.is_dir() {
            osfs::symlink_dir(src, dest).path(src)
        } else {
            osfs::symlink_file(src, dest).path(src)
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
    if !exists(dst).await {
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
    if !exists(dir).await {
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
    } else {
        tokio::task::spawn_blocking(move || archive.extract(extract_to)).await
    }
    .map_err(|n| zip::result::ZipError::Io(n.into()))??;

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

/// Extracts a `.tar.gz` file from a `&[u8]` buffer into the given directory.
///
/// Does not create a top-level directory,
/// extracting files directly into the target directory.
///
/// # Arguments
/// - `data`: A reference to the `.tar.gz` file as a byte slice.
/// - `output_dir`: Path to the directory where the contents will be extracted.
///
/// # Errors
/// - `std::io::Error` if the `.tar.gz` file was invalid.
pub fn extract_tar_gz(archive: &[u8], output_dir: &Path) -> std::io::Result<()> {
    // For extracting the `.gz`
    let decoder = GzDecoder::new(Cursor::new(archive));
    // For extracting the `.tar`
    let mut tar = tar::Archive::new(decoder);

    // Get the first entry path to determine the top-level directory
    let mut entries = tar.entries()?;
    let top_level_dir = if let (Some(entry), None) = (entries.next(), entries.next()) {
        entry?
            .path()?
            .components()
            .next()
            .map(|c| c.as_os_str().to_os_string())
    } else {
        None
    };

    let decoder = GzDecoder::new(Cursor::new(archive));
    let mut tar = tar::Archive::new(decoder);

    for entry in tar.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?;

        let new_path = top_level_dir
            .as_ref()
            .and_then(|top_level| entry_path.strip_prefix(top_level).ok())
            .unwrap_or(&entry_path);
        let full_path = output_dir.join(new_path);

        if let Some(parent) = full_path.parent() {
            // Not using async due to some weird thread safety error
            std::fs::create_dir_all(parent)?;
        }

        entry.unpack(full_path)?;
    }

    Ok(())
}
