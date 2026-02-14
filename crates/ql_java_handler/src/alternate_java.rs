//! A module to install Java from various third party sources
//! if Mojang doesn't provide Java for your specific platform.

use std::{
    env::consts::{ARCH, OS},
    io::Cursor,
    path::Path,
    sync::mpsc::Sender,
};

use cfg_if::cfg_if;
use owo_colors::OwoColorize;
use ql_core::{file_utils, pt, GenericProgress, JavaVersion};
use serde::Deserialize;

use crate::{extract_tar_gz, send_progress, JavaInstallError};

pub(crate) async fn install(
    version: JavaVersion,
    sender: Option<&Sender<GenericProgress>>,
    install_dir: &Path,
) -> Result<(), JavaInstallError> {
    let Some(url) = get_url(version).await? else {
        return Err(JavaInstallError::UnsupportedPlatform);
    };

    progress(sender, "Getting compressed archive", 0);
    pt!("URL: {}", url.bright_black());
    let file_bytes = file_utils::download_file_to_bytes(&url, false).await?;

    progress(sender, "Extracting archive", 1);
    if url.ends_with("tar.gz") {
        extract_tar_gz(&file_bytes, install_dir).map_err(JavaInstallError::TarGzExtract)?;
    } else if url.ends_with("zip") {
        file_utils::extract_zip_archive(Cursor::new(file_bytes), &install_dir, true).await?;
    } else {
        return Err(JavaInstallError::UnknownExtension(url.to_owned()));
    }
    Ok(())
}

fn progress(sender: Option<&Sender<GenericProgress>>, msg: &str, done: usize) {
    pt!("{msg}");
    send_progress(
        sender,
        GenericProgress {
            done,
            total: 2,
            message: Some(msg.to_owned()),
            has_finished: false,
        },
    );
}

async fn get_url(mut version: JavaVersion) -> Result<Option<String>, JavaInstallError> {
    #[cfg(all(target_os = "freebsd", target_arch = "x86_64"))]
    if let JavaVersion::Java8 = version {
        return Ok(Some("https://github.com/Mrmayman/get-jdk/releases/download/java8-1/jdk-8u452-freebsd-x64.tar.gz".to_owned()));
    }
    if let JavaVersion::Java21 = version {
        if cfg!(any(
            feature = "simulate_linux_arm32",
            all(target_os = "linux", target_arch = "arm")
        )) {
            return Ok(Some("https://download.bell-sw.com/java/21.0.10+10/bellsoft-jdk21.0.10+10-linux-arm32-vfp-hflt.tar.gz".to_owned()));
        } else if cfg!(target_arch = "x86") {
            if cfg!(target_os = "windows") {
                return Ok(Some("https://download.bell-sw.com/java/21.0.10+10/bellsoft-jdk21.0.10+10-windows-i586.zip".to_owned()));
            } else if cfg!(target_os = "linux") {
                return Ok(Some("https://download.bell-sw.com/java/21.0.10+10/bellsoft-jdk21.0.10+10-linux-i586.tar.gz".to_owned()));
            }
        }
    }

    let mut res = get_inner(version).await?;
    while let (true, Some(next)) = (res.is_none(), version.next()) {
        // Try newer javas if older ones aren't there
        version = next;
        res = get_inner(version).await?;
    }
    return Ok(res);
}

#[derive(Deserialize)]
struct ZuluOut {
    latest: bool,
    download_url: String,
}

async fn get_inner(version: JavaVersion) -> Result<Option<String>, JavaInstallError> {
    let os = get_os();
    let arch = get_arch();

    let mut url = format!(
        "https://api.azul.com/metadata/v1/zulu/packages?java_version={version}&os={os}&arch={arch}&page_size=1000",
        version = version as usize
    );
    if let JavaVersion::Java21 = version {
        // For optifine
        url.push_str("&java_package_type=jdk");
    }
    pt!("Fetching URL: {}", url.bright_black());
    let json: Vec<ZuluOut> = file_utils::download_file_to_json(&url, true).await?;
    let java = find_with_extension(&json, ".zip").or_else(|| find_with_extension(&json, ".tar.gz"));
    Ok(java.map(|n| n.download_url.clone()))
}

fn find_with_extension<'a>(json: &'a [ZuluOut], ext: &str) -> Option<&'a ZuluOut> {
    let ext = |n: &&ZuluOut| n.download_url.ends_with(ext);
    json.iter()
        .filter(ext)
        .find(|n| n.latest)
        .or_else(|| json.iter().find(ext))
}

fn get_os() -> &'static str {
    cfg_if!(if #[cfg(any(
        feature = "simulate_linux_arm32",
        feature = "simulate_linux_arm64",
    ))] {
        return "linux-glibc";
    } else if #[cfg(feature = "simulate_macos_arm64")] {
        return "macos"
    } else if #[cfg(all(target_os = "linux", target_env = "gnu"))] {
        return "linux-glibc";
    } else if #[cfg(all(target_os = "linux", target_env = "musl"))] {
        return "linux-musl";
    });
    #[allow(unreachable_code)]
    OS
}

fn get_arch() -> &'static str {
    cfg_if!(if #[cfg(feature = "simulate_linux_arm32")] {
        return "aarch32hf";
    } else if #[cfg(any(feature = "simulate_linux_arm64", feature = "simulate_macos_arm64"))] {
        return "aarch64";
    } else if #[cfg(target_arch = "arm")] {
        return "aarch32hf";
    } else if #[cfg(target_arch = "x86")] {
        return "i686";
    } else if #[cfg(all(target_arch = "sparc64", target_os = "solaris"))] {
        return "sparcv9-64";
    });
    #[allow(unreachable_code)]
    ARCH
}
