//! Quick, easy cross-platform Java.
//!
//! This crate allows you to get a path to any Java executable
//! (like `java`, `javac`, `jar`, etc). It auto-installs Java
//! if not present.
//!
//! See [`get_java_binary`] for examples.
//!
//! # Platform Support
//!
//! - ✅: [From Mojang](https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json)
//! - 🟢: Supported through [Azul Zulu](https://www.azul.com/downloads/#zulu)
//!   ([API](https://docs.azul.com/core/install/metadata-api))
//! - 🟢¹: Uses newer Java (with backwards compatibility)
//! - 🟢²: Installed from:
//!   - FreeBSD: <https://github.com/Mrmayman/get-jdk>
//!   - Others: <https://bell-sw.com/pages/downloads>
//!
//! | Platforms   | 8  | 16 | 17 | 21 | 25 |
//! |:------------|:--:|:--:|:--:|:--:|:--:|
//! | **Windows** `x86_64`  | ✅ | ✅ | ✅ | ✅ | ✅ |
//! |  *Windows*  `i686`    | ✅ | ✅ | ✅ | 🟢²|    |
//! | **Windows** `aarch64`²| 🟢¹| 🟢 | ✅ | ✅ | ✅ |
//! | | | | | |
//! | **macOS**   `x86_64`  | ✅ | ✅ | ✅ | ✅ | ✅ |
//! | **macOS**   `aarch64` | 🟢 | 🟢 | ✅ | ✅ | ✅ |
//! | | | | | |
//! | **Linux**      `x86_64`  | ✅ | ✅ | ✅ | ✅ | ✅ |
//! |  *Linux*       `i686`    | ✅ | 🟢 | 🟢 | 🟢²|    |
//! | **Linux**      `aarch64` | 🟢 | 🟢 | 🟢 | 🟢 | 🟢 |
//! |  *Linux*       `arm32`   | 🟢 | 🟢¹| 🟢 | 🟢²|    |
//! | **Linux** MUSL `x86_64`  | 🟢 | 🟢 | 🟢 | 🟢 | 🟢 |
//! | **Linux** MUSL `aarch64` | 🟢 | 🟢 | 🟢 | 🟢 | 🟢 |
//! | | | | | |
//! | **FreeBSD** `x86_64`  | 🟢²|    |    |    |    |
//! | **Solaris** `x86_64`  | 🟢 |    |    |    |    |
//! | **Solaris** `sparc64` | 🟢 |    |    |    |    |
//!
//! # TODO
//!
//! ## Linux platforms
//! - Risc-V
//! - PowerPC
//! - aarch64
//! - Alpha
//! - S390 (IBM Z)
//! - SPARC
//! - MIPS
//!
//! ## macOS platforms
//! - i686
//! - PowerPC

use json::{
    files::{JavaFile, JavaFileDownload, JavaFilesJson},
    list::JavaListJson,
};
use owo_colors::OwoColorize;
use std::{
    env::consts::ARCH,
    path::{Path, PathBuf},
    sync::{Mutex, mpsc::Sender},
};
use thiserror::Error;
use tokio::fs;

use ql_core::{
    GenericProgress, IntoIoError, IoError, JsonDownloadError, JsonError, LAUNCHER_DIR,
    RequestError,
    constants::OS_NAME,
    do_jobs_with_limit, err,
    file_utils::{self, DirItem, canonicalize_a, exists, extract_tar_gz},
    info, pt,
};

pub use ql_core::JavaVersion;
mod alternate_java;
mod json;

#[allow(dead_code)]
const fn which_java() -> &'static str {
    #[cfg(target_os = "windows")]
    return "javaw";
    #[cfg(not(target_os = "windows"))]
    "java"
}

/// Which Java to use for GUI apps.
///
/// `javaw` on Windows, `java` on all other platforms.
///
/// On Windows, `javaw` is used to avoid accidentally opening
/// secondary terminal window. This uses the Windows subsystem
/// instead of the Console subsystem, so the OS treats it as
/// a GUI app.
pub const JAVA: &str = which_java();

/// Returns a `PathBuf` pointing to a Java executable of your choice.
///
/// This downloads and installs Java if not already installed,
/// and if already installed, uses the existing installation.
///
/// # Arguments
/// - `version`: The version of Java you want to use ([`JavaVersion`]).
/// - `name`: The name of the executable you want to use.
///   For example, "java" for the Java runtime, or "javac" for the Java compiler.
/// - `java_install_progress_sender`: An optional `Sender<GenericProgress>`
///   to send progress updates to. If not needed, simply pass `None` to the function.
///   If you want, you can hook this up to a progress bar, by using a
///   `std::sync::mpsc::channel::<JavaInstallMessage>()`,
///   giving the sender to this function and polling the receiver frequently.
///
/// # Errors
/// If the Java installation fails, this function returns a [`JavaInstallError`].
/// There's a lot of possible errors, so I'm not going to list them all here.
///
/// # Example
/// ```no_run
/// # async fn get() -> Result<(), Box<dyn std::error::Error>> {
/// use ql_java_handler::{get_java_binary, JavaVersion};
/// use std::path::PathBuf;
///
/// let java: PathBuf =
///     get_java_binary(JavaVersion::Java16, "java", None).await?;
///
/// let command =
///     std::process::Command::new(java).arg("-version").output()?;
///
/// // Another built-in Java tool
///
/// let java_compiler: PathBuf =
///     get_java_binary(JavaVersion::Java16, "javac", None).await?;
///
/// let command = std::process::Command::new(java_compiler)
///     .args(&["MyApp.java", "-d", "."])
///     .output()?;
/// # Ok(())
/// # }
/// ```
///
/// Java may be fetched either from Mojang or other sources
/// depending on platform (see crate-level docs for more info)
pub async fn get_java_binary(
    version: JavaVersion,
    name: &str,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
) -> Result<PathBuf, JavaInstallError> {
    let java_dir = LAUNCHER_DIR.join("java_installs").join(version.to_string());
    let is_incomplete_install = exists(java_dir.join("install.lock")).await;

    if !exists(&java_dir).await || is_incomplete_install {
        info!("Installing Java: {version}");
        install_java(version, java_install_progress_sender).await?;
    }

    let bin_path = find_java_bin_in_dir(name, &java_dir).await?;
    Ok(canonicalize_a(&bin_path).await)
}

/// Intelligently searches the given path for the given Java binary name, and returns a `PathBuf` to if found.
///
/// # Errors
/// - Java binary not found
/// - Path doesn't exist, or user lacks permissions
pub async fn find_java_bin_in_dir(name: &str, path: &Path) -> Result<PathBuf, JavaInstallError> {
    let metadata = fs::metadata(path).await.path(path)?;
    if metadata.is_file() {
        return Ok(path.to_owned());
    }

    let names = [
        name.to_owned(),
        format!("bin/{name}"),
        format!("Contents/Home/bin/{name}"),
        format!("jre.bundle/Contents/Home/bin/{name}"),
        format!("jdk1.8.0_231/{name}"),
        format!("jdk1.8.0_231/bin/{name}"),
        format!("jdk-21.0.10/bin/{name}"),
    ];

    for name in names {
        let bin_path = if cfg!(target_os = "windows") {
            path.join(format!("{name}.exe"))
        } else {
            path.join(&name)
        };
        if exists(&bin_path).await {
            return Ok(bin_path);
        }
    }

    let entries = file_utils::read_filenames_from_dir(path).await;
    if let Ok(entries) = entries.as_deref() {
        if let Some(entry) = entries
            .iter()
            .find(|n| n.name.contains("bellsoft") || n.name.contains("zulu"))
        {
            return Box::pin(find_java_bin_in_dir(name, &path.join(&entry.name))).await;
        }
    }

    Err(JavaInstallError::NoJavaBinFound {
        name: name.to_owned(),
        path: path.to_owned(),
        entries,
    })
}

#[cfg(target_os = "macos")]
const CONCURRENCY_LIMIT: usize = 16;
#[cfg(not(target_os = "macos"))]
const CONCURRENCY_LIMIT: usize = 64;

async fn install_java(
    version: JavaVersion,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
) -> Result<(), JavaInstallError> {
    let install_dir = get_install_dir(version).await?;
    let lock_file = lock_init(&install_dir).await?;

    send_progress(java_install_progress_sender, GenericProgress::default());

    let java_list_json = JavaListJson::download().await?;
    let Some(java_files_url) = java_list_json.get_url(version) else {
        // Mojang doesn't officially provide java for som platforms.
        // In that case, fetch from alternate sources.
        alternate_java::install(version, java_install_progress_sender, &install_dir).await?;
        lock_finish(&lock_file).await?;
        return Ok(());
    };

    let json: JavaFilesJson = file_utils::download_file_to_json(&java_files_url, false).await?;

    let num_files = json.files.len();
    let file_num = Mutex::new(0);

    _ = do_jobs_with_limit(
        json.files.iter().map(|(file_name, file)| {
            java_install_fn(
                java_install_progress_sender,
                &file_num,
                num_files,
                file_name,
                &install_dir,
                file,
            )
        }),
        CONCURRENCY_LIMIT,
    )
    .await?;

    lock_finish(&lock_file).await?;
    send_progress(java_install_progress_sender, GenericProgress::finished());
    info!("Finished installing {}", version.to_string());

    Ok(())
}

async fn lock_finish(lock_file: &Path) -> Result<(), IoError> {
    tokio::fs::remove_file(lock_file).await.path(lock_file)?;
    Ok(())
}

async fn lock_init(install_dir: &Path) -> Result<PathBuf, IoError> {
    let lock_file = install_dir.join("install.lock");
    tokio::fs::write(
        &lock_file,
        "If you see this, java hasn't finished installing.",
    )
    .await
    .path(lock_file.clone())?;
    Ok(lock_file)
}

async fn get_install_dir(version: JavaVersion) -> Result<PathBuf, JavaInstallError> {
    let java_installs_dir = LAUNCHER_DIR.join("java_installs");
    tokio::fs::create_dir_all(&java_installs_dir)
        .await
        .path(java_installs_dir.clone())?;
    let install_dir = java_installs_dir.join(version.to_string());
    tokio::fs::create_dir_all(&install_dir)
        .await
        .path(java_installs_dir.clone())?;
    Ok(install_dir)
}

fn send_progress(sender: Option<&Sender<GenericProgress>>, progress: GenericProgress) {
    if let Some(sender) = sender {
        _ = sender.send(progress);
    }
}

async fn java_install_fn(
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
    file_num: &Mutex<usize>,
    num_files: usize,
    file_name: &str,
    install_dir: &Path,
    file: &JavaFile,
) -> Result<(), JavaInstallError> {
    let file_path = install_dir.join(file_name);
    match file {
        JavaFile::file {
            downloads,
            executable,
        } => {
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await.path(parent)?;
            }
            let file_bytes = download_file(downloads).await?;
            tokio::fs::write(&file_path, &file_bytes)
                .await
                .path(file_path.clone())?;
            if *executable {
                #[cfg(target_family = "unix")]
                file_utils::set_executable(&file_path).await?;
            }
        }
        JavaFile::directory {} => {
            tokio::fs::create_dir_all(&file_path)
                .await
                .path(file_path)?;
        }
        JavaFile::link { .. } => {
            // TODO: Deal with java install symlink.
            // file_utils::create_symlink(src, dest)
        }
    }

    let file_num = {
        let mut file_num = file_num.lock().unwrap();
        send_progress(
            java_install_progress_sender,
            GenericProgress {
                done: *file_num,
                total: num_files,
                message: Some(format!("Installed file: {file_name}")),
                has_finished: false,
            },
        );
        *file_num += 1;
        *file_num
    } - 1;

    pt!(
        "{} ({file_num}/{num_files}): {file_name}",
        file.get_kind_name()
    );

    Ok(())
}

async fn download_file(downloads: &JavaFileDownload) -> Result<Vec<u8>, JavaInstallError> {
    async fn normal_download(downloads: &JavaFileDownload) -> Result<Vec<u8>, JavaInstallError> {
        Ok(file_utils::download_file_to_bytes(&downloads.raw.url, false).await?)
    }

    let Some(lzma) = &downloads.lzma else {
        return normal_download(downloads).await;
    };
    let mut lzma = std::io::BufReader::new(std::io::Cursor::new(
        file_utils::download_file_to_bytes(&lzma.url, false).await?,
    ));

    let mut out = Vec::new();
    match lzma_rs::lzma_decompress(&mut lzma, &mut out) {
        Ok(()) => Ok(out),
        Err(err) => {
            err!(
                "Could not decompress lzma file: {err}\n  ({})",
                downloads.raw.url.bright_black()
            );
            Ok(normal_download(downloads).await?)
        }
    }
}

const ERR_PREF1: &str = "while installing/managing Java (OS: ";
const UNSUPPORTED_MESSAGE: &str = r"Automatic Java installation isn’t supported on your platform for this Minecraft version.
You can:
- Install Java manually and set the executable path in the Instance → Edit tab
- Try an older Minecraft version
- Download the 64-bit launcher if you’re using the 32-bit version";

#[derive(Debug, Error)]
pub enum JavaInstallError {
    #[error("{ERR_PREF1}{OS_NAME} {ARCH}):\n{0}")]
    JsonDownload(#[from] JsonDownloadError),
    #[error("{ERR_PREF1}{OS_NAME} {ARCH}):\n{0}")]
    Request(#[from] RequestError),
    #[error("{ERR_PREF1}{OS_NAME} {ARCH}):\n{0}")]
    Json(#[from] JsonError),
    #[error("{ERR_PREF1}{OS_NAME} {ARCH}):\n{0}")]
    Io(#[from] IoError),
    #[error(
        r"{ERR_PREF1}{OS_NAME} {ARCH}):
couldn't find java binary ({name})
(this is a bug! please report on discord!)

at: {path:?}

{entries:?}"
    )]
    NoJavaBinFound {
        name: String,
        path: PathBuf,
        entries: Result<Vec<DirItem>, IoError>,
    },

    #[error("({OS_NAME} {ARCH})\n{UNSUPPORTED_MESSAGE}")]
    UnsupportedPlatform,

    #[error("{ERR_PREF1}{OS_NAME} {ARCH}):\nzip extract error:\n{0}")]
    ZipExtract(#[from] zip::result::ZipError),
    #[error("{ERR_PREF1}{OS_NAME} {ARCH}):\ncouldn't extract java tar.gz:\n{0}")]
    TarGzExtract(std::io::Error),
    #[error(
        "{ERR_PREF1}{OS_NAME} {ARCH}):\nunknown extension for java: {0}\n\nThis is a bug, please report on discord!"
    )]
    UnknownExtension(String),
}

/// Deletes all the auto-installed Java installations.
///
/// They are stored in `QuantumLauncher/java_installs/`
/// and are *completely cleared*. If you try to use
/// [`get_java_binary`] later, they will
/// *automatically get reinstalled*.
pub async fn delete_java_installs() {
    info!("Clearing Java installs");
    let java_installs = LAUNCHER_DIR.join("java_installs");
    if !exists(&java_installs).await {
        return;
    }
    if let Err(err) = tokio::fs::remove_dir_all(&java_installs).await {
        err!("Could not delete `java_installs` dir: {err}");
    }
}
