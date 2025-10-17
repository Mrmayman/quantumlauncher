use json::{
    files::{JavaFile, JavaFileDownload, JavaFilesJson},
    list::JavaListJson,
};
use std::{
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Mutex},
};
use thiserror::Error;

use ql_core::{
    constants::OS_NAME,
    do_jobs_with_limit, err,
    file_utils::{self, DirItem},
    info, pt, GenericProgress, IntoIoError, IoError, JsonDownloadError, JsonError, RequestError,
    LAUNCHER_DIR,
};

mod compression;
pub use compression::extract_tar_gz;

pub mod alternate_java;
mod json;

pub use json::list::JavaVersion;

#[cfg(target_os = "windows")]
pub const JAVA: &str = "javaw";
#[cfg(not(target_os = "windows"))]
pub const JAVA: &str = "java";

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
/// let java_binary: PathBuf = get_java_binary(JavaVersion::Java16, "java", None).await?;
///
/// let command = std::process::Command::new(java_binary).arg("-version").output()?;
///
/// let java_compiler_binary: PathBuf = get_java_binary(JavaVersion::Java16, "javac", None).await?;
///
/// let command = std::process::Command::new(java_compiler_binary)
///     .args(&["MyApp.java", "-d", "."])
///     .output()?;
/// # Ok(())
/// # }
/// ```
///
/// # Side notes
/// - On aarch64 linux, this function installs Amazon Corretto Java.
/// - On all other platforms, this function installs Java from Mojang.
pub async fn get_java_binary(
    mut version: JavaVersion,
    name: &str,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
) -> Result<PathBuf, JavaInstallError> {
    let java_dir = LAUNCHER_DIR.join("java_installs").join(version.to_string());
    let is_incomplete_install = java_dir.join("install.lock").exists();

    if cfg!(target_os = "windows") && cfg!(target_arch = "aarch64") {
        version = match version {
            // Java 8 and 16 are unsupported on Windows aarch64.

            // 17 should be backwards compatible with 8 and 16
            // for the most part, but some things like Beta ModLoader
            // might break?
            JavaVersion::Java8 | JavaVersion::Java16 | JavaVersion::Java17 => JavaVersion::Java17,
            JavaVersion::Java21 => JavaVersion::Java21,
        }
    }

    if !java_dir.exists() || is_incomplete_install {
        info!("Installing Java: {version}");
        install_java(version, java_install_progress_sender).await?;
    }

    let bin_path = find_java_bin(name, &java_dir).await?;
    Ok(bin_path.canonicalize().path(bin_path)?)
}

async fn find_java_bin(name: &str, java_dir: &Path) -> Result<PathBuf, JavaInstallError> {
    let names = [
        format!("bin/{name}"),
        format!("Contents/Home/bin/{name}"),
        format!("jre.bundle/Contents/Home/bin/{name}"),
        format!("jdk1.8.0_231/{name}"),
        format!("jdk1.8.0_231/bin/{name}"),
    ];

    for name in names {
        let path = java_dir.join(&name);
        if path.exists() {
            return Ok(path);
        }

        let path2 = java_dir.join(format!("{name}.exe"));
        if path2.exists() {
            return Ok(path2);
        }
    }

    let entries = file_utils::read_filenames_from_dir(java_dir).await;

    Err(JavaInstallError::NoJavaBinFound(entries))
}

async fn install_java(
    version: JavaVersion,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
) -> Result<(), JavaInstallError> {
    #[cfg(target_os = "macos")]
    const LIMIT: usize = 16;
    #[cfg(not(target_os = "macos"))]
    const LIMIT: usize = 64;

    let install_dir = get_install_dir(version).await?;
    let lock_file = lock_init(&install_dir).await?;

    send_progress(java_install_progress_sender, GenericProgress::default());

    let java_list_json = JavaListJson::download().await?;
    let Some(java_files_url) = java_list_json.get_url(version) else {
        // Mojang doesn't officially provide java for som platforms.
        // In that case, fetch from alternate sources.
        return alternate_java::install(version, java_install_progress_sender, &install_dir).await;
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
        LIMIT,
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

fn send_progress(
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
    progress: GenericProgress,
) {
    if let Some(java_install_progress_sender) = java_install_progress_sender {
        if let Err(err) = java_install_progress_sender.send(progress) {
            err!("Error sending java install progress: {err}\nThis should probably be safe to ignore");
        }
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
                "Could not decompress lzma file: {err} ({})",
                downloads.raw.url
            );
            Ok(normal_download(downloads).await?)
        }
    }
}

const ERR_PREF1: &str = "while installing Java (OS: ";

#[derive(Debug, Error)]
pub enum JavaInstallError {
    #[error("{ERR_PREF1}{OS_NAME}):\n{0}")]
    JsonDownload(#[from] JsonDownloadError),
    #[error("{ERR_PREF1}{OS_NAME}):\n{0}")]
    Request(#[from] RequestError),
    #[error("{ERR_PREF1}{OS_NAME}):\n{0}")]
    Json(#[from] JsonError),
    #[error("{ERR_PREF1}{OS_NAME}):\n{0}")]
    Io(#[from] IoError),
    #[error(
        "{ERR_PREF1}{OS_NAME}):\ncouldn't find java binary (this is a bug! please report on discord!)\n{0:?}"
    )]
    NoJavaBinFound(Result<Vec<DirItem>, IoError>),

    #[error("on your platform, only Java 8 (Minecraft 1.16.5 and below) is supported!\n")]
    UnsupportedOnlyJava8,
    #[error("Java auto-installation is not supported on your platform!\nPlease manually install Java,\nand add the executable path in instance Edit tab")]
    UnsupportedPlatform,

    #[error("{ERR_PREF1}{OS_NAME}):\nzip extract error:\n{0}")]
    ZipExtract(#[from] zip::result::ZipError),
    #[error("{ERR_PREF1}{OS_NAME}):\ncouldn't extract java tar.gz:\n{0}")]
    TarGzExtract(std::io::Error),
    #[error("{ERR_PREF1}{OS_NAME}):\nunknown extension for java: {0}\n\nThis is a bug, please report on discord!")]
    UnknownExtension(String),
}

pub async fn delete_java_installs() {
    info!("Clearing Java installs");
    let java_installs = LAUNCHER_DIR.join("java_installs");
    if !java_installs.exists() {
        return;
    }
    if let Err(err) = tokio::fs::remove_dir_all(&java_installs).await {
        err!("Could not delete `java_installs` dir: {err}");
    }
}
