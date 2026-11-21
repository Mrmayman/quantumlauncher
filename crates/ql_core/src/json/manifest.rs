use std::sync::LazyLock;

use crate::{
    JsonDownloadError, download,
    json::{V_1_12_2, V_A_1_0_15},
};
use cfg_if::cfg_if;
use chrono::DateTime;
use serde::Deserialize;

static MANIFEST: LazyLock<tokio::sync::RwLock<Option<Manifest>>> =
    LazyLock::new(|| tokio::sync::RwLock::new(None));

const BETTERJSONS_BORKED: &[(&str, &str)] = &[
    ("26.2-pre1", "26.2-pre-1"),
    ("26.1-snap11", "26.1-snapshot-11"),
    ("26.1-snap10", "26.1-snapshot-10"),
    ("26.1-snap9", "26.1-snapshot-9"),
    ("26.1-snap8", "26.1-snapshot-8"),
];

/// An official Minecraft version manifest
/// (list of all versions and their download links)
#[derive(Deserialize, Clone, Debug)]
pub struct Manifest {
    latest: Latest,
    pub versions: Vec<ManifestVersion>,
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
        const ARM64: &str = "https://raw.githubusercontent.com/theofficialgman/piston-meta-arm64/refs/heads/main/mc/game/version_manifest_v2.json";
        const ARM32: &str = "https://raw.githubusercontent.com/theofficialgman/piston-meta-arm32/refs/heads/main/mc/game/version_manifest_v2.json";

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

        let old = download(OLDER_VERSIONS_JSON).uncached();
        let new = download(NEWER_VERSIONS_JSON).uncached();

        let (mut older_manifest, mut newer_manifest) =
            tokio::try_join!(old.json::<Self>(), new.json::<Self>())?;

        let last_lwjgl2 =
            DateTime::parse_from_rfc3339(V_1_12_2).expect("statically known to be valid");
        older_manifest.versions.retain(|n| {
            let is_already_lwjgl3 = n.releaseTime > last_lwjgl2;
            !(n.id.ends_with("-lwjgl3") && is_already_lwjgl3)
        });

        for &(borked, working) in BETTERJSONS_BORKED {
            let Some(working_version) = newer_manifest.find_name(working).cloned() else {
                continue;
            };
            let Some(slot) = older_manifest.versions.iter_mut().find(|n| n.id == borked) else {
                continue;
            };
            *slot = working_version;
        }

        if let Some(latest_time) = older_manifest.versions.first().map(|n| n.releaseTime) {
            older_manifest.versions.splice(
                0..0,
                take_versions_newer_than(&newer_manifest.versions, latest_time),
            );
        }

        Ok(older_manifest)
    }

    /// Looks up a version by its name.
    /// This searches for an *exact match*.
    #[must_use]
    pub fn find_name(&self, name: &str) -> Option<&ManifestVersion> {
        self.versions.iter().find(|n| n.id == name)
    }

    /// Gets the latest stable release
    ///
    /// This only returns a `None` if the .latest field's
    /// data is *wrong* (impossible normally, if you just
    /// [`Manifest::download`] it). So it's mostly safe
    /// to unwrap.
    #[must_use]
    pub fn get_latest_release(&self) -> Option<&ManifestVersion> {
        self.find_name(&self.latest.release)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Latest {
    release: String,
    // snapshot: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Clone, Debug)]
pub struct ManifestVersion {
    pub id: String,
    pub r#type: String,
    pub url: String,
    // time: String,
    pub releaseTime: DateTime<chrono::FixedOffset>,
}

impl ManifestVersion {
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
    #[allow(clippy::missing_panics_doc)] // will never panic
    pub fn supports_server(&self) -> bool {
        if !Self::guess_if_supports_server(&self.id) {
            return false;
        }

        if self.id.starts_with("a1.") {
            // Minecraft a1.0.15: Added multiplayer to alpha
            let a1_0_15 =
                DateTime::parse_from_rfc3339(V_A_1_0_15).expect("statically known to be valid");
            if self.releaseTime < a1_0_15 {
                return false;
            }
        }
        true
    }
}

fn take_versions_newer_than(
    vec: &[ManifestVersion],
    time: DateTime<chrono::FixedOffset>,
) -> Vec<ManifestVersion> {
    if let Some(pos) = vec.iter().position(|n| n.releaseTime <= time) {
        vec[..pos].to_vec()
    } else {
        vec.to_owned()
    }
}
