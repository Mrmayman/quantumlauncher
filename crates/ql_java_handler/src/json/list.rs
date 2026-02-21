use crate::file_utils;
use cfg_if::cfg_if;
use ql_core::JavaVersion;
use serde::Deserialize;

use crate::JsonDownloadError;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub struct JavaListJson {
    // gamecore: JavaList,
    linux: JavaList,
    linux_i386: JavaList,
    mac_os: JavaList,
    mac_os_arm64: JavaList,
    windows_arm64: JavaList,
    windows_x86: JavaList,
    windows_x64: JavaList,
}

impl JavaListJson {
    pub async fn download() -> Result<Self, JsonDownloadError> {
        pub const JAVA_LIST_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
        file_utils::download_file_to_json(JAVA_LIST_URL, false).await
    }

    fn get_platform(&self) -> Option<&JavaList> {
        cfg_if!(if #[cfg(any(feature = "simulate_linux_arm64", feature = "simulate_linux_arm32"))] {
            return None;
        } else if #[cfg(any(feature = "simulate_macos_arm64", all(
            target_os = "macos", target_arch = "aarch64"
        )))] {
            return Some(&self.mac_os_arm64);
        } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
            return Some(&self.linux);
        } else if #[cfg(all(target_os = "linux", target_arch = "x86"))] {
            return Some(&self.linux_i386);
        } else if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
            return Some(&self.mac_os);
        } else if #[cfg(all(target_os = "windows", target_arch = "aarch64"))] {
            return Some(&self.windows_arm64);
        } else if #[cfg(all(target_os = "windows", target_arch = "x86_64"))] {
            return Some(&self.windows_x64);
        } else if #[cfg(all(target_os = "windows", target_arch = "x86"))] {
            return Some(&self.windows_x86);
        });
        #[allow(unreachable_code)]
        return None;
    }

    pub fn get_url(&self, mut version: JavaVersion) -> Option<String> {
        let java_list = self.get_platform()?;
        let mut fetched = read_ver_from_list(version, java_list);
        while fetched.is_none() {
            version = version.next()?;
            fetched = read_ver_from_list(version, java_list);
        }
        Some(fetched?.manifest.url.clone())
    }
}

fn read_ver_from_list(version: JavaVersion, java_list: &JavaList) -> Option<&JavaInstallListing> {
    let version_listing = match version {
        JavaVersion::Java16 => &java_list.java_runtime_alpha,
        JavaVersion::Java17 => {
            if !java_list.java_runtime_gamma.is_empty() {
                &java_list.java_runtime_gamma
            } else if !java_list.java_runtime_gamma_snapshot.is_empty() {
                &java_list.java_runtime_gamma_snapshot
            } else {
                &java_list.java_runtime_beta
            }
        }
        JavaVersion::Java21 => &java_list.java_runtime_delta,
        JavaVersion::Java25 => &java_list.java_runtime_epsilon,
        JavaVersion::Java8 => &java_list.jre_legacy,
    };
    version_listing.first()
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct JavaList {
    /// Java 16.0.1.9.1
    java_runtime_alpha: Vec<JavaInstallListing>,
    /// Java 17.0.1.12.1
    java_runtime_beta: Vec<JavaInstallListing>,
    /// Java 21.0.3
    java_runtime_delta: Vec<JavaInstallListing>,
    /// Java 17.0.8
    java_runtime_gamma: Vec<JavaInstallListing>,
    /// Java 17.0.8
    java_runtime_gamma_snapshot: Vec<JavaInstallListing>,
    /// Java 25.0.1
    java_runtime_epsilon: Vec<JavaInstallListing>,
    /// Java 8
    jre_legacy: Vec<JavaInstallListing>,
    // Ugly windows specific thing that doesn't seem to be required?
    // minecraft_java_exe: Vec<JavaInstallListing>,
}

#[derive(Deserialize, Debug)]
pub struct JavaInstallListing {
    // availability: JavaInstallListingAvailability,
    manifest: JavaInstallListingManifest,
    // version: JavaInstallListingVersion,
}

// WTF: Yes this is approaching Java levels of name length.
// #[derive(Deserialize, Debug)]
// pub struct JavaInstallListingAvailability {
// group: i64,
// progress: i64,
// }

#[derive(Deserialize, Debug)]
pub struct JavaInstallListingManifest {
    // sha1: String,
    // size: usize,
    url: String,
}

// #[derive(Deserialize, Debug)]
// pub struct JavaInstallListingVersion {
// name: String,
// released: String,
// }
