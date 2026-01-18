use std::sync::LazyLock;

use crate::{err, file_utils, IntoJsonError, JsonDownloadError};
use cfg_if::cfg_if;
use chrono::DateTime;
use serde::Deserialize;

static MANIFEST: LazyLock<tokio::sync::RwLock<Option<Manifest>>> =
    LazyLock::new(|| tokio::sync::RwLock::new(None));

/// An official Minecraft version manifest
/// (list of all versions and their download links)
#[derive(Deserialize, Clone, Debug)]
pub struct Manifest {
    latest: Latest,
    pub versions: Vec<Version>,
}

impl Manifest {
    /// Downloads a complete manifest by combining:
    /// - A *curated, but outdated* manifest
    ///   ([BetterJSONs](https://mcphackers.org/BetterJSONs/version_manifest_v2.json)).
    /// - An *up-to-date but unpolished* manifest:
    ///   Platform-dependent URLs (see below)
    ///
    /// This ensures a consistent, high-quality manifest by preserving curated data
    /// for older versions (up to `1.21.11`) and appending newer versions
    /// from the official or forked manifests.
    ///
    /// # Platform-specific URLs
    /// - ARM64 linux: <https://raw.githubusercontent.com/theofficialgman/piston-meta-arm64/refs/heads/main/mc/game/version_manifest_v2.json>
    /// - ARM32 linux: <https://raw.githubusercontent.com/theofficialgman/piston-meta-arm32/refs/heads/main/mc/game/version_manifest_v2.json>
    /// - Other platforms: <https://launchermeta.mojang.com/mc/game/version_manifest_v2.json>
    ///
    /// # Errors
    /// Returns an error if either file cannot be downloaded or parsed into JSON.
    pub async fn download() -> Result<Manifest, JsonDownloadError> {
        if let Some(m) = MANIFEST.read().await.clone() {
            return Ok(m);
        }
        let manifest = Self::load().await?;
        *MANIFEST.write().await = Some(manifest.clone());
        Ok(manifest)
    }

    #[allow(unused)]
    async fn load() -> Result<Manifest, JsonDownloadError> {
        const ARM64: &str =
            "https://raw.githubusercontent.com/theofficialgman/piston-meta-arm64/refs/heads/main/mc/game/version_manifest_v2.json";
        const ARM32: &str =
            "https://raw.githubusercontent.com/theofficialgman/piston-meta-arm32/refs/heads/main/mc/game/version_manifest_v2.json";

        const LAST_BETTERJSONS: &str = "26.1-snapshot-1";

        // An out-of-date but curated manifest
        const OLDER_VERSIONS_JSON: &str =
            "https://mcphackers.org/BetterJSONs/version_manifest_v2.json";

        // An up-to-date manifest that lacks some fixes/polish
        cfg_if!(if #[cfg(feature = "simulate_linux_arm64")] { use ARM64 as NEWER_VERSIONS_JSON;
        } else if #[cfg(feature = "simulate_linux_arm32")] { use ARM32 as NEWER_VERSIONS_JSON;
        } else if #[cfg(all(target_os = "linux", target_arch = "aarch64"))] { use ARM64 as NEWER_VERSIONS_JSON;
        } else if #[cfg(all(target_os = "linux", target_arch = "arm"))] { use ARM32 as NEWER_VERSIONS_JSON;
        } else {
            const NEWER_VERSIONS_JSON: &str =
                "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
        });

        let (older_manifest, newer_manifest) = tokio::try_join!(
            file_utils::download_file_to_string(OLDER_VERSIONS_JSON, false),
            file_utils::download_file_to_string(NEWER_VERSIONS_JSON, false)
        )?;
        let mut older_manifest: Self =
            serde_json::from_str(&older_manifest).json(older_manifest)?;
        let newer_manifest: Self = serde_json::from_str(&newer_manifest).json(newer_manifest)?;

        // Removes newer versions from out-of-date manifest
        // if it ever gets updated, to not mess up the list.
        older_manifest.versions =
            exclude_versions_after(&older_manifest.versions, |n| n.id == LAST_BETTERJSONS);
        // Add newer versions (that lack fixes/polish) to the manifest
        older_manifest.versions.splice(
            0..0,
            include_versions_after(&newer_manifest.versions, |n| n.id == LAST_BETTERJSONS),
        );

        Ok(older_manifest)
    }

    /// Looks up a version by its name.
    /// This searches for an *exact match*.
    #[must_use]
    pub fn find_name(&self, name: &str) -> Option<&Version> {
        self.versions.iter().find(|n| n.id == name)
    }

    /// Gets the latest stable release
    ///
    /// This only returns a `None` if the .latest field's
    /// data is *wrong* (impossible normally, if you just
    /// [`Manifest::download`] it). So it's mostly safe
    /// to unwrap.
    #[must_use]
    pub fn get_latest_release(&self) -> Option<&Version> {
        self.find_name(&self.latest.release)
    }

    /// Gets the latest snapshot (experimental) release.
    ///
    /// This only returns a `None` if the .latest field's
    /// data is *wrong* (impossible normally, if you just
    /// [`Manifest::download`] it). So it's mostly safe
    /// to unwrap.
    #[must_use]
    pub fn get_latest_snapshot(&self) -> Option<&Version> {
        self.find_name(&self.latest.snapshot)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Clone, Debug)]
pub struct Version {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub releaseTime: String,
}

impl Version {
    #[must_use]
    pub fn guess_if_supports_server(id: &str) -> bool {
        if id.starts_with("inf-") || id.starts_with("in-") || id.starts_with("pc-") {
            return false;
        }
        if let Some(name) = id.strip_prefix("c0.") {
            if name.contains("_st") || name.contains("-s") {
                return false;
            }
            if name.starts_with("0.11")
                || name.starts_with("0.12")
                || name.starts_with("0.13")
                || name.starts_with("0.14")
                || name.starts_with("0.15")
            {
                return false;
            }
        }
        true
    }

    #[must_use]
    pub fn supports_server(&self) -> bool {
        if !Self::guess_if_supports_server(&self.id) {
            return false;
        }

        if self.id.starts_with("a1.") {
            // Minecraft a1.0.15: Added multiplayer to alpha
            let a1_0_15 = DateTime::parse_from_rfc3339("2010-08-03T19:47:25+00:00").unwrap();
            match DateTime::parse_from_rfc3339(&self.releaseTime) {
                Ok(dt) => {
                    if dt < a1_0_15 {
                        return false;
                    }
                }
                Err(e) => {
                    err!("Could not parse instance date/time: {e}");
                }
            }
        }
        true
    }
}

fn exclude_versions_after<T, F>(vec: &[T], predicate: F) -> Vec<T>
where
    T: Clone,
    F: FnMut(&T) -> bool,
{
    if let Some(pos) = vec.iter().position(predicate) {
        vec[pos..].to_vec()
    } else {
        Vec::new()
    }
}

fn include_versions_after<T, F>(vec: &[T], predicate: F) -> Vec<T>
where
    T: Clone,
    F: FnMut(&T) -> bool,
{
    if let Some(pos) = vec.iter().position(predicate) {
        vec[..pos].to_vec()
    } else {
        vec.to_owned()
    }
}
