use std::fmt::Display;

use ql_core::{
    file_utils, info,
    json::{VersionDetails, V_OFFICIAL_FABRIC_SUPPORT},
    pt, InstanceSelection, IntoJsonError, JsonDownloadError, RequestError,
};
use serde::Deserialize;

use crate::loaders::fabric::{download_file_to_string, FabricInstallError};

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct FabricVersionListItem {
    pub loader: FabricVersion,
}

impl Display for FabricVersionListItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.loader.version)
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct FabricVersion {
    // pub separator: String,
    // pub build: usize,
    // pub maven: String,
    pub version: String,
    // pub stable: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackendType {
    Fabric,
    Quilt,
    LegacyFabric,
    OrnitheMCFabric,
    OrnitheMCQuilt,
    CursedLegacy,
    Babric,
}

impl BackendType {
    #[must_use]
    pub fn get_url(self) -> &'static str {
        match self {
            BackendType::Fabric => "https://meta.fabricmc.net/v2",
            BackendType::Quilt => "https://meta.quiltmc.org/v3",
            BackendType::LegacyFabric => "https://meta.legacyfabric.net/v2",
            BackendType::OrnitheMCFabric | BackendType::OrnitheMCQuilt => {
                unreachable!("OrnitheMC uses a different meta API and can't be used like this")
            }
            BackendType::Babric => "https://meta.babric.glass-launcher.net/v2",
            BackendType::CursedLegacy => {
                unreachable!("cursed legacy fabric uses a custom git commit system, not meta API")
            }
        }
    }

    #[must_use]
    pub fn is_quilt(self) -> bool {
        matches!(self, BackendType::Quilt | BackendType::OrnitheMCQuilt)
    }
}

impl Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BackendType::Fabric => "Fabric",
                BackendType::Quilt => "Quilt",
                BackendType::LegacyFabric => "Fabric (Legacy)",
                BackendType::OrnitheMCFabric => "Fabric (OrnitheMC)",
                BackendType::OrnitheMCQuilt => "Quilt (OrnitheMC)",
                BackendType::CursedLegacy => "Fabric (Cursed Legacy)",
                BackendType::Babric => "Fabric (Babric)",
            }
        )
    }
}

type List = Vec<FabricVersionListItem>;

#[derive(Debug, Clone)]
pub enum FabricVersionList {
    Beta173 {
        ornithe_mc: List,
        babric: List,
        cursed_legacy: List,
    },
    Quilt(List),
    Fabric(List),

    LegacyFabric(List),
    OrnitheMCQuilt(List),
    OrnitheMCFabric(List),
    Both {
        legacy_fabric: List,
        ornithe_mc: List,
    },
    Unsupported,
}

impl FabricVersionList {
    #[must_use]
    pub fn just_get_one(self) -> (List, BackendType) {
        match self {
            FabricVersionList::Quilt(l) => (l, BackendType::Quilt),
            FabricVersionList::Fabric(l) => (l, BackendType::Fabric),
            FabricVersionList::LegacyFabric(l) => (l, BackendType::LegacyFabric),
            FabricVersionList::OrnitheMCFabric(l) => (l, BackendType::OrnitheMCFabric),
            FabricVersionList::OrnitheMCQuilt(l) => (l, BackendType::OrnitheMCQuilt),

            // Opinionated, feel free to tell me
            // if there's a better choice
            #[allow(unused)]
            FabricVersionList::Beta173 {
                ornithe_mc,
                babric,
                cursed_legacy,
            } => (babric, BackendType::Babric),
            #[allow(unused)]
            FabricVersionList::Both {
                legacy_fabric,
                ornithe_mc,
            } => (legacy_fabric, BackendType::LegacyFabric),
            FabricVersionList::Unsupported => (Vec::new(), BackendType::Fabric),
        }
    }

    #[must_use]
    pub fn get_specific(self, backend: BackendType) -> Option<List> {
        match (self, backend) {
            (FabricVersionList::Beta173 { ornithe_mc, .. }, BackendType::OrnitheMCFabric)
            | (FabricVersionList::Both { ornithe_mc, .. }, BackendType::OrnitheMCFabric) => {
                Some(ornithe_mc)
            }

            (FabricVersionList::Beta173 { cursed_legacy, .. }, BackendType::CursedLegacy) => {
                Some(cursed_legacy)
            }

            (FabricVersionList::Beta173 { babric, .. }, BackendType::Babric) => Some(babric),

            (FabricVersionList::Both { legacy_fabric, .. }, BackendType::LegacyFabric) => {
                Some(legacy_fabric)
            }

            (FabricVersionList::Fabric(l), BackendType::Fabric)
            | (FabricVersionList::LegacyFabric(l), BackendType::LegacyFabric)
            | (FabricVersionList::OrnitheMCFabric(l), BackendType::OrnitheMCFabric)
            | (FabricVersionList::Quilt(l), BackendType::Quilt)
            | (FabricVersionList::OrnitheMCQuilt(l), BackendType::OrnitheMCQuilt) => Some(l),

            _ => None,
        }
    }

    #[must_use]
    pub fn is_unsupported(&self) -> bool {
        match self {
            FabricVersionList::Quilt(l)
            | FabricVersionList::Fabric(l)
            | FabricVersionList::LegacyFabric(l)
            | FabricVersionList::OrnitheMCQuilt(l)
            | FabricVersionList::OrnitheMCFabric(l) => l.is_empty(),

            FabricVersionList::Beta173 {
                ornithe_mc,
                babric,
                cursed_legacy,
            } => ornithe_mc.is_empty() && babric.is_empty() && cursed_legacy.is_empty(),
            FabricVersionList::Both {
                legacy_fabric,
                ornithe_mc,
            } => legacy_fabric.is_empty() && ornithe_mc.is_empty(),
            FabricVersionList::Unsupported => true,
        }
    }
}

pub async fn get_list_of_versions(
    instance: InstanceSelection,
    is_quilt: bool,
) -> Result<FabricVersionList, FabricInstallError> {
    info!("Loading fabric version list...");
    let is_server = instance.is_server();
    let version_json = VersionDetails::load(&instance).await?;

    let mut result = get_list_of_versions_inner(&version_json, is_quilt, is_server).await;
    if result.is_err() {
        for _ in 0..5 {
            result = get_list_of_versions_inner(&version_json, is_quilt, is_server).await;
            match &result {
                Ok(_) => break,
                Err(JsonDownloadError::RequestError(RequestError::DownloadError {
                    code, ..
                })) if code.as_u16() == 404 => {
                    // Unsupported version
                    pt!("Unsupported fabric version? (404)");
                    return Ok(if is_quilt {
                        FabricVersionList::Quilt(Vec::new())
                    } else {
                        FabricVersionList::Fabric(Vec::new())
                    });
                }
                Err(_) => {}
            }
        }
    }

    result.map_err(FabricInstallError::from)
}

pub async fn get_list_of_versions_from_backend(
    version: &str,
    backend: BackendType,
    is_server: bool,
) -> Result<List, JsonDownloadError> {
    let versions: List = if let BackendType::CursedLegacy = backend {
        vec![FabricVersionListItem {
            loader: FabricVersion {
                version: "b1.7.3".to_owned(),
            },
        }]
    } else {
        let list = if let BackendType::OrnitheMCFabric | BackendType::OrnitheMCQuilt = backend {
            let name = if backend.is_quilt() {
                "quilt"
            } else {
                "fabric"
            };
            let url1 = format!("https://meta.ornithemc.net/v3/versions/{name}-loader/{version}");
            let url2 = format!(
                "https://meta.ornithemc.net/v3/versions/{name}-loader/{version}-{}",
                if is_server { "server" } else { "client" }
            );

            let list = file_utils::download_file_to_json::<List>(&url1, false).await?;
            if list.is_empty() {
                if let Ok(new_list) = file_utils::download_file_to_json::<List>(&url2, false).await
                {
                    new_list
                } else {
                    list
                }
            } else {
                list
            }
        } else {
            let list =
                download_file_to_string(&format!("/versions/loader/{version}"), backend).await?;
            serde_json::from_str(&list).json(list)?
        };

        list
    };
    Ok(versions)
}

async fn get_list_of_versions_inner(
    version_json: &VersionDetails,
    is_quilt: bool,
    is_server: bool,
) -> Result<FabricVersionList, JsonDownloadError> {
    let version = version_json.get_id();
    if is_quilt {
        let (versions, should_try_ornithe) = if version_json
            .is_after_or_eq(V_OFFICIAL_FABRIC_SUPPORT)
        {
            match get_list_of_versions_from_backend(version, BackendType::Quilt, is_server).await {
                // If the list is empty or an error 404
                // then try OrnitheMC backend, otherwise
                // stick to official Quilt backend
                Ok(n) => {
                    let is_empty = n.is_empty();
                    (n, is_empty)
                }
                Err(JsonDownloadError::RequestError(RequestError::DownloadError {
                    code, ..
                })) if code.as_u16() == 404 => (Vec::new(), true),
                Err(err) => Err(err)?,
            }
        } else {
            (Vec::new(), true)
        };

        return Ok(if should_try_ornithe {
            let versions =
                get_list_of_versions_from_backend(version, BackendType::OrnitheMCQuilt, is_server)
                    .await?;
            if versions.is_empty() {
                FabricVersionList::Unsupported
            } else {
                FabricVersionList::OrnitheMCQuilt(versions)
            }
        } else {
            FabricVersionList::Quilt(versions)
        });
    }

    let official_versions =
        get_list_of_versions_from_backend(version, BackendType::Fabric, is_server).await?;
    if !official_versions.is_empty() {
        return Ok(FabricVersionList::Fabric(official_versions));
    }

    if version == "b1.7.3" {
        let (ornithe_mc, cursed_legacy, babric) = tokio::try_join!(
            get_list_of_versions_from_backend(version, BackendType::OrnitheMCFabric, is_server),
            get_list_of_versions_from_backend(version, BackendType::CursedLegacy, is_server),
            get_list_of_versions_from_backend(version, BackendType::Babric, is_server),
        )?;

        return Ok(FabricVersionList::Beta173 {
            ornithe_mc,
            babric,
            cursed_legacy,
        });
    }

    let (legacy_fabric, ornithe_mc) = tokio::try_join!(
        get_list_of_versions_from_backend(version, BackendType::LegacyFabric, is_server),
        get_list_of_versions_from_backend(version, BackendType::OrnitheMCFabric, is_server)
    )?;

    Ok(match (legacy_fabric.is_empty(), ornithe_mc.is_empty()) {
        (true, true) => FabricVersionList::Unsupported,
        (true, false) => FabricVersionList::OrnitheMCFabric(ornithe_mc),
        (false, true) => FabricVersionList::LegacyFabric(legacy_fabric),
        (false, false) => FabricVersionList::Both {
            legacy_fabric,
            ornithe_mc,
        },
    })
}

pub async fn get_latest_cursed_legacy_commit() -> Result<String, FabricInstallError> {
    #[derive(Deserialize)]
    struct GithubCommit {
        sha: String,
    }

    fn first_seven_chars(input: &str) -> &str {
        let len = input.chars().count();
        let slice_end = if len < 7 { len } else { 7 };
        &input[0..slice_end]
    }

    let version: Vec<GithubCommit> = file_utils::download_file_to_json(
        "https://api.github.com/repos/minecraft-cursed-legacy/Cursed-fabric-loader/commits",
        true,
    )
    .await?;

    Ok(version.first().map_or("5e8a1e8".to_owned(), |n| {
        first_seven_chars(&n.sha).to_owned()
    }))
}
