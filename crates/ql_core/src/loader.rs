use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::err;

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Loader {
    #[serde(rename = "Vanilla")]
    #[default]
    Vanilla,
    #[serde(rename = "Fabric")]
    Fabric,
    #[serde(rename = "Quilt")]
    Quilt,
    #[serde(rename = "Forge")]
    Forge,
    #[serde(rename = "NeoForge")]
    Neoforge,

    // The launcher supports these, but modrinth doesn't
    // (so no Mod Store):
    #[serde(rename = "OptiFine")]
    OptiFine,
    #[serde(rename = "Paper")]
    Paper,

    // The launcher doesn't currently support these:
    #[serde(rename = "LiteLoader")]
    Liteloader,
    #[serde(rename = "ModLoader")]
    Modloader,
    #[serde(rename = "Rift")]
    Rift,
}

impl Display for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = serde_json::to_string(self)
            .ok()
            .and_then(|n| n.strip_prefix("\"").map(str::to_owned))
            .and_then(|n| n.strip_suffix("\"").map(str::to_owned))
        {
            write!(f, "{s}")
        } else {
            write!(f, "{self:?}")
        }
    }
}

impl Loader {
    pub const ALL: &[Self] = &[
        Self::Vanilla,
        Self::Fabric,
        Self::Quilt,
        Self::Forge,
        Self::Neoforge,
        Self::OptiFine,
        Self::Paper,
        Self::Liteloader,
        Self::Modloader,
        Self::Rift,
    ];

    #[must_use]
    pub fn not_vanilla(self) -> Option<Self> {
        (!self.is_vanilla()).then_some(self)
    }

    #[must_use]
    pub fn is_vanilla(self) -> bool {
        matches!(self, Loader::Vanilla)
    }

    #[must_use]
    pub fn to_modrinth_str(self) -> &'static str {
        match self {
            Loader::Forge => "forge",
            Loader::Fabric => "fabric",
            Loader::Quilt => "quilt",
            Loader::Liteloader => "liteloader",
            Loader::Modloader => "modloader",
            Loader::Rift => "rift",
            Loader::Neoforge => "neoforge",
            Loader::OptiFine => "optifine",
            Loader::Paper => "paper",
            Loader::Vanilla => " ",
        }
    }

    #[must_use]
    pub fn to_curseforge_num(&self) -> &'static str {
        match self {
            Loader::Forge => "1",
            Loader::Fabric => "4",
            Loader::Quilt => "5",
            Loader::Neoforge => "6",
            Loader::Liteloader => "3",
            Loader::Rift
            | Loader::Paper
            | Loader::Modloader
            | Loader::OptiFine
            | Loader::Vanilla => {
                err!("Unsupported loader for curseforge: {self:?}");
                "0"
            } // Not supported
        }
    }
}
