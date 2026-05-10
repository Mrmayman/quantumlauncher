//! # A crate for managing Minecraft servers
//!
//! **Not recommended to use this in your own projects!**
//!
//! This is a crate of
//! [Quantum Launcher](https://mrmayman.github.io/quantumlauncher)
//! for managing Minecraft servers.

use std::path::PathBuf;

use ql_core::{IoError, JsonError, RequestError, impl_3_errs_jri};
use ql_java_handler::JavaInstallError;

mod create;
mod run;
mod server_properties;
// mod ssh;
pub use create::{create_server, delete_server};
pub use run::run;
pub use server_properties::ServerProperties;
// pub use ssh::run_tunnel;

use thiserror::Error;

const SERVER_ERR_PREFIX: &str = "while managing server:\n";

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("{SERVER_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("while downloading server\nserver version not found in manifest: {0}")]
    VersionNotFoundInManifest(String),
    #[error("{SERVER_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{SERVER_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{SERVER_ERR_PREFIX}{0}")]
    JavaInstall(#[from] JavaInstallError),
    #[error(
        "{SERVER_ERR_PREFIX}couldn't find download field:\n(details.json).downloads.server is null"
    )]
    NoServerDownload,
    #[error("server name is invalid (empty/disallowed characters)")]
    InvalidName,
    #[error("A server with that name already exists!")]
    ServerAlreadyExists,
    #[error("{SERVER_ERR_PREFIX}zip extract error:\n{0}")]
    ZipExtract(#[from] zip::result::ZipError),
    #[error("{SERVER_ERR_PREFIX}couldn't find forge shim file")]
    NoForgeShimFound,
    #[error("{SERVER_ERR_PREFIX}couldn't convert PathBuf to str: {0:?}")]
    PathBufToStr(PathBuf),
}

impl_3_errs_jri!(ServerError, Json, Request, Io);

// Below is for historical purposes, if anyone's interested

/*fn convert_classic_to_real_name(classic: &str) -> &str {
    let Some(classic) = classic.strip_prefix("classic/c") else {
        return classic;
    };
    match classic {
        "1.2" => "classic/c0.0.16a",
        "1.3" => "classic/c0.0.17a",
        "1.4-1327" => "classic/c0.0.18a, c0.0.18a_01 (1)",
        "1.4-1422" => "classic/c0.0.18a, c0.0.18a_01 (2)",
        "1.4.1" => "classic/c0.0.18a_02",
        "1.5" => "classic/c0.0.19a - c0.0.19a_03",
        "1.6" => "classic/c0.0.19a_04 - c0.0.19a_06",
        "1.8" => "classic/c0.0.20a (1)",
        "1.8.1" => "classic/c0.0.20a (2)",
        "1.8.2" => "classic/c0.0.20a_01 - c0.0.23a",
        "1.8.3" | "1.9" => "classic/c0.28",
        "1.9.1" => "classic/c0.29",
        "1.10" => "classic/c0.30 (1)",
        "1.10.1" => "classic/c0.30 (2)",
        _ => classic,
    }
}

fn convert_alpha_to_real_name(alpha: &str) -> &str {
    let Some(alpha) = alpha.strip_prefix("alpha/a") else {
        return alpha;
    };
    match alpha {
        "0.1.0" => "alpha/a1.0.15",
        "0.1.1-1707" => "alpha/a1.0.16",
        "0.1.2_01" => "alpha/a1.0.16_01",
        "0.1.3" => "alpha/a1.0.16_02",
        "0.1.4" => "alpha/a1.0.17",
        "0.2.0" => "alpha/a1.1.0 (1)",
        "0.2.0_01" => "alpha/a1.1.0 (2)",
        "0.2.1" => "alpha/a1.1.1, a1.1.2",
        "0.2.2" => "alpha/a1.2.0",
        "0.2.2_01" => "alpha/a1.2.0_01, a1.2.0_02",
        "0.2.3" => "alpha/a1.2.1",
        "0.2.4" => "alpha/a1.2.2",
        "0.2.5-1004" => "alpha/a1.2.3, a1.2.3_01 (1)",
        "0.2.5-0923" => "alpha/a1.2.3, a1.2.3_01 (2)",
        "0.2.5_01" => "alpha/a1.2.3_02",
        "0.2.5_02" => "alpha/a1.2.3_04",
        "0.2.6" => "alpha/a1.2.3_05, a1.2.4 (1)",
        "0.2.6_01" => "alpha/a1.2.3_05, a1.2.4 (2)",
        "0.2.6_02" => "alpha/a1.2.4_01",
        "0.2.7" => "alpha/a1.2.5",
        "0.2.8" => "alpha/a1.2.6",
        _ => alpha,
    }
}*/
