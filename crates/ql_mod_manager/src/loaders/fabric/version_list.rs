use std::fmt::Display;

use ql_core::{
    json::VersionDetails, InstanceSelection, IntoJsonError, JsonDownloadError, RequestError,
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
    OrnitheMC,
    CursedLegacy,
    Babric,
}

impl BackendType {
    pub fn get_url(self) -> &'static str {
        match self {
            BackendType::Fabric => "https://meta.fabricmc.net/v2",
            BackendType::Quilt => "https://meta.quiltmc.org/v3",
            BackendType::LegacyFabric => "https://meta.legacyfabric.net/v2",
            BackendType::OrnitheMC => "https://meta.ornithemc.net/v2",
            BackendType::Babric => "https://meta.babric.glass-launcher.net/v2",
            BackendType::CursedLegacy => todo!(),
        }
    }

    pub fn is_quilt(self) -> bool {
        matches!(self, BackendType::Quilt)
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
                BackendType::OrnitheMC => "Fabric (OrnitheMC)",
                BackendType::CursedLegacy => "Fabric (Cursed Legacy)",
                BackendType::Babric => "Fabric (Babric)",
            }
        )
    }
}

type List = Vec<FabricVersionListItem>;

#[derive(Debug, Clone)]
pub enum VersionList {
    Beta173 {
        ornithe_mc: List,
        babric: List,
        cursed_legacy: List,
    },
    Quilt(List),
    Fabric(List),

    LegacyFabric(List),
    OrnitheMC(List),
    Both {
        legacy_fabric: List,
        ornithe_mc: List,
    },
    Unsupported,
}

impl VersionList {
    pub fn just_get_one(self) -> (List, BackendType) {
        match self {
            VersionList::Quilt(l) => (l, BackendType::Quilt),
            VersionList::Fabric(l) => (l, BackendType::Fabric),
            VersionList::LegacyFabric(l) => (l, BackendType::LegacyFabric),
            VersionList::OrnitheMC(l) => (l, BackendType::OrnitheMC),

            // Opinionated, feel free to tell me
            // if there's a better choice
            #[allow(unused)]
            VersionList::Beta173 {
                ornithe_mc,
                babric,
                cursed_legacy,
            } => (babric, BackendType::Babric),
            #[allow(unused)]
            VersionList::Both {
                legacy_fabric,
                ornithe_mc,
            } => (legacy_fabric, BackendType::LegacyFabric),
            VersionList::Unsupported => (Vec::new(), BackendType::Fabric),
        }
    }

    pub fn get_choices(&self) -> &'static [BackendType] {
        match self {
            VersionList::Quilt(_) => &[BackendType::Quilt],
            VersionList::Fabric(_) => &[BackendType::Fabric],
            VersionList::LegacyFabric(_) => &[BackendType::LegacyFabric],
            VersionList::OrnitheMC(_) => &[BackendType::OrnitheMC],
            VersionList::Unsupported => &[BackendType::Fabric],

            #[allow(unused)]
            VersionList::Beta173 {
                ornithe_mc,
                babric,
                cursed_legacy,
            } => &[
                BackendType::OrnitheMC,
                BackendType::Babric,
                BackendType::CursedLegacy,
            ],
            #[allow(unused)]
            VersionList::Both {
                legacy_fabric,
                ornithe_mc,
            } => &[BackendType::LegacyFabric, BackendType::OrnitheMC],
        }
    }

    pub fn get_specific(self, backend: BackendType) -> Option<Vec<FabricVersionListItem>> {
        match (self, backend) {
            (VersionList::Beta173 { ornithe_mc, .. }, BackendType::OrnitheMC) => Some(ornithe_mc),
            (VersionList::Beta173 { cursed_legacy, .. }, BackendType::CursedLegacy) => {
                Some(cursed_legacy)
            }
            (VersionList::Beta173 { babric, .. }, BackendType::Babric) => Some(babric),
            (VersionList::Both { legacy_fabric, .. }, BackendType::LegacyFabric) => {
                Some(legacy_fabric)
            }
            (VersionList::Both { ornithe_mc, .. }, BackendType::OrnitheMC) => Some(ornithe_mc),

            (VersionList::Fabric(l), BackendType::Fabric)
            | (VersionList::LegacyFabric(l), BackendType::LegacyFabric)
            | (VersionList::OrnitheMC(l), BackendType::OrnitheMC)
            | (VersionList::Quilt(l), BackendType::Quilt) => Some(l),

            _ => None,
        }
    }

    pub fn is_unsupported(&self) -> bool {
        match self {
            VersionList::Quilt(l)
            | VersionList::Fabric(l)
            | VersionList::LegacyFabric(l)
            | VersionList::OrnitheMC(l) => l.is_empty(),

            VersionList::Beta173 {
                ornithe_mc,
                babric,
                cursed_legacy,
            } => ornithe_mc.is_empty() && babric.is_empty() && cursed_legacy.is_empty(),
            VersionList::Both {
                legacy_fabric,
                ornithe_mc,
            } => legacy_fabric.is_empty() && ornithe_mc.is_empty(),
            VersionList::Unsupported => true,
        }
    }
}

pub async fn get_list_of_versions(
    instance: InstanceSelection,
    is_quilt: bool,
) -> Result<VersionList, FabricInstallError> {
    async fn try_backend(version: &str, backend: BackendType) -> Result<List, JsonDownloadError> {
        let version_list =
            download_file_to_string(&format!("/versions/loader/{version}"), backend).await?;
        let versions: List = serde_json::from_str(&version_list).json(version_list)?;
        Ok(versions)
    }

    async fn inner(version: &str, is_quilt: bool) -> Result<VersionList, JsonDownloadError> {
        if is_quilt {
            let versions = try_backend(version, BackendType::Quilt).await?;
            return Ok(VersionList::Quilt(versions));
        }

        let official_versions = try_backend(version, BackendType::Fabric).await?;
        if !official_versions.is_empty() {
            return Ok(VersionList::Fabric(official_versions));
        }

        if version == "b1.7.3" {
            let (ornithe_mc, cursed_legacy, babric) = tokio::try_join!(
                try_backend(version, BackendType::OrnitheMC),
                try_backend(version, BackendType::CursedLegacy),
                try_backend(version, BackendType::Babric),
            )?;

            return Ok(VersionList::Beta173 {
                ornithe_mc,
                babric,
                cursed_legacy,
            });
        }

        let (legacy_fabric, ornithe_mc) = tokio::try_join!(
            try_backend(version, BackendType::LegacyFabric),
            try_backend(version, BackendType::OrnitheMC)
        )?;

        return Ok(match (legacy_fabric.is_empty(), ornithe_mc.is_empty()) {
            (true, true) => VersionList::Unsupported,
            (true, false) => VersionList::OrnitheMC(ornithe_mc),
            (false, true) => VersionList::LegacyFabric(legacy_fabric),
            (false, false) => VersionList::Both {
                legacy_fabric,
                ornithe_mc,
            },
        });
    }

    let version_json = VersionDetails::load(&instance).await?;
    let version = version_json.get_id();

    let mut result = inner(version, is_quilt).await;
    if result.is_err() {
        for _ in 0..5 {
            result = inner(version, is_quilt).await;
            match &result {
                Ok(_) => break,
                Err(JsonDownloadError::RequestError(RequestError::DownloadError {
                    code, ..
                })) if code.as_u16() == 404 => {
                    // Unsupported version
                    return Ok(if is_quilt {
                        VersionList::Quilt(Vec::new())
                    } else {
                        VersionList::Fabric(Vec::new())
                    });
                }
                Err(_) => {}
            }
        }
    }

    result.map_err(FabricInstallError::from)
}
