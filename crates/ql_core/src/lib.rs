//! Core utilities for
//! [Quantum Launcher](https://mrmayman.github.io/quantumlauncher)
//! used by the various crates.
//!
//! **Not recommended to use in your own projects!**
//!
//! # Contains
//! - Java auto-installer
//! - File and download utilities
//! - Error types
//! - JSON structs for version, instance config, Fabric, Forge, Optifine, etc.
//! - Logging macros
//! - And much more

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::missing_errors_doc)]

mod error;
/// Common utilities for working with files.
pub mod file_utils;
pub mod jarmod;
/// JSON structs for version, instance config, Fabric, Forge, Optifine, Quilt, Neoforge, etc.
pub mod json;
mod loader;
/// Logging macros.
pub mod print;
mod progress;
mod urlcache;

use futures::StreamExt;
use json::VersionDetails;
use std::{
    fmt::{Debug, Display},
    future::Future,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub use error::{
    DownloadFileError, IntoIoError, IntoJsonError, IntoStringError, IoError, JsonDownloadError,
    JsonError, JsonFileError,
};
pub use file_utils::{RequestError, LAUNCHER_DIR};
pub use loader::Loader;
pub use print::{logger_finish, LogType, LoggingState, LOGGER};
pub use progress::{DownloadProgress, GenericProgress, Progress};
pub use urlcache::url_cache_get;

use regex::Regex;

pub static REGEX_SNAPSHOT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d{2}w\d*[a-zA-Z]+").unwrap());

pub const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

pub const WEBSITE: &str = "https://mrmayman.github.io/quantumlauncher";

/// To prevent spawning of terminal (windows only).
///
/// Takes in a `Command` (owned or mutable reference, both are fine).
/// This supports `process::Command` of both `tokio` and `std`.
#[macro_export]
macro_rules! no_window {
    ($cmd:expr) => {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            // 0x08000000 => CREATE_NO_WINDOW
            $cmd.creation_flags(0x08000000);
        }
    };
}

pub static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// Executes multiple async tasks concurrently (e.g., downloading files).
///
/// # Calling
///
/// - Takes in `Iterator` over `Future` (the thing returned by `async fn -> Result<T, E>`).
/// - Returns `Result<Vec<T>, E>`.
///
/// The entire operation fails if any task fails.
///
/// # Example
/// ```no_run
/// # use ql_core::do_jobs;
/// # async fn download_file(url: &str) -> Result<String, String> {
/// #     Ok("Hello".to_owned())
/// # }
/// # async fn trying() -> Result<(), String> {
/// #   let files: [&str; 1] = ["test"];
/// do_jobs(files.iter().map(|url| {
///     // Async function that returns Result<T, E>
///     // No need to await
///     download_file(url)
/// })).await?;
/// #   Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns whatever error the input function returns.
pub async fn do_jobs<T, E>(
    results: impl Iterator<Item = impl Future<Output = Result<T, E>>>,
) -> Result<Vec<T>, E> {
    #[cfg(target_os = "macos")]
    const JOBS: usize = 32;
    #[cfg(not(target_os = "macos"))]
    const JOBS: usize = 64;
    do_jobs_with_limit(results, JOBS).await
}

/// Executes multiple async tasks concurrently (e.g., downloading files),
/// with an **explicit limit** on concurrent jobs.
///
/// # Calling
///
/// - Takes in `Iterator` over `Future` (the thing returned by `async fn -> Result<T, E>`).
/// - Returns `Result<Vec<T>, E>`.
///
/// The entire operation fails if any task fails.
///
/// This function allows you to set an explicit
/// limit on how many jobs can run at the same time,
/// so you can stay under any `ulimit -n` file descriptor
/// limits.
///
/// # Example
/// ```no_run
/// # use ql_core::do_jobs_with_limit;
/// # async fn download_file(url: &str) -> Result<String, String> {
/// #     Ok("Hello".to_owned())
/// # }
/// # async fn trying() -> Result<(), String> {
/// #   let files: [&str; 1] = ["test"];
/// do_jobs_with_limit(files.iter().map(|url| {
///     // Async function that returns Result<T, E>
///     // No need to await
///     download_file(url)
/// }), 64).await?; // up to 64 jobs at the same time
/// #   Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns whatever error the input function returns.
pub async fn do_jobs_with_limit<T, E>(
    results: impl Iterator<Item = impl Future<Output = Result<T, E>>>,
    limit: usize,
) -> Result<Vec<T>, E> {
    let mut tasks = futures::stream::FuturesUnordered::new();
    let mut outputs = Vec::new();

    for result in results {
        tasks.push(result);
        if tasks.len() > limit {
            if let Some(task) = tasks.next().await {
                outputs.push(task?);
            }
        }
    }

    while let Some(task) = tasks.next().await {
        outputs.push(task?);
    }
    Ok(outputs)
}

/// Retries a non-deterministic function up to 5 times if it fails.
///
/// Useful for inherently unreliable operations (e.g., network requests) that may
/// fail intermittently, reducing the overall failure rate by retrying.
/// Maybe we might get lucky and get it working the second time, or the third...
///
/// # Calling
/// Accepts a closure that returns a `Future`
/// (the thing that async functions return) of `Result<T, E>`.
///
/// # Example
/// ```no_run
/// # use ql_core::retry;
/// async fn download_file(url: &str) -> Result<String, String> {
///     // Insert network operation here
///     Ok("Hi".to_owned())
/// }
/// # async fn download_something_important() -> Result<String, String> {
/// retry(|| download_file("example.com/my_file")).await
/// # }
/// ```
///
/// Notice how we don't await on `download_file`? Here's another one.
///
/// ```no_run
/// // Use this pattern for inline async blocks
/// retry(|| async move {
///     download_file("example.com/my_file").await;
///     download_file("example.com/another_file").await;
/// }).await
/// ```
///
/// # Errors
/// Returns whatever error the original function returned.
pub async fn retry<T, E, Res, Func>(f: Func) -> Result<T, E>
where
    Res: Future<Output = Result<T, E>>,
    Func: Fn() -> Res,
{
    const LIMIT: usize = 5;
    let mut result = f().await;
    for _ in 0..LIMIT {
        if result.is_ok() {
            break;
        }
        result = f().await;
    }
    result
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum InstanceSelection {
    Instance(String),
    Server(String),
}

impl InstanceSelection {
    #[must_use]
    pub fn new(name: &str, is_server: bool) -> Self {
        if is_server {
            Self::Server(name.to_owned())
        } else {
            Self::Instance(name.to_owned())
        }
    }

    /// Gets the path where launcher-specific things are stored.
    ///
    /// - Instances: `QuantumLauncher/instances/<NAME>/`
    /// - Servers: `QuantumLauncher/servers/<Name>/` (identical to `dot_minecraft_path`)
    #[must_use]
    pub fn get_instance_path(&self) -> PathBuf {
        match self {
            Self::Instance(name) => LAUNCHER_DIR.join("instances").join(name),
            Self::Server(name) => LAUNCHER_DIR.join("servers").join(name),
        }
    }

    /// Gets the path where files used by the game itself are stored,
    /// also called the `.minecraft` folder.
    ///
    /// - Instances: `QuantumLauncher/instances/<NAME>/.minecraft/`
    /// - Servers: `QuantumLauncher/servers/<NAME>/` (identical to `instance_path`)
    #[must_use]
    pub fn get_dot_minecraft_path(&self) -> PathBuf {
        match self {
            InstanceSelection::Instance(name) => {
                LAUNCHER_DIR.join("instances").join(name).join(".minecraft")
            }
            InstanceSelection::Server(name) => LAUNCHER_DIR.join("servers").join(name),
        }
    }

    #[must_use]
    pub fn get_name(&self) -> &str {
        match self {
            Self::Instance(name) | Self::Server(name) => name,
        }
    }

    #[must_use]
    pub fn is_server(&self) -> bool {
        matches!(self, Self::Server(_))
    }

    pub fn set_name(&mut self, name: &str) {
        match self {
            Self::Instance(ref mut n) | Self::Server(ref mut n) => name.clone_into(n),
        }
    }

    #[must_use]
    pub fn get_pair(&self) -> (&str, bool) {
        (self.get_name(), self.is_server())
    }
}

/// An enum representing a Minecraft version.
///
/// # Fields
/// - `name`: The name of the version according to
///   [BetterJSONs](https://mcphackers.org/BetterJSONs/version_manifest_v2.json)
/// - `is_classic_server`: Whether it is a Minecraft Classic entry
#[derive(Debug, Clone)]
pub struct ListEntry {
    pub name: String,
    pub is_classic_server: bool,
}

impl Display for ListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub const LAUNCHER_VERSION_NAME: &str = "0.4.2";

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModId {
    Modrinth(String),
    Curseforge(String),
}

impl ModId {
    #[must_use]
    pub fn get_internal_id(&self) -> &str {
        match self {
            ModId::Modrinth(n) | ModId::Curseforge(n) => n,
        }
    }

    #[must_use]
    pub fn get_index_str(&self) -> String {
        match self {
            ModId::Modrinth(n) => n.clone(),
            ModId::Curseforge(n) => format!("CF:{n}"),
        }
    }

    #[must_use]
    pub fn from_index_str(n: &str) -> Self {
        if n.starts_with("CF:") {
            ModId::Curseforge(n.strip_prefix("CF:").unwrap_or(n).to_owned())
        } else {
            ModId::Modrinth(n.to_owned())
        }
    }

    #[must_use]
    pub fn from_pair(n: &str, t: StoreBackendType) -> Self {
        match t {
            StoreBackendType::Modrinth => Self::Modrinth(n.to_owned()),
            StoreBackendType::Curseforge => Self::Curseforge(n.to_owned()),
        }
    }

    #[must_use]
    pub fn to_pair(self) -> (String, StoreBackendType) {
        let backend = match self {
            ModId::Modrinth(_) => StoreBackendType::Modrinth,
            ModId::Curseforge(_) => StoreBackendType::Curseforge,
        };

        (self.get_internal_id().to_owned(), backend)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackendType {
    Modrinth,
    Curseforge,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum SelectedMod {
    Downloaded { name: String, id: ModId },
    Local { file_name: String },
}

impl SelectedMod {
    #[must_use]
    pub fn from_pair(name: String, id: Option<ModId>) -> Self {
        match id {
            Some(id) => Self::Downloaded { name, id },
            None => Self::Local { file_name: name },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OptifineUniqueVersion {
    V1_5_2,
    V1_2_5,
    B1_7_3,
    B1_6_6,
}

impl OptifineUniqueVersion {
    #[must_use]
    pub async fn get(instance: &InstanceSelection) -> Option<Self> {
        VersionDetails::load(instance)
            .await
            .ok()
            .and_then(|n| match n.get_id() {
                "1.5.2" => Some(OptifineUniqueVersion::V1_5_2),
                "1.2.5" => Some(OptifineUniqueVersion::V1_2_5),
                "b1.7.3" => Some(OptifineUniqueVersion::B1_7_3),
                "b1.6.6" => Some(OptifineUniqueVersion::B1_6_6),
                _ => None,
            })
    }

    #[must_use]
    pub fn get_url(&self) -> (&'static str, bool) {
        match self {
            OptifineUniqueVersion::V1_5_2 => ("https://optifine.net/adloadx?f=OptiFine_1.5.2_HD_U_D5.zip", false),
            OptifineUniqueVersion::V1_2_5 => ("https://optifine.net/adloadx?f=OptiFine_1.5.2_HD_U_D2.zip", false),
            OptifineUniqueVersion::B1_7_3 => ("https://b2.mcarchive.net/file/mcarchive/47df260a369eb2f79750ec24e4cfd9da93b9aac076f97a1332302974f19e6024/OptiFine_1_7_3_HD_G.zip", true),
            OptifineUniqueVersion::B1_6_6 => ("https://optifine.net/adloadx?f=beta_OptiFog_Optimine_1.6.6.zip", false),
        }
    }
}

pub fn get_jar_path(
    version_json: &VersionDetails,
    instance_dir: &Path,
    optifine_jar: Option<&Path>,
    custom_jar: Option<&str>,
) -> PathBuf {
    fn get_path_from_id(instance_dir: &Path, id: &str) -> PathBuf {
        instance_dir
            .join(".minecraft/versions")
            .join(id)
            .join(format!("{id}.jar"))
    }

    if let Some(custom_jar_path) = custom_jar {
        if !custom_jar_path.trim().is_empty() {
            return LAUNCHER_DIR.join("custom_jars").join(custom_jar_path);
        }
    }

    optifine_jar.map_or_else(
        || {
            let id = version_json.get_id();
            let path1 = get_path_from_id(instance_dir, id);
            if path1.exists() {
                path1
            } else {
                get_path_from_id(instance_dir, &version_json.id)
            }
        },
        Path::to_owned,
    )
}

/// Find the launch jar file for a Forge server.
/// The name is `forge-*-shim.jar`, this performs a search for it.
pub async fn find_forge_shim_file(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }

    file_utils::find_item_in_dir(dir, |path, name| {
        path.is_file() && name.starts_with("forge-") && name.ends_with("-shim.jar")
    })
    .await
    .ok()
    .flatten()
}
